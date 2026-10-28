# Keeping the Android port up to date

The port is upstream SolidCraft plus the Android work ([android.md](android.md)). Upstream moves
fast; this is how the port follows it.

## The setup

| Remote | Repository | Role |
|--------|------------|------|
| `upstream` | https://github.com/tangency-labs/solidcraft | SolidCraft itself: only fetched from |
| `origin` | https://github.com/deveshrai/Solidcraft-Android-Port | This port: pushed to |

The port's `main` is upstream's `main` with the Android commits on it. Updating **merges**
upstream into it (no rebase), so the history pushed to `origin` is never rewritten and nothing
needs a force push.

A fresh clone sets up the second remote once:

```sh
git clone https://github.com/deveshrai/Solidcraft-Android-Port solidcraft-android
cd solidcraft-android
git remote add upstream https://github.com/tangency-labs/solidcraft
```

## How the port stays mergeable

Upstream is free to change anything, in any direction, as often as it likes: the port never asks
it to hold still. Instead the port knows exactly where it touches upstream, and proves after
every merge that each of those places still works.

- **The port lives in its own files where it can**: the Android app (`apps/solidcraft-android/`),
  the gear generator (`crates/engine/src/gear_profile.rs`), the MCP endpoint, the port's docs, and
  upstream's own folders for unfolded additions (pending strings, attribution rows and
  design-format changes). Upstream never edits those, so they never conflict.
- **Every upstream file the port changes is registered** in
  [`scripts/android-port-areas.json`](../scripts/android-port-areas.json), in one of seven areas
  (touch input, helical threads, gears, design files, agents and the MCP server, kernel speed, the
  Android build), and each area names the tests that prove it. About 45 upstream files carry
  port changes as of 0.25.1a, most of them a few lines.
- **A fix a merge could quietly undo carries a marker**: a `SolidCraft` comment the area lists, so
  a merge that takes upstream's version of the file is caught even when everything builds.
- **Designs saved by port releases are kept** in upstream's design corpus
  (`apps/solidcraft-cli/tests/corpus/port-*.solidcraft`, each saved by that release's CLI with
  its expected evaluation), so every merge proves old port designs still open and evaluate as
  saved, even when upstream reuses a format number the port wrote (as 0.24.1 did with 40).
- **Upstream's own failures are listed, not hidden**: a test that fails on a clean upstream checkout
  too goes under `upstream_failures` in the manifest, with why; the area runs skip it, and
  `upstream-failures` re-checks the list against clean upstream so a skip never outlives the bug.
- **Checks fail loudly**: an unregistered touch point, a missing marker, a port string upstream
  now gives another meaning, an area test failing, or a test filter that matches **no** test (a
  renamed test would otherwise stop guarding anything) each stop the sync with the fix to make.

`scripts/android-port.py` does the checking:

| Command | What it does |
|---|---|
| `preview [TAG]` | A trial merge of TAG (default: the newest upstream release) in a scratch worktree: the releases it brings, the design-format change, the files both sides changed by port area, the conflicts and string clashes. Changes nothing. |
| `report [TAG]` | The same without the trial merge. |
| `touchpoints` | Every upstream file the port changes is in an area; listed files the port no longer changes are noted. |
| `markers` | The marked fixes are in their files. |
| `strings [--fix]` | Port strings upstream now has: the same text is dropped with `--fix`; other text is a clash to rename. |
| `tests [AREA]` | Each area's tests (or one area's: `tests gears`), skipping `upstream_failures`. |
| `upstream-failures [TAG]` | Builds a clean upstream checkout and runs the tests listed as upstream's own failures: each must still fail there. One that passes is fixed upstream: drop it from the list, and the port tests it again. Slow (a full build); run it at each upstream release. |

