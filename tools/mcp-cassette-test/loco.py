"""A 3D-printable model locomotive mechanism on a section of G-scale track (45 mm gauge), built
from SolidCraft commands: a frame and deck, two cylinders on saddles, four spoked wheels on two
axles, crank pins, coupling rods and washers, and 400 mm of track (rails on sleepers).

  python3 loco.py script OUT.json      write a solidcraft-cli script (run, snapshot)
  python3 loco.py mcp [--spin]         build it in the running app over MCP (mcp.py's MCP_URL,
                                       MCP_TOKEN), timing every call; --spin then
                                       turns the view for 8 s

Sizes: 45 mm gauge, rails 8.4 mm high (code 332), 4.4 mm heads; wheels Ø52 on the tread, Ø58
over the flange, 41.3 mm back to back; 70 mm wheelbase; printed parts in grey and white PLA,
steel pins and axles, brass rails, black sleepers."""
import json, sys

GAUGE = 45.0
SLEEPER_TOP = 5.0
RAIL_TOP = SLEEPER_TOP + 8.4
TREAD_R, FLANGE_R, RIM_IN_R, HUB_R = 26.0, 29.0, 21.0, 7.0
WZ = RAIL_TOP + TREAD_R  # axle height
AXLES = (-35.0, 35.0)
CRANK = 13.0  # crank pin radius
BACK = 20.65  # half the back-to-back distance (41.3 mm): the flange's inner face, 0.7 mm play in the gauge
GREY, WHITE, STEEL, BRASS, BLACK = "#6d7075", "#efeee9", "#c3c6ca", "#b8954f", "#1f1f1f"

cmds = []


def c(command, **params):
    cmds.append((command, params))


def box(name, x, y, z, op=None, target=None, feature=None):
    """A box over the ranges x, y, z (each (min, max))."""
    p = {"corner": [x[0], y[0], z[0]], "length": x[1] - x[0], "width": y[1] - y[0], "height": z[1] - z[0]}
    if op:
        p.update(operation=op, targets=[target])
    else:
        p["body_name"] = name
    if feature:
        p["name"] = feature
    c("solid.box", **p)


def cyl(name, r, base, axis, h, op=None, target=None):
    p = {"radius": r, "height": h, "base": list(base), "axis": list(axis)}
    if op:
        p.update(operation=op, targets=[target])
    else:
        p["body_name"] = name
    c("solid.cylinder", **p)


def ys(s, a, b):
    """The range a..b (a, b >= 0) on side s (+1 left, -1 right)."""
    return (a, b) if s > 0 else (-b, -a)


def track():
    for i in range(16):
        x = -187.5 + 25.0 * i
        box(f"Sleeper {i + 1}", (x - 4.5, x + 4.5), (-35, 35), (0, SLEEPER_TOP))
    for s, side in ((1, "L"), (-1, "R")):
        head_in = GAUGE / 2
        mid = head_in + 2.2
        name = f"Rail {side}"
        box(name, (-200, 200), ys(s, mid - 4, mid + 4), (SLEEPER_TOP, SLEEPER_TOP + 1.5))
        box(name, (-200, 200), ys(s, mid - 1.2, mid + 1.2), (SLEEPER_TOP + 1.5, RAIL_TOP - 2.4), "join", name)
        box(name, (-200, 200), ys(s, head_in, head_in + 4.4), (RAIL_TOP - 2.4, RAIL_TOP), "join", name)


def wheel(s, x, side, n):
    name = f"Wheel {side}{n}"
    ax = (0, s, 0)
    cyl(name, FLANGE_R, (x, s * BACK, WZ), ax, 1.5)
    cyl(name, TREAD_R, (x, s * (BACK + 1.5), WZ), ax, 7, "join", name)
    cyl(name, RIM_IN_R, (x, s * (BACK - 0.5), WZ), ax, 10, "cut", name)
    cyl(name, HUB_R, (x, s * (BACK + 1), WZ), ax, 9, "join", name)
    spoke = f"Spoke {side}{n}"
    box(name, (x - 2, x + 2), ys(s, 24, 28), (WZ + HUB_R - 1, WZ + RIM_IN_R + 1), "join", name, spoke)
    c("solid.pattern.circular", features=[spoke], axis={"origin": [x, 0, WZ], "dir": [0, 1, 0]}, count=10)
    cyl(name, 5, (x, s * 24, WZ + CRANK), ax, 5.5, "join", name)
    cyl(name, 3.1, (x, s * (BACK - 1), WZ), ax, 12, "cut", name)
    cyl(name, 2.0, (x, s * 23, WZ + CRANK), ax, 8, "cut", name)


def copy(body, name, **how):
    """A copy of `body` (moved or mirrored), renamed: a copy is named `body (2)`."""
    if "plane" in how:
        c("solid.mirror", bodies=[body], **how)
    else:
        c("solid.move", bodies=[body], copy=True, **how)
    c("body.rename", body=f"{body} (2)", name=name)


