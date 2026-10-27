# Known issues in the Android port

What is wrong or missing in the current port release, and what is planned for the next one. Each
entry says how to work around it today. Fixed entries move to [Fixed](#fixed) with the release that
fixed them.

Current: **port 0.24.0a** (upstream 0.24.0). Next: **0.24.0b**, or **0.25.0a** if upstream releases
first. See also [android-vs-upstream.md](android-vs-upstream.md) for what upstream has that the port
doesn't, and the reverse.

- [For the next release](#for-the-next-release)
- [Planned, larger](#planned-larger)
- [Upstream's, not the port's](#upstreams-not-the-ports)
- [By design](#by-design)
- [Fixed](#fixed)
- [Keeping this page current](#keeping-this-page-current)

## For the next release

| Issue | Seen on | Work-around today | Plan |
|---|---|---|---|
| **The helical thread is hard to find.** Thread opens as upstream's does: Modeled off, and once ticked, Geometry on Faceted. The port's Helical B-rep, Custom and Clearance look missing until you know where they are. | Galaxy Z Fold, 0.23.0a; still so in 0.24.0a | Tick **Modeled**, set **Geometry** to **Helical B-rep** ([android-threads.md](android-threads.md#opening-thread)) | Decide whether the port opens Thread with Modeled ticked and Helical B-rep chosen |
| **A file saved to a device folder can lose its extension.** Save As writes the name typed in Android's picker; type *gear* and the file is `gear`, not `gear.solidcraft`. The app itself names the tab and its copy correctly and opens the file again, but file managers and other apps don't know what it is. | Pixel 9 and Galaxy Z Fold, 0.23.0a and b | Type the extension: *gear.solidcraft*, *part.stl* | Ask the picker for the name with its extension kept, or rename the saved document through the provider |
| **The version shown in the app is upstream's.** The title bar shows *v0.24.0*; the port letter (a) is only in Android's app info and the APK name. | All, every port release | Settings › Apps › SolidCraft shows *0.24.0a* | Show the port version next to upstream's |
| **Samsung's window handle covers the new-tab button.** On Samsung devices the handle at the top centre sits over the tab strip; a tap there can open Samsung's window menu instead of **+**. | Galaxy Z Fold | **Ctrl+N** or **File › New Design** | Keep the tab strip's buttons out of the handle's area on Samsung devices |
| **Designs saved by 0.22.0a open as "Untitled (n)".** That release's Save As didn't store the typed name in the design (fixed in 0.23.0a), and Open names the tab from the design. | Any, files from 0.22.0a | **Save As** once to rename it | Fall back to the file's name when a design's name is a default "Untitled" one |

## Planned, larger

| Issue | Work-around today | Plan |
|---|---|---|
| **The on-screen keyboard covers the lower half of the screen**; the app doesn't move above it, so a field low in a dialog is typed into blind. | Hide the keyboard with its ⌄ key to see the result; a hardware keyboard avoids it | Resize the window above the keyboard |
| **No autosave recovery.** If Android ends the app in the background, unsaved work is lost. | Save often (Ctrl+S on a design opened from a file writes back to it) | Upstream's recovery folder on Android, offered at the next start |
| **No "Open with".** Opening a `.solidcraft`, STEP or STL from a file manager or a mail doesn't offer SolidCraft. | **File › Open** inside the app | Register the file types and take the shared file as an Open |
| **Checks between large bodies are slow.** Interference and clearance between the two halves of the [cassette test](android-mcp-live-test.md) take about 38 s and 35 s on a Galaxy Z Fold (about 1 s each before its tape-path blocks were added). | Check the pairs that matter (`bodies`, `against`) | Find what makes those pairs slow |
| **A thread's face can't name its body.** `solid.thread` picks the face by a point; with a screw already in its tapped hole the point is on both, and the wrong one is threaded ("something else of the body meets the thread"). | Thread the part before putting it in place | A `body` parameter, as other commands have |
| **Helical threads against helical threads aren't intersected.** Interference and clearance list such pairs as not checked. | Check a thread fit on meshes ([how](android-mcp-live-test.md#how-the-fit-was-checked)) | Upstream's kernel |
| **MCP: nothing on the device says Claude Code is waiting.** With SolidCraft in the background, an agent's call waits 15 s and then tells the agent to ask you to bring SolidCraft back; nothing on the phone itself says so. | Keep SolidCraft on screen next to Termux: split screen or a pop-up window ([android-mcp.md](android-mcp.md#keep-solidcraft-on-screen)) | A notification while a call waits, tapping it brings SolidCraft back |
| **The phone layout is cramped.** It works in portrait and landscape, but the ribbon and dialogs are tight. | A tablet, or a foldable's inner screen | Larger touch targets and a compact ribbon on narrow screens |

## Upstream's, not the port's

These come with upstream's code and fail the same way on a clean upstream checkout. They are
reported or left to upstream, not patched in the port, so merges stay clean.

- Four UI tests fail on upstream 0.23.0 as well:
  `dialogs_hole::tests::recessed_drill_point_total_depth_dimension_is_outside_partial_tapping`,
  `dialogs_thread::contribution_tests::contribution_metric_designation_rendering_preserves_authored_legacy_and_custom_text`,
  `dialogs_thread::contribution_tests::contribution_thread_catalogue_is_parseable_and_rendering_keeps_custom_designations`
  and `toolbar::dropdown_tests::sketch_create_family_rows_paint_the_first_variants_icons`.
  The app is unaffected.

## By design

- **Upstream's releases don't open designs the port saved.** The port saves one file format past
  upstream's (38 against 37) for the helical thread fields, so upstream asks to be updated rather
  than silently rebuilding helical threads as faceted ones. Export STEP to share
  ([android-vs-upstream.md](android-vs-upstream.md#design-files-and-compatibility)).
- **Split Body nudges its plane** (by micrometres) when the exact split fails, and later cuts across that
  seam can fail. Upstream's kernel; build the two sides as separate solids instead.
- **The MCP server runs the agent's calls only while SolidCraft is on screen.** Android pauses an
  app in the background, so no app can run them then; the call says so rather than hang
  ([android-mcp.md](android-mcp.md#keep-solidcraft-on-screen)).
- **The MCP server is plain HTTP.** As any local development server, it relies on the token and on
  the network: use **Another computer** on networks you trust, or **This device**, which nothing
  else can reach ([android-mcp.md](android-mcp.md#security)).
- **Typing with no field selected runs shortcuts**, as on the desktop: with a hardware keyboard,
  *E* starts Extrude and so on.

## Fixed

| Issue | Fixed in |
|---|---|
| Back with Settings open sent the app to the background instead of closing Settings | 0.24.0a |
| A helical thread tapped into a boss under a hollow shell was refused ("the groove removes … mm³") | 0.24.0a |
| An internal ISO thread with a clearance in too small a hole came out too shallow for its bolt, silently; it now names the drill | 0.24.0a |
| Ribbon menus taller than the screen couldn't be scrolled by touch, so on a foldable's inner screen **Model › Create** stopped at Web and **Gear**, Boss, Lip, Snap Fit, Rest and Frame Member couldn't be reached from it | 0.23.0b |
| Save As kept the tab named "Untitled" instead of the name typed in the picker | 0.23.0a |

## Keeping this page current

On every port release:

1. Set the versions in the opening lines.
2. Move what the release fixed to [Fixed](#fixed), with the release.
3. Add what testing on a device found, with the device and the release, and a work-around.
4. Check the upstream entries against the new upstream release; drop what upstream fixed.
