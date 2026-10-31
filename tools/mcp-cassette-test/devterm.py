"""A Clockwork DevTerm-like handheld built over SolidCraft's MCP server, every call timed.
Sizes are estimates from photos (scale from the 6.86" 1280x480 screen, ~0.305 mm/px).
X = width (218), Y = height up the face (0 at the keyboard edge, 172 at the top), Z = thickness
(0 = rear of the keyboard half, front face at 23).

    MCP_URL=... MCP_TOKEN=... MCP_LOG=... python3 devterm.py NAME
A step that fails is logged and the build goes on (each failure is a finding)."""
import sys, os, time, json, base64, math
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
DRY = os.environ.get("DRY")
if not DRY:
    import mcp
    mcp.init()
script = []
DEV = sys.argv[1] if len(sys.argv) > 1 else "dev"
F = "/storage/emulated/0/Android/data/io.solidcraft.android/files/Agent/"
W, H, T, FR = 218.0, 172.0, 20.0, 3.0   # body width, height, shell depth, frame thickness
X0 = -W / 2
fails = []
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
def rows_sketch(name, z, rects, label):
    """One sketch of rectangles (cx, cy, w, h) on the plane z, for one extrude."""
    ex("sketch.create", {"plane": {"origin": [0, 0, z], "x_dir": [1, 0, 0], "y_dir": [0, 1, 0]}, "name": name}, f"{label}: sketch")
    for (cx, cy, w, h) in rects:
        ex("sketch.rectangle.two_point", {"p0": [cx - w / 2, cy - h / 2], "p1": [cx + w / 2, cy + h / 2]}, f"{label}: rect at {cx:.1f},{cy:.1f}")
    ex("sketch.finish", {}, f"{label}: finish sketch")

t_all = time.time()
SH, FRM, KB, SCR, KL, KR, PR, BAT = "Back shell", "Front frame", "Keyboard", "Screen", "Knob left", "Knob right", "Printer", "Battery cover"
PCB, PL, PRT = "Main board", "Port plate left", "Port plate right"

stage("1 back shell")
tool("new_design", {"name": "DevTerm"}, "new design")
ex("solid.box", {"length": W, "width": H, "height": T, "corner": [X0, 0, 0], "body_name": SH}, "shell: block 218x172x20")
ex("solid.box", {"length": W, "width": 80, "height": 8, "corner": [X0, 92, -8], "operation": "join", "targets": [SH]}, "shell: rear bump 218x80x8 (screen half)")
ex("solid.fillet", {"edges": [[X0, 0, 10], [-X0, 0, 10], [X0, H, 6], [-X0, H, 6]], "radius": 6, "body": SH}, "shell: R6 corners")
ex("solid.box", {"length": W - 4, "width": H - 4, "height": T, "corner": [X0 + 2, 2, 2], "operation": "cut", "targets": [SH]}, "shell: cavity, 2 walls")
rows_sketch("Vents", 0, [(0, 14 + 6 * i, 180, 2) for i in range(12)], "shell: 12 vent slots")
ex("solid.extrude", {"distance": 1.5, "direction": "positive", "operation": "cut", "targets": [SH]}, "shell: cut 12 vents 1.5 deep")
# Right side ports, x = +109
ex("solid.box", {"length": 6, "width": 13, "height": 5.5, "center": [-X0 - 1, 45, 10], "operation": "cut", "targets": [SH]}, "shell: USB-A port")
ex("solid.box", {"length": 6, "width": 9, "height": 3.3, "center": [-X0 - 1, 31, 10], "operation": "cut", "targets": [SH]}, "shell: USB-C port")
ex("solid.box", {"length": 6, "width": 7, "height": 3.6, "center": [-X0 - 1, 22.5, 10], "operation": "cut", "targets": [SH]}, "shell: micro-HDMI port")
ex("solid.cylinder", {"diameter": 6, "height": 6, "base": [-X0 + 2, 13, 10], "axis": [-1, 0, 0], "operation": "cut", "targets": [SH]}, "shell: 3.5 mm jack")
# Four standoffs for the main board, on the floor (2 mm) up to 6
BOARD = (-100.0, 8.0, 120.0, 78.0)   # x, y, length, width
STAND = [(BOARD[0] + 5, BOARD[1] + 5), (BOARD[0] + BOARD[2] - 5, BOARD[1] + 5), (BOARD[0] + 5, BOARD[1] + BOARD[3] - 5), (BOARD[0] + BOARD[2] - 5, BOARD[1] + BOARD[3] - 5)]
for i, (x, y) in enumerate(STAND):
    ex("solid.cylinder", {"diameter": 5, "height": 4, "base": [x, y, 2], "operation": "join", "targets": [SH]}, f"shell: board standoff {i + 1}")
