# Known issues in the Android port

What is wrong or missing in the current port release, and what is planned for the next one. Each
entry says how to work around it today. Fixed entries move to [Fixed](#fixed) with the release that
fixed them.

Current: **port 0.23.0b** (upstream 0.23.0). Next: **0.23.0c**, or **0.24.0a** if upstream releases
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
| **The helical thread is hard to find.** Thread opens as upstream's does: Modeled off, and once ticked, Geometry on Faceted. The port's Helical B-rep, Custom and Clearance look missing until you know where they are. | Galaxy Z Fold, 0.23.0a | Tick **Modeled**, set **Geometry** to **Helical B-rep** ([android-threads.md](android-threads.md#opening-thread)) | Decide whether the port opens Thread with Modeled ticked and Helical B-rep chosen |
| **A file saved to a device folder can lose its extension.** Save As writes the name typed in Android's picker; type *gear* and the file is `gear`, not `gear.solidcraft`. The app itself names the tab and its copy correctly and opens the file again, but file managers and other apps don't know what it is. | Pixel 9 and Galaxy Z Fold, 0.23.0a and b | Type the extension: *gear.solidcraft*, *part.stl* | Ask the picker for the name with its extension kept, or rename the saved document through the provider |
| **The version shown in the app is upstream's.** The title bar shows *v0.23.0*; the port letter (b) is only in Android's app info and the APK name. | All, every port release | Settings › Apps › SolidCraft shows *0.23.0b* | Show the port version next to upstream's |
| **Samsung's window handle covers the new-tab button.** On Samsung devices the handle at the top centre sits over the tab strip; a tap there can open Samsung's window menu instead of **+**. | Galaxy Z Fold | **Ctrl+N** or **File › New Design** | Keep the tab strip's buttons out of the handle's area on Samsung devices |
| **Designs saved by 0.22.0a open as "Untitled (n)".** That release's Save As didn't store the typed name in the design (fixed in 0.23.0a), and Open names the tab from the design. | Any, files from 0.22.0a | **Save As** once to rename it | Fall back to the file's name when a design's name is a default "Untitled" one |

## Planned, larger

| Issue | Work-around today | Plan |
|---|---|---|
| **The on-screen keyboard covers the lower half of the screen**; the app doesn't move above it, so a field low in a dialog is typed into blind. | Hide the keyboard with its ⌄ key to see the result; a hardware keyboard avoids it | Resize the window above the keyboard |
| **No autosave recovery.** If Android ends the app in the background, unsaved work is lost. | Save often (Ctrl+S on a design opened from a file writes back to it) | Upstream's recovery folder on Android, offered at the next start |
| **No "Open with".** Opening a `.solidcraft`, STEP or STL from a file manager or a mail doesn't offer SolidCraft. | **File › Open** inside the app | Register the file types and take the shared file as an Open |
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
- **Typing with no field selected runs shortcuts**, as on the desktop: with a hardware keyboard,
  *E* starts Extrude and so on.

## Fixed

| Issue | Fixed in |
|---|---|
| Ribbon menus taller than the screen couldn't be scrolled by touch, so on a foldable's inner screen **Model › Create** stopped at Web and **Gear**, Boss, Lip, Snap Fit, Rest and Frame Member couldn't be reached from it | 0.23.0b |
| Save As kept the tab named "Untitled" instead of the name typed in the picker | 0.23.0a |

## Keeping this page current

On every port release:

1. Set the versions in the opening lines.
2. Move what the release fixed to [Fixed](#fixed), with the release.
3. Add what testing on a device found, with the device and the release, and a work-around.
4. Check the upstream entries against the new upstream release; drop what upstream fixed.