**Adding to the port**: a new upstream file you change goes into the area it serves (or a new
area) in `android-port-areas.json`, with a test in that area's existing test file; a port release
that changes what a design file holds adds a corpus file saved by its CLI. Agents read the same
rules in [`skills/solidcraft-android-port/SKILL.md`](../skills/solidcraft-android-port/SKILL.md).

## Updating

First look, without changing anything:

```sh
scripts/android-sync.sh --preview          # the newest upstream release
scripts/android-sync.sh --preview v0.26.0  # or a given one
```

Then merge it:

```sh
scripts/android-sync.sh --latest
```

It stops at the first step that fails and says what to do. In order:

1. **Fetch, report and merge** the newest upstream release tag (needs a clean working tree);
   `--to <tag>` merges a given tag, and no option merges `upstream/main`. Before merging it prints
   what the release changes in each port area. Nothing new: it goes on to the checks anyway.
2. **Strings**: port strings upstream now has with the same text are dropped from
   `assets/locales/pending/android-port.json` and committed; the same id with other text stops
   the sync.
3. **winit**: checks the patched `vendor/winit` is still the winit in use. When upstream's eframe
   moves to another winit, Cargo stops using the patch and the mouse and keyboard would quietly
   go back to acting as a finger; the script stops instead and names the version wanted.
