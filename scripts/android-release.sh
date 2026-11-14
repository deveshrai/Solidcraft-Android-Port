#!/usr/bin/env bash
# Build the Android APK and publish it as a GitHub release of this port.
#
#   scripts/android-release.sh                 release SolidCraft's version (Cargo.toml): tag
#                                              v<version>-android, APKs SolidCraft-<version>-android-arm64.apk
#                                              (phones, tablets, Arm Googlebooks) and -x86_64.apk (Intel
#                                              Googlebooks)
#   scripts/android-release.sh --revision a    a port-only release of the same SolidCraft version
#                                              (fixes to the port): 0.21.0a, then b, c…; tag
#                                              v<version><letter>-android
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
        --revision)
            r="${2:?--revision needs a letter (a, b, …) or a number (1, 2, …)}"
            case "$r" in
                [a-z]) revision=$(( $(printf '%d' "'$r") - 96 )) ;;
                [0-9] | [12][0-9]) revision="$r" ;;
                *) echo "--revision: a letter a…z or a number 1…26" >&2; exit 2 ;;
            esac
            shift ;;
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

version=$(sed -n 's/^version = "\([0-9.]*\(-rc\.[0-9]*\)\{0,1\}\)"$/\1/p' Cargo.toml | head -1)
[ -n "$version" ] || fail "no version in Cargo.toml"
letter=""
[ "$revision" -gt 0 ] && letter=$(printf "\\$(printf '%03o' $((96 + revision)))")
name="$version$letter"
tag="v$name-android"
apk_name="SolidCraft-$name-android-arm64.apk"
apk_x86="SolidCraft-$name-android-x86_64.apk"
out="target/android-release"

if [ "$dry" = 0 ]; then
    command -v gh >/dev/null || fail "the GitHub CLI is not installed (https://cli.github.com)"
    gh auth status >/dev/null 2>&1 || fail "the GitHub CLI is not logged in: gh auth login"
    git diff --quiet && git diff --cached --quiet || fail "uncommitted changes: commit them first"
    git rev-parse -q --verify "refs/tags/$tag" >/dev/null && fail "tag $tag exists: for a port-only release use --revision a (then b, c…)"
    git fetch -q origin
    [ "$(git rev-parse HEAD)" = "$(git rev-parse '@{upstream}' 2>/dev/null || echo none)" ] \
        || fail "HEAD is not pushed (git push origin HEAD), or the branch tracks no origin branch"
fi

# One APK per ABI, so neither carries the other's libraries: arm64 for phones, tablets and Arm
# Googlebooks, x86_64 for Intel Googlebooks (which would otherwise translate the arm64 code).
mkdir -p "$out"
for abi in arm64-v8a:"$apk_name" x86_64:"$apk_x86"; do
    step "Building SolidCraft $name (${abi%%:*} release APK)"
    (cd apps/solidcraft-android/android && ./gradlew -q assembleRelease -Psolidcraft.abis="${abi%%:*}" -Psolidcraft.portRevision="$revision") \
        || fail "gradle assembleRelease (${abi%%:*})"
    cp "apps/solidcraft-android/android/app/build/outputs/apk/release/SolidCraft-$name-release.apk" "$out/${abi#*:}"
    badging=$("$ANDROID_HOME"/build-tools/*/aapt2 dump badging "$out/${abi#*:}" 2>/dev/null | head -2 || true)
    echo "$badging"
    grep -q "versionName='$name'" <<<"$badging" || fail "the APK's versionName is not $name"
    ls -l "$out/${abi#*:}"
done

# The upstream commit this is built on: upstream's own release tag when it is merged (releases
# are cut on their own line, off main), else the newest commit shared with upstream/main.
if git merge-base --is-ancestor "v$version" HEAD 2>/dev/null; then
    base=$(git rev-parse "v$version^{commit}")
    base_line="[v$version](https://github.com/tangency-labs/solidcraft/releases/tag/v$version) ([\`${base:0:8}\`](https://github.com/tangency-labs/solidcraft/commit/$base))"
else
    base=$(git merge-base HEAD upstream/main 2>/dev/null || true)
    base_line="${base:+[\`${base:0:8}\`](https://github.com/tangency-labs/solidcraft/commit/$base)}"
fi
notes="$out/notes-$name.md"
cat > "$notes" <<EOF
SolidCraft $name for Android and Googlebook OS$( [ "$revision" -gt 0 ] && echo " (SolidCraft $version, port release $letter)").

Built from upstream [SolidCraft](https://github.com/tangency-labs/solidcraft) ${base_line:-$version} with the Android port on top.

**Install:** on an Android 8.0+ phone or tablet, or a Googlebook with a Snapdragon (Arm) chip, download \`$apk_name\`; on a Googlebook with an Intel chip, \`$apk_x86\`. Open it, allowing installs from that source, or \`adb install -r\` it. Signed with a development key: uninstall a build from elsewhere first if Android refuses to update it.

**Works:** the full SolidCraft app (sketches, features, the timeline, the Render workspace) on Vulkan or GLES; mouse and keyboard as on the desktop (hover, selection, navigation buttons, wheel, right-click menu, shortcuts); touch (one finger orbits, tap selects, two fingers pan, pinch zooms, a finger drags a dialog's arrows); Back closes what is open; open, save, import and export through Android's file picker (STEP, STL, 3MF and OBJ tested); the on-screen keyboard for typing. Desktop windows (Samsung DeX, Googlebook OS) with the app's own bar below the window's title bar; an MCP server on the network for AI agents (Settings › MCP server). Designs are upstream's own format: they open in upstream SolidCraft of the same version, and plugins (Preferences › Plugins) install as upstream's do.

**Not yet:** the on-screen keyboard covers the lower half of the screen (the app doesn't move above it), the phone layout is usable but cramped (best on a tablet or a foldable's inner screen), autosave recovery. Known issues, with work-arounds: [docs/android-known-issues.md](https://github.com/deveshrai/Solidcraft-Android-Port/blob/$tag/docs/android-known-issues.md). How to use it: [docs/android-touch.md](https://github.com/deveshrai/Solidcraft-Android-Port/blob/$tag/docs/android-touch.md); details: [docs/android.md](https://github.com/deveshrai/Solidcraft-Android-Port/blob/$tag/docs/android.md).
EOF
echo "notes: $notes"

if [ "$dry" = 1 ]; then
    step "Dry run: nothing published"
    exit 0
fi

step "Tagging $tag and publishing the release"
git tag -a "$tag" -m "SolidCraft $name for Android and Googlebook OS"
git push origin "$tag"
# The port's own repository (origin), not upstream's: OWNER/REPO from the remote's URL.
repo=$(git remote get-url origin | sed -E 's#^(https://github\.com/|git@github\.com:)##; s#\.git$##')
# An upstream pre-release (x.y.z-rc.n) is published as one here too.
prerelease=()
[[ "$version" == *-rc.* ]] && prerelease=(--prerelease)
gh release create "$tag" "$out/$apk_name" "$out/$apk_x86" --repo "$repo" "${prerelease[@]}" \
    --title "SolidCraft $name for Android and Googlebook OS" --notes-file "$notes"
