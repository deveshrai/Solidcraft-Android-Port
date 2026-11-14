"""A limited-slip spur-gear differential built over SolidCraft's MCP server, its gears made by the
SolidCraft Gears plugin: two sun gears, three pairs of planets, a clutch pack and preload spring
behind each sun, the carrier cage with its planet pins, axles, sleeved bushings, two pillow-block
brackets and a base plate. Every call is timed (see README.md).

    MCP_URL=http://PHONE:27880/mcp MCP_TOKEN=... python3 lsd.py NAME

The plugin's package must be unpacked in the agent's folder (Agent/solidcraft-gears): an agent may
run a package there, not one installed in the app's plugins folder.
"""
import mcp, json, math, time, base64, sys

A = "/storage/emulated/0/Android/data/io.solidcraft.android/files/Agent/"
PLUGIN = A + "solidcraft-gears"
DEV = sys.argv[1] if len(sys.argv) > 1 else "emu"
M, ZS, ZP = 1.5, 18, 10                      # module, sun and planet teeth (18: three pairs at 120°)
RS, RP = M * ZS / 2, M * ZP / 2              # pitch radii 13.5, 7.5
BACKLASH = 0.05                              # each mesh's centres this much further apart than standard
R = RS + RP + BACKLASH                       # planets' pitch circle, 21.05
DELTA = math.degrees(2 * math.asin((RP + BACKLASH / 2) / R))  # angle between a pair's planets, so they mesh
PS, PP = 360 / ZS, 360 / ZP                  # tooth pitch angles
SUN_W, PLANET_W = 10, 15.5
X_SUN_A, X_SUN_B, X_P1, X_P2 = 0, 16, 0, 10.5
POST_R, PLATE_D = 34, 78


def shot(name):
    r = mcp.tool("screenshot", {}, label=f"screenshot {name}")
    open(f"{DEV}-{name}.png", "wb").write(base64.b64decode(r["_images"][0]["data"]))


def frac(v):
    return v - math.floor(v)


def place(occ, x, deg_c, alpha, label):
    """A gear made along +Z, turned to run along +X: its centre at polar angle deg_c on the
    planets' circle (or the axis), turned alpha degrees about its own axis."""
    c, s = math.cos(math.radians(alpha)), math.sin(math.radians(alpha))
    u, v = (R * math.cos(math.radians(deg_c)), R * math.sin(math.radians(deg_c))) if deg_c is not None else (0, 0)
    mcp.ex("occurrence.move", {"occurrence": occ, "to_matrix": [0, 0, 1, x, c, -s, 0, u, s, c, 0, v]}, label=label)


def ring(name, x0, length, od, idia, label, operation="new", targets=None):
    p = {"diameter": od, "height": length, "base": [x0, 0, 0], "axis": "X", "operation": operation}
    if operation == "new":
        p["body_name"] = name
    if targets:
        p["targets"] = targets
    mcp.ex("solid.cylinder", p, label=label)
    if idia:
        mcp.ex("solid.cylinder", {"diameter": idia, "height": length + 2, "base": [x0 - 1, 0, 0], "axis": "X", "operation": "cut",
                                  "targets": [name]}, label=f"{label}: bore {idia}")


t_all = time.time()
print("\n== 1 gears, made by the plugin", flush=True)
mcp.init()
approval = mcp.ex("plugin.inspect", {"path": PLUGIN}, label="plugin.inspect: SolidCraft Gears 0.1.1")["approval"]
for name, z, w, bore, file in (("Sun gear", ZS, SUN_W, 10.2, "lsd-sun.sc"), ("Planet gear", ZP, PLANET_W, 4.4, "lsd-planet.sc")):
    mcp.tool("new_design", {"name": name}, label=f"new design: {name}")
    r = mcp.ex("plugin.run", {"path": PLUGIN, "approval": approval, "entry": "make-gear",
                              "params": {"module": M, "teeth": z, "face_width": w, "bore_diameter": bore}},
               label=f"plugin: make-gear m{M} z{z}, {w} wide, bore {bore}")
    print("  ", json.dumps(r.get("result"))[:200], flush=True)
    mcp.ex("view.isometric", {}, label="view iso")
    mcp.tool("save", {"path": A + file}, label=f"save {file}")

