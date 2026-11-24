"""A 650 mm linear stage built over SolidCraft's MCP server: a 2060 T-slot extrusion (TechSquid
Extrusions plugin), an MGN12 rail and MGN12H carriage on its 60 mm face, a module 1.5 rack and
16-tooth pinion in mesh on its 20 mm side (bbonsen Gears plugin, entry `rack`), and a carriage
plate carrying a NEMA 17 motor that drives the pinion. Every call is timed.

    MCP_URL=... MCP_TOKEN=... MCP_LOG=... python3 stage.py NAME

Both plugins' packages must be unpacked in the agent's folder, each folder named after its plugin's
id: Agent/gears (bbonsen/solidcraftGearsPlugin 0.10.0, its gears-0.10.0.zip renamed) and
Agent/techsquid-extrusions (TechSquidTV/SolidCraft-Plugins 0.3.0).

World frame: the stage runs along +X from 0 to 650; the extrusion's 60 mm face is on top (Z 20),
its 20 mm sides face +Y and -Y; the rack is on the +Y side.
"""
import mcp, json, math, time, base64, sys

A = "/storage/emulated/0/Android/data/io.solidcraft.android/files/Agent/"
GEARS, EXTRUSIONS = A + "gears", A + "techsquid-extrusions"
DEV = sys.argv[1] if len(sys.argv) > 1 else "emu"
L = 650.0
M, RACK_TEETH, PINION_TEETH = 1.5, 138, 16      # 138 x pi x 1.5 = 650.3 mm of rack
PITCH_Z = 24.0                                  # rack pitch line height; its bar reaches down to Z 0
BACK = PITCH_Z - 1.25 * M                       # bar depth below the tooth roots
FACE = 10.0                                     # rack and pinion face width: Y 30..40
RAIL_W, RAIL_H, RAIL_PITCH = 12.0, 8.0, 25.0    # MGN12 rail
CAR_L, CAR_W, CAR_Z0, CAR_Z1 = 45.4, 27.0, 23.0, 33.0  # MGN12H block: 3 mm above the base, 13 mm high


def shot(name):
    r = mcp.tool("screenshot", {}, label=f"screenshot {name}")
    open(f"{DEV}-{name}.png", "wb").write(base64.b64decode(r["_images"][0]["data"]))


def box(name, x0, x1, y0, y1, z0, z1, label, operation="new", targets=None):
    p = {"length": x1 - x0, "width": y1 - y0, "height": z1 - z0, "corner": [x0, y0, z0], "operation": operation}
    if operation == "new":
        p["body_name"] = name
    if targets:
        p["targets"] = targets
    return mcp.ex("solid.box", p, label=label)


def cyl(d, h, base, axis, label, operation="new", name=None, targets=None):
    p = {"diameter": d, "height": h, "base": base, "axis": axis, "operation": operation}
    if name:
        p["body_name"] = name
    if targets:
        p["targets"] = targets
    return mcp.ex("solid.cylinder", p, label=label)


t_all = time.time()
mcp.init()
approval = {}
for path, label in ((EXTRUSIONS, "TechSquid Extrusions 0.3.0"), (GEARS, "bbonsen Gears 0.10.0")):
    approval[path] = mcp.ex("plugin.inspect", {"path": path}, label=f"plugin.inspect: {label}")["approval"]

print("\n== 1 rack and pinion, made by the Gears plugin", flush=True)
mcp.tool("new_design", {"name": "Rack and pinion"}, label="new design: Rack and pinion")
r = mcp.ex("plugin.run", {"path": GEARS, "approval": approval[GEARS], "entry": "rack",
                          "params": {"teeth": RACK_TEETH, "module": M, "thickness": FACE, "back": BACK,
                                     "pinion_teeth": PINION_TEETH, "pinion_bore": 5.1, "profile_shift": 0.07}},
           label=f"plugin: rack m{M} x{RACK_TEETH} + pinion z{PINION_TEETH}, {FACE:g} wide")
