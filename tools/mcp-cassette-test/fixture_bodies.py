"""a2nr's fixture (port issue #2) built the way the owner's test-1 design builds it: every piece one
extrude of a flat sketch, as its own body, so no join meets a drafted or curved face while modelling.
Proportions from a2nr's screenshot (big boss diameter : boss spacing about 1 : 3). Then, separately
timed, the pieces combined into one solid.

    MCP_URL=... MCP_TOKEN=... MCP_LOG=... python3 fixture_bodies.py      (or: python3 fixture_bodies.py dry)
"""
import sys, os, json, base64
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
DRY = len(sys.argv) > 1 and sys.argv[1] == "dry"
script = []
if not DRY:
    import mcp
    mcp.init()
def ex(c, p, label):
    if DRY:
        script.append({"command": c, "params": p}); return {}
    return mcp.ex(c, p, label=label)
XY = lambda z: {"origin": [0, 0, z], "x_dir": [1, 0, 0], "y_dir": [0, 1, 0]}
XZ = {"origin": [0, 0, 0], "x_dir": [1, 0, 0], "y_dir": [0, 0, 1]}
import math
L = 120.0                                  # boss to boss
BIG = dict(d=40.0, h=38.0, bore=30.0)      # big boss: base diameter, height, bore
SMALL = dict(d=20.0, h=13.0, hole=11.0)
T = 4.0                                    # each step's thickness
# The arm runs flat from the big boss, then drops at 16 deg (106 deg to the boss wall), then runs
# flat again into the small boss, as in a2nr's screenshot and the owner's test-1 design.
X_BEND, X_FLAT, ANGLE = 42.0, 102.0, 16.0
DROP = (X_FLAT - X_BEND) * math.tan(math.radians(ANGLE))
if not DRY:
    mcp.tool("new_design", {"name": "Fixture (bodies)"}, label="new design")

def boss(name, x, z, b, hole):
    ex("sketch.create", {"plane": XY(z), "name": name}, f"sketch {name}")
    ex("sketch.circle.center", {"center": [x, 0], "radius": b["d"] / 2}, f"{name}: circle")
    ex("sketch.circle.center", {"center": [x, 0], "radius": hole / 2}, f"{name}: hole circle")
    ex("sketch.finish", {}, "finish")
    # The bore is the sketch's inner loop: one extrude makes the drafted boss and its bore.
    ex("solid.extrude", {"distance": b["h"], "taper": "-2 deg", "body_name": name, "operation": "new"}, f"{name}: extrude {b['h']}, 2 deg draft")

def side(z0, x_end):
    """A step's side view: flat, down at ANGLE, flat; from z0 to z0 + T, from inside the big boss to x_end."""
    low = [(-5.0, z0), (X_BEND, z0), (X_FLAT, z0 - DROP), (x_end, z0 - DROP)]
    return low + [(x, z + T) for x, z in reversed(low)]

def arm_step(name, z0, x_end, w0, wx, x1, x2, w2, mid):
    """A step of the arm, as its own body: its side view extruded wide, then intersected with its
    plan, which flares into the big boss with a concave arc each side (half-width w0 inside the boss
    to wx at x1) and runs straight to x2 (half-width w2). Side view and plan share no face."""
    ex("sketch.create", {"plane": XZ, "name": f"{name} side"}, f"sketch {name} side view")
    ex("sketch.line", {"points": [list(p) for p in side(z0, x_end)], "closed": True}, f"{name}: side outline")
    ex("sketch.finish", {}, "finish")
    ex("solid.extrude", {"distance": 2 * w0 + 6, "direction": "symmetric", "body_name": name, "operation": "new"}, f"{name}: side view extruded")
    ex("sketch.create", {"plane": XY(-DROP - 10), "name": f"{name} plan"}, f"sketch {name} plan")
    for s_ in (-1, 1):
        ex("sketch.arc.three_point", {"start": [0, s_ * w0], "end": [x1, s_ * wx], "through": [mid[0], s_ * mid[1]]}, f"{name}: flare arc")
    ex("sketch.line", {"points": [[x1, -wx], [x2, -w2], [x2, w2], [x1, wx]]}, f"{name}: sides and end")
    ex("sketch.line", {"points": [[0, w0], [0, -w0]]}, f"{name}: closing line in the boss")
    ex("sketch.finish", {}, "finish")
    ex("solid.extrude", {"distance": DROP + 30, "operation": "intersect", "targets": [name]}, f"{name}: intersect with the plan")

boss("Big boss", 0.0, 0.0, BIG, BIG["bore"])
boss("Small boss", L, -DROP, SMALL, SMALL["hole"])
# Step 1, the flange: 26 wide at the boss to 16 at the small boss, on to its centre.
arm_step("Flange", 0.0, L + 5, 19.0, 13.0, 34.0, L, 8.0, (20.0, 15.6))
# Step 2: 18 wide to 10, stopping short of the small boss (arrow 2).
arm_step("Step", T, L - 18 + 5, 15.0, 9.0, 30.0, L - 18.0, 5.0, (17.0, 11.4))
# The rib: its side view, from near the top of the big boss down to the small boss, on the step.
top = -DROP + SMALL["h"] + 2
ex("sketch.create", {"plane": XZ, "name": "Rib"}, "sketch Rib (side view)")
ex("sketch.line", {"points": [[14, 2 * T], [14, BIG["h"] - 2], [L - 28, top], [L - 18, top], [L - 18, 2 * T - DROP], [X_BEND, 2 * T]], "closed": True}, "Rib: outline")
ex("sketch.finish", {}, "finish")
ex("solid.extrude", {"distance": 7, "direction": "symmetric", "body_name": "Rib", "operation": "new"}, "Rib: extrude 7 symmetric")
ex("view.isometric", {}, "view iso")
ex("view.fit", {}, "fit")

# One solid, a piece at a time and in this order: the flat arm first (milliseconds), then the arm into
# the small boss, then into the big one. All four into the big boss at once took 36.7 s (Mac), with
# retries; this order 0.85 s, the same solid.
COMBINE = [("Flange", "Step"), ("Flange", "Rib"), ("Small boss", "Flange"), ("Big boss", "Small boss")]

if DRY:
    json.dump({"commands": script}, open(sys.argv[2] if len(sys.argv) > 2 else "fixture-bodies.json", "w"))
    # The combine, as its own script, so it can be timed apart.
    combine = [{"command": "solid.combine", "params": {"target": t, "tools": [b], "operation": "join"}} for t, b in COMBINE]
    json.dump({"commands": script + combine}, open((sys.argv[2] if len(sys.argv) > 2 else "fixture-bodies.json").replace(".json", "-combined.json"), "w"))
else:
    shot = mcp.tool("screenshot", {}, label="screenshot bodies")
    open("fixture-bodies-tab.png", "wb").write(base64.b64decode(shot["_images"][0]["data"]))
    if os.environ.get("COMBINE"):
        for t, b in COMBINE:
            ex("solid.combine", {"target": t, "tools": [b], "operation": "join"}, f"combine: {b} into {t}")
        shot = mcp.tool("screenshot", {}, label="screenshot combined")
        open("fixture-combined-tab.png", "wb").write(base64.b64decode(shot["_images"][0]["data"]))
