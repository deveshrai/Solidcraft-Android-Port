//! Opening and saving through Android's document pickers (the Storage Access Framework).
//!
//! The engine reads and writes paths; Android hands out `content://` documents. Each document the
//! app opens or saves is kept as a local copy in the app's `documents` folder, linked to its
//! document: opening copies the document in, and every finished write to the copy (a save, an
//! export, the next Ctrl+S) is copied back out. The links are kept in `documents.json`, so a
//! design opened again from the recent list still saves to where it came from.
//!
//! The pickers and the copies are Java (`SolidCraftActivity`); this asks for them through JNI and
//! takes their results each frame.

use std::cell::RefCell;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant, SystemTime};

use jni::objects::{JObject, JObjectArray, JString};
use jni::refs::Global;
use jni::{Env, JValue, JavaVM, jni_sig, jni_str};
use solidcraft_ui_egui::{FilePickerFuture, SolveApp};

use crate::names::{file_name, mime};

/// How often the local copies are checked for writes to copy out.
const CHECK_EVERY: Duration = Duration::from_millis(250);
/// How long the checks go on after the last input or document activity. A write follows the
/// input that asked for it (a save running longer keeps the app drawing frames, so checking).
const WATCH_FOR: Duration = Duration::from_secs(5);

/// A local copy and the document it stands for.
struct Link {
    path: PathBuf,
    uri: String,
    /// The copy's modified time and length when it last matched the document.
    synced: Option<(SystemTime, u64)>,
    /// A new state seen once: copied out when the next check finds it unchanged (a write done).
    settling: Option<(SystemTime, u64)>,
    /// A copy to or from the document is under way.
    copying: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Purpose {
    Open,
    Save,
}

/// The picker's answer, for the future the app waits on.
type Slot = Rc<RefCell<Option<Option<String>>>>;

struct Pick {
    purpose: Purpose,
    /// Saving: the name the app asked for. The copy keeps it (the engine knows formats by their
    /// extension), whatever name the provider gives the document ("part.stl (1)").
    name: Option<String>,
    slot: Slot,
    /// Opening: the local copy being filled.
    filling: Option<PathBuf>,
}

pub struct Documents {
    activity: Global<JObject<'static>>,
    dir: PathBuf,
    links: Vec<Link>,
    pick: Option<Pick>,
    checked: Instant,
    /// The last input or document activity: the checks go on for `WATCH_FOR` after it.
    active: Instant,
    /// Errors to show in the status bar.
    messages: Vec<String>,
}

/// The activity, as a reference that lives as long as the app. The activity glue only hands it out
/// as a raw JNI pointer, and the JVM as a process-wide singleton.
#[allow(unsafe_code)]
fn activity(raw: *mut std::ffi::c_void) -> jni::errors::Result<Global<JObject<'static>>> {
    JavaVM::singleton()?.attach_current_thread(|env| -> jni::errors::Result<_> {
        let raw = raw as jni::sys::jobject;
        // SAFETY: the activity glue keeps a global reference to the activity for as long as the
        // app runs; the cast borrows it (it is never deleted through it) and we keep our own.
        let activity = unsafe { env.as_cast_raw::<JObject>(&raw)? };
        env.new_global_ref(&*activity)
    })
}

fn string<'local>(env: &mut Env<'local>, s: &str) -> jni::errors::Result<JString<'local>> {
    env.new_string(s)
}