# Openings in the side walls for the port plates
ex("solid.box", {"length": 4, "width": 46, "height": 12, "center": [-X0 - 1, 30, 10], "operation": "cut", "targets": [SH]}, "shell: right port-plate opening")
ex("solid.box", {"length": 4, "width": 30, "height": 12, "center": [X0 + 1, 30, 10], "operation": "cut", "targets": [SH]}, "shell: left port-plate opening")
ex("view.isometric", {}, "view iso"); shot("1-shell")

stage("2 front frame")
ex("solid.box", {"length": W, "width": H, "height": FR, "corner": [X0, 0, T], "body_name": FRM, "operation": "new"}, "frame: plate 218x172x3")
ex("solid.fillet", {"edges": [[X0, 0, T + 1.5], [-X0, 0, T + 1.5], [X0, H, T + 1.5], [-X0, H, T + 1.5]], "radius": 6, "body": FRM}, "frame: R6 corners")
ex("solid.box", {"length": 189, "width": 67, "height": FR + 2, "center": [0, 128, T + 1.5], "operation": "cut", "targets": [FRM]}, "frame: screen window 189x67")
ex("solid.box", {"length": 206, "width": 87, "height": FR + 2, "center": [0, 48, T + 1.5], "operation": "cut", "targets": [FRM]}, "frame: keyboard opening 206x87")
ex("solid.fillet", {"edges": [[sx * 94.5, y, T + 1.5] for sx in (-1, 1) for y in (94.5, 161.5)], "radius": 2, "body": FRM}, "frame: R2 window corners")

stage("3 screen")
ex("solid.box", {"length": 189, "width": 67, "height": 1, "corner": [-94.5, 94.5, T - 1], "body_name": SCR, "operation": "new"}, "screen: panel 189x67x1")

stage("4 keyboard deck and keys")
ex("solid.box", {"length": 204, "width": 85, "height": 3, "corner": [-102, 5.5, T - 3], "body_name": KB, "operation": "new"}, "keyboard: deck 204x85x3")
P, K = 12.2, 10.5
rows = {
    "top row": [(-52.6, 76, 13, 8), (-36.7, 76, 13, 8), (-21, 76, 11, 8), (14.5, 76, K, 8), (29.6, 76, K, 8), (48, 76, 12, 8), (64, 76, 12, 8)],
    "F row": [(-66.3 + 14.2 * i, 65, 11, 8) for i in range(10)],
    "Q row": [(-92, 53.5, 14, K)] + [(-72.4 + P * i, 53.5, K, K) for i in range(10)] + [(49.6, 53.5, K, K), (86, 53.5, 20, K)],
    "A row": [(-90, 42, 16, K)] + [(-60 + P * i, 42, K, K) for i in range(9)] + [(49.8, 42, K, K), (62, 42, K, K), (88, 42, 20, K)],
    "Z row": [(-88, 30.5, 20, K)] + [(-68.4 + P * i, 30.5, K, K) for i in range(7)] + [(17, 30.5, K, K), (29.2, 30.5, K, K), (41.4, 30.5, K, K), (53.6, 30.5, K, K), (80, 30.5, 18, K)],
    "bottom row": [(-90, 19, 14, K), (-74, 19, 14, K), (-58, 19, 12, K), (-44, 19, 12, K), (-2.3, 19, 52, K), (36, 19, 12, K), (50, 19, 12, K), (65, 19, K, K), (79, 19, K, K), (93, 19, K, K)],
}
for name, rects in rows.items():
    rows_sketch(f"Keys {name}", T, rects, f"keys {name} ({len(rects)})")
    ex("solid.extrude", {"distance": 4, "direction": "positive", "operation": "join", "targets": [KB]}, f"keys {name}: join {len(rects)} keys 4 tall")
