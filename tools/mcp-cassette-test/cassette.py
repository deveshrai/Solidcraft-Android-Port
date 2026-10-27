"""A compact-cassette housing built over SolidCraft's MCP server: two halves, five M2 screws with
helical B-rep threads, and the checks that they fit. Every call is timed (see README.md).

    MCP_URL=http://PHONE:27880/mcp MCP_TOKEN=... ADB_SERIAL=... python3 cassette.py NAME
"""
import mcp, json, time, base64, sys, os
HERE = os.path.dirname(os.path.abspath(__file__))
mcp.init()
L, W, T, WALL, S = 100.4, 63.8, 8.7, 1.2, 4.35
SCREWS = [(-46.2, 4.0), (46.2, 4.0), (-46.2, 59.8), (46.2, 59.8), (0.0, 17.0)]
B, A = "Bottom (side B)", "Top (side A)"
F = "/storage/emulated/0/Android/data/io.solidcraft.android/files/Agent/"
DEV = sys.argv[1] if len(sys.argv) > 1 else "emu"
stage_t = {}
def stage(name):
    stage_t[name] = time.time(); print(f"\n== {name}", flush=True)
def shot(name):
    r = mcp.tool("screenshot", {}, label=f"screenshot {name}")
    open(f"{DEV}-{name}.png", "wb").write(base64.b64decode(r["_images"][0]["data"]))
t_all = time.time()
corners = lambda z: [[-L/2, 0, z], [L/2, 0, z], [-L/2, W, z], [L/2, W, z]]
plane = {"origin": [0, 0, S], "x_dir": [1, 0, 0], "y_dir": [0, 1, 0]}

stage("1 bottom half")
mcp.tool("new_design", {"name": "Cassette housing"}, label="new design")
mcp.ex("solid.box", {"length": L, "width": W, "height": S, "corner": [-L/2, 0, 0], "body_name": B}, label="B: block 100.4x63.8x4.35")
mcp.ex("solid.fillet", {"edges": corners(S/2), "radius": 1.5, "body": B}, label="B: R1.5 corners")
mcp.ex("solid.box", {"length": L-2*WALL, "width": W-2*WALL, "height": 4.0, "corner": [-L/2+WALL, WALL, WALL], "operation": "cut", "targets": [B]}, label="B: cavity, 1.2 walls")
for i, (x, y) in enumerate(SCREWS):
    mcp.ex("solid.cylinder", {"diameter": 4.4, "height": S-WALL, "base": [x, y, WALL], "operation": "join", "targets": [B]}, label=f"B: screw boss {i+1}")
mcp.ex("sketch.create", {"plane": plane, "name": "Lip"}, label="B: sketch lip")
mcp.ex("sketch.rectangle.rounded", {"p0": [-L/2+0.6, 0.6], "p1": [L/2-0.6, W-0.6], "radius": 0.9}, label="B: lip outer")
mcp.ex("sketch.rectangle.two_point", {"p0": [-L/2+WALL, WALL], "p1": [L/2-WALL, W-WALL]}, label="B: lip inner")
mcp.ex("sketch.finish", {}, label="B: finish sketch")
mcp.ex("solid.extrude", {"distance": 0.8, "operation": "join", "targets": [B]}, label="B: extrude lip 0.8")

stage("2 top half")
mcp.ex("solid.box", {"length": L, "width": W, "height": T-S, "corner": [-L/2, 0, S], "body_name": A, "operation": "new"}, label="A: block 100.4x63.8x4.35")
mcp.ex("solid.fillet", {"edges": corners(S + (T-S)/2), "radius": 1.5, "body": A}, label="A: R1.5 corners")
mcp.ex("solid.box", {"length": L-2*WALL, "width": W-2*WALL, "height": 3.4, "corner": [-L/2+WALL, WALL, T-WALL-3.4], "operation": "cut", "targets": [A]}, label="A: cavity, 1.2 walls")
for i, (x, y) in enumerate(SCREWS):
    mcp.ex("solid.cylinder", {"diameter": 4.4, "height": T-WALL-S, "base": [x, y, S], "operation": "join", "targets": [A]}, label=f"A: screw boss {i+1}")