res = r["result"]
print("  ", json.dumps(res)[:400], flush=True)
rack_len = res["length_mm"]
px, py = res["pinion"]["centre_in_sketch_mm"]
mcp.ex("view.isometric", {}, label="view iso")
mcp.ex("view.fit", {}, label="fit")
shot("1-rack")
mcp.tool("save", {"path": A + "stage-rack.solidcraft"}, label="save stage-rack.solidcraft")

print("\n== 2 the 2060 extrusion, made by the Extrusions plugin", flush=True)
mcp.tool("new_design", {"name": "Linear stage 650"}, label="new design: Linear stage 650")
r = mcp.ex("plugin.run", {"path": EXTRUSIONS, "approval": approval[EXTRUSIONS], "entry": "make-extrusion",
                          "params": {"size": "2060", "slot": "T-slot", "length": L, "mode": "Fixed length"}},
           label=f"plugin: 2060 T-slot x {L:g}")
ext_occ = r["result"]["component"]["occurrences"][0]
# It stands along +Z, 20 along X and 60 along Y: lay it along +X with its 60 mm face up.
mcp.ex("occurrence.ground", {"occurrence": ext_occ, "grounded": False}, label="unground the extrusion")
mcp.ex("occurrence.move", {"occurrence": ext_occ, "to_matrix": [0, 0, 1, 0, 0, 1, 0, 0, -1, 0, 0, 10]},
       label="lay the extrusion along X, 60 mm face up")
mcp.ex("occurrence.ground", {"occurrence": ext_occ, "grounded": True}, label="ground the extrusion")
mcp.ex("view.isometric", {}, label="view iso")
mcp.ex("view.fit", {}, label="fit")

