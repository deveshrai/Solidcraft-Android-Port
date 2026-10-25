# SolidCraft on Android

An Android port of SolidCraft: the same application as the desktop and browser builds
(`SolveApp` in `crates/ui-egui`), on Android phones and tablets, with mouse and keyboard,
touch, and opening, saving, importing and exporting through Android's document pickers.

**Using the app** (touch, mouse and keyboard, files, foldables): [android-touch.md](android-touch.md).

**Threads** (helical B-rep, custom threads, clearance): [android-threads.md](android-threads.md).

This port lives in [deveshrai/Solidcraft-Android-Port](https://github.com/deveshrai/Solidcraft-Android-Port)
and follows upstream [tangency-labs/solidcraft](https://github.com/tangency-labs/solidcraft);
[android-sync.md](android-sync.md) is how it is kept up to date.

- [Status](#status)
- [Building and installing](#building-and-installing)
- [How it works](#how-it-works)
- [Changes to upstream code](#changes-to-upstream-code)
- [Testing](#testing)
- [Not there yet](#not-there-yet)

## Status

Tested on an Android 16 tablet emulator (Pixel Tablet, arm64, Vulkan through MoltenVK):

| Area | What works |
|------|------------|
| Rendering | The full UI and the viewport on Vulkan (GLES 3 where a device has no Vulkan), fullscreen; the scene draws offscreen with 4x MSAA (1x on GLES) and the UI pass at 1x, as on the desktop |
| Modelling | The kernel runs on the device: sketches, features, the timeline, every workspace |
| Mouse | Hover highlighting, click to select, the navigation preset's buttons (default Fusion: middle drag pans, Shift+middle and right drag orbit), the wheel zooms to the cursor, the right-click menu |
| Keyboard | Shortcuts with and without modifiers (tested: Ctrl+Z, Ctrl+S, Ctrl+O, E, Esc, S to search), typing into fields |
| Touch | One finger orbits, a tap selects, two fingers pan, a pinch zooms about the fingers; a finger drags a dialog's arrows (Extrude distance, Fillet radius…) |
| Threads | Port-only: modelled threads as exact helical B-rep, custom pitch on any diameter, clearance, offsets, left-handed ([android-threads.md](android-threads.md)); the faceted and cosmetic ISO threads are unchanged |
| Back | Closes the open dialog, menu, tool, sketch or keyboard, as Escape; with nothing open, the app goes to the background as Home does, designs kept |
| Tested devices | Pixel Tablet emulator (Android 16); Galaxy Z Fold (SM-F971B, Android 17, Adreno 840): inner screen, landscape and portrait |
| Files | Open, Open Recent, Save, Save As, Insert STEP/Mesh/Design, Save as mesh (STL, 3MF…), Export STEP/OBJ/DXF/URDF/JSON, from and to any documents provider (device, SD card, Drive…) |

## Building and installing

### Requirements

- Rust (stable) with the Android targets, and cargo-ndk:

  ```sh
  rustup target add aarch64-linux-android x86_64-linux-android
  cargo install cargo-ndk
  ```

- The Android SDK with platform 36 and an NDK (Android Studio's SDK manager, or
  `sdkmanager "platforms;android-36" "ndk;29.0.14206865" "platform-tools"`).
- A JDK 17 or newer for Gradle; Android Studio's bundled JDK works
  (`export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"` on macOS).
- `ANDROID_HOME` pointing at the SDK (or `sdk.dir` in `apps/solidcraft-android/android/local.properties`)
  and `ANDROID_NDK_HOME` at the NDK.

### Build

```sh
cd apps/solidcraft-android/android
./gradlew assembleDebug                                   # arm64-v8a and x86_64
./gradlew assembleDebug -Psolidcraft.abis=arm64-v8a       # phones and tablets only (smaller, faster)
```

Gradle's `preBuild` runs `cargo ndk` (task `cargoNdk`), which builds `libsolidcraft_android.so`
into `app/src/main/jniLibs`; the APK is `app/build/outputs/apk/debug/SolidCraft-<version>-debug.apk`. The Rust
library is always an optimised build (the kernel is far too slow without);
`-Psolidcraft.cargoProfile=<profile>` picks another Cargo profile. The first build compiles the
whole CAD stack and takes 15–25 minutes; later builds only what changed.

Its version is SolidCraft's (from `Cargo.toml`): `SolidCraft-<version>-debug.apk` /
`-release.apk`, `versionName` the same, `versionCode` derived from it
([android-sync.md, "Releasing"](android-sync.md#releasing)). Published APKs are on the
[releases page](https://github.com/deveshrai/Solidcraft-Android-Port/releases);
`scripts/android-release.sh` makes them.

An APK with both ABIs is about 170 MB, with arm64 alone about 80 MB: Gradle strips the
library's debug information (the release profile keeps line tables for desktop crash reports).
If an incremental build reports a much larger APK, `./gradlew clean assembleDebug` (Gradle's
incremental packager reuses the old file and leaves its space unclaimed).

### Install and run

```sh
adb install -r app/build/outputs/apk/debug/SolidCraft-*-debug.apk
adb shell am start -n io.solidcraft.android/.SolidCraftActivity
adb logcat -s SolidCraft               # the app's log
```

Or copy the APK to the device and open it (allow installs from that source). The debug build is
signed with the debug key, which sideloading accepts and app stores don't.

## How it works

```text
┌──────────────────────────── io.solidcraft.android ────────────────────────────┐
│ SolidCraftActivity (Java)            extends android.app.NativeActivity       │
│   document pickers, copies between content:// documents and local files      │
│        ▲ JNI (pickOpen, pickSave, copy, takeResult)                            │
│ libsolidcraft_android.so (Rust, apps/solidcraft-android)                      │
│   android_main → eframe (winit + wgpu)  →  SolveApp  (crates/ui-egui)          │
│   documents.rs: local copies of documents, mirrored back after each write     │
│   vendor/winit: Android input as mouse, keyboard and touch                    │
└───────────────────────────────────────────────────────────────────────────────┘
```

### The host crate: `apps/solidcraft-android`

A `cdylib` the activity loads as `libsolidcraft_android.so`. All of its code is
`cfg(target_os = "android")`, so it compiles (to nothing) on the desktop and doesn't disturb the
workspace's other builds. It does what `apps/solidcraft` does for the desktop, without the
desktop-only parts (updates, the agent chat's local servers, the space mouse, the custom title
bar):

- `android_main`, the entry point the activity glue calls by name, sets up logging to logcat,
  the crash journal in the app's private folder, and starts eframe with the `AndroidApp`.
- The `Shell` (`eframe::App`) runs `SolveApp` the way the desktop app does: logic, raw input,
  preferences saved through eframe's storage (`app.ron` in the private folder), a clean exit.
- `Services` gives the app the document pickers (`documents.rs`) for open, save and attach.

Its `unsafe_code` lint is `deny` rather than the workspace's `forbid`, like the desktop app's,
for the two places with no safe API (listed in AGENTS.md): `android_main`'s `no_mangle` and
the raw JNI reference to the activity in `documents::activity`.

### The activity: `SolidCraftActivity.java`

The one Java class. It extends `NativeActivity` (which loads the library and runs it on its own
thread) only to use what needs Java:

- `pickOpen()` / `pickSave(name, mime)` start `ACTION_OPEN_DOCUMENT` / `ACTION_CREATE_DOCUMENT`;
  `onActivityResult` takes a persistable permission for the document, so it can be read and
  written after a restart.
- `copy(uri, path, toDocument)` copies a document into a local file or a local file out to a
  document, on a single background thread (in order, off the UI thread).
- `takeResult()` hands Rust the next result (`picked`, `cancelled`, `opened`, `copied`, `error`):
  Rust polls it each frame, so there are no native callbacks to register.
- When a provider resolves a name clash by numbering after the extension (`part.step (1)`, which
  no other app would know as STEP), the new document is renamed `part (1).step` where the
  provider supports renaming.

The manifest names this activity, keeps `android.app.lib_name = solidcraft_android`, uses a
fullscreen theme (Android 15+ lays apps out edge to edge, and the status bar covered the title
bar), and handles configuration changes itself, so rotating the device or attaching a keyboard
resizes the app instead of restarting it.

### Files: `documents.rs`

The engine reads and writes paths; Android hands out `content://` documents. Each document the
app opens or saves has a **local copy** in the app's private `files/documents` folder, linked to
its document:

1. **Open**: the picker returns a document; it is copied in (`copy`, to the local file) and the
   app opens the local path.
2. **Save, Save As, export**: the picker names the document; the app writes the local path,
   named as the app asked (the engine knows formats by extension).
3. **After every write**: each frame (every 250 ms while there was input or file activity in
   the last five seconds) the local copies are checked; a copy whose modified time and length
   changed, and held still for one more check (the write is done), is copied out to its
   document. So Ctrl+S on a design opened from Drive saves to Drive.

The links (local path, document URI, the state last copied) are kept in
`files/documents/documents.json`. A design reopened from the recent list still saves to where
it came from, and a write the app had no time to copy out (it was stopped) is copied out at the
next start. Only files inside the folder are ever linked; names from providers are cleaned
(`names.rs`) so they can't leave it.

### Input: the vendored winit

eframe gets its window and input from winit. winit 0.30's Android backend reported every motion
event as a touch and never reported modifier keys, so a mouse acted as a finger (no hover, no
right or middle button, no wheel) and Ctrl/Shift/Alt shortcuts never matched.
`vendor/winit` is winit 0.30.13 with a patch to `src/platform_impl/android/mod.rs` only (the
other platforms are untouched); the same patch is kept on its own in
`apps/solidcraft-android/patches/winit-android-input.patch` to re-apply to a newer winit:

- A motion event whose pointer is a mouse (tool type mouse, which includes a touchpad driving
  the pointer) becomes `CursorEntered`/`CursorMoved`/`CursorLeft`, `MouseInput` for each button
  whose state changed (left, right, middle, back, forward, from the event's button state) and
  `MouseWheel` for the scroll axes. The mouse events are marked handled, so Android doesn't turn
  the right button into Back.
- Key and motion events report `ModifiersChanged` from their meta state when Shift, Ctrl, Alt
  or Meta changed, before the event they modify.

With that, a mouse and keyboard reach egui exactly as on the desktop, and everything else (the
navigation presets, selection, menus, shortcuts) is the shared code, unchanged.

### Touch: `navigation::handle`

Every page with a camera goes through `crates/ui-egui/src/navigation.rs`, so touch is handled
there, for every workspace:

- **One finger** dragged on the model orbits (over bodies too), in place of the box selection a
  mouse drag makes. It defers to what a finger must be able to drag as a mouse does: while a
  sketch is edited or a tool is on, over a dialog's manipulator, a dimension or a constraint
  glyph.
- **A tap** selects. egui-winit ends a touch with `PointerGone` in the frame of the release, so
  the click had no hover position and picked nothing; the click now uses where the finger was
  (`interact_pos`), and the view counts as under it when it took the click.
- **Two fingers** pan with the fingers and **a pinch** zooms about the point between them (to
  the model under it, with Zoom to the cursor on). From the second finger on, the touch is that
  gesture until every finger lifts, so the finger left behind neither orbits nor selects.

A mouse is never a touch (egui counts touches from touch events only), so desktop mouse
behaviour is unchanged. On a desktop touch screen the same touch gestures apply.

## Changes to upstream code

Everything Android-only is new (`apps/solidcraft-android`, `vendor/winit`, `scripts/android-sync.sh`,
these docs). Upstream files changed, and why:

| File | Change |
|------|--------|
| `Cargo.toml` | `[patch.crates-io] winit = { path = "vendor/winit" }` |
| `Cargo.lock` | The Android crate's dependencies (`jni`, `android_logger`) and the patched winit |
| `crates/ui-egui/Cargo.toml` | `arboard` (clipboard pictures and file lists) only off Android: it has no Android backend |
| `crates/ui-egui/src/agent_chat/paste.rs`, `mod.rs` | On Android a paste in the agent chat is text only (no picture or file list) |
| `crates/ui-egui/src/fonts.rs` | Android's system fonts as fallbacks: the CJK search also looks in `/system/fonts`, and Noto Sans CJK and Noto Sans Thai are candidates for the Chinese, Hangul and Thai fallbacks |
| `crates/ui-egui/src/navigation.rs` | Touch: one finger orbits, two pan, pinch zooms; a lifted finger's click picks where it was |
| `crates/ui-egui/src/viewport.rs` | `touch_gesture` state; tests for the touch gestures and the tap |
| `crates/kernel/src/thread_brep.rs` (new), `crates/kernel/src/lib.rs` | Port-only feature: the helical B-rep thread (`helical_thread`, `HelicalThread`) |
| `crates/doc/src/document.rs`, `eval.rs`, `lib.rs` | The Thread feature's helical fields; `helical_thread_on`; `thread_face`/`thread_span` split out of the thread lookup |
| `crates/doc/src/format/pending/helical-threads.rs` (new) | The helical fields as a pending design-format change (below) |
| `crates/engine/src/cmd/features.rs` | `solid.thread`'s helical parameters |
| `crates/ui-egui/src/dialogs_thread.rs`, `dialogs.rs`, `dialogs_dense.rs` | The Thread dialog's Type, Geometry and Helical B-rep section |
| `crates/*/src/thread_tests.rs`, `dialogs_all_tests.rs` | Tests for the helical threads; the Thread dialog's first row is now Type |
| `xtask/src/layers.rs` | `solidcraft-android` registered as an app (exempt from layering, like the web app) |
| `xtask/src/assets.rs` | The asset check skips Gradle's `build/` output |
| `AGENTS.md` | The Android crate's two `unsafe` exceptions added to the list |
| `ATTRIBUTION.md`, `vendor/README.md` | The vendored winit and its patch |
| `docs/building.md` | How to build the APK |
| `.gitignore` | Local `continue-*.sh` session scripts |

The Android changes keep the desktop and browser builds as they were: the Android code paths are
behind `cfg(target_os = "android")`, and the touch changes act only on touch input. The thread
changes are a feature in the shared crates, so they are in every build made from this repository;
the faceted and cosmetic threads behave as upstream's.

**Design files.** The helical thread fields are a pending design-format change, so the port
writes one format number past upstream's release (36 while upstream's is 35) with
`"format_pending": ["helical-threads"]`. The port opens every upstream design. Upstream's
releases refuse a design saved by the port ("saved by a newer SolidCraft") rather than opening
it and quietly rebuilding its helical threads as faceted ones; designs to share with desktop
SolidCraft are best exported as STEP. Deleting that one file would make the port's designs open
upstream again, at the cost of that silent change.

## Testing

Automated (all run by `scripts/android-sync.sh`):

- `cargo test -p solidcraft-ui-egui --lib -- one_finger_drag_orbits two_fingers_pan_and_pinch a_tap_selects`:
  a finger's drag orbits and a mouse drag still box-selects; two fingers pan and zoom without
  orbiting or selecting, also after one lifts; a tap selects the face under it, with the events
  in the frames egui-winit sends them in.
- `cargo test -p solidcraft-android`: names for local copies and MIME types of saved documents.
- `cargo ndk -t arm64-v8a clippy -p solidcraft-android -- -D warnings`, desktop clippy of the
  changed crates, the wasm build of the UI crate, `cargo xtask layers` and `cargo xtask assets`.

On a device or emulator: create an AVD (`avdmanager create avd -n solidcraft -k
"system-images;android-36;google_apis;arm64-v8a" -d pixel_tablet`), start it with
`emulator -avd solidcraft -gpu host`, install, and drive it. The emulator's window only sends
touches and `adb shell input` has no right or middle button, wheel or second finger;
`apps/solidcraft-android/tools/virtual-input.py` creates a real virtual USB mouse, keyboard and
multi-touch screen through Android's `uinput` tool (usage in the file). `adb exec-out screencap -p
> shot.png` takes a screenshot (the mouse pointer included).

## Not there yet

- **On-screen keyboard**: it shows when a text field takes focus and typing works, but it covers
  the lower half of the screen instead of the app moving above it.
- **Phone layout**: the layout is the desktop one; it fits a tablet, a phone is cramped.
- **Recovery autosaves** and other per-user folders are found from `HOME`/`XDG_*`, which Android
  doesn't set; crash reports and settings are kept (in the app's folder), autosaves are not.
- **Opening from other apps** ("Open with" in a file manager): no file types are registered yet.
- **glTF with separate buffers**: only the picked `.gltf` is copied in, not its `.bin` (GLB works).
- **Accessibility**: accesskit needs GameActivity.
- **Release signing**: builds use the debug key.