mcp.ex("sketch.create", {"plane": plane, "name": "Rabbet"}, label="A: sketch rabbet")
mcp.ex("sketch.rectangle.rounded", {"p0": [-L/2+0.5, 0.5], "p1": [L/2-0.5, W-0.5], "radius": 1.0}, label="A: rabbet outer")
mcp.ex("sketch.rectangle.two_point", {"p0": [-L/2+1.4, 1.4], "p1": [L/2-1.4, W-1.4]}, label="A: rabbet inner")
mcp.ex("sketch.finish", {}, label="A: finish sketch")
mcp.ex("solid.extrude", {"distance": 0.9, "operation": "cut", "targets": [A]}, label="A: cut rabbet 0.9")

stage("3 reels, window, label, edge openings")
for body, tag in ((B, "B"), (A, "A")):
    for x in (-21.25, 21.25):
        mcp.ex("solid.cylinder", {"diameter": 13, "height": T+1, "base": [x, 33, -0.5], "operation": "cut", "targets": [body]}, label=f"{tag}: hub hole x={x}")
    mcp.ex("solid.box", {"length": 24, "width": 9, "height": T+1, "center": [0, 33, T/2], "operation": "cut", "targets": [body]}, label=f"{tag}: tape window")
mcp.ex("solid.box", {"length": 80, "width": 38, "height": 0.5, "center": [0, 34, T], "operation": "cut", "targets": [A]}, label="A: label recess 0.25")
for body, tag, zc in ((B, "B", 1.25 + 4.2/2), (A, "A", 7.45 - 4.2/2)):
    for cx, w, cy in [(0, 16, 0.6), (-26, 7, 0.6), (26, 7, 0.6), (-40, 3.5, W-0.6), (40, 3.5, W-0.6)]:
        mcp.ex("solid.box", {"length": w, "width": 2.6, "height": 4.2, "center": [cx, cy, zc], "operation": "cut", "targets": [body]}, label=f"{tag}: edge opening x={cx}" + (" back" if cy > 30 else ""))

stage("3b tape-path block (12 mm thick there) and its holes")
BUMP, BF, BB, BD = 1.65, 33.0, 29.0, 12.0   # height per side, half widths front/back, depth
for body, tag, z, sign in ((B, "B", 0.0, "negative"), (A, "A", T, "positive")):
    mcp.ex("sketch.create", {"plane": {"origin": [0, 0, z], "x_dir": [1, 0, 0], "y_dir": [0, 1, 0]}, "name": f"Tape-path block {tag}"}, label=f"{tag}: sketch tape-path block")
    mcp.ex("sketch.line", {"points": [[-BF, 0], [BF, 0], [BB, BD], [-BB, BD]], "closed": True}, label=f"{tag}: trapezoid 66/58 x 12")
    mcp.ex("sketch.finish", {}, label=f"{tag}: finish sketch")
    mcp.ex("solid.extrude", {"distance": BUMP, "direction": sign, "operation": "join", "targets": [body]}, label=f"{tag}: extrude block 1.65 outward")
for body, tag in ((B, "B"), (A, "A")):
    for x, d, what in ((-26, 2.6, "capstan"), (26, 2.6, "capstan"), (-11, 3.0, "locating"), (11, 3.0, "locating")):
        mcp.ex("solid.cylinder", {"diameter": d, "height": T + 2*BUMP + 2, "base": [x, 6.0, -BUMP - 1], "operation": "cut", "targets": [body]}, label=f"{tag}: {what} hole x={x}")
mcp.ex("view.isometric", {}, label="view iso"); shot("1-halves")

stage("4 screw holes and tapped threads")
for i, (x, y) in enumerate(SCREWS):
    mcp.ex("solid.hole", {"position": [x, y, 0], "direction": [0, 0, 1], "diameter": 2.4, "type": "counterbore", "cb_diameter": 4.0, "cb_depth": 1.4, "targets": [B]}, label=f"B: counterbored hole {i+1}")