print("\n== 3 MGN12 rail on the top face", flush=True)
box("MGN12 rail", 0, L, -RAIL_W / 2, RAIL_W / 2, 20, 20 + RAIL_H, f"MGN12 rail 12 x 8 x {L:g}")
first = RAIL_PITCH / 2
cb = cyl(6, 4.5, [first, 0, 20 + RAIL_H - 4.5], "Z", "rail: counterbore 6 x 4.5", operation="cut", targets=["MGN12 rail"])
th = cyl(3.5, RAIL_H + 2, [first, 0, 19], "Z", "rail: hole 3.5", operation="cut", targets=["MGN12 rail"])
n = int((L - first) // RAIL_PITCH) + 1
mcp.ex("solid.pattern.rectangular", {"features": [cb["name"], th["name"]], "dir1": [1, 0, 0], "count1": n, "spacing1": RAIL_PITCH},
       label=f"rail: {n} screw holes every {RAIL_PITCH:g}")

print("\n== 4 rack and pinion on the +Y side", flush=True)
r = mcp.ex("component.insert_design", {"path": A + "stage-rack.solidcraft", "name": "Rack and pinion"}, label="insert the rack and pinion")
rack_occ = r["occurrence"]
tx = (L - rack_len) / 2
# Sketch x -> world X, sketch y (teeth) -> world +Z, its face width (+z) -> world -Y from Y 40.
mcp.ex("occurrence.move", {"occurrence": rack_occ, "to_matrix": [1, 0, 0, tx, 0, 0, -1, 30 + FACE, 0, 1, 0, PITCH_Z]},
       label="rack on the +Y side, teeth up, pitch line Z 24")
XC, ZC = tx + px, PITCH_Z + py                  # the pinion's axis: along Y at X XC, Z ZC
print(f"   pinion axis at X {XC:.2f}, Z {ZC:.2f}", flush=True)

print("\n== 5 MGN12H carriage, plate and motor over the pinion", flush=True)
c = "MGN12H carriage"
box(c, XC - CAR_L / 2, XC + CAR_L / 2, -CAR_W / 2, CAR_W / 2, CAR_Z0, CAR_Z1, "MGN12H carriage 45.4 x 27")
box(c, XC - CAR_L / 2 - 1, XC + CAR_L / 2 + 1, -6.5, 6.5, CAR_Z0 - 1, 20 + RAIL_H + 0.5, "carriage: rail channel", operation="cut", targets=[c])
for dx in (-10, 10):
    for dy in (-10, 10):
        cyl(2.5, 5, [XC + dx, dy, CAR_Z1 - 5], "Z", f"carriage: M3 tap {dx:+d},{dy:+d}", operation="cut", targets=[c])
p = "Carriage plate"
box(p, XC - 40, XC + 40, -35, 28, CAR_Z1, CAR_Z1 + 6, "carriage plate 80 x 63 x 6")
box(p, XC - 30, XC + 30, 16, 28, CAR_Z1 + 6, 58, "plate: riser", operation="join", targets=[p])
box(p, XC - 30, XC + 30, 16, 50, 52, 58, "plate: bridge over the pinion", operation="join", targets=[p])
box(p, XC - 30, XC + 30, 44, 50, 14, 58, "plate: motor face", operation="join", targets=[p])
cyl(22.2, 8, [XC, 43, ZC], "Y", "motor face: boss bore 22.2", operation="cut", targets=[p])
for dx in (-15.5, 15.5):
    for dz in (-15.5, 15.5):
        cyl(3.4, 8, [XC + dx, 43, ZC + dz], "Y", f"motor face: M3 {dx:+g},{dz:+g}", operation="cut", targets=[p])
for dx in (-10, 10):
    for dy in (-10, 10):
        cyl(3.4, 8, [XC + dx, dy, CAR_Z1 - 1], "Z", f"plate: M3 {dx:+d},{dy:+d}", operation="cut", targets=[p])
mo = "NEMA 17 motor"
box(mo, XC - 21, XC + 21, 50, 90, ZC - 21, ZC + 21, "NEMA 17 body 42 x 42 x 40")
cyl(22, 4, [XC, 46, ZC], "Y", "motor: pilot boss 22", operation="join", targets=[mo])
cyl(5, 20.5, [XC, 29.5, ZC], "Y", "motor: shaft 5 through the pinion", operation="join", targets=[mo])
mcp.ex("view.isometric", {}, label="view iso")
mcp.ex("view.fit", {}, label="fit")
shot("2-stage")

print("\n== 6 checks", flush=True)
r = mcp.ex("inspect.interference", {}, label="interference: everything")
print("   interferences:", r.get("count"), json.dumps([(i["a"], i["b"], round(i.get("volume_mm3", 0), 3))
                                                      for i in r.get("interferences", [])])[:900], "unchecked:", r.get("unchecked"), flush=True)
d = mcp.tool("inspect_design", {"measure": True}, label="inspect + measure design")
print("   bodies:", [(b["name"], b["faces"], round(b["volume_mm3"])) for b in d["bodies"]], "closed:", all(b["closed"] for b in d["bodies"]), flush=True)
mcp.ex("view.apply", {"camera": {"target": [XC, 10, 30], "yaw": -0.7, "pitch": 0.45, "distance": 260}}, label="camera on the carriage")
shot("3-carriage")
mcp.ex("view.apply", {"camera": {"target": [XC, 35, 36], "yaw": -1.5708, "pitch": 0.0, "distance": 120}}, label="camera along X: the 2060 profile, rail, carriage, pinion, motor")
shot("4-end")
mcp.ex("inspect.section", {"plane": "XZ", "at": 30 + FACE / 2}, label="section through the rack and pinion (Y 35)")
mcp.ex("view.apply", {"camera": {"target": [XC, 35, 30], "yaw": 3.1416, "pitch": 0.0, "distance": 100}}, label="camera along the pinion axis")
shot("5-mesh")
mcp.ex("inspect.section", {"clear": True}, label="clear section")
mcp.ex("view.isometric", {}, label="view iso")
mcp.ex("view.fit", {}, label="fit")
mcp.tool("export", {"path": A + "linear-stage-650.step"}, label="export STEP")
mcp.tool("save", {"path": A + "linear-stage-650.solidcraft"}, label="save design")
print(f"\nTOTAL {time.time()-t_all:.1f} s", flush=True)