4. **Checks**: `cargo fmt --check`; clippy for Android (arm64); clippy on the desktop for the
   UI crate, the Android crate and the desktop app; the UI crate's wasm build; `cargo xtask
   layers`, `assets`, `lists` and `skills`; then the port's own: `touchpoints`, `markers`,
   `strings` and every area's tests (about 15 minutes in all).
5. **APK**: `./gradlew assembleDebug` (skip with `--no-build`).

Then look over what came in, test on a device, and publish:

```sh
git log --oneline -20
git push origin HEAD
```

Upstream cuts releases on their own line (v0.23.0 is not on `main`), so merging the tag gets
exactly that release and the APK gets its version. Merging `main` later is fine; both are merges.

Options: `--preview [tag]`, `--latest`, `--to <tag>`, `--no-build` (no APK), `--check-only` (no
fetch or merge: check the tree as it is), `--revendor X.Y.Z` (below). `UPSTREAM_REMOTE` /
`UPSTREAM_BRANCH` override `upstream` / `main`.

How often: at each upstream release. Upstream lands dozens of changes a day and sometimes
several releases a night (0.24.1, 0.25.0 and 0.25.1 in one night); smaller, more frequent merges
have fewer conflicts.

## Releasing

The APK carries SolidCraft's own version (`[workspace.package] version` in `Cargo.toml`, read by
`apps/solidcraft-android/android/app/build.gradle.kts`), so after a sync that brought a new
SolidCraft version the next build has it. To publish an APK on the port's
[releases page](https://github.com/deveshrai/Solidcraft-Android-Port/releases):

```sh
gh auth login                    # once: the GitHub CLI, https://cli.github.com
git push origin HEAD             # the release is of a pushed commit
scripts/android-release.sh       # or --dry-run first: build the APK and write the notes only
```

It builds the arm64 release APK, checks its version, tags the commit `v<version>-android`
(upstream's own tag `v<version>` is SolidCraft's and stays its), pushes the tag and creates the
release with `SolidCraft-<version>-android-arm64.apk` and notes naming the upstream commit it is
built on.

A release of fixes to the port with SolidCraft's version unchanged takes a letter until the next
upstream version: `scripts/android-release.sh --revision a` makes version `0.21.0a`, tag
`v0.21.0a-android`, then `--revision b` makes `0.21.0b`, and so on (z at most). The version code (what Android compares for updates) is
`major·10⁶ + minor·10⁴ + patch·10² + revision`: 0.16.2 is 160200, 0.16.2a is 160201, and
every later SolidCraft version is higher. Releases are signed with the debug key for now (see
[android.md, "Not there yet"](android.md#not-there-yet)); keep signing with the same key, or
Android refuses to update an installed copy.

## When a step fails

### Conflicts

The Android changes to upstream files are few and small ([android.md, "Changes to upstream
code"](android.md#changes-to-upstream-code)), so conflicts are rare and usually in:

- `Cargo.lock`: take upstream's, then let Cargo add ours back:
  `git checkout --theirs Cargo.lock && cargo update -p winit --offline || cargo update -p winit`.
- `Cargo.toml`, `.gitignore`, `AGENTS.md`, `ATTRIBUTION.md`, `vendor/README.md`, `docs/building.md`:
  keep both sides (upstream's change and our added lines).
- `assets/locales/*.json`, `ATTRIBUTION.md`'s files table: since 0.23 the port's UI strings live in
  `assets/locales/pending/android-port.json` and its attribution rows in
  `attribution/pending/android-port.md`, upstream's folders for additions that haven't been folded
  yet. Upstream folds pending files only on its own `main`, so ours stay put and the catalogs merge
  without conflicts. Take upstream's catalogs as they are, add new port strings and rows to those
  two files, and never run `cargo xtask fold-pending` here. If upstream later adds a string with
  the same id, `cargo xtask lists` says so: delete ours from the pending file.
- The thread feature's files (`crates/doc/src/eval.rs`, `document.rs`, `crates/engine/src/cmd/features.rs`,
  `crates/ui-egui/src/dialogs_thread.rs`, `dialogs.rs`, `dialogs_all_tests.rs`): upstream works on
  threads and holes too. Keep upstream's change and the port's helical parts (`thread_face`,
  `thread_span`, `helical_thread_on`, the `helical` branch of `FeatureKind::Thread`'s evaluation,
  the dialog's Type/Geometry rows). Merging v0.23.0, upstream's refusal for sizes past the coarse table (`beyond_coarse_range`)
  applies to ISO threads only; a custom pitch threads any diameter. Merging v0.22.0, upstream's tapped-hole catalogues turned
  `thread_on` into `thread_on_impl`; the port's version keeps that and calls `thread_face` and
  `thread_span`. Then `cargo test -p solidcraft-kernel helical`, `-p solidcraft-engine thread`.
- `crates/ui-egui/src/navigation.rs` / `viewport.rs`: upstream changed the camera input or the
  viewport state; keep upstream's change and re-apply the touch parts (the `touching`/`touch`
  block, `touch_orbit`, the `lifted`/`over` test for `inside`, the `touch_gesture` field). The
  touch tests say whether they still work.

- `vendor/truck-geometry` (upstream re-vendors truck) or `crates/ui-egui/src/control_job.rs` /
  `control.rs` (upstream reworks the agent's control path): see [The port's speed fixes](#the-ports-speed-fixes).

Then `git add` the files, `git commit`, and `scripts/android-sync.sh --check-only`.

### Size caps

Upstream caps AGENTS.md and each skill (`cargo xtask skills`); the port's lines count toward the
cap, so when upstream grows a file the port adds to, shorten the port's lines. 0.25.1 grew
`skills/solidcraft-crash-safety/SKILL.md` past it with the port's Android `unsafe` line.

### The port's speed fixes

Four port changes make gears, threads and agents' long calls quick on a phone. A merge can drop
one without breaking the build, so `scripts/android-sync.sh` checks each after the tests and
stops naming it. Take upstream's change when it does the same job better (the gear and timing
tests still pass and the times don't get worse), else keep the port's:

| Fix | Where | Checked by |
|---|---|---|
| B-spline surfaces skip zero basis terms | `vendor/truck-geometry/src/nurbs/bspsurface.rs`, six places marked `SolidCraft:` | the markers; re-apply them from `git show <the port's commit> -- vendor/truck-geometry` if upstream re-vendors truck |
| A gear flank is one to three Béziers, not a spline of many faces | `crates/engine/src/gear_profile.rs` (`flank_plan`, `flank_curves`) | `gear_profile` tests, `a_flank_is_one_face_a_curve` |
| Agents' commands, timed batches too, compute off the window thread | `apps/solidcraft-android/src/lib.rs` (`agent_off_thread = true`), `crates/ui-egui/src/control_job.rs` (`Steps::ms`), `control.rs` (`ahead_timings`) | `a_timed_batch_computes_off_the_window_thread`, `an_agent_design_change_computes_off_the_window_thread` |
| One MCP session's long call doesn't hold up another | `apps/solidcraft-android/src/mcp_http.rs` (port-only) | `a_long_call_holds_up_only_its_own_session` |

After merging an upstream release, or landing a port-only feature, update
[android-vs-upstream.md](android-vs-upstream.md) (its last section says what to check).

Upstream's releases fold their pending design-format changes into a new `v<N>/` folder. The
port's `crates/doc/src/format/pending/helical-threads.rs` stays pending (never fold it here):
it is what makes upstream refuse a port design instead of losing its helical threads.

### winit

The script names the winit version upstream now wants:

```sh
scripts/android-sync.sh --revendor 0.30.14
```

downloads that winit from crates.io into `vendor/winit`, applies
`apps/solidcraft-android/patches/winit-android-input.patch`, updates the version in
`vendor/README.md` and `ATTRIBUTION.md`, and runs the checks. If the patch no longer applies
(winit changed its Android backend), apply the rejected parts (`vendor/winit/**/*.rej`) by hand
following [android.md, "Input"](android.md#input-the-vendored-winit), delete the `.rej` and
`.orig` files, and refresh the patch from the new pristine source:

```sh
V=0.30.14   # the new version
mkdir -p /tmp/w && cd /tmp/w && curl -fsSL https://static.crates.io/crates/winit/winit-$V.crate | tar -xz
mkdir -p a b && cp -R winit-$V/src a/ && cp -R ~-/vendor/winit/src b/
diff -ruN a/src b/src > ~-/apps/solidcraft-android/patches/winit-android-input.patch; cd -
```

A new major winit (0.31) is a bigger change: eframe will have moved its Android support too.
Check upstream winit's Android backend first; it may report mice itself by then, and the
vendored copy (and `[patch.crates-io]` entry) can go.

### Android build errors

- `cannot find crate` in `control_server.rs`: the Android app builds the desktop app's control
  server from its own file (`#[path]`), so a crate upstream adds there must be added to
  `apps/solidcraft-android/Cargo.toml` too (0.25.1 added `serde`).
