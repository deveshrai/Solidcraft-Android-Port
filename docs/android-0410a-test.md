# 0.41.0a: the test

Upstream's 0.41.0 merged into SolidCraft for Android and Googlebook OS, the port's gears and
helical threads retired, and what was checked before release. No phone or tablet was attached for
this release; the device run is the Android emulator (a Pixel Tablet image, Android 16, arm64,
which draws on the CPU), so its times are slow and only show that everything works. After release,
the cassette test ran on a Galaxy Tab ([section 4](#4-the-cassette-test-on-a-galaxy-tab)).

- [In short](#in-short)
- [1. The locomotive on 0.41.0a](#1-the-locomotive-on-0410a)
- [2. Designs saved by earlier port releases](#2-designs-saved-by-earlier-port-releases)
- [3. The APKs](#3-the-apks)
- [4. The cassette test on a Galaxy Tab](#4-the-cassette-test-on-a-galaxy-tab)
- [5. Not tested yet](#5-not-tested-yet)

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

## 4. The cassette test on a Galaxy Tab

The [cassette test](android-mcp-live-test.md) (a compact-cassette housing in two halves held by five
M2 screws) on a Galaxy Tab (SM-X840, full screen, not DeX), with the released arm64
APK, over the MCP server through `adb forward`. Its threads are now upstream's modelled threads
(`modeled: true`) in standard 1.6 mm tap drills; before 0.41.0a they were the port's helical threads
with a 0.05 mm print clearance.

![The cassette housing built over MCP on a Galaxy Tab, 0.41.0a](screenshots/android/cassette-tab-0410a.gif)

*The tablet's screen during the run, at its real speed ([MP4](screenshots/android/cassette-tab-0410a.mp4)).*

| | Galaxy Tab, 0.41.0a | Galaxy Z Fold, 0.24.0a |
|---|---|---|
| Operations | 121, **all succeeded** | 121, all succeeded |
| Time | **41.8 s** (37.1 s in calls), recording the screen | 105 s (99.7 s in calls) |
| Checks stage (interference, clearance, measure) | **10.1 s** | 78.3 s |
| Screws | exact B-rep threads, 23 faces each | helical threads |
| Screw against its tapped thread, exact | **no interference**; they touch (0 mm) | not checked: helical against helical wasn't intersected |
| Screw against its tapped thread, on meshes | −0.003 mm, the meshes' tolerance | +0.049 mm (the 0.05 mm clearance) |
| Halves, screw heads | no overlap; heads seated on their counterbores | the same |

| Stage | Calls | Time in calls |
|---|---:|---:|
| Bottom half (side B) | 14 | 1.2 s |
| Top half (side A) | 13 | 1.1 s |
| Reels, window, label, edge openings | 17 | 2.8 s |
| Tape-path block and its holes | 18 | 4.7 s |
| Screw holes and tapped threads | 15 | 11.2 s |
| The screw | 7 | 1.7 s |
| Fit: turn the screw into mesh | 7 | 1.8 s |
| Four more screws, components | 9 | 0.9 s |
| Checks | 18 | 10.1 s |
| Export and save | 3 | 1.7 s |

![A section through screw 1 in its tapped hole](screenshots/android/cassette-tab-0410a-section.jpg)

### On a Galaxy Z Fold

The same test on a phone: a Galaxy Z Fold (SM-F971B, Android 17, the inner screen), over Wi-Fi as an
agent on another computer would connect, with the released arm64 APK.

![The cassette housing built over MCP on a Galaxy Z Fold, 0.41.0a](screenshots/android/cassette-fold-0410a.gif)

*The Fold's inner screen during the run, at its real speed ([MP4](screenshots/android/cassette-fold-0410a.mp4)).*

| | Galaxy Z Fold, 0.41.0a | Galaxy Tab, 0.41.0a |
|---|---|---|
| Operations | 121, **all succeeded** | 121, all succeeded |
| Time | **44.8 s** (40.2 s in calls), over Wi-Fi, recording the screen | 41.8 s (37.1 s in calls), over USB |
| Screw holes and tapped threads | 13.5 s | 11.2 s |
| Checks | 8.7 s | 10.1 s |
| Screw against its tapped thread, exact | no interference (4.6 s for the pair) | no interference (3.3 s) |
| Volumes, faces | identical: side B 9563.73 mm³, side A 8729.04 mm³, each screw 26.17 mm³ and 23 faces | |

What it shows:

- **Upstream's threads are exact and fast to check.** The interference check now intersects a screw
  with its tapped thread (3.3 s for one pair) instead of listing the pair as unchecked, and the whole
  checks stage is about 8× faster than the Fold's with the port's helical threads.
- **Upstream's threads have no fit allowance.** Screw and tapped hole share the basic profile and
  touch: right for the model, too tight to print. A print needs a larger tap drill or a scaled
  screw until there is a clearance option (a candidate in the
  [pipeline](android-vs-upstream.md#pipeline)); the mesh check's −0.003 mm is that contact, within the
  meshes' tolerance, not an overlap.
- The tapped threads take 1.1 to 2.5 s each, growing with every thread already in the body.

## 5. Not tested yet

- Phones other than a Galaxy Z Fold: this release ran on the emulator and, after release, a Galaxy
  Tab and a Galaxy Z Fold (section 4). The DeX
  window fix of 0.40.0-rc.3b was checked on a Galaxy Tab.
- Googlebooks other than an HP and a Dell XPS model. On those two, after release, it worked exactly
  as on other Android devices (the Android underneath is the same); other models have other
  resolutions and scaling.
- **Plugins** on a device: installing and running SolidCraft Gears. Upstream's plugin runtime
  compiles WebAssembly to machine code at run time; whether Android allows that for this app is
  the first thing to check.
