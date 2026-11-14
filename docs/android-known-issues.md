# Known issues in the Android port

What is wrong or missing in the current port release, and what is planned for the next one. Each
entry says how to work around it today. Fixed entries move to [Fixed](#fixed) with the release that
fixed them.

Current: **port 0.41.0a** (upstream's 0.41.0). Next:
**0.41.0b**, or the next upstream release's **a**, if upstream releases
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
| **On Adreno GPUs bodies are built on the CPU.** Upstream 0.40's GPU tessellation shader crashes Qualcomm's compiler, so the port skips it there. Showing a large design can take longer than it would on the GPU. | Galaxy Z Fold (Adreno 840), 0.40.0-rc.2a | None needed | Report the crash upstream with the shader; turn the GPU back on for Adreno once a driver or shader change avoids it |
| **Googlebook screens differ.** The app runs on Googlebooks exactly as on other Android devices, since the Android underneath is the same; it was fine on an HP and a Dell XPS Googlebook (0.41.0a). Other models have other resolutions and scaling, which haven't been tried. | HP and Dell XPS Googlebooks, 0.41.0a | If the layout looks wrong, resize the app's window | Report screenshots from other models: [issues](https://github.com/deveshrai/Solidcraft-Android-Port/issues) |
| **On a phone held upright the isometric view doesn't fit a wide part.** View › Isometric (and an agent's `view.isometric`) leaves a part wider than tall running off both sides of the view; the fit seems to go by the view's height only. | Pixel 9 in portrait, 0.41.0a | Turn the phone sideways, or zoom out with a pinch | Fit by width too, in the port or upstream (check upstream first) |
| **On a phone held upright the crosshair button is half off the screen**, at the view bar's left edge. It still works. | Pixel 9 in portrait, 0.41.0a | Tap its visible half, or turn the phone sideways | Keep the view bar on screen on narrow views |
| **A launch that dies in the GPU's driver moves the app to OpenGL ES for good.** The startup guard from 0.30.1a can't tell a crash while compiling a shader from graphics that don't start, so after the Adreno crash above the next launch used OpenGL ES (MSAA 1x) and kept it. | Galaxy Z Fold, the first 0.40.0-rc.2a build | Stop the app in its first quarter-second once (it then goes back to Vulkan), or reinstall | Keep OpenGL ES only when Vulkan failed before the window drew |
| **Going back to an earlier release needs an uninstall.** Android refuses a lower version code; uninstalling deletes the app's settings and crash reports. Designs saved by 0.41.0a (format 55) don't open in earlier port releases. | Any | Save designs to a device folder first, and keep 0.30.1a copies of anything important | By design for a pre-release |
| **A tap on a dialog's OK can also select in the view.** With the crosshair button on, tapping **OK** in the Extrude window (opened by Press Pull on a screw head) closed it and then selected the face of the body behind it, as if the tap went through. Enter in the value field instead did not. | Galaxy Tab, 0.28.0a | Confirm with Enter, or tap empty space after | Find whether the closing tap reaches the view, and stop it there |
| **Preferences' page list once ignored taps.** Right after the first start on OpenGL ES, three taps (finger and mouse) on page names didn't change the page; after a minute and from then on, every tap did. | Pixel Tablet emulator, 0.30.1a | Tap again | Reproduce it; if it is the first seconds after start, find what the window is still busy with |
| **The first crash report's title is cut short** ("Closed …") when it carries a count (×2): the row has one more button (Save a copy) than upstream's. | Pixel Tablet emulator, 0.30.1a | The message below the title says it in full | Wrap the buttons under the title on narrow rows |
| **A file saved to a device folder can lose its extension.** Save As writes the name typed in Android's picker; type *gear* and the file is `gear`, not `gear.sc`. The app itself names the tab and its copy correctly and opens the file again, but file managers and other apps don't know what it is. | Pixel 9 and Galaxy Z Fold, 0.23.0a and b | Type the extension: *gear.sc*, *part.stl* | Ask the picker for the name with its extension kept, or rename the saved document through the provider |
| **Reopen designs doesn't reopen them.** Upstream 0.24.1's Settings › General › *Reopen the designs that were open* records them when the app quits; Android never quits an app that way (it stops it in the background), so nothing is recorded. | Galaxy Tab, 0.25.1a | **File › Open Recent** | Record the open designs when the app goes to the background |
| **Samsung's window handle covers the new-tab button.** On Samsung devices the handle at the top centre sits over the tab strip; a tap there can open Samsung's window menu instead of **+**. | Galaxy Z Fold | **Ctrl+N** or **File › New Design** | Keep the tab strip's buttons out of the handle's area on Samsung devices |
| **Designs saved by 0.22.0a open as "Untitled (n)".** That release's Save As didn't store the typed name in the design (fixed in 0.23.0a), and Open names the tab from the design. | Any, files from 0.22.0a | **Save As** once to rename it | Fall back to the file's name when a design's name is a default "Untitled" one |

## Planned, larger

The kernel rows below (bores, drafted joins) were found on truck. Since 0.40 the port runs on
solidcore, and they haven't been checked there yet.

| Issue | Work-around today | Plan |
|---|---|---|
| **The on-screen keyboard covers the lower half of the screen**; the app doesn't move above it, so a field low in a dialog is typed into blind. | Hide the keyboard with its ⌄ key to see the result; a hardware keyboard avoids it | Resize the window above the keyboard |
| **No autosave recovery.** If Android ends the app in the background, unsaved work is lost. | Save often (Ctrl+S on a design opened from a file writes back to it) | Upstream's recovery folder on Android, offered at the next start |
| **No "Open with".** Opening a `.solidcraft`, STEP or STL from a file manager or a mail doesn't offer SolidCraft. | **File › Open** inside the app | Register the file types and take the shared file as an Open |
| **A cylinder exactly on an existing bore, or a cut starting exactly on a gear's end face, is slow or fails.** A second Ø8 bore on the axis of an existing one fails ("no result"); upstream's kernel retries such contacts many times. (Joins that touch a gear's flat end face are quick since 0.24.0b.) | Start cuts outside the body, bore once after joining ([android-mcp.md](android-mcp.md#long-operations)) | Upstream's kernel: a cheaper path for coincident faces |
| **A join into a drafted face can fail first, or stall.** A feature that joins into a boss extruded with a taper (a cone), while sitting flush on another face, can need the kernel's retry ("Boolean retry: tool displacement…", seconds) or run for minutes: a 4 mm step clipping a 2° boss took 95 s on a Mac, 0.2 s with the step moved clear. Upstream builds measure the same. Details: [android-devterm-test.md](android-devterm-test.md#3-joins-that-fail-first-or-stall). | Build in separate bodies (side view cut by plan view, bosses on their own) and combine last, a piece at a time, flat pieces first: a2nr's fixture 17 s → 4 s on the tablet ([how](android-devterm-test.md#5-built-as-separate-bodies-instead)) | Upstream's kernel: join into a cone without a retry; give up a slow first attempt sooner |
| **A thread's face can't name its body.** `solid.thread` picks the face by a point; with a screw already in its tapped hole the point is on both, and the wrong one is threaded ("something else of the body meets the thread"). | Thread the part before putting it in place | A `body` parameter, as other commands have |
| **MCP: nothing on the device says Claude Code is waiting.** With SolidCraft in the background, an agent's call waits 15 s and then tells the agent to ask you to bring SolidCraft back; nothing on the phone itself says so. | Keep SolidCraft on screen next to Termux: split screen or a pop-up window ([android-mcp.md](android-mcp.md#keep-solidcraft-on-screen)) | A notification while a call waits, tapping it brings SolidCraft back |
| **The phone layout is cramped.** It works in portrait and landscape, but the ribbon and dialogs are tight. | A tablet, or a foldable's inner screen | Larger touch targets and a compact ribbon on narrow screens |

## Upstream's, not the port's

These come with upstream's code and fail the same way on a clean upstream checkout. They are
reported or left to upstream, not patched in the port, so merges stay clean.

- On this port's build Mac, 15 tests fail on a clean upstream 0.24.0 checkout exactly as on the port,
  so they are left to upstream. The app is unaffected.
  - Engine (3): `prism_local_tests::a_ring_cut_round_a_counterbored_hole_in_a_chamfered_disc`,
    `robustness_goldens::seed_36` and `seed_71`.
  - UI (12), mostly egui's texture check in the test harness: `dialogs_hole::…recessed_drill_point_total_depth_dimension_is_outside_partial_tapping`,
    the two `dialogs_thread::contribution_tests`, the three `edit_clipboard` middle-button tests,
    `render_workspace::…product_preview_error_uses_shared_wrapped_surface_and_explicit_retry_target`,
    `select_other::…long_names_stay_inside_the_popup_and_remain_available_on_hover`,
    `toolbar::dropdown_tests::sketch_create_family_rows_paint_the_first_variants_icons`, and
    `ui_kit`'s `positioned_shared_buttons_center_text_…`, `sample_grid_stretches_each_row_…` and
    `square_theme_search_and_focus_ring_…`.
  Everything else passes: 2532 engine tests and 2746 UI tests among them.
- Two more fail on a clean upstream 0.30.1 checkout exactly as on the port, both egui's checks in the
  test harness: `prefs::tests::origin_plane_names_change_only_the_names_shown` ("time shouldn't move
  backwards") and `prefs::tests::the_system_theme_leaves_the_window_to_follow_the_system` (unapplied
  texture deltas). The other 188 Preferences tests pass.

## By design

- **An agent can't install a plugin.** Over MCP, `plugin.install` refuses the plugins folder, which is
  outside the folders an agent may use. Upstream's rule on every platform: Install plugin… on
  Preferences › Plugins stays with the person at the device.

- **The first open after an update rebuilds the design.** A design keeps its computed model only for
  the build that computed it, so the first time a newer SolidCraft opens it, every feature is
  computed again (the worm gearbox from [android-speed.md](https://github.com/deveshrai/Solidcraft-Android-Port/blob/v0.40.0-rc.3b-android/docs/android-speed.md): 2 minutes on a Galaxy
  Tab); saved again, it opens at once (0.4 s). Upstream does the same on every platform.
- **Upstream's releases don't open designs saved by port releases up to 0.40.0-rc.3b** (they read
  the port's helical-thread format changes as a newer build's). Open them in 0.41.0a and save:
  their helical threads become upstream's modelled threads, the old values kept in the design's
  metadata, and the design opens in upstream too
  ([android-vs-upstream.md](android-vs-upstream.md#design-files-and-compatibility)). Designs
  0.41.0a saves are upstream's own format.
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
| Plugins couldn't be installed: Preferences › Plugins said they were for the desktop app, because upstream keeps plugins under `$XDG_DATA_HOME` or `$HOME`, which Android doesn't have | 0.41.0b: the app's own folder; SolidCraft Gears installs and runs on a Galaxy Z Fold ([the test](android-0410a-test.md#5-plugins-on-a-galaxy-z-fold)) |
| Checks between large bodies were slow: interference and clearance between the cassette test's two halves took about 38 s and 35 s on a Galaxy Z Fold | 0.41.0a: about 0.5 s each, the whole checks stage 9 s on the Fold ([android-0410a-test.md](android-0410a-test.md#on-a-galaxy-z-fold)) |
| The port's own gears and helical threads, and their open issues (slow gear-against-gear checks, a sketch on a helical groove's end, threads not stopping at a shoulder, the hard-to-find Helical B-rep option) | Retired in 0.41.0a for upstream's plugins (SolidCraft Gears) and modelled threads |
| In a Samsung DeX window the window's title bar covered the app's own top bar: File, Settings, the tabs and Report a bug couldn't be reached (full screen was fine) | 0.40.0-rc.3b ([android-0400rc3a-test.md](android-0400rc3a-test.md#4-0400-rc3b-settings-in-a-dex-window)) |
| Every launch died 0.2 s in on a Galaxy Z Fold (Adreno 840): upstream 0.40's GPU tessellation shader crashed Qualcomm's compiler | 0.40.0-rc.2a, before release: bodies on the CPU on Adreno ([android-0400rc2a-test.md](android-0400rc2a-test.md#2-a-crash-at-every-launch-on-adreno)) |
| A phone whose graphics couldn't start (a Galaxy A04, [#4](https://github.com/deveshrai/Solidcraft-Android-Port/issues/4)) was aborted at every launch, with no message | 0.30.1a: leaves cleanly, the next launch starts on OpenGL ES, `startup-error.txt` says why ([android-0301a-test.md](android-0301a-test.md#1-a-launch-that-cant-start-its-graphics)) |
| On OpenGL ES the 3D view was empty: upstream 0.30.1's viewport shader doesn't compile there | 0.30.1a, in the port ([why](android-0301a-test.md#2-the-empty-3d-view-on-opengl-es)) |
| Crash reports were kept but never shown, couldn't leave the device, and Report… drafted the issue on upstream's tracker | 0.30.1a: announced, **Save a copy**, the port's tracker |
| The title bar showed upstream's version (v0.25.1), not the port's | 0.27.1a: *v0.27.1a* |
| A helical thread of more than about 85 turns (an M10 longer than about 128 mm) hung or failed ("volume −651"), and measuring a long threaded body gave a wrong volume | 0.27.1a: a 220 mm M10 thread in 0.8 s on a Mac, 1.2 s on a Galaxy Tab ([android-thread-mating.md](https://github.com/deveshrai/Solidcraft-Android-Port/blob/v0.40.0-rc.3b-android/docs/android-thread-mating.md#5-the-test-run)) |
| Clearance and interference between bodies a hair apart (a screw in its bearing bore) ran into minutes, and clearance could report 0 mm | 0.27.1a: such pairs are meshed finer first (551 s → 4 s) |
| A slow agent command (a boolean on a gear) froze the app until Android closed it as not responding; the MCP client saw calls time out and sent them again | 0.24.0b: agent commands compute off the window thread, long calls answer "still running" and wait through `pending`, sessions don't block each other |
| Booleans on gears and helical threads were 2–3× slower than needed (each B-spline surface point summed every control point) | 0.24.0b |
| A gear's flank was a spline through nine points, so a gear had a face for every span (326 faces for 18 teeth) and every boolean on it was slow; a hub joined on a worm wheel hung for over 25 minutes on a tablet | 0.24.0b: a flank is one to three exact Béziers (110 faces for 18 teeth); a join touching a gear's flat face skips the flanks that can't reach it (a gear with bore, keyway and hub: 161 s → 2.5 s) |
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
