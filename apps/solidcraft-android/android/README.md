# SolidCraft on Android

Full documentation: [docs/android.md](../../../docs/android.md); keeping up with upstream and
releasing: [docs/android-sync.md](../../../docs/android-sync.md).

The same app as the desktop and the browser (`SolveApp` in `crates/ui-egui`), run by eframe on a
`NativeActivity`. The app is Rust: the activity loads `libsolidcraft_android.so`
(`apps/solidcraft-android`), and wgpu draws on Vulkan, or GLES 3 where the device has no Vulkan.
The one Java class, `SolidCraftActivity`, adds the system's document pickers.

## Requirements

- Android SDK with platform 36 and an NDK (`sdkmanager "platforms;android-36" "ndk;29.0.14206865"`);
  `ANDROID_HOME` (or `local.properties`' `sdk.dir`) points at the SDK, and `ANDROID_NDK_HOME` at the NDK.
- JDK 17 or newer for Gradle (Android Studio's bundled one works).
- Rust with the Android targets and cargo-ndk:

  ```sh
  rustup target add aarch64-linux-android x86_64-linux-android
  cargo install cargo-ndk
  ```

## Build and run

```sh
./gradlew assembleDebug                      # arm64-v8a and x86_64
./gradlew assembleDebug -Psolidcraft.abis=arm64-v8a
adb install -r app/build/outputs/apk/debug/SolidCraft-*-debug.apk
adb shell am start -n io.solidcraft.android/android.app.NativeActivity
adb logcat -s SolidCraft                     # the app's log
```

`preBuild` runs `cargo ndk` (task `cargoNdk`), which builds the Rust library into
`app/src/main/jniLibs`. The library is always an optimised build (`-Psolidcraft.cargoProfile=...`
picks another Cargo profile); the kernel is too slow without optimisation.

## Input

- **Mouse and keyboard** work as on the desktop: hover, click to select, the navigation preset's
  buttons (default: middle drag pans, Shift+middle and right drag orbit), the wheel, the right-click
  menu and the keyboard shortcuts. This needs the vendored winit (`vendor/winit`), whose
  upstream Android backend turned a mouse into a finger and sent no modifier keys.
- **Touch**: one finger orbits (a tap selects), two fingers pan, a pinch zooms about the
  fingers. A finger still draws and drags while a sketch is edited or a tool is on.

## Files

Open, Save, Save As, Insert and every import and export use the system's document pickers
(the Storage Access Framework), so files can come from and go to the device, an SD card,
Google Drive or any other documents provider. The engine works on paths, so each document has a
local copy in the app's `documents` folder (`src/documents.rs`): opening copies the document in,
and each finished write to the copy (a save, an export, the next Ctrl+S) is copied back out to the
document. The links survive a restart, so a design reopened from the recent list still saves to
where it came from, and a write the app had no time to copy out is copied out at the next start.
When a provider numbers a clashing name after its extension (`part.step (1)`), the document is
renamed `part (1).step` where the provider allows it.

## Not there yet

- A glTF file whose buffers are separate `.bin` files: only the file picked is copied in (GLB works).
- Opening a file from another app (a file manager's "Open with"): the app registers no file types yet.
- The on-screen keyboard covers the lower half of the screen instead of the app moving above it.
- Phone layout: the layout is the desktop one; it fits a tablet, a phone is cramped.
- Recovery autosaves and other per-user folders: they are found from `HOME`/`XDG_*`, which
  Android doesn't set, so only crash reports and settings (in the app's own folder) are kept.
- Accessibility (accesskit needs GameActivity) and the agent chat's picture paste.
