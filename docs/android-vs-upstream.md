# SolidCraft for Android and Googlebook OS, and upstream SolidCraft

How this port differs from upstream [tangency-labs/solidcraft](https://github.com/tangency-labs/solidcraft).
The port is upstream as an app for Android and Googlebook OS: phones, tablets, foldables, Samsung
DeX and Googlebooks, Google's laptops whose OS is Android-based. It is kept up to date with every
upstream release.

![The port and upstream](screenshots/android/hero-port-vs-upstream.png)

Current: **port 0.41.0a**, against **upstream 0.41.0** ([what's new](android-whats-new.md)). This page is
kept up to date with each port release ([keeping this page current](#keeping-this-page-current)).

- [At a glance](#at-a-glance)
- [What the port adds](#what-the-port-adds)
- [Retired in 0.41.0a: gears and helical threads](#retired-in-0410a-gears-and-helical-threads)
- [Design files and compatibility](#design-files-and-compatibility)
- [Where upstream is still ahead](#where-upstream-is-still-ahead)
- [Pipeline](#pipeline)
- [Keeping this page current](#keeping-this-page-current)

## At a glance

| | Upstream 0.41.0 | Port 0.41.0a |
|---|---|---|
| Platforms | Windows, macOS, Linux, web | All of upstream's, **plus Android and Googlebook OS**: phones, tablets, foldables, Samsung DeX, Googlebooks (an arm64 APK, and an x86_64 one for Intel Googlebooks) |
| Modelling | Everything: sketches, the solver, every feature and workspace, exact modelled threads, plugins | The same, unchanged |
| Plugins | Sandboxed WebAssembly plugins (first: [SolidCraft Gears](https://github.com/bherbruck/solidcraft-gears)), installed from a `.zip` | The same; the `.zip` comes through Android's file picker |
| AI agents | MCP over stdio on the same computer (`solidcraft-cli mcp`), and the agent chat | Upstream's, **plus an MCP server on the network** (Settings › MCP server): Claude Code in Termux on the device, or on a computer on the same Wi-Fi; long calls answer "still running" and wait through `pending`, each session on its own thread, timed batches timed where they computed |
| Touch | None | Orbit, pan, pinch, tap to select, finger-dragged dialog arrows, finger-scrolled drop-downs, **a crosshair to pick small edges and faces by finger** (view bar) |
| Desktop windows | The operating system's | In a DeX or Googlebook OS window the app keeps below the window's title bar |
| Files | Native file dialogs | Android's file picker: device, SD card, Google Drive, any storage app |
| Graphics | Vulkan, Metal, DirectX, OpenGL; Windows retries other APIs | Vulkan or OpenGL ES; a launch that can't start makes the next one use OpenGL ES; the viewport shader compiles on OpenGL ES; bodies are built on the CPU on Adreno, whose compiler crashes on the GPU tessellation shader |
| Crash reports | Kept, shown at the next start, Report… drafts an issue upstream | The same, plus **Save a copy** through Android's picker; Report… drafts it on the port's tracker |
| Design file format | 55 | 55: upstream's own |

## What the port adds

Everything below is about running SolidCraft on Android; the modelling is upstream's.

- **Touch:** one finger orbits, two fingers pan, a pinch zooms about the fingers, a tap selects,
  a finger drags a dialog's arrows and scrolls a tall drop-down, and the view bar's crosshair
  button picks small edges and faces a finger would cover. All of it goes through upstream's single
  camera-input path (`navigation::handle`). Guide: [android-touch.md](android-touch.md).
- **Mouse and keyboard as on the desktop:** a patched winit (`vendor/winit`) reports a mouse as a
  mouse, with hover, every button, the wheel and the right-click menu, and reports the modifier keys.
- **Desktop windows:** in Samsung DeX and on Googlebook OS the app runs in a window like any desktop
  app, its own top bar (File, Settings, the tabs, Report a bug) below the window's title bar.
- **Files** through Android's file picker, mirrored to local copies, so Ctrl+S on a design opened
  from Drive saves back to Drive.
- **The MCP server on the network:** Settings › MCP server lets Claude Code, or any MCP client,
  drive the app from Termux on the same device or from a computer on the same Wi-Fi. Every tool is
  upstream's own MCP server; the port adds the network transport (MCP Streamable HTTP with a bearer
  token). Guide: [android-mcp.md](android-mcp.md); a live test: [android-mcp-live-test.md](android-mcp-live-test.md).
- **Android behaviour:** Back closes what is open; the on-screen keyboard for fields; the layout
  follows rotation, folding and phone screens; the system bars are hidden so a tablet's taskbar
  doesn't cover the app; OK and agents' commands run long features off the window thread, so
  Android doesn't show "not responding".
- **Graphics fallbacks** for GPUs whose drivers trip on upstream's shaders (OpenGL ES, Adreno).

How it works: [android.md](android.md#how-it-works).

## Retired in 0.41.0a: gears and helical threads

Until 0.40.0-rc.3b the port also had modelling features of its own: an involute **gear generator**
(ten types, bore, keyway, hub) and **helical B-rep threads** (any pitch on any diameter, print
clearance, a start angle). Upstream now has plugins, the first of them a gear generator
([SolidCraft Gears](https://github.com/bherbruck/solidcraft-gears)), and exact modelled threads in
many forms, so the port's own versions were taken out; they may come back as plugins. The guides as
they were: [gears](https://github.com/deveshrai/Solidcraft-Android-Port/blob/v0.40.0-rc.3b-android/docs/android-gears.md),
[threads](https://github.com/deveshrai/Solidcraft-Android-Port/blob/v0.40.0-rc.3b-android/docs/android-threads.md).

- Gears made with the port stay in their designs as ordinary sketches and features; only **Edit
  Gear** is gone.
- Helical threads open as upstream's modelled threads (below).

## Design files and compatibility

- **The port opens every upstream design**, from any version, and **upstream opens the port's**:
  since 0.41.0a the port writes upstream's own format, with nothing pending of its own.
- **Designs saved by port releases up to 0.40.0-rc.3b open in the port** (not in upstream, which
  reads them as a newer build's). Each helical thread becomes upstream's modelled thread on the same
  face: an ISO designation keeps its size and pitch; a custom pitch gives way to the size the face
  takes. The thread's own values (pitch, clearance, offsets, start angle) are kept in the design's
  metadata, under `io.github.deveshrai.solidcraft-android`. Saved again, the design is upstream's
  format and opens there too. For the exact old geometry, export STEP from 0.40.0-rc.3b.

Details: [android.md, "Design files"](android.md#changes-to-upstream-code).

## Where upstream is still ahead

Kept honest, so nobody is surprised (the full list, with work-arounds and plans:
[android-known-issues.md](android-known-issues.md)):

- **Autosave recovery** isn't on Android yet: save your work.
- **"Open with"** from other apps isn't there yet: use File › Open.
- The **on-screen keyboard** covers the lower part of the screen, and the app doesn't move above
  it. A hardware keyboard avoids this.
- On a **phone**, the layout works but is cramped; a tablet, a foldable's inner screen or a
  Googlebook is best.
- Upstream's desktop app gets its fixes first. The port takes them at each upstream release,
  usually within a day ([android-sync.md](android-sync.md)).

## Pipeline

Port changes are built on a branch, then merged with docs and device screenshots and released as
the next letter (0.41.0a, b, c…).

| Next | Where it stands |
|---|---|
| "Open with" from other apps | Planned |
| The app moving above the on-screen keyboard | Planned |
| Gears and print-fit threads again, as plugins | To be re-evaluated against SolidCraft Gears and upstream's threads |

### Candidates from upstream's requests

Open, unclaimed feature requests on upstream that would most help the port's users, chosen from
the ~390 open requests on 2026-10-26. These are candidates, not commitments; one that gets picked
moves to the table above.

| | Candidate | Upstream request | Why |
|---|---|---|---|
| 1 | **Pen support**: hover highlights, the barrel button right-clicks, the palm is ignored | [#2828](https://github.com/tangency-labs/solidcraft/issues/2828), [#4271](https://github.com/tangency-labs/solidcraft/issues/4271) | Most tablet users have a pen, and today it acts as a finger |
| 2 | **ViewCube**: tap a face, edge or corner to turn the view | [#2124](https://github.com/tangency-labs/solidcraft/issues/2124), [#2198](https://github.com/tangency-labs/solidcraft/issues/2198), [#6064](https://github.com/tangency-labs/solidcraft/issues/6064) | The axis triad is a small target for a finger. Upstream already offers a cube navigator (Settings › Interface); see what the requests still miss before starting |
| 3 | **Drag a value to change it** | [#6477](https://github.com/tangency-labs/solidcraft/issues/6477) | Changes numbers without the on-screen keyboard |
| 4 | **Edit sizes on the model**: tap a hole or round, change its size in place | [#6642](https://github.com/tangency-labs/solidcraft/issues/6642) | Saves finding and opening the sketch on a small screen |
| 5 | **Clearance on Combine Cut**: a fit gap for mating parts | [#3774](https://github.com/tangency-labs/solidcraft/issues/3774) | The commonest reason prints don't fit |
| 6 | **Fillet that fits**: the largest radius the shape allows instead of a refusal | [#293](https://github.com/tangency-labs/solidcraft/issues/293) | Ends guessing smaller radii by trial and error |
| 7 | **Hexagonal pattern** | [#2574](https://github.com/tangency-labs/solidcraft/issues/2574) | Vents, grilles and lightweighting in printed parts |
| 8 | **Skip instances in a rectangular pattern** | [#3247](https://github.com/tangency-labs/solidcraft/issues/3247) | A grid with gaps without several patterns |

Before starting one, check upstream again: someone may have claimed it or shipped it since
(`gh pr list -R tangency-labs/solidcraft --state open --search "#N"`).

## Keeping this page current

On every port release:

1. Set the versions in the opening lines and the [At a glance](#at-a-glance) header.
2. A new port addition gets a row in the table and a line under
   [What the port adds](#what-the-port-adds), and leaves [Pipeline](#pipeline).
3. When upstream adds something the port had first, say so in its row, or remove the row if the
   two are now the same.
4. Check the format numbers in [At a glance](#at-a-glance).