for i, (x, y) in enumerate(SCREWS):
    mcp.ex("solid.hole", {"position": [x, y, S], "direction": [0, 0, 1], "diameter": 1.85, "depth": 2.9, "targets": [A]}, label=f"A: M2 tap drill {i+1} (1.85 for 0.05 clearance)")
for i, (x, y) in enumerate(SCREWS):
    mcp.ex("solid.thread", {"face": [x + 0.925, y, 5.8], "designation": "M2", "helical": True, "clearance": "0.05 mm"}, label=f"A: helical M2 internal thread {i+1}")

stage("5 screw")
x0, y0 = SCREWS[0]
s1 = "Screw 1"
# Made clear of the housing, then put in its hole: a thread's face is picked by a point, and in
# the hole that point is on the tapped thread too.
sx, sy = x0, -15.0
mcp.ex("solid.cylinder", {"diameter": 3.6, "height": 1.3, "base": [sx, sy, 0.1], "body_name": s1, "operation": "new"}, label="screw: pan head 3.6x1.3")
mcp.ex("solid.cylinder", {"diameter": 2.0, "height": 5.5, "base": [sx, sy, 1.4], "operation": "join", "targets": [s1]}, label="screw: shank 2.0x5.5")
mcp.ex("solid.box", {"length": 2.4, "width": 0.5, "height": 0.9, "center": [sx, sy, 0.45], "operation": "cut", "targets": [s1]}, label="screw: cross recess 1")
mcp.ex("solid.box", {"length": 0.5, "width": 2.4, "height": 0.9, "center": [sx, sy, 0.45], "operation": "cut", "targets": [s1]}, label="screw: cross recess 2")
mcp.ex("solid.chamfer", {"edges": [[sx + 1.0, sy, 6.9]], "distance": 0.2}, label="screw: tip chamfer 0.2")
mcp.ex("solid.thread", {"face": [sx + 1.0, sy, 6.0], "designation": "M2", "helical": True, "length": 3.5}, label="screw: helical M2 thread 3.5 long")
mcp.ex("solid.move", {"bodies": [s1], "translate": [x0 - sx, y0 - sy, 0]}, label="screw: into hole 1")

stage("6 fit: turn the screw until the threads mesh (checked on the meshes)")
import subprocess
ADB = ["adb", "-s", ADB_SERIAL] if (ADB_SERIAL := os.environ.get("ADB_SERIAL")) else ["adb"]
def mesh_fit(screw, sx, sy, tag):
    for body, f in ((screw, "fit-screw.stl"), (A, "fit-top.stl")):
        mcp.ex("file.export", {"path": F + f, "bodies": [body], "quality": "finest"}, label=f"export {body} STL for fit check")
        subprocess.run(ADB + ["pull", F + f, f], check=True, capture_output=True)
    t = time.time()
    out = subprocess.run([sys.executable, os.path.join(HERE, "fit.py"), "fit-screw.stl", "fit-top.stl", str(sx), str(sy)], capture_output=True, text=True).stdout
    best = [l for l in out.splitlines() if l.startswith("BEST")][0]; built = [l for l in out.splitlines() if l.startswith("AS BUILT")][0]
    ang = int(best.split()[2]); gap_best = float(best.split()[-2]); gap_built = float(built.split()[-2])
    print(f"   {tag}: as placed min radial gap {gap_built:+.4f} mm; best turn {ang} deg -> {gap_best:+.4f} mm ({time.time()-t:.1f} s on the Mac)", flush=True)
    return ang, gap_best, gap_built
def turn(screw, sx, sy, deg):
    mcp.ex("solid.move", {"bodies": [screw], "translate": [-sx, -sy, 0]}, label=f"{screw}: to origin")
    mcp.ex("solid.move", {"bodies": [screw], "axis": [0, 0, 1], "angle": f"{deg} deg"}, label=f"{screw}: turn {deg} deg")
    mcp.ex("solid.move", {"bodies": [screw], "translate": [sx, sy, 0]}, label=f"{screw}: back into its hole")