impl Documents {
    /// `files` is the app's private folder (`internal_data_path`); `activity` the activity's raw
    /// JNI reference.
    pub fn new(files: &Path, activity_ptr: *mut std::ffi::c_void) -> Option<Documents> {
        let activity = activity(activity_ptr).map_err(|e| log::error!("documents: no activity: {e}")).ok()?;
        let dir = files.join("documents");
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log::error!("documents: {}: {e}", dir.display());
        }
        let mut docs =
            Documents { activity, dir, links: Vec::new(), pick: None, checked: Instant::now(), active: Instant::now(), messages: Vec::new() };
        docs.load_links();
        Some(docs)
    }

    fn links_file(&self) -> PathBuf {
        self.dir.join("documents.json")
    }

    fn load_links(&mut self) {
        let Ok(text) = std::fs::read_to_string(self.links_file()) else { return };
        let Ok(serde_json::Value::Array(entries)) = serde_json::from_str::<serde_json::Value>(&text) else { return };
        for e in entries {
            if let (Some(path), Some(uri)) = (e.get("path").and_then(|p| p.as_str()), e.get("uri").and_then(|u| u.as_str())) {
                let path = PathBuf::from(path);
                // Only copies inside our folder: the file is ours to keep in step.
                if path.starts_with(&self.dir) {
                    // A copy written after its last copy out (the app was stopped first) differs
                    // from what was kept, so it is copied out now.
                    let synced = e.get("synced").and_then(state_from_json);
                    self.links.push(Link { path, uri: uri.to_string(), synced, settling: None, copying: false });
                }
            }
        }
    }

    fn save_links(&self) {
        let entries: Vec<_> = self
            .links
            .iter()
            .map(|l| serde_json::json!({"path": l.path.to_string_lossy(), "uri": l.uri, "synced": l.synced.map(state_to_json)}))
            .collect();
        let text = serde_json::Value::Array(entries).to_string();
        if let Err(e) = solidcraft_engine::io::safe_write::write_atomic(&self.links_file(), text.as_bytes(), false) {
            log::error!("documents: saving links: {e}");
        }
    }

    fn call(&self, name: &'static str, f: impl FnOnce(&mut Env, &JObject) -> jni::errors::Result<()>) {
        let activity = &self.activity;
        let result = JavaVM::singleton().and_then(|vm| vm.attach_current_thread(|env| f(env, activity)));
        if let Err(e) = result {
            log::error!("documents: {name}: {e}");
        }
    }

    /// Ask for a document to open; the future answers with its local copy.
    pub fn pick_open(&mut self) -> FilePickerFuture {
        self.start(Purpose::Open, None, |env, activity| {
            env.call_method(activity, jni_str!("pickOpen"), jni_sig!("()V"), &[])?;
            Ok(())
        })
    }

    /// Ask where to save `name`; the future answers with the local copy to write.
    pub fn pick_save(&mut self, name: &str) -> FilePickerFuture {
        let name = name.to_string();
        let mime = mime(&name);
        let asked = name.clone();
        self.start(Purpose::Save, Some(asked), move |env, activity| {
            let name = string(env, &name)?;
            let mime = string(env, mime)?;
            env.call_method(
                activity,
                jni_str!("pickSave"),
                jni_sig!("(Ljava/lang/String;Ljava/lang/String;)V"),
                &[JValue::Object(&name), JValue::Object(&mime)],
            )?;
            Ok(())
        })
    }

    fn start(&mut self, purpose: Purpose, name: Option<String>, f: impl FnOnce(&mut Env, &JObject) -> jni::errors::Result<()>) -> FilePickerFuture {
        let slot: Slot = Rc::new(RefCell::new(None));
        if self.pick.is_some() {
            // One picker at a time (the app asks for one at a time too).
            *slot.borrow_mut() = Some(None);
        } else {
            self.pick = Some(Pick { purpose, name, slot: slot.clone(), filling: None });
            self.call("picker", f);
        }
        Box::pin(Answer(slot))
    }

    fn answer(&mut self, path: Option<String>) {
        if let Some(pick) = self.pick.take() {
            *pick.slot.borrow_mut() = Some(path);
        }
    }

    /// Results from Java, then the local copies' writes to copy out. Called every frame.
    pub fn tick(&mut self, app: &mut SolveApp, ctx: &egui::Context) {
        while let Some(result) = self.take_result() {
            self.on_result(&result);
            self.active = Instant::now();
        }
        if ctx.input(|i| !i.events.is_empty()) || self.pick.is_some() || self.links.iter().any(|l| l.copying || l.settling.is_some()) {
            self.active = Instant::now();
        }
        // The app draws nothing while idle, so it is woken to see a write finish.
        if !self.links.is_empty() && self.active.elapsed() < WATCH_FOR {
            ctx.request_repaint_after(CHECK_EVERY);
        }
        if self.checked.elapsed() >= CHECK_EVERY {
            self.checked = Instant::now();
            self.copy_out_writes();
        }
        for m in self.messages.drain(..) {
            app.set_status(m, true);
        }
    }

    fn take_result(&self) -> Option<Vec<String>> {
        let activity = &self.activity;
        let result = JavaVM::singleton().and_then(|vm| {
            vm.attach_current_thread(|env| -> jni::errors::Result<Option<Vec<String>>> {
                let r = env.call_method(activity, jni_str!("takeResult"), jni_sig!("()[Ljava/lang/String;"), &[])?.l()?;
                if r.is_null() {
                    return Ok(None);
                }
                let array = env.cast_local::<JObjectArray<JString>>(r)?;
                let mut out = Vec::new();
                for i in 0..array.len(env)? {
                    let s = array.get_element(env, i)?;
                    out.push(if s.is_null() { String::new() } else { s.try_to_string(env)? });
                }
                Ok(Some(out))
            })
        });
        result.map_err(|e| log::error!("documents: takeResult: {e}")).ok().flatten()
    }

    fn on_result(&mut self, r: &[String]) {
        let field = |i: usize| r.get(i).map(String::as_str).unwrap_or_default();
        match field(0) {
            "picked" => self.picked(field(1), field(2)),
            "cancelled" => self.answer(None),
            "opened" => {
                let path = PathBuf::from(field(1));
                if let Some(l) = self.links.iter_mut().find(|l| l.path == path) {
                    l.copying = false;
                    l.synced = stat(&path);
                    l.settling = None;
                }
                self.save_links();
                if self.pick.as_ref().is_some_and(|p| p.filling.as_deref() == Some(path.as_path())) {
                    self.answer(Some(path.to_string_lossy().into_owned()));
                }
            }
            "copied" => {
                let path = PathBuf::from(field(1));
                if let Some(l) = self.links.iter_mut().find(|l| l.path == path) {
                    l.copying = false;
                }
            }
            "error" => {
                let path = PathBuf::from(field(2));
                if let Some(l) = self.links.iter_mut().find(|l| l.path == path) {
                    l.copying = false;
                }
                if self.pick.as_ref().is_some_and(|p| p.filling.as_deref() == Some(path.as_path())) {
                    self.answer(None);
                }
                self.messages.push(field(1).to_string());
            }
            other => log::warn!("documents: unknown result {other}"),
        }
    }

    /// The picker chose `uri` (named `name`): open copies it in, save links a copy to write.
    fn picked(&mut self, uri: &str, name: &str) {
        let Some((purpose, asked)) = self.pick.as_ref().map(|p| (p.purpose, p.name.clone())) else { return };
        let path = self.copy_path(uri, asked.as_deref().unwrap_or(name));
        let existing = self.links.iter().position(|l| l.uri == uri || l.path == path);
        let link = Link {
            path: path.clone(),
            uri: uri.to_string(),
            // Saving: whatever is there now is not yet the document; the app's write will be.
            synced: if purpose == Purpose::Save { stat(&path) } else { None },
            settling: None,
            copying: purpose == Purpose::Open,
        };
        match existing {
            Some(i) => self.links[i] = link,
            None => self.links.push(link),
        }
        self.save_links();
        match purpose {
            Purpose::Open => {
                if let Some(p) = self.pick.as_mut() {
                    p.filling = Some(path.clone());
                }
                let (uri, local) = (uri.to_string(), path.to_string_lossy().into_owned());
                self.call("copy in", |env, activity| copy(env, activity, &uri, &local, false));
            }
            Purpose::Save => self.answer(Some(path.to_string_lossy().into_owned())),
        }
    }

    /// The local copy for the document `uri` named `name`: its existing copy, else the name in
    /// our folder, made unique when another document's copy has it.
    fn copy_path(&self, uri: &str, name: &str) -> PathBuf {
        if let Some(l) = self.links.iter().find(|l| l.uri == uri) {
            return l.path.clone();
        }
        let name = file_name(name);
        let taken = |p: &Path| self.links.iter().any(|l| l.path == p && l.uri != uri);
        let first = self.dir.join(&name);
        if !taken(&first) {
            return first;
        }
        let (stem, ext) = match name.rsplit_once('.') {
            Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
            _ => (name.clone(), String::new()),
        };
        (2..1000).map(|n| self.dir.join(format!("{stem} ({n}){ext}"))).find(|p| !taken(p)).unwrap_or(first)
    }

    /// Copy each local copy written since it last matched its document out to it, once the write
    /// is done (the same modified time and length on two checks in a row).
    fn copy_out_writes(&mut self) {
        let mut out = Vec::new();
        for l in self.links.iter_mut().filter(|l| !l.copying) {
            let now = stat(&l.path);
            if now.is_none() || now == l.synced {
                l.settling = None;
            } else if now == l.settling {
                l.synced = now;
                l.settling = None;
                l.copying = true;
                out.push((l.uri.clone(), l.path.to_string_lossy().into_owned()));
            } else {
                l.settling = now;
            }
        }
        if !out.is_empty() {
            self.save_links();
        }
        for (uri, path) in out {
            self.call("copy out", |env, activity| copy(env, activity, &uri, &path, true));
        }
    }

    /// Copy out what is still settling now (the app is closing).
    pub fn flush(&mut self) {
        for l in &mut self.links {
            l.settling = stat(&l.path).filter(|s| Some(*s) != l.synced);
        }
        self.copy_out_writes();
    }
}

