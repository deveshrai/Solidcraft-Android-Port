"""a2nr's fixture (port issue #2), estimated from their screenshot: a drafted boss with a bore, a
small drafted boss, an arm in two steps and a tapered rib between the bosses."""
import sys, os, base64, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mcp
DRY = len(sys.argv) > 1 and sys.argv[1] == "dry"
script = []
def ex(c, p, label):
    if DRY: script.append({"command": c, "params": p}); return {}
    return mcp.ex(c, p, label=label)
if not DRY: mcp.init(); mcp.tool("new_design", {"name": "Fixture"}, label="new design")
XY = lambda z: {"origin": [0, 0, z], "x_dir": [1, 0, 0], "y_dir": [0, 1, 0]}
H, R, L = 36.0, 20.0, 95.0          # big boss height and base radius; boss-to-boss distance
B = "Fixture"
# Big boss, drafted 2 deg inward
ex("sketch.create", {"plane": XY(0), "name": "Big boss"}, "sketch big boss")
ex("sketch.circle.center", {"center": [0, 0], "radius": R}, "circle R20")
ex("sketch.finish", {}, "finish")
ex("solid.extrude", {"distance": H, "taper": "-2 deg", "body_name": B, "operation": "new"}, "big boss 36 tall, 2 deg draft")
# Small boss, drafted 2 deg inward
ex("sketch.create", {"plane": XY(0), "name": "Small boss"}, "sketch small boss")
ex("sketch.circle.center", {"center": [L, 0], "radius": 9}, "circle R9")
ex("sketch.finish", {}, "finish")
ex("solid.extrude", {"distance": 12, "taper": "-2 deg", "operation": "join", "targets": [B]}, "small boss 12 tall, 2 deg draft")
def flared(name, z, w0, wx, x1, w1, x2, w2, mid, label):
    """An arm outline that flares into the big boss: from half-width w0 at x 0 (inside the boss)
    a concave arc to the side line at x1 (half-width wx), straight on to x2 (half-width w2)."""
    ex("sketch.create", {"plane": XY(z), "name": name}, f"sketch {label}")
    for sgn in (-1, 1):
        ex("sketch.arc.three_point", {"start": [0, sgn * w0], "end": [x1, sgn * wx], "through": [mid[0], sgn * mid[1]]}, f"{label}: flare arc {'+' if sgn > 0 else '-'}y")
    ex("sketch.line", {"points": [[x1, -wx], [x2, -w2], [x2, w2], [x1, wx]]}, f"{label}: sides and end")
    ex("sketch.line", {"points": [[0, w0], [0, -w0]]}, f"{label}: closing line in the boss")
    ex("sketch.finish", {}, "finish")
# Step 1: the base flange, flaring into the big boss (arrow 1), 26 -> 14 wide, on to the small boss
flared("Flange", 0, 19, 10.85, 34, None, L, 7, (19, 13.2), "flange")
ex("solid.extrude", {"distance": 4, "operation": "join", "targets": [B]}, "step 1: flange 4 thick")
# Step 2: narrower, flaring into the big boss too, ending 6 short of the small boss (arrow 2)
flared("Step", 4, 14, 6.875, 30, None, L - 15, 5, (16, 9.3), "step")
ex("solid.extrude", {"distance": 4, "operation": "join", "targets": [B]}, "step 2: 4 more")
# The tapered rib, from near the top of the big boss down to the small boss
ex("sketch.create", {"plane": {"origin": [0, 0, 0], "x_dir": [1, 0, 0], "y_dir": [0, 0, 1]}, "name": "Rib"}, "sketch rib (XZ)")
ex("sketch.line", {"points": [[12, 8], [12, H - 4], [L - 20, 14], [L - 9, 14], [L - 9, 8]], "closed": True}, "rib outline")
ex("sketch.finish", {}, "finish")
ex("solid.extrude", {"distance": 8, "direction": "symmetric", "operation": "join", "targets": [B]}, "rib 8 thick")
# Bore and hole
ex("solid.cylinder", {"diameter": 28, "height": H + 2, "base": [0, 0, -1], "operation": "cut", "targets": [B]}, "bore D28")
ex("solid.cylinder", {"diameter": 9, "height": 14, "base": [L, 0, -1], "operation": "cut", "targets": [B]}, "hole D9")
ex("view.isometric", {}, "view iso")
ex("view.fit", {}, "fit")
if DRY:
    json.dump({"commands": script}, open("fixture-script.json", "w"))
else:
    r = mcp.tool("screenshot", {}, label="screenshot")
    open("fixture-tab.png", "wb").write(base64.b64decode(r["_images"][0]["data"]))
