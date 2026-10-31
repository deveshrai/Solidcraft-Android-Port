"""A Clockwork DevTerm-like enclosure over MCP: back shell, front frame and the eight M2 screws that
hold them together, every call timed. Sizes estimated from product photos (scale from the 6.86"
1280x480 screen, ~0.305 mm/px). X = width (218), Y = up the face (0 at the keyboard edge), Z =
thickness (0 = rear of the keyboard half, the frame's face at 23).

    MCP_URL=... MCP_TOKEN=... MCP_LOG=... python3 devterm_enclosure.py NAME   (or: DRY=script.json)
"""
import sys, os, time, json, base64
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
DRY = os.environ.get("DRY")
if not DRY:
    import mcp
    mcp.init()
DEV = sys.argv[1] if len(sys.argv) > 1 else "dev"
F = "/storage/emulated/0/Android/data/io.solidcraft.android/files/Agent/"
script, fails = [], []
def ex(cmd, params, label):
    if DRY:
        script.append({"command": cmd, "params": params}); return {}
    try:
        return mcp.ex(cmd, params, label=label)
    except Exception as e:
        fails.append((label, str(e)[:300])); return {}
def tool(name, args, label):
    if DRY:
        return {}
    try:
        return mcp.tool(name, args, label=label)
    except Exception as e:
        fails.append((label, str(e)[:300])); return {}
def shot(name):
    r = tool("screenshot", {}, f"screenshot {name}")
    if r.get("_images"):
        open(f"{DEV}-{name}.png", "wb").write(base64.b64decode(r["_images"][0]["data"]))
def stage(name):
    print(f"\n== {name}", flush=True)
XY = lambda z: {"origin": [0, 0, z], "x_dir": [1, 0, 0], "y_dir": [0, 1, 0]}
def circles(name, z, pts, d, label):
    ex("sketch.create", {"plane": XY(z), "name": name}, f"{label}: sketch")
    for x, y in pts:
        ex("sketch.circle.center", {"center": [x, y], "radius": d / 2}, f"{label}: circle at {x},{y}")
    ex("sketch.finish", {}, f"{label}: finish")

W, H, T, FR, BUMP = 218.0, 172.0, 20.0, 3.0, 8.0   # width, height, shell depth, frame, rear bump
X0 = -W / 2
SH, FRM = "Back shell", "Front frame"
# M2 screws, from the back into the frame. The frame's border is 10 mm along the top (above the
# screen window), 6 mm down the sides: four along the top, two down each side.
TOP = [(-100.0, 167.0), (-35.0, 167.0), (35.0, 167.0), (100.0, 167.0)]
SIDE_UPPER = [(-105.0, 128.0), (105.0, 128.0)]
SIDE_LOWER = [(-105.0, 48.0), (105.0, 48.0)]
IN_BUMP = TOP + SIDE_UPPER          # behind the screen: the shell is 8 mm deeper there
HEAD_D, HEAD_H, SHANK_D = 3.8, 1.3, 2.0
CB_D, CB_DEPTH, CLEAR_D, PILOT_D, POST_D = 4.2, 1.6, 2.2, 1.6, 5.0
t_all = time.time()

stage("1 back shell")
tool("new_design", {"name": "DevTerm enclosure"}, "new design")
ex("solid.box", {"length": W, "width": H, "height": T, "corner": [X0, 0, 0], "body_name": SH}, "shell: block 218x172x20")
ex("solid.box", {"length": W, "width": 80, "height": BUMP, "corner": [X0, 92, -BUMP], "operation": "join", "targets": [SH]}, "shell: rear bump 218x80x8 (screen half)")
ex("solid.fillet", {"edges": [[X0, 0, 10], [-X0, 0, 10], [X0, H, 6], [-X0, H, 6]], "radius": 6, "body": SH}, "shell: R6 corners")
ex("solid.box", {"length": W - 4, "width": H - 4, "height": T, "corner": [X0 + 2, 2, 2], "operation": "cut", "targets": [SH]}, "shell: cavity, 2 walls")
ex("solid.box", {"length": W - 4, "width": 76, "height": BUMP + 2.5, "corner": [X0 + 2, 94, -BUMP + 2], "operation": "cut", "targets": [SH]}, "shell: bump hollowed, 2 floor")
# Screw posts, from the floor up to the frame
circles("Posts (screen half)", -BUMP + 2, IN_BUMP, POST_D, "posts behind the screen")
ex("solid.extrude", {"distance": T + BUMP - 2, "operation": "join", "targets": [SH]}, "posts behind the screen: up to the frame")
circles("Posts (keyboard half)", 2, SIDE_LOWER, POST_D, "posts by the keyboard")
ex("solid.extrude", {"distance": T - 2, "operation": "join", "targets": [SH]}, "posts by the keyboard: up to the frame")
# Counterbores and clearance holes, from the back
circles("Counterbores (bump)", -BUMP, IN_BUMP, CB_D, "counterbores behind the screen")
ex("solid.extrude", {"distance": CB_DEPTH, "direction": "positive", "operation": "cut", "targets": [SH]}, "counterbores behind the screen: 1.6 deep")
circles("Counterbores (keyboard)", 0, SIDE_LOWER, CB_D, "counterbores by the keyboard")
ex("solid.extrude", {"distance": CB_DEPTH, "direction": "positive", "operation": "cut", "targets": [SH]}, "counterbores by the keyboard: 1.6 deep")
circles("Clearance holes", -BUMP - 1, IN_BUMP + SIDE_LOWER, CLEAR_D, "clearance holes")
ex("solid.extrude", {"distance": T + BUMP + 2, "direction": "positive", "operation": "cut", "targets": [SH]}, "clearance holes: through the posts")
# Vents and side ports
ex("sketch.create", {"plane": XY(0), "name": "Vents"}, "vents: sketch")
for i in range(12):
    y = 14 + 6 * i
    ex("sketch.rectangle.two_point", {"p0": [-90, y - 1], "p1": [90, y + 1]}, f"vent {i + 1}")
