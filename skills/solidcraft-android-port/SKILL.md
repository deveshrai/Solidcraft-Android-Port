---
name: solidcraft-android-port
description: "Use when working in the Android port (deveshrai/Solidcraft-Android-Port): merging an upstream SolidCraft release, changing an upstream file for the port, adding a port feature or fix, or releasing a port APK."
---

# The Android port: upstream moves freely, the port never breaks

The port is upstream SolidCraft plus an Android app and port-only features (gears, helical threads,
the MCP server on the network). Upstream changes anything it likes; the port's job is to notice what
that touches and prove its own features still work. The long form is `docs/android-sync.md`.

## Rules
- Prefer files upstream doesn't have (`apps/solidcraft-android/`, `crates/engine/src/gear_profile.rs`, `docs/android-*.md`, pending files). Change an upstream file only where a hook is needed, as few lines as possible.
- Every upstream file the port changes is listed in an area of `scripts/android-port-areas.json`, with the tests that prove it; `scripts/android-port.py touchpoints` fails otherwise.
- A port fix in an upstream file gets a regression test in the area's existing test file, and, where a merge could quietly take upstream's version (vendored crates), a `SolidCraft` comment the area lists under `markers`.
- Strings go in `assets/locales/pending/android-port.json`, attribution rows in `attribution/pending/android-port.md`, design-format changes in `crates/doc/src/format/pending/`. Never run `cargo xtask fold-pending` in the port.
- A design saved by a port release goes into `apps/solidcraft-cli/tests/corpus/` (saved by that release's CLI, never regenerated), so every later merge proves it still opens.
- Resolve a conflict by keeping both sides: upstream's change and the port's hook. Never drop upstream code to keep a port change simple.
- Merge upstream release tags (`--latest`), not `main`, for a release; port releases are lettered (0.25.1a, b…) until upstream's next.
- A test that fails on clean upstream too goes under `upstream_failures` in the manifest (exact name, why); `scripts/android-port.py upstream-failures` re-checks the list at each upstream release.
- Pushing and publishing need the owner's go-ahead.

## Routine
1. `scripts/android-sync.sh --preview`: a trial merge of the newest upstream release in a scratch worktree; it prints the conflicts, the port areas each upstream change touches, string clashes and the design-format change. Nothing is changed.
2. `scripts/android-sync.sh --latest`: merges, drops port strings upstream now has (same text), then checks: fmt, clippy for Android and the desktop, wasm, layers, assets, touchpoints, markers, and every area's tests.
3. A failure names its fix. A test filter that matches nothing fails too: a renamed test would otherwise stop guarding.
4. Build and test on a device (`docs/android.md`, Testing); update `docs/android-whats-new.md`, `docs/android-vs-upstream.md` and `docs/android-known-issues.md`; release with `scripts/android-release.sh`.

## Tools
`scripts/android-port.py` (`touchpoints`, `markers`, `strings [--fix]`, `tests [area]`, `report [tag]`, `preview [tag]`) reads `scripts/android-port-areas.json`. When it reports a file under `(no area: register it)`, add it to the area it serves.
