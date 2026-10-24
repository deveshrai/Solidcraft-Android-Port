# Native B-rep, schema 3

This is SolidCraft's geometry data model in `solidcraft_geom::brep`. It has no
modelling-backend dependency. It is the body encoding of a design's model cache
([the container format](design-container.md#model-cache)); it is not a STEP
replacement for external interchange. The container reader, binary codec and
kernel adapters are separate layers. The truck adapter writes it exactly: a body
decoded by the producer that encoded it is that body, number for number.

## Geometry and topology

Coordinates are IEEE-754 binary64, in millimetres, right-handed Z-up. Angles are
radians. Integer references are unsigned 32-bit shell-local indices, never kernel
addresses or runtime IDs. The schema version is an explicit unsigned 32-bit field.
A body contains ordered shells; a shell contains ordered vertices, edges and faces.
Order is retained for SolidCraft's face provenance and naming. Closed and open
shells are representable; material containment is not inferred from shell order.

An edge references its start/end vertices in its curve's forward direction. Each
face contains ordered closed loops of oriented edge uses. A reversed use swaps
edge endpoints. Loops record the physical face-oriented traversal. Face `forward` flips only the
surface normal; it never swaps surface parameters or reverses these stored loops.
Adapters must convert their internal boundary conventions separately. Shared edges are stored once per shell. Closed edges may reference the same endpoint vertex; individual adapters may refuse them.
The validator checks references and connected closed loops. It does not certify
watertightness, geometric coincidence, absence of self-intersections or trustworthy
modelling state; backend extraction has additional requirements.

Curves are tagged mathematical records:

- `line`: start/end points, linear parameter from 0 to 1.
- `polyline`: ordered points, segment i spans parameter i to i+1.
- `spline`: full nondecreasing knot vector and Cartesian control points. Degree is
  knot count minus control-point count minus one.
- `rational_spline`: the same basis with homogeneous `[x*w,y*w,z*w,w]` coefficients,
  retaining the original coefficients rather than normalizing/reapproximating them.
- `circle_arc` and `ellipse_arc`: an orthonormal frame, radius/radii and an
  increasing angular range. Position is origin + x*rx*cos(t) + y*ry*sin(t).
- `intersection`: an explicit ordinary 3D polynomial spline (knots and Cartesian
  control points), positive fitting tolerance in millimetres, and optional two
  shell-local face references as hints. No supporting surfaces, leader, backend
  solver or parameter-search convention is required to evaluate it. (Read only;
  schema 3 writers store the intersection itself, below.)
- `surface_intersection`: the intersection of two surface records, traced from a
  `leader` (a polyline or spline near it, its knots as the producer holds them) to
  within `tolerance` mm. The curve is the procedural one, not a fit: a different
  kernel must trace it again.

Surfaces include planes (origin and independent parameter axes), polynomial and
rational tensor-product splines, and explicit analytic records. Analytic frames
store origin and orthonormal x/y axes; z is x cross y. Angular ranges are radians:

- cylinder: radius, angular range and axial height range;
- cone: radius at origin, half angle, angular range and height range; radius at
  height h is radius_at_origin + h*tan(half_angle);
- sphere: radius, longitude and latitude ranges;
- torus: major/minor radii, longitude and meridian ranges;
- revolution: profile, origin, unit axis and angular range. Rotate the profile
  around that axis using the right-hand rule. There is no persisted affine matrix
  or backend orientation/parameter-swap flag. (Read only in schema 3.)
- placed revolution: a full revolution of a profile about an origin and unit axis,
  then mapped by a column-major affine `matrix` (non-singular), with the normal
  reversed unless `forward`. Nothing is baked into the profile.

A `plane_through` record is a plane given by three points: parameter (0, 0) at
`origin`, (1, 0) at `u_point` and (0, 1) at `v_point`. Writers use it rather than
the axis form, whose axes would be rounded differences of the points.

Cylinder/cone positions are origin + h*z + r(h)*(cos(u)*x + sin(u)*y).
Sphere positions are origin + r*(cos(v)*(cos(u)*x + sin(u)*y) + sin(v)*z).
Torus positions are origin + (R+r*cos(v))*(cos(u)*x + sin(u)*y) + r*sin(v)*z.
Analytic normals point outward; face orientation can flip them. Each adapter
must preserve the geometric locus and explicit face-oriented boundaries, even
when its runtime uses a different parameterization.

## Validation and trust

Schema 3 caps 1,024 shells, 1,000,000 combined topology/geometry items, geometry
nesting at 16, spline degree at 16 and control points per spline dimension at
8,192. Numbers must be finite and bounded in magnitude by 1e12. Knot order, active
spline domain, multiplicities, rectangular grids, positive rational weights,
nondegenerate planes, orthonormal frames, unit revolution axes, positive radii,
valid angular/height ranges and intersection face references are checked before an adapter is called. These are resource and structural guards,
not a geometric validity certificate. The binary reader must additionally bound
bytes, allocation and nesting before creating these structures.

Schema tests build and validate an open triangular face without constructing or
linking a kernel body. Kernel-specific structures must appear only inside adapters;
they must never acquire authority over persisted field names or encodings. A
second backend still requires an implemented and tested adapter before claiming
cross-kernel interoperability.

Face-source certificates, operation history, appearances and persistent document
identities belong to a versioned SolidCraft body/state envelope around this geometry.
They must be preserved by the cache codec, not reconstructed from face counts or
floating-point comparisons. A decoded cache remains provisional until authoritative
re-evaluation finishes. The document and original blobs remain sufficient to reopen
when a cache, representation, producer or adapter is unsupported.

## Body binary envelope, version 4

`SCBREP04` is the eight-byte envelope signature. Geometry schema 3 is nested in
`solid`; metadata schema 1 is nested in `metadata`; `runtime` holds what the body
measured about itself that later operations read (its size, from which kernel
tolerances follow, and the orientation mesh's missing-face count), so a decoded
body works to the same tolerances. These are separate version numbers because
each contract can evolve independently. Signatures 01 and 02 were development
prototypes; 02 introduced the explicit geometry schema while 03 also separated
metadata from runtime Rust types and 04 made the encoding exact. Unsupported
versions are cache misses, not design-file migration failures.

The envelope uses its own explicitly versioned records for color, paint, authored
boundaries, face-source certificates, operation history and boundary protection.
Plane, region and segment records are also owned by the wire schema. Exhaustive
manual adapter conversions separate these from runtime FaceSource, History,
FaceProv, EdgeProv, Paint, Seg2, Region2 and Plane types. Their serde derives or
internal refactors cannot silently redefine the saved fields. No truck serde
struct, pointer, tessellation or STEP string is written. Changing field semantics
requires a format version change, even if a Rust type keeps its name.

Values use tags: null=0, false=1, true=2, signed integer=3, unsigned integer=4,
binary64=5, UTF-8 string=6, array=7, object=8. Numbers use eight little-endian
bytes. Strings and collections use unsigned 32-bit little-endian lengths; object
keys are length-prefixed UTF-8, emitted lexicographically. Readers reject duplicate
keys, unknown tags, non-finite numbers and trailing bytes. Limits per body are
256 MiB, 20,000,000 values, depth 48, 1,000,000 entries per collection and 1 MiB per
string; a design's cache bounds the total (see the container format).

The truck adapter writes every curve and surface as truck holds it: lines,
B-spline and NURBS coefficients, the three points of a plane, a revolution's
profile, axis, matrix and orientation, and an intersection's two surfaces, leader
and tolerance. Decoding builds the same structures without the constructors that
would round them (normalizing an axis, composing a matrix with the identity), and
rebuilds each shell's topology as recorded, including closed edges on one vertex
and seam edges a wire uses twice. On reading it refuses standalone polylines,
unclamped spline domains and non-full angular carrier surfaces in the portable
records it does not write. Circle/ellipse arcs use exact rational quadratic loci;
the adapter may reparameterize them. Normal flips and backend boundary orientation
are converted separately.

Hashes prove which document and payload a cache belongs to, not who wrote it: the
container's key ties a cache to the producer, application build and platform that
computed it, and the decoder applies the structural and resource checks above to
every body. A second backend needs its own tested adapter before cross-kernel
interoperability can be claimed.

The box-and-fillet golden JSON fixture pins the decoded version-4 envelope,
field names, tags, topology, geometry and metadata. Integer and structural values
are exact; floating coefficients allow 1e-10 relative/absolute platform rounding.
The signature is asserted separately. This protects the wire contract against
accidental runtime-refactor changes.

## Bounded provisional display

`native_preview_mesh` samples at most 32 points along each curved edge, caps each
face's boundary at 4,096 points, and bounds spline controls and nested surfaces
before parameter searches. Searches use at most 16 refinement trials. Revolution
profiles involving procedural intersections are refused in this display path.
The separate mesher uses a fixed grid of at most 16 subdivisions per direction
and fixed periodic-seam connections; it never invokes adaptive edge, surface or
aspect-ratio refinement. Planes use a single grid cell.

Before projection, the caller reserves a conservative triangle bound covering
duplicated projected boundary points, periodic seams and the grid for the complete
body. Output is checked against that reservation and for finite coordinates.
Missing faces or exceeded budgets cause a cache miss. This mesh is approximate,
read-only display; normal modelling/export tessellation is unchanged.
