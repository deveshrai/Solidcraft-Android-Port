#!/usr/bin/env bash
# Bring the Android port up to date with upstream SolidCraft, check it and build the APK.
#
#   scripts/android-sync.sh                 fetch upstream, merge upstream/main, check, build the APK
#   scripts/android-sync.sh --to v0.21.0    merge an upstream release tag instead of upstream/main
#   scripts/android-sync.sh --latest        merge the newest upstream release tag
#   scripts/android-sync.sh --preview [TAG] change nothing: a trial merge of TAG (default the newest
#                                           release) in a scratch worktree, its conflicts and what it
#                                           changes in each port area
#   scripts/android-sync.sh --no-build      the same without the APK
#   scripts/android-sync.sh --check-only    no fetch or merge: check the tree as it is (and build)
#   scripts/android-sync.sh --revendor X.Y.Z
#                                           replace vendor/winit with winit X.Y.Z from crates.io and
#                                           re-apply the Android input patch, then check
#
# Remotes: `upstream` is SolidCraft (github.com/tangency-labs/solidcraft), `origin` is this port.
# See docs/android-sync.md for what each step does and what to do when one fails.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
PATCH="apps/solidcraft-android/patches/winit-android-input.patch"
UPSTREAM_REMOTE="${UPSTREAM_REMOTE:-upstream}"
UPSTREAM_BRANCH="${UPSTREAM_BRANCH:-main}"
NDK_TARGET="aarch64-linux-android"

build=1
merge=1
revendor=""
preview=""
to=""
while [ $# -gt 0 ]; do
    case "$1" in
        --no-build) build=0 ;;
        --check-only) merge=0 ;;
        --to) to="${2:?--to needs an upstream tag or commit}"; shift ;;
        --revendor) revendor="${2:?--revendor needs a winit version}"; merge=0; shift ;;
        --latest) to="latest" ;;
        --preview)
            preview="latest"
            if [ $# -gt 1 ] && [[ "$2" != --* ]]; then preview="$2"; shift; fi ;;
        -h | --help) sed -n '2,18p' "$0"; exit 0 ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
    shift
done

step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }
fail() { printf '\n\033[31mFAILED: %s\033[0m\n' "$*" >&2; exit 1; }

# ---- tools -------------------------------------------------------------------------------------
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
    ANDROID_NDK_HOME="$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | sort -V | tail -1 || true)"
    export ANDROID_NDK_HOME
fi
command -v cargo >/dev/null || fail "cargo not found (install Rust: https://rustup.rs)"
cargo ndk --version >/dev/null 2>&1 || fail "cargo-ndk not found (cargo install cargo-ndk)"
[ -d "${ANDROID_NDK_HOME:-}" ] || fail "no Android NDK (set ANDROID_NDK_HOME, or sdkmanager \"ndk;29.0.14206865\")"

PORT="python3 scripts/android-port.py"

# ---- preview -----------------------------------------------------------------------------------
if [ -n "$preview" ]; then
    git fetch -q --tags "$UPSTREAM_REMOTE" "$UPSTREAM_BRANCH"
    [ "$preview" = latest ] && preview=""
    $PORT preview $preview
    exit 0
fi

# ---- merge upstream ----------------------------------------------------------------------------
if [ "$merge" = 1 ]; then
    git diff --quiet && git diff --cached --quiet || fail "uncommitted changes: commit or stash them first"
    git remote get-url "$UPSTREAM_REMOTE" >/dev/null 2>&1 \
        || fail "no '$UPSTREAM_REMOTE' remote (git remote add upstream https://github.com/tangency-labs/solidcraft)"
    step "Fetching $UPSTREAM_REMOTE"
    git fetch --tags "$UPSTREAM_REMOTE" "$UPSTREAM_BRANCH"
    # A release tag (upstream cuts releases on their own line, not always on main) or main.
    if [ "$to" = latest ]; then
        to=$(git tag -l 'v[0-9]*' --sort=-v:refname | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | head -1)
        echo "newest upstream release: $to"
    fi
    target="${to:-$UPSTREAM_REMOTE/$UPSTREAM_BRANCH}"
    git rev-parse -q --verify "$target^{commit}" >/dev/null || fail "no upstream '$target' (tags: git tag -l 'v*')"
    new=$(git rev-list --count "HEAD..$target")
    echo "$new new upstream commit(s) in $target"
    if [ "$new" -gt 0 ]; then
        step "What $target changes in the port's areas"
        $PORT report "$target" || true
        step "Merging $target"
        if ! git merge --no-edit -m "Merge upstream $target" "$target"; then
            echo
            git diff --name-only --diff-filter=U | sed 's/^/  conflict: /'
            fail "merge conflicts: resolve them keeping both sides (docs/android-sync.md, 'Conflicts'), git commit, then run scripts/android-sync.sh --check-only"
        fi
        step "Port strings upstream now has too"
        if ! $PORT strings --fix; then
            fail "a port string id now means something else upstream: rename it (assets/locales/pending/android-port.json)"
        fi
        if ! git diff --quiet -- assets/locales/pending/android-port.json; then
            git commit -q -m "Drop the port's strings upstream $target now has" -- assets/locales/pending/android-port.json
            echo "dropped and committed"
        fi
    fi
