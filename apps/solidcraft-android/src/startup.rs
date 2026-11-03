//! Getting the window up on a device whose graphics don't start (port issue #4): a launch that
//! never draws a frame leaves a marker, and the next launch starts on OpenGL ES instead of the
//! default (Vulkan where the device has one). A fallback that draws is kept; one that doesn't
//! goes back to the default. As the desktop app's `startup` module, for one activity.
//!
//! Plain files in the app's own folder, read and written here, built and tested on every system.

use std::path::Path;

/// Written before the graphics start, removed once a frame is on screen: holds the backend tried.
const MARKER: &str = "starting";
/// The backend a fallback found working, kept for the launches after.
const KEPT: &str = "graphics-backend";
/// What a launch that could not start says, where a computer reaches it over USB.
pub const ERROR_FILE: &str = "startup-error.txt";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    /// wgpu's choice: Vulkan where the device has it, GLES otherwise.
    Default,
    /// OpenGL ES only.
    Gles,
}

impl Backend {
    fn name(self) -> &'static str {
        match self {
            Backend::Default => "default",
            Backend::Gles => "gles",
        }
    }
}

fn read(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// The backend this launch starts on, from what the launches before left in `dir`; the marker
/// is written. The bool is whether this is a fallback from a launch that never drew.
pub fn begin(dir: &Path) -> (Backend, bool) {
    let kept = if read(&dir.join(KEPT)).as_deref() == Some("gles") { Backend::Gles } else { Backend::Default };
    let (backend, fallback) = match read(&dir.join(MARKER)).as_deref() {
        // The last launch never drew on the default: try GLES.
        Some("default") => (Backend::Gles, true),
        // Nor on GLES: back to the default, and forget GLES.
        Some("gles") => {
            let _ = std::fs::remove_file(dir.join(KEPT));
            (Backend::Default, false)
        }
        _ => (kept, false),
    };
    let _ = std::fs::write(dir.join(MARKER), backend.name());
    (backend, fallback)
}

/// A frame is on screen: the marker goes, and GLES is kept when it was a fallback that worked.
pub fn drew(dir: &Path, backend: Backend, fallback: bool) {
    let _ = std::fs::remove_file(dir.join(MARKER));
    if backend == Backend::Gles && fallback {
        let _ = std::fs::write(dir.join(KEPT), backend.name());
    }
}

/// The text [`ERROR_FILE`] holds for a launch that could not start.
pub fn error_text(version: &str, backend: Backend, error: &str) -> String {
    let next = match backend {
        Backend::Default => "The next launch tries OpenGL ES instead.",
        Backend::Gles => "OpenGL ES did not start either; the next launch tries the default again.",
    };
    format!(
        "SolidCraft {version} for Android could not start.\n\nGraphics: {}\nError: {error}\n\n{next}\n\
         Please attach this file to an issue: https://github.com/deveshrai/Solidcraft-Android-Port/issues\n",
        backend.name()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("sc-android-startup-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::create_dir_all(&d);
        d
    }

    #[test]
    fn a_launch_that_never_draws_makes_the_next_one_try_gles_and_a_working_fallback_is_kept() {
        let d = dir("fallback");
        assert_eq!(begin(&d), (Backend::Default, false), "a first launch starts on the default");
        // It never drew (the process ended): the next launch falls back.
        let (b, fallback) = begin(&d);
        assert_eq!((b, fallback), (Backend::Gles, true));
        drew(&d, b, fallback);
        assert_eq!(begin(&d), (Backend::Gles, false), "GLES worked, so it stays");
        drew(&d, Backend::Gles, false);
        assert_eq!(begin(&d), (Backend::Gles, false));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn when_gles_never_draws_either_the_default_comes_back() {
        let d = dir("both");
        let _ = begin(&d);
        assert_eq!(begin(&d), (Backend::Gles, true));
        assert_eq!(begin(&d), (Backend::Default, false), "GLES failed too: back to the default");
        assert_eq!(begin(&d), (Backend::Gles, true), "and then GLES once more");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_launch_that_draws_leaves_nothing_behind() {
        let d = dir("ok");
        let (b, f) = begin(&d);
        drew(&d, b, f);
        assert!(!d.join(MARKER).exists());
        assert!(!d.join(KEPT).exists(), "the default needs nothing kept");
        assert_eq!(begin(&d), (Backend::Default, false));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_error_file_says_what_failed_and_what_happens_next() {
        let t = error_text("0.30.1a", Backend::Default, "no suitable adapter");
        assert!(t.contains("0.30.1a") && t.contains("Graphics: default") && t.contains("no suitable adapter"));
        assert!(t.contains("tries OpenGL ES"));
        assert!(error_text("0.30.1a", Backend::Gles, "x").contains("tries the default again"));
    }
}