t_fit = time.time()
ang, g, g0 = mesh_fit(s1, x0, y0, "screw 1")
if g0 < g - 1e-4:
    turn(s1, x0, y0, -ang)  # the app turns the other way from fit.py
    _, g2, g_now = mesh_fit(s1, x0, y0, "screw 1 after turning")
else:
    g_now = g0
fit_ok = g_now > 0
with open(mcp.LOG, "a") as f:
    f.write(json.dumps({"label": f"FIT: screw 1 turned {ang} deg, min radial gap to the tapped thread {g_now:+.4f} mm", "tool": "fit", "s": round(time.time()-t_fit, 2), "ok": fit_ok, "error": "" if fit_ok else "threads overlap"}) + "\n")

stage("7 four more screws, components")
names = [s1]
for i, (x, y) in enumerate(SCREWS[1:], start=2):
    r = mcp.ex("solid.move", {"bodies": [s1], "translate": [x - x0, y - y0, 0], "copy": True}, label=f"copy screw to position {i}")
    new = (r.get("created_bodies") or [None])[0]
    if new:
        mcp.ex("body.rename", {"body": new, "name": f"Screw {i}"}, label=f"rename screw {i}"); names.append(f"Screw {i}")
mcp.ex("component.from_bodies", {"bodies": [B, A] + names}, label="components from 7 bodies")

stage("8 checks")
for k, (x, y) in enumerate(SCREWS[1:], start=2):
    if f"Screw {k}" in names:
        _, _, gk = mesh_fit(f"Screw {k}", x, y, f"screw {k} as copied")
        with open(mcp.LOG, "a") as f:
            f.write(json.dumps({"label": f"FIT: screw {k}, min radial gap {gk:+.4f} mm", "tool": "fit", "s": 0, "ok": gk > 0, "error": ""}) + "\n")
r = mcp.ex("inspect.interference", {"bodies": [B, A]}, label="interference: bottom vs top half")
print("   halves interferences:", r.get("interferences"), "unchecked:", r.get("unchecked"), flush=True)
r = mcp.ex("inspect.clearance", {"bodies": [B], "against": [A], "clearance": 0.2}, label="clearance: bottom vs top (lip/rabbet)")
print("   halves:", json.dumps({k: r.get(k) for k in ("overlaps", "near", "unchecked")})[:500], flush=True)
r = mcp.ex("inspect.clearance", {"bodies": names, "against": [B], "clearance": 0.3}, label="clearance: 5 screws vs bottom half (heads seated)")
print("   screws/bottom:", json.dumps({k: r.get(k) for k in ("overlaps", "near", "unchecked")})[:900], flush=True)
d = mcp.tool("inspect_design", {"measure": True}, label="inspect + measure design")
for b in d["bodies"]:
    bb = b["bbox"]; size = [round(bb["max"][i]-bb["min"][i], 3) for i in range(3)]
    print(f"   {b['name']:18s} volume {b['volume_mm3']:9.2f} mm3  size {size}  closed {b['closed']}  faces {b['faces']}", flush=True)
mcp.ex("inspect.section", {"plane": "YZ", "at": x0}, label="section through screw 1 (YZ)")
mcp.ex("view.apply", {"camera": {"target": [x0, y0, 4.5], "yaw": -1.5708, "pitch": 0.0, "distance": 14}}, label="camera close on screw 1"); shot("2-section-screw")
mcp.ex("inspect.section", {"clear": True}, label="clear section")
mcp.ex("view.isometric", {}, label="view iso"); shot("3-assembly")

stage("9 export and save")
mcp.tool("export", {"path": F + "cassette-housing.step"}, label="export STEP (7 parts)")
mcp.tool("export", {"path": F + "cassette-housing.3mf"}, label="export 3MF")
mcp.tool("save", {"path": F + "cassette-housing.solidcraft"}, label="save design")
print(f"\nTOTAL {time.time()-t_all:.1f} s", flush=True)
