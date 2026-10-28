#!/usr/bin/env python3
"""The Android port's merge guard: what the port changes in upstream SolidCraft, and proof it still works.

    scripts/android-port.py touchpoints        every upstream file the port changes is in an area
    scripts/android-port.py markers            the marked port fixes are still in their files
    scripts/android-port.py strings [--fix]    port UI strings that upstream now has too
    scripts/android-port.py tests [AREA...]    each area's tests (all areas without a name)
    scripts/android-port.py report [TARGET]    what an upstream release or commit changes, by area
    scripts/android-port.py preview [TARGET]   a trial merge in a scratch worktree: conflicts and report
    scripts/android-port.py upstream-failures [TARGET]
                                               the tests skipped as upstream's own failures still fail on
                                               a clean upstream checkout (slow: builds it)

TARGET defaults to the newest upstream release tag. The areas are in scripts/android-port-areas.json;
docs/android-sync.md ("How the port stays mergeable") says how to use this and what to do when a
check fails. Exit status 1 when a check fails."""
import fnmatch
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
AREAS = os.path.join(ROOT, "scripts", "android-port-areas.json")
PENDING_STRINGS = "assets/locales/pending/android-port.json"
UPSTREAM = os.environ.get("UPSTREAM_REMOTE", "upstream")


def git(*args, cwd=ROOT, check=True):
    r = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True)
    if check and r.returncode != 0:
        sys.exit(f"git {' '.join(args)}: {r.stderr.strip()}")
    return r.stdout


def manifest():
    with open(AREAS, encoding="utf-8") as f:
        return json.load(f)


def areas():
    return manifest()["areas"]


def upstream_failures(pkg=None):
    """Tests that fail on a clean upstream checkout too: skipped in the areas' runs."""
    return [u for u in manifest().get("upstream_failures", []) if pkg is None or u["package"] == pkg]


def bold(s):
    return f"\033[1m{s}\033[0m" if sys.stdout.isatty() else s


def red(s):
    return f"\033[31m{s}\033[0m" if sys.stdout.isatty() else s


def last_upstream(rev="HEAD"):
    """The upstream commit last merged into `rev`: the second parent of the newest upstream merge."""
    out = git("log", "--merges", "--grep=^Merge upstream", "-1", "--format=%P", rev).split()
    if len(out) < 2:
        sys.exit("no 'Merge upstream …' commit found: merge upstream with scripts/android-sync.sh")
    return out[1]


def newest_release():
    tags = git("tag", "-l", "v[0-9]*", "--sort=-v:refname").split()
    tags = [t for t in tags if re.fullmatch(r"v\d+\.\d+\.\d+", t)]
    if not tags:
        sys.exit(f"no upstream release tags: git fetch --tags {UPSTREAM}")
    return tags[0]


def upstream_has(rev, path):
    return subprocess.run(["git", "cat-file", "-e", f"{rev}:{path}"], cwd=ROOT, capture_output=True).returncode == 0


def area_of(path):
    for a in areas():
        if any(fnmatch.fnmatch(path, g) for g in a.get("files", [])):
            return a["name"]
    return None


# ---- touchpoints --------------------------------------------------------------------------------
def touched(base, rev="HEAD", cwd=ROOT):
    """Upstream files (they exist at `base`) that `rev` changes."""
    names = git("diff", "--name-only", base, rev, cwd=cwd).split("\n")
    return [n for n in names if n and upstream_has(base, n)]


def cmd_touchpoints(_args):
    base = last_upstream()
    files = touched(base)
    loose = [f for f in files if area_of(f) is None]
    print(f"{len(files)} upstream files carry port changes (against {git('log', '-1', '--format=%h %s', base).strip()})")
    listed = {g for a in areas() for g in a.get("files", [])}
    stale = sorted(g for g in listed if not any(fnmatch.fnmatch(f, g) for f in files))
    for g in stale:
        print(f"  note: {g} is listed but the port no longer changes it (remove it from {os.path.relpath(AREAS, ROOT)})")
    if loose:
        print(red("Upstream files the port changes that no area lists:"))
        for f in loose:
            print(f"  {f}")
        print(f"Add each to the area it belongs to in {os.path.relpath(AREAS, ROOT)}, with a test that proves it (docs/android-sync.md).")
        return 1
    print("every one is in an area")
    return 0


# ---- markers ------------------------------------------------------------------------------------
def cmd_markers(_args):
    bad = 0
    for a in areas():
        for m in a.get("markers", []):
            path = os.path.join(ROOT, m["file"])
            try:
                with open(path, encoding="utf-8") as f:
                    n = len(re.findall(m["pattern"], f.read()))
            except OSError:
                n = 0
            ok = n >= m["count"]
            bad += not ok
            print(f"  {'ok ' if ok else red('MISSING')} {a['name']}: {m['file']} ({n} of {m['count']})")
    if bad:
        print(red("A port fix is gone from its file: a merge took upstream's version. Re-apply it (docs/android-sync.md, 'The port's speed fixes')."))
    return 1 if bad else 0


