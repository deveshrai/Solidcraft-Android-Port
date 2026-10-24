package io.solidcraft.android;

import android.app.NativeActivity;
import android.content.ContentResolver;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.window.OnBackInvokedDispatcher;
import android.provider.DocumentsContract;
import android.provider.OpenableColumns;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * The app is Rust (libsolidcraft_android.so, run by NativeActivity); this adds only what needs
 * Java: the system's document pickers (Storage Access Framework) and copying between their
 * content:// documents and files the engine reads and writes (apps/solidcraft-android/src/documents.rs).
 *
 * Rust calls pickOpen/pickSave/copyTo and polls takeResult for what came of them; each result is
 * {kind, ...}: {"picked", uri, name}, {"cancelled"}, {"opened", path}, {"copied", path},
 * {"error", message}, {"back"}.
 *
 * Back (the gesture or the key) never finishes the activity: it is handed to Rust as {"back"},
 * which closes what is open (as Escape) or asks for toBackground(). Finishing the activity would
 * end the app's one event loop, which can't start again in the same process.
 */
public class SolidCraftActivity extends NativeActivity {
    private static final int OPEN = 1;
    private static final int SAVE = 2;

    /** Copies run here, one at a time and in order, off the UI thread. */
    private final ExecutorService io = Executors.newSingleThreadExecutor();
    private final ConcurrentLinkedQueue<String[]> results = new ConcurrentLinkedQueue<>();

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        if (Build.VERSION.SDK_INT >= 33) {
            getOnBackInvokedDispatcher().registerOnBackInvokedCallback(
                    OnBackInvokedDispatcher.PRIORITY_DEFAULT, () -> results.add(new String[] {"back"}));
        }
    }

    /** Back before Android 13 (from 13 on, the callback registered in onCreate takes it). */
    @Override
    @SuppressWarnings("deprecation")
    public void onBackPressed() {
        results.add(new String[] {"back"});
    }

    /** Leave the app as Home does, keeping it and its designs as they are. */
    public void toBackground() {
        runOnUiThread(() -> moveTaskToBack(true));
    }
    /** The name the last save asked for. */
    private volatile String saveName;

    /** Ask for a document to open (read and, where the provider allows it, write back). */
    public void pickOpen() {
        runOnUiThread(() -> {
            Intent i = new Intent(Intent.ACTION_OPEN_DOCUMENT);
            i.addCategory(Intent.CATEGORY_OPENABLE);
            i.setType("*/*");
            i.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION
                    | Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION);
            start(i, OPEN);
        });
    }

    /** Ask where to save a new document, suggesting `name`. */
    public void pickSave(String name, String mime) {
        saveName = name;
        runOnUiThread(() -> {
            Intent i = new Intent(Intent.ACTION_CREATE_DOCUMENT);
            i.addCategory(Intent.CATEGORY_OPENABLE);
            i.setType(mime);
            i.putExtra(Intent.EXTRA_TITLE, name);
            start(i, SAVE);
        });
    }

    private void start(Intent i, int request) {
        try {
            startActivityForResult(i, request);
        } catch (RuntimeException e) {
            results.add(new String[] {"error", "No document picker: " + e.getMessage()});
            results.add(new String[] {"cancelled"});
        }
    }

    @Override
    protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        Uri uri = data == null ? null : data.getData();
        if (result != RESULT_OK || uri == null) {
            results.add(new String[] {"cancelled"});
            return;
        }
        // Keep access across restarts, so a design opened again from the recent list still saves
        // back to where it came from.
        int flags = Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION;
        try {
            getContentResolver().takePersistableUriPermission(uri, flags);
        } catch (SecurityException e) {
            try {
                getContentResolver().takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION);
            } catch (SecurityException ignored) {
                // Access lasts until the app ends.
            }
        }
        if (request == SAVE) {
            uri = keepExtension(uri, saveName);
        }
        results.add(new String[] {"picked", uri.toString(), displayName(uri)});
    }

    /**
     * A provider resolving a name clash may number the name after its extension ("part.step (1)"),
     * which other apps no longer know as a STEP file: rename it "part (1).step" where the provider
     * can rename. Returns the document's URI (a rename may change it).
     */
    private Uri keepExtension(Uri uri, String asked) {
        String got = displayName(uri);
        int dot = asked == null ? -1 : asked.lastIndexOf('.');
        if (dot <= 0 || got.equals(asked) || !got.startsWith(asked)) {
            return uri;
        }
        Matcher m = Pattern.compile(" \\(\\d+\\)").matcher(got.substring(asked.length()));
        if (!m.matches()) {
            return uri;
        }
        String fixed = asked.substring(0, dot) + m.group() + asked.substring(dot);
        try {
            Uri renamed = DocumentsContract.renameDocument(getContentResolver(), uri, fixed);
            return renamed == null ? uri : renamed;
        } catch (Exception e) {
            // The provider can't rename: the document keeps the name it was given.
            return uri;
        }
    }

    private String displayName(Uri uri) {
        try (Cursor c = getContentResolver().query(uri, new String[] {OpenableColumns.DISPLAY_NAME}, null, null, null)) {
            if (c != null && c.moveToFirst() && !c.isNull(0)) {
                return c.getString(0);
            }
        } catch (RuntimeException ignored) {
            // Fall back to the URI's last segment.
        }
        String last = uri.getLastPathSegment();
        return last == null ? "document" : last;
    }

    /** Copy a document into `path` (opening it), or `path` into a document (saving it). */
    public void copy(String uri, String path, boolean toDocument) {
        io.execute(() -> {
            ContentResolver cr = getContentResolver();
            Uri u = Uri.parse(uri);
            try {
                if (toDocument) {
                    try (InputStream in = new FileInputStream(path); OutputStream out = cr.openOutputStream(u, "wt")) {
                        pipe(in, out);
                    }
                    results.add(new String[] {"copied", path});
                } else {
                    File f = new File(path);
                    File tmp = new File(f.getParentFile(), "." + f.getName() + ".part");
                    try (InputStream in = cr.openInputStream(u); OutputStream out = new FileOutputStream(tmp)) {
                        pipe(in, out);
                    }
                    if (!tmp.renameTo(f)) {
                        throw new IOException("could not place " + f.getName());
                    }
                    results.add(new String[] {"opened", path});
                }
            } catch (IOException | RuntimeException e) {
                results.add(new String[] {"error", (toDocument ? "Saving " : "Opening ") + new File(path).getName() + ": " + e.getMessage(), path});
            }
        });
    }

    private static void pipe(InputStream in, OutputStream out) throws IOException {
        if (in == null || out == null) {
            throw new IOException("the document could not be opened");
        }
        byte[] buf = new byte[1 << 16];
        for (int n; (n = in.read(buf)) > 0; ) {
            out.write(buf, 0, n);
        }
    }

    /** The oldest result not yet taken, or null. */
    public String[] takeResult() {
        return results.poll();
    }
}