fn copy(env: &mut Env, activity: &JObject, uri: &str, path: &str, to_document: bool) -> jni::errors::Result<()> {
    let uri = string(env, uri)?;
    let path = string(env, path)?;
    env.call_method(
        activity,
        jni_str!("copy"),
        jni_sig!("(Ljava/lang/String;Ljava/lang/String;Z)V"),
        &[JValue::Object(&uri), JValue::Object(&path), JValue::Bool(to_document)],
    )?;
    Ok(())
}

fn state_to_json((modified, len): (SystemTime, u64)) -> serde_json::Value {
    let d = modified.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    serde_json::json!([d.as_secs(), d.subsec_nanos(), len])
}

fn state_from_json(v: &serde_json::Value) -> Option<(SystemTime, u64)> {
    let a = v.as_array()?;
    let (secs, nanos, len) = (a.first()?.as_u64()?, a.get(1)?.as_u64()?, a.get(2)?.as_u64()?);
    let at = SystemTime::UNIX_EPOCH.checked_add(Duration::new(secs, u32::try_from(nanos).ok()?.min(999_999_999)))?;
    Some((at, len))
}

fn stat(path: &Path) -> Option<(SystemTime, u64)> {
    let m = std::fs::metadata(path).ok().filter(|m| m.is_file())?;
    Some((m.modified().ok()?, m.len()))
}

/// The picker's answer, once it comes.
struct Answer(Slot);

impl Future for Answer {
    type Output = Option<String>;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<String>> {
        // The host's tick fills the slot; the app polls again every frame while it waits.
        match self.0.borrow_mut().take() {
            Some(answer) => Poll::Ready(answer),
            None => Poll::Pending,
        }
    }
}