# ---- strings ------------------------------------------------------------------------------------
def cmd_strings(args, cwd=ROOT):
    """Port strings upstream now has too. The same text: ours is dropped (with --fix). Different
    text for the same id: upstream's wins at build time, so the port's must be renamed or dropped."""
    fix = "--fix" in args
    path = os.path.join(cwd, PENDING_STRINGS)
    with open(path, encoding="utf-8") as f:
        pending = json.load(f)
    same, differ = [], []
    for lang, table in pending.items():
        if not isinstance(table, dict):
            continue
        catalog = os.path.join(cwd, "assets", "locales", f"{lang}.json")
        upstream = {}
        if os.path.exists(catalog):
            with open(catalog, encoding="utf-8") as f:
                upstream = json.load(f)
        for other in sorted(os.listdir(os.path.join(cwd, "assets", "locales", "pending"))):
            if other.endswith(".json") and other != os.path.basename(PENDING_STRINGS):
                with open(os.path.join(cwd, "assets", "locales", "pending", other), encoding="utf-8") as f:
                    upstream.update(json.load(f).get(lang, {}))
        for key, text in table.items():
            if key in upstream:
                (same if upstream[key] == text else differ).append((lang, key, text, upstream[key]))
    for lang, key, text, _ in same:
        print(f"  upstream has it too, the same: {lang} {key!r}" + (" (dropped from the port's file)" if fix else ""))
    for lang, key, text, up in differ:
        print(red(f"  upstream has it with other text: {lang} {key!r}: port {text!r}, upstream {up!r}"))
    if fix and same:
        for lang, key, _, _ in same:
            pending[lang].pop(key, None)
        with open(path, "w", encoding="utf-8") as f:
            json.dump(pending, f, ensure_ascii=False, indent=2)
            f.write("\n")
    if differ:
        print("Rename the port's id (and its uses) or take upstream's text, in " + PENDING_STRINGS)
        return 1
    if same and not fix:
        print("Run with --fix to drop the port's copies.")
        return 1
    if not same:
        print("  no port string collides with upstream's")
    return 0


# ---- tests --------------------------------------------------------------------------------------
def run_test(spec, cwd=ROOT, env=None):
    """`package [--lib|--test NAME] [filters…]`, via cargo test, skipping upstream's own
    failures; (ok, tests run)."""
    words = spec.split()
    pkg, rest = words[0], words[1:]
    kind, filters = [], rest
    if rest and rest[0] == "--lib":
        kind, filters = ["--lib"], rest[1:]
    elif rest and rest[0] == "--test":
        kind, filters = rest[:2], rest[2:]
    skips = [w for u in upstream_failures(pkg) for w in ("--skip", u["test"])]
    cmd = ["cargo", "test", "-q", "-p", pkg, *kind, "--", *filters, *skips]
    r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, env=env)
    out = r.stdout + r.stderr
    passed = sum(int(n) for n in re.findall(r"test result: \w+\. (\d+) passed", out))
    failed = re.findall(r"^test (\S+) \.\.\. FAILED", out, flags=re.M)
    if r.returncode != 0 and not failed:
        tail = "\n".join(out.strip().splitlines()[-15:])
        print(tail)
    for t in failed:
        print(red(f"      FAILED {t}"))
    return r.returncode == 0, passed


def cmd_tests(args):
    want = [a for a in args if not a.startswith("-")]
    bad = 0
    for a in areas():
        if want and not any(w.lower() in a["name"].lower() for w in want):
            continue
        print(bold(f"  {a['name']}"))
        for spec in a.get("tests", []):
            ok, n = run_test(spec)
            if ok and n == 0:
                ok = False
                print(red(f"    no tests matched: {spec} (renamed or removed? fix the filter in {os.path.relpath(AREAS, ROOT)})"))
            else:
                print(f"    {'ok' if ok else red('FAILED')}  {n:4d} tests  {spec}")
            bad += not ok
    return 1 if bad else 0


# ---- report and preview -------------------------------------------------------------------------
def releases_between(old, new):
    """Upstream release tags after `old`, up to and including `new`."""
    tags = git("tag", "-l", "v[0-9]*", "--sort=v:refname", "--contains", old, "--merged", new).split()
    old_sha = git("rev-parse", old).strip()
    return [t for t in tags if re.fullmatch(r"v\d+\.\d+\.\d+", t) and git("rev-parse", f"{t}^{{commit}}").strip() != old_sha]


def format_of(rev):
    """The design format a revision writes: the newest folded v<N>/ folder, plus one with pending files."""
    names = git("ls-tree", "--name-only", f"{rev}:crates/doc/src/format", check=False).split()
    folded = max((int(n[1:]) for n in names if re.fullmatch(r"v\d+", n)), default=0)
    pending = [p for p in git("ls-tree", "--name-only", f"{rev}:crates/doc/src/format/pending", check=False).split() if p.endswith(".rs")]
    return folded, pending


