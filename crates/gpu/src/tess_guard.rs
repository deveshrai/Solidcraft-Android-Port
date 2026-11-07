//! SolidCraft (Android port): building bodies on the CPU where the driver dies compiling the GPU
//! evaluation. A shader compiler that crashes (Adreno 840's on upstream 0.40's tessellation
//! shader, a null read inside `vkCreateComputePipelines`) takes the process with it, so the
//! fallback in [`crate::view::tessellation`] never sees an error. A marker file is written
//! before the compile and removed after it: one left behind means a launch died there, and every
//! later launch builds bodies on the CPU. The GPUs known to crash are skipped from the start.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The marker's file name, in the folder the app gives [`remember_in`].
pub const MARKER: &str = "gpu-tessellation-compiling";

static FILE: OnceLock<PathBuf> = OnceLock::new();

/// Keep the marker in `dir` (the app's own data folder). Without it nothing is written, and
/// only the known GPUs are skipped.
pub fn remember_in(dir: &Path) {
    let _ = FILE.set(dir.join(MARKER));
}

/// GPUs whose shader compiler crashes on the evaluation shader: Adreno on Android (Vulkan).
pub fn crashes_compiling(backend: wgpu::Backend, name: &str) -> bool {
    cfg!(target_os = "android") && known_crash(backend, name)
}

fn known_crash(backend: wgpu::Backend, name: &str) -> bool {
    backend == wgpu::Backend::Vulkan && name.contains("Adreno")
}

/// Whether a launch before this one died compiling the evaluation (its marker is still there).
pub fn died_compiling() -> bool {
    FILE.get().is_some_and(|f| f.exists())
}

/// Run the compile `f` between writing the marker and removing it.
pub fn compiling<T>(f: impl FnOnce() -> T) -> T {
    let file = FILE.get();
    if let Some(file) = file {
        let _ = std::fs::write(file, b"");
    }
    let out = f();
    if let Some(file) = file {
        let _ = std::fs::remove_file(file);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tess_guard_adreno_on_vulkan_is_a_known_crash_and_others_are_not() {
        assert!(known_crash(wgpu::Backend::Vulkan, "Adreno (TM) 840"));
        assert!(!known_crash(wgpu::Backend::Gl, "Adreno (TM) 840"));
        assert!(!known_crash(wgpu::Backend::Vulkan, "Mali-G715"));
        assert!(!crashes_compiling(wgpu::Backend::Metal, "Apple M2"));
    }

    #[test]
    fn tess_guard_a_marker_left_behind_means_a_launch_died_compiling() {
        let d = std::env::temp_dir().join(format!("sc-tess-guard-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        remember_in(&d);
        assert!(!died_compiling());
        // A compile that returns removes its marker.
        assert_eq!(compiling(|| died_compiling()), true, "the marker is there while it compiles");
        assert!(!died_compiling());
        // One that never returned (the process died) left it.
        let _ = std::fs::write(d.join(MARKER), b"");
        assert!(died_compiling());
        let _ = std::fs::remove_dir_all(&d);
    }
}
