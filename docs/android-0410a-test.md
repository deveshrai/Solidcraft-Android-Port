# 0.41.0a: the test

Upstream's 0.41.0 merged into SolidCraft for Android and Googlebook OS, the port's gears and
helical threads retired, and what was checked before release. No phone or tablet was attached for
this release; the device run is the Android emulator (a Pixel Tablet image, Android 16, arm64,
which draws on the CPU), so its times are slow and only show that everything works.

- [In short](#in-short)
- [1. The locomotive on 0.41.0a](#1-the-locomotive-on-0410a)
- [2. Designs saved by earlier port releases](#2-designs-saved-by-earlier-port-releases)
- [3. The APKs](#3-the-apks)
- [4. Not tested yet](#4-not-tested-yet)

## In short

| | |
|---|---|
| Upstream | 0.41.0 merged (a stable release); 13 conflicts, each keeping upstream's change and the port's hook, which moved with upstream's code into its new `ui-kit`, `viewport-gpu` and `control` crates |
| The port's checks | every area's tests pass (touch, the MCP server, crash reports, OpenGL ES, the Adreno guard, design files), and the Android, desktop and browser builds have no warnings |
| Locomotive and track | built over MCP on the emulator: 95 calls, 30.4 s, none failed; no two of the 51 parts overlap |
| Old port designs | all five corpus designs from earlier port releases open, evaluate and save again; their helical threads are upstream's modelled threads, their values kept in the design's metadata |
| Format | 55, upstream's own |

## 1. The locomotive on 0.41.0a

The G-scale locomotive mechanism and track from the [0.40.0-rc.3a test](android-0400rc3a-test.md),
built by the same recipe ([`tools/mcp-cassette-test/loco.py`](../tools/mcp-cassette-test/loco.py))
over the MCP server on 0.41.0a:

![The locomotive on 0.41.0a, on the Android emulator](screenshots/android/locomotive-emulator-0410a.jpg)

| On the emulator, over MCP | s |
|---|---|
| The whole build (95 calls) | 30.4 |
| Longest call (a box) | 2.0 |
| Interference between all 51 bodies (none) | 0.9 |

On a Galaxy Tab in DeX, 0.40.0-rc.3a built it in 9.0 s. The title bar shows upstream's new
**Plugins** menu; Settings lists upstream's **Plugins** page and the port's **MCP server** page.

## 2. Designs saved by earlier port releases

Port releases up to 0.40.0-rc.3b saved helical threads as pending design-format changes. 0.41.0a
knows those changes (`crates/doc/src/port_retired.rs`), so the designs open. The corpus test
(`apps/solidcraft-cli/tests/design_corpus.rs`) opens the five designs earlier port releases saved
(0.24.0b to 0.40.0-rc.2a) and checks that each evaluates without a failed feature and saves and
reads back the same; the four with helical threads have them as modelled threads, their values kept.

The 0.40.0-rc.2a design, opened by the 0.41.0 command-line app:

| Feature | Saved by 0.40.0-rc.2a | Opened by 0.41.0a |
|---|---|---|
| Thread on a Ø10 rod | helical M10, left-handed, start angle 90° | modelled M10, left-handed; `start_angle: 90 deg` kept in metadata |
| Thread in a nut's Ø10.2 bore | helical, custom 2.2 mm pitch, clearance 0.15 mm, run off both faces | modelled thread sized from the bore; pitch, clearance and offsets kept in metadata |
| Spur gear m2 z16 | the port's Gear | the same body, ordinary sketches and features |
| File | format 51 with three pending port changes | format 55, nothing pending: upstream's own |

## 3. The APKs

| APK | For | Size |
|---|---|---|
| `SolidCraft-0.41.0a-android-arm64.apk` | phones, tablets, foldables, Googlebooks with a Snapdragon chip | 200 MB |
| `SolidCraft-0.41.0a-android-x86_64.apk` | Googlebooks with an Intel chip | 239 MB |

The arm64 APK grew from 169 MB (0.40.0-rc.3b): the native library's code is about 139 MB and its
read-only data 32 MB, with no debug information, so the growth is upstream 0.41's own, much of it
the WebAssembly runtime (wasmtime and its compiler) that plugins run on.

## 4. Not tested yet

- A phone, tablet or Googlebook: this release ran on the emulator only. The DeX window fix of
  0.40.0-rc.3b was checked on a Galaxy Tab.
- **Plugins** on a device: installing and running SolidCraft Gears. Upstream's plugin runtime
  compiles WebAssembly to machine code at run time; whether Android allows that for this app is
  the first thing to check.
- The x86_64 APK: built, never run.