print("\n== 2 the gear train: two suns, three planet pairs, in mesh", flush=True)
mcp.tool("new_design", {"name": "Limited-slip differential"}, label="new design: Limited-slip differential")
occ = {}
for name, file in [("Sun A", "lsd-sun.sc"), ("Sun B", "lsd-sun.sc")] + [(f"Planet {k}{s}", "lsd-planet.sc") for k in (1, 2, 3) for s in "ab"]:
    r = mcp.ex("component.insert_design", {"path": A + file, "name": name}, label=f"insert {name}")
    occ[name] = r["occurrence"]
# Phases: where two gears meet, one's tooth faces the other's gap (s_a + s_b = 1/2, s the tooth
# phase at the contact direction). Both suns have 18 teeth, so each pair sees the suns alike.
alpha_sa, alpha_sb = 0.0, None
for k in range(3):
    psi = 90 + 120 * k
    f1, f2 = psi - DELTA / 2, psi + DELTA / 2
    a1 = f1 + 180 - PP * (0.5 - frac((f1 - alpha_sa) / PS))
    c1 = (R * math.cos(math.radians(f1)), R * math.sin(math.radians(f1)))
    c2 = (R * math.cos(math.radians(f2)), R * math.sin(math.radians(f2)))
    t12 = math.degrees(math.atan2(c2[1] - c1[1], c2[0] - c1[0]))
    a2 = t12 + 180 - PP * (0.5 - frac((t12 - a1) / PP))
    if alpha_sb is None:
        alpha_sb = f2 - PS * (0.5 - frac((f2 + 180 - a2) / PP))
    place(occ[f"Planet {k+1}a"], X_P1, f1, a1, f"place planet {k+1}a (meshes sun A)")
    place(occ[f"Planet {k+1}b"], X_P2, f2, a2, f"place planet {k+1}b (meshes sun B and {k+1}a)")
# The first part inserted is grounded; free sun A to place it, then ground it again.
mcp.ex("occurrence.ground", {"occurrence": occ["Sun A"], "grounded": False}, label="unground sun A")
place(occ["Sun A"], X_SUN_A, None, alpha_sa, "place sun A")
mcp.ex("occurrence.ground", {"occurrence": occ["Sun A"], "grounded": True}, label="ground sun A")
place(occ["Sun B"], X_SUN_B, None, alpha_sb, "place sun B")
r = mcp.ex("inspect.interference", {}, label="interference: the gear train")
print("   gear train interferences:", r.get("count"), [(i["a_part"]["component"], i["b_part"]["component"]) for i in r.get("interferences", [])][:6], flush=True)
mcp.ex("view.isometric", {}, label="view iso"); shot("1-gears")

print("\n== 3 clutch packs and preload springs (the limited slip)", flush=True)
for side, xs, sign in (("A", 0, -1), ("B", 26, 1)):
    for i in range(3):
        x0 = xs + sign * (0.2 + i) if sign > 0 else xs - (1 + i)
        ring(f"Clutch disc {side}{i+1}", x0, 0.8, 26, 11, f"clutch disc {side}{i+1}")
    ring(f"Preload spring {side}", xs + 3.2 if sign > 0 else xs - 4, 0.8, 26, 11, f"preload spring {side}")

print("\n== 4 carrier: end plates, posts, hubs, planet pins", flush=True)
ring("Carrier", -8, 4, PLATE_D, None, "carrier: end plate A")
ring("Carrier", 30, 4, PLATE_D, None, "carrier: end plate B", operation="join", targets=["Carrier"])
for k in range(3):
    a = math.radians(150 + 120 * k)
    mcp.ex("solid.cylinder", {"diameter": 8, "height": 34, "base": [-4, POST_R * math.cos(a), POST_R * math.sin(a)], "axis": "X",
                              "operation": "join", "targets": ["Carrier"]}, label=f"carrier: post {k+1}")
ring("Carrier", -18, 10, 16, None, "carrier: hub A", operation="join", targets=["Carrier"])
ring("Carrier", 34, 10, 16, None, "carrier: hub B", operation="join", targets=["Carrier"])
mcp.ex("solid.cylinder", {"diameter": 10.4, "height": 66, "base": [-19, 0, 0], "axis": "X", "operation": "cut", "targets": ["Carrier"]},
       label="carrier: axle bore 10.4")