def running_gear():
    """The left side's wheels, crank pins, coupling rod and washers; the right side mirrors them.
    One wheel is built (its spokes are one pattern, the slow step) and copied."""
    wheel(1, AXLES[1], "L", 1)
    copy("Wheel L1", "Wheel L2", translate=[AXLES[0] - AXLES[1], 0, 0])
    rod = "Coupling rod L"
    zc = WZ + CRANK
    cyl(rod, 7, (AXLES[0], 31, zc), (0, 1, 0), 4)
    cyl(rod, 7, (AXLES[1], 31, zc), (0, 1, 0), 4, "join", rod)
    box(rod, AXLES, (31, 35), (zc - 4, zc + 4), "join", rod)
    for x in AXLES:
        cyl(rod, 2.15, (x, 30, zc), (0, 1, 0), 6, "cut", rod)
    cyl("Crank pin L1", 2.0, (AXLES[1], 24, zc), (0, 1, 0), 13.5)
    copy("Crank pin L1", "Crank pin L2", translate=[AXLES[0] - AXLES[1], 0, 0])
    cyl("Washer L1", 4, (AXLES[1], 35.2, zc), (0, 1, 0), 2)
    cyl("Washer L1", 2.0, (AXLES[1], 35, zc), (0, 1, 0), 3, "cut", "Washer L1")
    copy("Washer L1", "Washer L2", translate=[AXLES[0] - AXLES[1], 0, 0])
    for b in ("Wheel L1", "Wheel L2", "Coupling rod L", "Crank pin L1", "Crank pin L2", "Washer L1", "Washer L2"):
        copy(b, b.replace(" L", " R"), plane="XZ")


def chassis():
    f = "Chassis"
    box(f, (-90, 90), (-18, 18), (30, 72))
    for xa, xb in ((-15, 15), (62, 82), (-82, -62)):
        box(f, (xa, xb), (-20, 20), (38, 64), "cut", f)
    for x in AXLES:
        cyl(f, 3.3, (x, -20, WZ), (0, 1, 0), 40, "cut", f)
    box(f, (-100, 100), (-40, 40), (72, 78), "join", f)
    box(f, (100, 106), (-40, 40), (60, 78), "join", f)
    box(f, (-106, -100), (-40, 40), (60, 78), "join", f)
    for x in AXLES:
        cyl(f"Axle {AXLES.index(x) + 1}", 3.0, (x, -31, WZ), (0, 1, 0), 62)
    for sx, end in ((1, "front"), (-1, "rear")):
        for sy, side in ((1, "L"), (-1, "R")):
            b = f"Buffer {end} {side}"
            cyl(b, 4, (sx * 106, sy * 28, 68), (sx, 0, 0), 8)
            cyl(b, 6, (sx * 114, sy * 28, 68), (sx, 0, 0), 2, "join", b)


def cylinders():
    for cx, n in ((52.0, 1), (-52.0, 2)):
        o = 1 if cx > 0 else -1  # the end the piston rod leaves by
        zc = 104.0
        box(f"Saddle {n}", (cx - 20, cx + 20), (-14, 14), (78, 88))
        barrel = f"Cylinder {n}"
        cyl(barrel, 16, (cx - 26, 0, zc), (1, 0, 0), 52)
        cyl(f"Cylinder cover {n}a", 19, (cx - 32, 0, zc), (1, 0, 0), 6)
        cyl(f"Cylinder cover {n}b", 19, (cx + 26, 0, zc), (1, 0, 0), 6)
        gland = f"Gland {n}"
        cyl(gland, 6, (cx + o * 32, 0, zc), (o, 0, 0), 4)
        cyl(f"Piston rod {n}", 2.5, (cx + o * 36, 0, zc), (o, 0, 0), 12)


def colours():
    names = [p.get("body_name") or p.get("name") for cmd, p in cmds if p.get("body_name") or cmd == "body.rename"]
    def pick(*starts):
        return [n for n in names if n.startswith(starts)]
    c("appearance.assign", bodies=pick("Chassis", "Wheel", "Cylinder 1", "Cylinder 2"), color=GREY)
    c("appearance.assign", bodies=pick("Saddle", "Cylinder cover", "Gland", "Coupling rod", "Washer"), color=WHITE)
    c("appearance.assign", bodies=pick("Axle", "Crank pin", "Piston rod", "Buffer"), color=STEEL, pbr={"metallic": 1, "roughness": 0.3, "dielectric_ior": 1.5})
    c("appearance.assign", bodies=pick("Rail"), color=BRASS, pbr={"metallic": 1, "roughness": 0.35, "dielectric_ior": 1.5})
    c("appearance.assign", bodies=pick("Sleeper"), color=BLACK)


# Where the app's view is reframed during an MCP build (display only, not in a script).
VIEWS = []


def build():
    for part in (track, chassis, running_gear, cylinders):
        part()
        VIEWS.append(len(cmds))
    colours()


if __name__ == "__main__":
    build()
    if sys.argv[1] == "script":
        json.dump({"commands": [{"command": k, "params": p} for k, p in cmds]}, open(sys.argv[2], "w"), indent=1)
        print(len(cmds), "commands")
    else:
        import time
        import mcp
        mcp.init()
        t0 = time.time()
        mcp.ex("file.new", {"name": "Locomotive"})
        mcp.ex("view.isometric", {})
        for i, (k, p) in enumerate(cmds):
            if i in VIEWS:
                mcp.ex("view.fit", {})
            mcp.ex(k, p)
        mcp.ex("view.fit", {})
        print(f"{len(cmds)} calls, {time.time() - t0:.1f} s")
        if "--spin" in sys.argv:
            mcp.ex("view.spin", {"on": True})
            time.sleep(8)
            mcp.ex("view.spin", {"on": False})