# D-pad: a cross of two bars; ABXY; trackball in its ring
ex("solid.box", {"length": 20, "width": 7, "height": 4, "center": [-86.7, 74.4, T + 2], "operation": "join", "targets": [KB]}, "d-pad: bar across")
ex("solid.box", {"length": 7, "width": 20, "height": 4, "center": [-86.7, 74.4, T + 2], "operation": "join", "targets": [KB]}, "d-pad: bar along")
for nm, (x, y) in {"Y": (74, 79.9), "X": (87.1, 79.9), "B": (81, 68.3), "A": (94.4, 68.3)}.items():
    ex("solid.cylinder", {"diameter": 8.5, "height": 3, "base": [x, y, T], "operation": "join", "targets": [KB]}, f"button {nm}")
ex("solid.cylinder", {"diameter": 12, "height": 1, "base": [-0.7, 76, T], "operation": "join", "targets": [KB]}, "trackball ring")
ex("solid.sphere", {"radius": 3.5, "center": [-0.7, 76, T + 1.5], "operation": "join", "targets": [KB]}, "trackball ball")
ex("view.isometric", {}, "view iso"); shot("2-keyboard")

stage("5 knobs with knurls")
ZK, YK = 10.0, 130.8
for nm, x, ax in ((KL, X0, -1), (KR, -X0, 1)):
    ex("solid.cylinder", {"diameter": 16, "height": 8, "base": [x, YK, ZK], "axis": [ax, 0, 0], "body_name": nm, "operation": "new"}, f"{nm}: Ø16x8")
    for i in range(12):
        a = 2 * math.pi * i / 12
        ex("solid.cylinder", {"diameter": 1.6, "height": 10, "base": [x - ax, YK + 8 * math.cos(a), ZK + 8 * math.sin(a)], "axis": [ax, 0, 0], "operation": "cut", "targets": [nm]}, f"{nm}: knurl {i + 1}/12")
    ex("solid.chamfer", {"edges": [[x + ax * 8, YK + 8 * math.cos(math.pi / 12), ZK + 8 * math.sin(math.pi / 12)]], "distance": 0.6}, f"{nm}: outer chamfer 0.6")

stage("5b main board and port plates")
ex("solid.box", {"length": BOARD[2], "width": BOARD[3], "height": 1.6, "corner": [BOARD[0], BOARD[1], 6], "body_name": PCB, "operation": "new"}, "board: 120x78x1.6 on the standoffs")
for i, (x, y) in enumerate(STAND):
    ex("solid.cylinder", {"diameter": 2.6, "height": 3, "base": [x, y, 5.3], "operation": "cut", "targets": [PCB]}, f"board: mounting hole {i + 1}")
for nm, (x, y, l, w, h) in {"SoC": (-55, 40, 15, 15, 1.5), "RAM": (-30, 40, 12, 10, 1.2), "USB-A": (12, 45, 14, 13, 5.5), "USB-C": (14, 31, 9, 8, 3.2), "HDMI": (14, 22.5, 7, 7, 3.4), "SD slot": (-80, 20, 15, 14, 1.8)}.items():
    ex("solid.box", {"length": l, "width": w, "height": h, "center": [x, y, 7.6 + h / 2], "operation": "join", "targets": [PCB]}, f"board: {nm}")
ex("solid.box", {"length": 2, "width": 44, "height": 10, "center": [-X0 - 1, 30, 10], "body_name": PRT, "operation": "new"}, "port plate right 44x10x2")
for nm, (y, w, h) in {"USB-A": (45, 13, 5.5), "USB-C": (31, 9, 3.3), "HDMI": (22.5, 7, 3.6)}.items():
    ex("solid.box", {"length": 4, "width": w, "height": h, "center": [-X0 - 1, y, 10], "operation": "cut", "targets": [PRT]}, f"port plate right: {nm} opening")
ex("solid.cylinder", {"diameter": 6, "height": 4, "base": [-X0 + 1, 13, 10], "axis": [-1, 0, 0], "operation": "cut", "targets": [PRT]}, "port plate right: jack hole")
ex("solid.box", {"length": 2, "width": 28, "height": 10, "center": [X0 + 1, 30, 10], "body_name": PL, "operation": "new"}, "port plate left 28x10x2")
ex("solid.box", {"length": 4, "width": 6, "height": 3, "center": [X0 + 1, 30, 10], "operation": "cut", "targets": [PL]}, "port plate left: switch slot")