fi

# ---- winit -------------------------------------------------------------------------------------
if [ -n "$revendor" ]; then
    step "Re-vendoring winit $revendor"
    tmp="$(mktemp -d)"
    curl -fsSL "https://static.crates.io/crates/winit/winit-$revendor.crate" -o "$tmp/winit.crate" \
        || fail "could not download winit $revendor from crates.io"
    tar -xzf "$tmp/winit.crate" -C "$tmp"
    rm -rf vendor/winit
    mkdir -p vendor/winit
    cp -R "$tmp/winit-$revendor"/{Cargo.toml,Cargo.toml.orig,LICENSE,README.md,build.rs,src} vendor/winit/
    if ! (cd vendor/winit && patch -p1 --no-backup-if-mismatch < "$ROOT/$PATCH"); then
        fail "the Android input patch does not apply to winit $revendor: apply the rejected hunks (vendor/winit/**/*.rej) by hand, then refresh $PATCH (docs/android-sync.md, 'winit')"
    fi
    sed -i.bak -E "s/winit [0-9]+\.[0-9]+\.[0-9]+/winit $revendor/g" vendor/README.md ATTRIBUTION.md && rm -f vendor/README.md.bak ATTRIBUTION.md.bak
    rm -rf "$tmp"
    cargo update -p winit >/dev/null 2>&1 || true
fi

step "Checking that the patched winit is the one in use"
want=$(sed -n 's/^version = "\(.*\)"/\1/p' vendor/winit/Cargo.toml | head -1)
tree=$(cargo tree -p solidcraft-android --target "$NDK_TARGET" -i winit 2>&1 || true)
if ! grep -q "vendor/winit" <<<"$tree"; then
    echo "$tree" | head -5
    req=$(cargo metadata --format-version 1 2>/dev/null | python3 -c 'import json,sys
m=json.load(sys.stdin)
print(next((d["req"] for p in m["packages"] if p["name"]=="eframe" for d in p["dependencies"] if d["name"]=="winit"), "?"))' 2>/dev/null || echo "?")
    fail "the vendored winit ($want) is not used: upstream's eframe now wants winit $req. Run: scripts/android-sync.sh --revendor <that version>"
fi
echo "vendor/winit $want is in use"

# ---- checks ------------------------------------------------------------------------------------
# The quick checks first: a list, a skill or a registry problem shows in seconds, not after
# the builds.
step "Repository checks (layers, assets, lists, skills)"
cargo xtask layers >/dev/null || fail "cargo xtask layers"
cargo xtask assets || fail "cargo xtask assets"
cargo xtask lists || fail "cargo xtask lists: a list is out of order, or a port string clashes with upstream's"
cargo xtask skills || fail "cargo xtask skills: AGENTS.md or a skill (the port's lines count toward upstream's size caps)"

step "Every upstream file the port changes is registered"
$PORT touchpoints || fail "register the file in scripts/android-port-areas.json, with the test that proves it"

step "The port's marked fixes are in place"
$PORT markers || fail "a merge dropped a port fix (docs/android-sync.md, 'The port's speed fixes')"

step "Port strings against upstream's"
$PORT strings || fail "port strings collide with upstream's: scripts/android-port.py strings --fix"

step "Formatting"
cargo fmt --all -- --check || fail "cargo fmt: run 'cargo fmt --all' and commit"

step "Android build (clippy, $NDK_TARGET)"
cargo ndk -t arm64-v8a --platform 26 clippy -p solidcraft-android -- -D warnings || fail "the Android crate does not build or has warnings"

step "Desktop build of the shared crates (clippy)"
cargo clippy -p solidcraft-ui-egui -p solidcraft-android -p solidcraft --all-targets -- -D warnings || fail "clippy"

step "Browser build of the UI crate (wasm32)"
rustup target list --installed | grep -q wasm32-unknown-unknown || rustup target add wasm32-unknown-unknown
cargo check -p solidcraft-ui-egui --target wasm32-unknown-unknown || fail "wasm build"

step "Each port area's tests"
# Gears, helical threads, design files (old port designs still open), the MCP server, agents off
# the window thread, touch input and the kernel's speed fixes; a filter that matches no test fails.
$PORT tests || fail "a port area's tests fail: the area's files are in scripts/android-port-areas.json"

# ---- APK ---------------------------------------------------------------------------------------
if [ "$build" = 1 ]; then
    step "Building the APK"
    (cd apps/solidcraft-android/android && ./gradlew -q assembleDebug) || fail "gradle assembleDebug"
    ls -l apps/solidcraft-android/android/app/build/outputs/apk/debug/SolidCraft-*-debug.apk
fi

step "Done"
if [ "$merge" = 1 ]; then
    echo "Review with 'git log --oneline -20', then publish with: git push origin HEAD"
fi
