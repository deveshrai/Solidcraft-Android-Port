#!/usr/bin/env bash
# Build the Android APK and publish it as a GitHub release of this port.
#
#   scripts/android-release.sh                 release SolidCraft's version (Cargo.toml): tag
#                                              v<version>-android, APK SolidCraft-<version>-android-arm64.apk
#   scripts/android-release.sh --revision N    a port-only release of the same SolidCraft version
#                                              (fixes to the port): v<version>-android.N
#   scripts/android-release.sh --dry-run       build the APK and write the notes, publish nothing
#
# Needs the GitHub CLI logged in (gh auth login) and the commit pushed to `origin`.
# See docs/android-sync.md, "Releasing".
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
revision=0
dry=0
while [ $# -gt 0 ]; do
    case "$1" in
        --revision) revision="${2:?--revision needs a number}"; shift ;;
        --dry-run) dry=1 ;;
        -h | --help) sed -n '2,11p' "$0"; exit 0 ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
    shift
done

fail() { printf '\n\033[31mFAILED: %s\033[0m\n' "$*" >&2; exit 1; }
step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"

version=$(sed -n 's/^version = "\([0-9.]*\)"$/\1/p' Cargo.toml | head -1)
[ -n "$version" ] || fail "no version in Cargo.toml"
name="$version"
[ "$revision" -gt 0 ] && name="$version-android.$revision"
tag="v$version-android"
[ "$revision" -gt 0 ] && tag="v$name"
apk_name="SolidCraft-$name-android-arm64.apk"
out="target/android-release"

if [ "$dry" = 0 ]; then
    command -v gh >/dev/null || fail "the GitHub CLI is not installed (https://cli.github.com)"
    gh auth status >/dev/null 2>&1 || fail "the GitHub CLI is not logged in: gh auth login"
    git diff --quiet && git diff --cached --quiet || fail "uncommitted changes: commit them first"
    git rev-parse -q --verify "refs/tags/$tag" >/dev/null && fail "tag $tag exists: for a port-only release use --revision N"
    git fetch -q origin
    [ "$(git rev-parse HEAD)" = "$(git rev-parse '@{upstream}' 2>/dev/null || echo none)" ] \
        || fail "HEAD is not pushed (git push origin HEAD), or the branch tracks no origin branch"
fi

step "Building SolidCraft $name (arm64 release APK)"
(cd apps/solidcraft-android/android && ./gradlew -q assembleRelease -Psolidcraft.abis=arm64-v8a -Psolidcraft.portRevision="$revision") \
    || fail "gradle assembleRelease"
mkdir -p "$out"
cp "apps/solidcraft-android/android/app/build/outputs/apk/release/SolidCraft-$name-release.apk" "$out/$apk_name"
badging=$("$ANDROID_HOME"/build-tools/*/aapt2 dump badging "$out/$apk_name" 2>/dev/null | head -1 || true)
echo "$badging"
grep -q "versionName='$name'" <<<"$badging" || fail "the APK's versionName is not $name"
ls -l "$out/$apk_name"

# The upstream commit this is built on: the newest one both histories share.
base=$(git merge-base HEAD upstream/main 2>/dev/null || true)
base_line="${base:+[\`${base:0:8}\`](https://github.com/tangency-labs/solidcraft/commit/$base)}"
notes="$out/notes-$name.md"
cat > "$notes" <<EOF
SolidCraft $version for Android${revision:+$( [ "$revision" -gt 0 ] && echo " (port revision $revision)")}.

Built from upstream [SolidCraft](https://github.com/tangency-labs/solidcraft) ${base_line:-$version} with the Android port on top.

**Install:** download \`$apk_name\` on an Android 8.0+ phone or tablet (arm64) and open it, allowing installs from that source; or \`adb install -r $apk_name\`. Signed with a development key: uninstall a build from elsewhere first if Android refuses to update it.

**Works:** the full SolidCraft app (sketches, features, the timeline) on Vulkan or GLES; mouse and keyboard as on the desktop (hover, selection, navigation buttons, wheel, right-click menu, shortcuts); touch (one finger orbits, tap selects, two fingers pan, pinch zooms); open, save, import and export through Android's file picker (STEP and STL tested; the other formats take the same path).

**Not yet:** the on-screen keyboard (use a hardware keyboard for text), a phone-sized layout (best on a tablet), autosave recovery. Details: [docs/android.md](https://github.com/deveshrai/Solidcraft-Android-Port/blob/$tag/docs/android.md).
EOF
echo "notes: $notes"

if [ "$dry" = 1 ]; then
    step "Dry run: nothing published"
    exit 0
fi

step "Tagging $tag and publishing the release"
git tag -a "$tag" -m "SolidCraft $name for Android"
git push origin "$tag"
# The port's own repository (origin), not upstream's: OWNER/REPO from the remote's URL.
repo=$(git remote get-url origin | sed -E 's#^(https://github\.com/|git@github\.com:)##; s#\.git$##')
gh release create "$tag" "$out/$apk_name" --repo "$repo" \
    --title "SolidCraft $name for Android" --notes-file "$notes"