stage("6 printer and battery cover")
ex("solid.box", {"length": 82, "width": 6, "height": 20, "corner": [11.6, H, -8], "body_name": PR, "operation": "new"}, "printer: 82x6x20 on the top edge")
ex("solid.box", {"length": 60, "width": 5, "height": 1.5, "center": [52.6, H + 4, 4], "operation": "cut", "targets": [PR]}, "printer: paper slot 60x1.5")
ex("solid.box", {"length": 95, "width": 50, "height": 1, "corner": [-2.5, 115, -9], "body_name": BAT, "operation": "new"}, "battery cover 95x50x1 on the bump")
for i in range(4):
    ex("solid.box", {"length": 14 - 2 * abs(i - 1.5), "width": 0.8, "height": 0.6, "center": [45, 155 - 2 * i, -9], "operation": "cut", "targets": [BAT]}, f"battery cover: grip line {i + 1}")

stage("7 checks")
d = tool("inspect_design", {"measure": True}, "inspect + measure design")
for b in d.get("bodies", []):
    bb = b["bbox"]; size = [round(bb["max"][i] - bb["min"][i], 2) for i in range(3)]
    print(f"   {b['name']:14s} volume {b['volume_mm3']:11.1f} mm3  size {size}  closed {b['closed']}  faces {b['faces']}", flush=True)
r = ex("inspect.interference", {}, "interference: all parts")
print("   interferences:", json.dumps(r.get("interferences"))[:600], "unchecked:", r.get("unchecked"), flush=True)
r = ex("inspect.clearance", {"bodies": [KB, SCR], "against": [FRM, SH], "clearance": 0.5}, "clearance: keyboard, screen vs frame, shell")
print("   clearance:", json.dumps({k: r.get(k) for k in ("overlaps", "near", "unchecked")})[:600], flush=True)
ex("view.isometric", {}, "view iso"); shot("3-assembly")
ex("view.apply", {"camera": {"target": [0, 86, 10], "yaw": 3.1416, "pitch": -0.5, "distance": 420}}, "view from behind"); shot("4-rear")

stage("8 export and save")
tool("export", {"path": F + "devterm.step"}, "export STEP")
tool("save", {"path": F + "devterm.solidcraft"}, "save design")
stage("9 exploded view, orbited")
ALL = [SH, FRM, SCR, KB, KL, KR, PCB, PRT, PL, PR, BAT]
ex("browser.visibility", {"visible": False, "folder": "origin"}, "hide the origin")
ex("component.from_bodies", {"bodies": ALL}, "components from 11 bodies")
EXPLODE = {BAT: [0, 0, -80], PCB: [0, 0, 70], PL: [-60, 0, 70], PRT: [60, 0, 70], SCR: [0, 0, 140], KB: [0, 0, 210], FRM: [0, 0, 280], KL: [-80, 0, 140], KR: [80, 0, 140], PR: [0, 60, 0]}
ex("explode.create", {"name": "Exploded", "moves": [{"occurrence": b, "translate": v} for b, v in EXPLODE.items()]}, "explode: create view")
ex("explode.show", {"view": "Exploded"}, "explode: show")
CAM = lambda yaw: {"camera": {"target": [0, 90, 100], "yaw": yaw, "pitch": 0.32, "distance": 1050}}
ex("view.apply", CAM(-0.7), "exploded: camera"); shot("5-exploded")
if os.environ.get("ORBIT"):
    open(os.environ["ORBIT"], "w").write(str(time.time()))
    for i in range(181):
        ex("view.apply", CAM(-0.7 + 2 * math.pi * i / 180), f"orbit {i}")
        if not DRY: time.sleep(0.05)
    open(os.environ["ORBIT"], "a").write("\n" + str(time.time()))
print(f"\nTOTAL {time.time() - t_all:.1f} s; {len(fails)} failed step(s)", flush=True)
for l, e in fails:
    print(f"  FAILED {l}: {e}", flush=True)
if DRY:
    json.dump({"commands": script}, open(DRY, "w"), indent=0)
    print(f"wrote {len(script)} commands to {DRY}")
