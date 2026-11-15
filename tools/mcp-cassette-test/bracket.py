"""A pillow bracket built over SolidCraft's MCP server, from a practice drawing ("Practice Model
#03"): a 120 x 80 x 15 base with R10 corners, three Ø16 through holes and an open slot 30 wide, an
upright 50 wide and 40 thick with an R25 top and a Ø40 through bore whose centre is 75 up, and a
10 thick triangular rib in front of the upright. Every call is timed (see README.md).

    MCP_URL=http://PHONE:27880/mcp MCP_TOKEN=... python3 bracket.py NAME

The drawing doesn't give every size. Estimated from the sizes it does give and the part's
proportions in the picture:
- The upright's centre is the base's middle (60 from the left end, the "60" along the front),
  so it runs 35 to 85; it stands at the back edge. Its width is the R25 top's, 50.
- The bore's centre is 75 up (the "75"): with Ø40, its bottom is 40 above the base (the "40").
- The left holes are 25 from the left end and 25 from the front and back edges (the two "25"s);
  the right hole is 15 from the end and 18 from the back, as drawn.
- The slot opens in the right end, 30 wide (the "30"), 14 from the front edge (the "14"), and
  30 deep, about as deep as it is wide in the picture.
- The rib is 10 thick (the "10"), in front of the upright under the bore's right half; it climbs
  40 up the upright, to the bore's bottom, and reaches 30 forward.
- The "20" at the right end reads as a height, but the base is drawn 15 thick throughout: it is
  left out. The R10 at the rib is the rib's own corner radius in the drawing's style; also left
  out, as the rib is a plain triangle.
"""
import mcp, json, time, base64, sys

A = "/storage/emulated/0/Android/data/io.solidcraft.android/files/Agent/"
DEV = sys.argv[1] if len(sys.argv) > 1 else "emu"
B = "Bracket"
L, W, T = 120, 80, 15                       # base: length (X), width (Y), thickness (Z)
UX0, UW, UT, UC, UR, BORE = 35, 50, 40, 75, 25, 40   # upright: from x, wide, thick, bore centre height, top radius, bore
UY0 = W - UT                                # the upright stands at the back edge
RIB_X, RIB_T, RIB_H, RIB_L = 70, 10, 40, 30  # rib: from x, thick, height up the upright, reach forward
SLOT_W, SLOT_D, SLOT_Y = 30, 30, 14         # open slot in the right end: wide, deep, from the front edge
HOLES = [(25, 25), (25, 55), (105, 62)]     # Ø16 through holes (picked: 25 from the left end and the sides)


def shot(name):
    r = mcp.tool("screenshot", {}, label=f"screenshot {name}")
    open(f"{DEV}-{name}.png", "wb").write(base64.b64decode(r["_images"][0]["data"]))


t_all = time.time()
mcp.init()
mcp.tool("new_design", {"name": "Practice bracket"}, label="new design: Practice bracket")

print("\n== 1 base", flush=True)
mcp.ex("solid.box", {"length": L, "width": W, "height": T, "corner": [0, 0, 0], "body_name": B, "operation": "new"}, label="base 120x80x15")
mcp.ex("solid.fillet", {"edges": [[0, 0, T / 2], [L, 0, T / 2], [0, W, T / 2], [L, W, T / 2]], "radius": 10, "body": B}, label="base: R10 corners")
mcp.ex("solid.box", {"length": SLOT_D + 1, "width": SLOT_W, "height": T + 2, "corner": [L - SLOT_D, SLOT_Y, -1], "operation": "cut", "targets": [B]},
       label="base: open slot 30 wide")
for i, (x, y) in enumerate(HOLES):
    mcp.ex("solid.cylinder", {"diameter": 16, "height": T + 2, "base": [x, y, -1], "operation": "cut", "targets": [B]}, label=f"base: Ø16 through hole {i + 1}")
mcp.ex("view.isometric", {}, label="view iso"); shot("1-base")

print("\n== 2 upright with its bore", flush=True)
mcp.ex("solid.box", {"length": UW, "width": UT, "height": UC - T, "corner": [UX0, UY0, T], "operation": "join", "targets": [B]}, label="upright 50x40, up to the bore centre")
mcp.ex("solid.cylinder", {"diameter": 2 * UR, "height": UT, "base": [UX0 + UW / 2, UY0, UC], "axis": "Y", "operation": "join", "targets": [B]},
       label="upright: R25 rounded top")
mcp.ex("solid.cylinder", {"diameter": BORE, "height": UT + 2, "base": [UX0 + UW / 2, UY0 - 1, UC], "axis": "Y", "operation": "cut", "targets": [B]},
       label="upright: Ø40 through bore")

print("\n== 3 rib", flush=True)
mcp.ex("sketch.create", {"plane": {"origin": [RIB_X, 0, 0], "x_dir": [0, 1, 0], "y_dir": [0, 0, 1]}, "name": "Rib"}, label="sketch rib (YZ)")
mcp.ex("sketch.line", {"points": [[UY0, T], [UY0, T + RIB_H], [UY0 - RIB_L, T]], "closed": True}, label="rib: triangle")
mcp.ex("sketch.finish", {}, label="finish sketch")
mcp.ex("solid.extrude", {"distance": RIB_T, "operation": "join", "targets": [B]}, label="rib 10 thick")
mcp.ex("view.isometric", {}, label="view iso"); shot("2-bracket")

print("\n== 4 checks", flush=True)
d = mcp.tool("inspect_design", {"measure": True}, label="inspect + measure design")
for b in d["bodies"]:
    print("  ", b.get("name"), "closed:", b.get("closed"), "volume:", b.get("volume_mm3") or b.get("volume"), flush=True)
mcp.ex("inspect.section", {"plane": "XZ", "at": UY0 + UT / 2}, label="section through the bore")
mcp.ex("view.apply", {"camera": {"target": [60, 60, 45], "yaw": -1.5707, "pitch": 0.0, "distance": 260}}, label="camera from the front"); shot("3-section")
mcp.ex("inspect.section", {"clear": True}, label="clear section")
mcp.ex("view.isometric", {}, label="view iso")
mcp.tool("export", {"path": A + "practice-bracket.step"}, label="export STEP")
mcp.tool("save", {"path": A + "practice-bracket.sc"}, label="save design")
print(f"\nTOTAL {time.time()-t_all:.1f} s", flush=True)
