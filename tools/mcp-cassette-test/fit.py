"""Thread fit from meshes: unwind both threads along the M2x0.4 helix into one axial section and
find, for every turn of the screw, the smallest radial gap between the screw and the tapped hole."""
import sys, struct, numpy as np
X0, Y0, P = float(sys.argv[3]) if len(sys.argv) > 3 else -46.2, float(sys.argv[4]) if len(sys.argv) > 4 else 4.0, 0.4
Z_LO, Z_HI = 4.9, 6.2       # where both threads run (away from either thread's ends)
def stl(path):
    b = open(path, "rb").read()
    n = struct.unpack_from("<I", b, 80)[0]
    a = np.frombuffer(b, dtype=np.dtype([("n", "<3f4"), ("v", "<9f4"), ("x", "<u2")]), count=n, offset=84)
    tri = a["v"].reshape(-1, 3, 3).astype(float)
    # Points spread over every triangle, not just its corners.
    rng = np.random.default_rng(1)
    k = 24
    u, v = rng.random((2, tri.shape[0], k))
    flip = u + v > 1
    u[flip], v[flip] = 1 - u[flip], 1 - v[flip]
    pts = tri[:, None, 0] + u[..., None] * (tri[:, None, 1] - tri[:, None, 0]) + v[..., None] * (tri[:, None, 2] - tri[:, None, 0])
    return np.concatenate([pts.reshape(-1, 3), tri.reshape(-1, 3)])
def section(v, keep):
    x, y, z = v[:, 0] - X0, v[:, 1] - Y0, v[:, 2]
    r = np.hypot(x, y); th = np.arctan2(y, x)
    m = keep(r) & (z > Z_LO) & (z < Z_HI)
    zeta = np.mod(z[m] - P * th[m] / (2 * np.pi), P)   # right-handed: z rises with the angle
    return zeta, r[m]
BINS = 400
def profile(zeta, r, f):
    idx = np.minimum((zeta / P * BINS).astype(int), BINS - 1)
    out = np.full(BINS, np.nan)
    for i in range(BINS):
        s = r[idx == i]
        if s.size: out[i] = f(s)
    return out
screw = stl(sys.argv[1]); top = stl(sys.argv[2])
ez, er = section(screw, lambda r: r < 1.2)
iz, ir = section(top, lambda r: r < 1.25)
rext = profile(ez, er, np.max)   # screw: its outermost point in each slice
rint = profile(iz, ir, np.min)   # hole: its innermost point in each slice
print(f"screw section points {ez.size}, hole section points {iz.size}; bins empty: screw {np.isnan(rext).sum()}, hole {np.isnan(rint).sum()}")
print(f"screw r {np.nanmin(rext):.3f}..{np.nanmax(rext):.3f} (major 2.0 -> 1.000), hole r {np.nanmin(rint):.3f}..{np.nanmax(rint):.3f}")
best = None
rows = []
for deg in range(0, 360):
    shift = int(round(deg / 360 * BINS))
    gap = rint - np.roll(rext, shift)   # turning the screw by +deg moves its thread up by deg/360 pitch
    g = np.nanmin(gap)
    rows.append((deg, g))
    if best is None or g > best[1]: best = (deg, g)
for deg, g in rows[::30]:
    print(f"  turn {deg:3d} deg: min radial gap {g:+.4f} mm" + ("  (overlap)" if g < 0 else ""))
print(f"BEST turn {best[0]} deg: min radial gap {best[1]:+.4f} mm")
print(f"AS BUILT (0 deg): min radial gap {rows[0][1]:+.4f} mm")