ex("sketch.finish", {}, "vents: finish")
ex("solid.extrude", {"distance": 1.5, "direction": "positive", "operation": "cut", "targets": [SH]}, "shell: cut 12 vents 1.5 deep")
for nm, (y, w, h) in {"USB-A": (45, 13, 5.5), "USB-C": (31, 9, 3.3), "micro-HDMI": (22.5, 7, 3.6)}.items():
    ex("solid.box", {"length": 6, "width": w, "height": h, "center": [-X0 - 1, y, 10], "operation": "cut", "targets": [SH]}, f"shell: {nm} port")
ex("solid.cylinder", {"diameter": 6, "height": 6, "base": [-X0 + 2, 13, 10], "axis": [-1, 0, 0], "operation": "cut", "targets": [SH]}, "shell: 3.5 mm jack")
ex("solid.box", {"length": 6, "width": 6, "height": 3, "center": [X0 + 1, 30, 10], "operation": "cut", "targets": [SH]}, "shell: power switch slot")

stage("2 front frame")
ex("solid.box", {"length": W, "width": H, "height": FR, "corner": [X0, 0, T], "body_name": FRM, "operation": "new"}, "frame: plate 218x172x3")
ex("solid.fillet", {"edges": [[X0, 0, T + 1.5], [-X0, 0, T + 1.5], [X0, H, T + 1.5], [-X0, H, T + 1.5]], "radius": 6, "body": FRM}, "frame: R6 corners")
ex("solid.box", {"length": 189, "width": 67, "height": FR + 2, "center": [0, 128, T + 1.5], "operation": "cut", "targets": [FRM]}, "frame: screen window 189x67")
ex("solid.box", {"length": 206, "width": 87, "height": FR + 2, "center": [0, 48, T + 1.5], "operation": "cut", "targets": [FRM]}, "frame: keyboard opening 206x87")
circles("Pilot holes", T, IN_BUMP + SIDE_LOWER, PILOT_D, "pilot holes")
ex("solid.extrude", {"distance": 2, "direction": "positive", "operation": "cut", "targets": [FRM]}, "frame: M2 pilot holes 2 deep")

stage("3 screws")
names = []
for i, (x, y) in enumerate(IN_BUMP + SIDE_LOWER):
    rear = -BUMP if (x, y) in IN_BUMP else 0.0
    seat = rear + CB_DEPTH
    nm = f"Screw {i + 1}"
    ex("solid.cylinder", {"diameter": HEAD_D, "height": HEAD_H, "base": [x, y, seat - HEAD_H], "body_name": nm, "operation": "new"}, f"{nm}: pan head 3.8x1.3")
    ex("solid.cylinder", {"diameter": SHANK_D, "height": T + 2 - seat, "base": [x, y, seat], "operation": "join", "targets": [nm]}, f"{nm}: M2 shank {T + 2 - seat:.1f} long")
    for lx, ly in ((2.4, 0.5), (0.5, 2.4)):
        ex("solid.box", {"length": lx, "width": ly, "height": 0.9, "center": [x, y, seat - HEAD_H + 0.4], "operation": "cut", "targets": [nm]}, f"{nm}: cross recess")
    names.append(nm)

stage("4 checks and views")
ex("browser.visibility", {"visible": False, "folder": "origin"}, "hide the origin")
d = tool("inspect_design", {"measure": True}, "inspect + measure design")
for b in d.get("bodies", []):
    bb = b["bbox"]; size = [round(bb["max"][i] - bb["min"][i], 2) for i in range(3)]
    print(f"   {b['name']:12s} volume {b['volume_mm3']:10.1f} mm3  size {size}  closed {b['closed']}  faces {b['faces']}", flush=True)
r = ex("inspect.interference", {}, "interference: shell, frame, 8 screws")
print("   interferences:", json.dumps(r.get("interferences"))[:600], "unchecked:", r.get("unchecked"), flush=True)
ex("view.isometric", {}, "view iso"); ex("view.fit", {}, "fit"); shot("1-front")
ex("view.apply", {"camera": {"target": [30, 120, 0], "yaw": 2.55, "pitch": -0.55, "distance": 330}}, "view from behind"); shot("2-rear")
tool("save", {"path": F + "devterm-enclosure.solidcraft"}, "save design")

if DRY:
    json.dump({"commands": script}, open(DRY, "w"))
    print(f"wrote {len(script)} commands to {DRY}")
print(f"\nTOTAL {time.time() - t_all:.1f} s; {len(fails)} failed step(s)", flush=True)
for l, e in fails:
    print(f"  FAILED {l}: {e}", flush=True)