for k in range(3):
    for j, f in enumerate((90 + 120 * k - DELTA / 2, 90 + 120 * k + DELTA / 2)):
        y, z = R * math.cos(math.radians(f)), R * math.sin(math.radians(f))
        mcp.ex("solid.cylinder", {"diameter": 4, "height": 44, "base": [-9, y, z], "axis": "X", "operation": "cut", "targets": ["Carrier"]},
               label=f"carrier: pin hole {k+1}{'ab'[j]}")
        mcp.ex("solid.cylinder", {"diameter": 4, "height": 42, "base": [-8, y, z], "axis": "X", "operation": "new",
                                  "body_name": f"Pin {k+1}{'ab'[j]}"}, label=f"planet pin {k+1}{'ab'[j]}")
mcp.ex("view.isometric", {}, label="view iso"); shot("2-carrier")

print("\n== 5 axles, sleeved bushings, brackets, base", flush=True)
ring("Axle A", -60, 70, 10, None, "axle A")
ring("Axle B", 16, 70, 10, None, "axle B")
ring("Bushing A", -18, 9.5, 22, 16.2, "bronze sleeve bushing A")
ring("Bushing B", 34.5, 9.5, 22, 16.2, "bronze sleeve bushing B")
for side, x0 in (("A", -18), ("B", 34.5)):
    b = f"Bracket {side}"
    mcp.ex("solid.box", {"length": 9.5, "width": 44, "height": 46, "corner": [x0, -22, -46], "body_name": b, "operation": "new"},
           label=f"{b}: upright")
    mcp.ex("solid.cylinder", {"diameter": 44, "height": 9.5, "base": [x0, 0, 0], "axis": "X", "operation": "join", "targets": [b]},
           label=f"{b}: rounded top")
    mcp.ex("solid.box", {"length": 21.5, "width": 76, "height": 6, "corner": [x0 - 6, -38, -52], "operation": "join", "targets": [b]},
           label=f"{b}: foot")
    mcp.ex("solid.cylinder", {"diameter": 22, "height": 11.5, "base": [x0 - 1, 0, 0], "axis": "X", "operation": "cut", "targets": [b]},
           label=f"{b}: bushing bore 22")
    for y in (-30, 30):
        mcp.ex("solid.cylinder", {"diameter": 6.5, "height": 8, "base": [x0 + 4.75, y, -53], "operation": "cut", "targets": [b]},
               label=f"{b}: bolt hole y={y}")
mcp.ex("solid.box", {"length": 110, "width": 90, "height": 6, "corner": [-40, -45, -58], "body_name": "Base plate", "operation": "new"},
       label="base plate 110x90x6")
mcp.ex("solid.fillet", {"edges": [[-40, -45, -55], [70, -45, -55], [-40, 45, -55], [70, 45, -55]], "radius": 6, "body": "Base plate"},
       label="base plate: R6 corners")

print("\n== 6 checks", flush=True)
r = mcp.ex("inspect.interference", {}, label="interference: everything")
print("   interferences:", r.get("count"), json.dumps([(i["a"], i.get("a_part", {}).get("component"), i["b"], i.get("b_part", {}).get("component"),
                                                        round(i.get("volume_mm3", 0), 3)) for i in r.get("interferences", [])])[:900],
      "unchecked:", r.get("unchecked"), flush=True)
d = mcp.tool("inspect_design", {"measure": True}, label="inspect + measure design")
print("   bodies:", len(d["bodies"]), "closed:", all(b["closed"] for b in d["bodies"]), flush=True)
mcp.ex("view.isometric", {}, label="view iso"); shot("3-assembly")
mcp.ex("inspect.section", {"plane": "XY", "at": 0}, label="section through the axis (XY)")
mcp.ex("view.apply", {"camera": {"target": [13, 0, 0], "yaw": 0.0, "pitch": 1.5707, "distance": 230}}, label="camera from above"); shot("4-section")
mcp.ex("inspect.section", {"clear": True}, label="clear section")
mcp.ex("view.isometric", {}, label="view iso")
mcp.tool("export", {"path": A + "limited-slip-differential.step"}, label="export STEP")
mcp.tool("save", {"path": A + "limited-slip-differential.sc"}, label="save design")
print(f"\nTOTAL {time.time()-t_all:.1f} s", flush=True)