def report(old, new, conflicts=None):
    print(bold(f"Upstream {git('log', '-1', '--format=%h', old).strip()} → {new}"))
    count = git("rev-list", "--count", f"{old}..{new}").strip()
    rel = releases_between(old, new)
    print(f"  {count} upstream commits" + (f"; releases: {', '.join(rel)}" if rel else ""))
    up_folded, up_pending = format_of(new)
    port_folded, port_pending = format_of("HEAD")
    if up_folded != port_folded:
        print(f"  design format: upstream now folds to {up_folded} (the port was on {port_folded}); the port will write {up_folded + 1} with its pending changes")
    if up_pending:
        print(f"  upstream has pending format changes of its own: {', '.join(up_pending)}")
    changed = set(git("diff", "--name-only", old, new).split())
    mine = set(touched(old))
    both = sorted(changed & mine)
    print(f"  {len(changed)} files changed upstream; {len(both)} of them carry port changes:")
    by_area = {}
    for f in both:
        by_area.setdefault(area_of(f) or "(no area: register it)", []).append(f)
    for name, files in sorted(by_area.items()):
        print(f"    {name}:")
        for f in files:
            mark = "  CONFLICT" if conflicts and f in conflicts else ""
            print(f"      {f}{mark}")
    if conflicts:
        others = [c for c in conflicts if c not in both]
        for c in others:
            print(f"    CONFLICT {c}")
    deps = [f for f in changed if f.startswith("apps/solidcraft/src/control_server.rs") or f.endswith("Cargo.toml")]
    if deps:
        print("  check: files the Android app builds from or depends on changed: " + ", ".join(sorted(deps)))


def cmd_report(args):
    target = args[0] if args else newest_release()
    report(last_upstream(), target)
    return 0


def cmd_preview(args):
    target = args[0] if args else newest_release()
    tmp = tempfile.mkdtemp(prefix="sc-preview-")
    try:
        git("worktree", "add", "--detach", tmp, "HEAD")
        r = subprocess.run(["git", "merge", "--no-commit", "--no-ff", target], cwd=tmp, capture_output=True, text=True)
        conflicts = [c for c in git("diff", "--name-only", "--diff-filter=U", cwd=tmp).split("\n") if c]
        report(last_upstream(), target, conflicts)
        print(bold("  strings after the merge:"))
        cmd_strings([], cwd=tmp)
        if r.returncode == 0:
            print(f"  the merge applies without conflicts")
        else:
            print(f"  {len(conflicts)} conflict(s) to resolve")
    finally:
        subprocess.run(["git", "merge", "--abort"], cwd=tmp, capture_output=True)
        git("worktree", "remove", "--force", tmp, check=False)
        shutil.rmtree(tmp, ignore_errors=True)
    return 0


def cmd_upstream_failures(args):
    """Run the listed upstream failures on a clean checkout of TARGET: each must still fail there.
    One that passes there is fixed upstream: drop it from the list (and so test it again)."""
    target = args[0] if args else last_upstream()
    tmp = tempfile.mkdtemp(prefix="sc-upstream-")
    target_dir = os.environ.get("UPSTREAM_TARGET_DIR", os.path.join(tempfile.gettempdir(), "sc-upstream-target"))
    bad = 0
    try:
        git("worktree", "add", "--detach", tmp, target)
        env = dict(os.environ, CARGO_TARGET_DIR=target_dir)
        for u in upstream_failures():
            r = subprocess.run(["cargo", "test", "-q", "-p", u["package"], "--lib", "--", u["test"]], cwd=tmp, capture_output=True, text=True, env=env)
            out = r.stdout + r.stderr
            failed = re.findall(r"^test (\S+) \.\.\. FAILED", out, flags=re.M)
            if failed:
                print(f"  still fails upstream ({len(failed)}): {u['test']}")
            else:
                bad += 1
                print(red(f"  passes on upstream {target}: {u['test']}: remove it from upstream_failures in {os.path.relpath(AREAS, ROOT)}"))
    finally:
        git("worktree", "remove", "--force", tmp, check=False)
        shutil.rmtree(tmp, ignore_errors=True)
    return 1 if bad else 0


def main(argv):
    if len(argv) < 2 or argv[1] in ("-h", "--help"):
        print(__doc__)
        return 0
    cmds = {"touchpoints": cmd_touchpoints, "markers": cmd_markers, "strings": cmd_strings, "tests": cmd_tests, "report": cmd_report, "preview": cmd_preview, "upstream-failures": cmd_upstream_failures}
    if argv[1] not in cmds:
        print(__doc__)
        return 2
    return cmds[argv[1]](argv[2:])


if __name__ == "__main__":
    sys.exit(main(sys.argv))
