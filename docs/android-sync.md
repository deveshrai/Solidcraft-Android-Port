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

## Updating

```sh
scripts/android-sync.sh
```

It stops at the first step that fails and says what to do. In order:

1. **Fetch and merge** `upstream/main` (needs a clean working tree). Nothing new: it goes on to
   the checks anyway.
2. **winit**: checks the patched `vendor/winit` is still the winit in use. When upstream's eframe
   moves to another winit, Cargo stops using the patch and the mouse and keyboard would quietly
   go back to acting as a finger; the script stops instead and names the version wanted.
3. **Checks**: `cargo fmt --check`; clippy for Android (arm64); clippy on the desktop for the
   UI crate, the Android crate and the desktop app; the UI crate's wasm build; `cargo xtask
   layers` and `assets`; the touch and Android tests.
4. **APK**: `./gradlew assembleDebug` (skip with `--no-build`).

Then look over what came in and publish:

```sh
git log --oneline -20
git push origin HEAD
```

To follow an upstream **release** rather than `main`, merge its tag: upstream cuts releases on
their own line (v0.21.0 is not on `main`), so `scripts/android-sync.sh --to v0.21.0` merges
exactly that release and the APK gets its version. Merging `main` later is fine; both are merges.

Options: `--to <tag>` (merge that upstream tag), `--no-build` (no APK), `--check-only` (no fetch or merge: check the tree as it is),
`--revendor X.Y.Z` (below). `UPSTREAM_REMOTE` / `UPSTREAM_BRANCH` override `upstream` / `main`.

How often: whenever you want the new features; upstream lands several changes a day, and
smaller, more frequent merges have fewer conflicts.

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

A release of fixes to the port with SolidCraft's version unchanged takes a revision:
`scripts/android-release.sh --revision 1` makes version `<version>-android.1`, tag
`v<version>-android.1`. The version code (what Android compares for updates) is
`major·10⁶ + minor·10⁴ + patch·10² + revision`: 0.16.2 is 160200, its revision 1 is 160201, and
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
- `crates/ui-egui/src/navigation.rs` / `viewport.rs`: upstream changed the camera input or the
  viewport state; keep upstream's change and re-apply the touch parts (the `touching`/`touch`
  block, `touch_orbit`, the `lifted`/`over` test for `inside`, the `touch_gesture` field). The
  touch tests say whether they still work.

Then `git add` the files, `git commit`, and `scripts/android-sync.sh --check-only`.

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