- A missing field in a port test's struct literal (0.25.1 added `GpuTarget::oit`): add the field
  with the value upstream's own code uses.
- In `apps/solidcraft-android/src/lib.rs`: upstream changed `SolveApp`'s or `Services`'s API, or
  the desktop app's setup; compare with `apps/solidcraft/src/main.rs`, which does the same job.
- In `documents.rs`: the `jni` crate's version follows `android-activity` (pulled in by winit);
  a new major version changes the JNI calls.
- In a shared crate, for Android only: upstream added a desktop-only dependency (like `arboard`
  was); keep it off Android in that crate's `Cargo.toml` (`cfg(not(target_os = "android"))`)
  with a fallback, as `crates/ui-egui/src/agent_chat/paste.rs` does.

### Gradle

The Android Gradle plugin (`apps/solidcraft-android/android/build.gradle.kts`) and Gradle
(`gradle/wrapper/gradle-wrapper.properties`) are pinned; update them on purpose, not with
upstream. A new `compileSdk` needs that platform installed (`sdkmanager "platforms;android-NN"`).

## Upstream's own checks

The script runs the parts of upstream's gate (`cargo xtask ci`) that the port touches. Running the
full gate also runs upstream's whole test suite, some of which may already fail on upstream's own
`main` on a given machine (on the macOS machine the port was made on, 11 `crates/ui-egui` tests
fail on a clean upstream checkout too); compare with a checkout of `upstream/main` before
assuming the port broke something.
