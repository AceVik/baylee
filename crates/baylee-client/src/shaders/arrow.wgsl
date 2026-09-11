// A combat arrow: a bowed stroke with a head on it and a current running
// along it, drawn on one quad.
//
// It replaces a stretched rectangle, and the reason is not prettiness. Two
// creatures and the seat they are pointed at make a *fan* of straight lines
// crossing the middle of the table, and a straight line has no end: which of
// its two ends is the attacker and which is the defender was a thing the
// player had to know already. A curve separates lines that share an end — two
// arcs to the same seat leave it at different angles where two segments lie
// on top of each other — and a head says which way to read it.
//
// Everything below is in **table units**, taken off the uniform rather than
// off the quad, because the quad is a unit `Rectangle` scaled by the renderer
// and a fragment cannot read a scale back out of its own transform. The
// curve is a quadratic Bézier with its ends on the chord and its control
// point lifted off it, so `bulge` is the apex's height above the chord and
// zero is exactly the old straight line.
//
// # The browser's budget
//
// Uniforms only, no textures — the WebGL2 envelope, kept although the browser
// build renders through WebGPU now. See `cardmat`'s header. The one loop here
// has a constant bound and no dynamic indexing, which GLSL ES 3.0 unrolls.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::globals

struct ArrowParams {
    /// The arrow's colour, and in `w` its opacity — `STANDING` for a
    /// declaration the engine has accepted, `PROPOSED` for one this seat is
    /// still building.
    color: vec4<f32>,
    /// The quad's size in table units, so every length here is in the same
    /// units and a fragment can rebuild the arrow's own space from `uv`.
    box_size: vec2<f32>,
    /// The straight-line distance between the two ends.
    chord: f32,
    /// How far the curve's apex stands off that chord. Signed: the arrow
    /// always bows to one hand of its own direction, so two arrows between
    /// the same pair of things never lie on top of each other.
    bulge: f32,
    /// Half the width of the shaft.
    width: f32,
    /// Half the width of the head where it is widest, at its base.
    head_width: f32,
    /// How much of the curve the head takes, as a fraction.
    head_frac: f32,
    /// How many dashes the current is cut into over the whole curve.
    dashes: f32,
    /// How many dashes pass a fixed point each second.
    speed: f32,
    /// The clock: `cardmat::MOVING`, or `cardmat::STILL` for a player who has
    /// asked the table to hold still. Zero stops the current where it is and
    /// changes nothing else, which is the shape every animation here takes.
    motion: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: ArrowParams;

/// How many straight pieces the curve is measured as.
///
/// The distance is taken to each *segment* and not to each sampled point, so
/// this is not a resolution in the way a point sample would be: a mild bow
/// over twenty pieces is already within a thousandth of a table unit of the
/// true curve, which is far under a pixel at any camera this table allows.
/// What it does set is how finely `t` is known, and `t` is what the current
/// runs on.
const PIECES: i32 = 20;

/// Distance from `p` to the segment `a`–`b`, and how far along it the nearest
/// point lies.
///
/// `x` is the distance and `y` the parameter, returned together because the
/// caller wants both and computing the foot of the perpendicular twice is the
/// whole cost of the function.
fn sd_piece(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-9), 0.0, 1.0);
    return vec2<f32>(length(pa - ba * h), h);
}

/// The curve, in the arrow's own space: both ends on the x axis and the apex
/// `bulge` above it.
///
/// A quadratic Bézier's apex sits at `(P0 + 2·P1 + P2) / 4`, so the control
/// point is lifted *twice* the bulge to put the curve itself at the bulge.
fn curve_at(t: f32) -> vec2<f32> {
    let half = params.chord * 0.5;
    let p0 = vec2<f32>(-half, 0.0);
    let p1 = vec2<f32>(0.0, params.bulge * 2.0);
    let p2 = vec2<f32>(half, 0.0);
    let s = 1.0 - t;
    return s * s * p0 + 2.0 * s * t * p1 + t * t * p2;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // The quad's own space in table units, x along the chord and y across it.
    // The mesh is a `Rectangle` laid flat and yawed, so `uv.y` runs the
    // opposite way to the local axis and is turned round here rather than in
    // the renderer, where it would be a sign nobody could check.
    let p = vec2<f32>(
        (in.uv.x - 0.5) * params.box_size.x,
        (0.5 - in.uv.y) * params.box_size.y,
    );

    // Nearest point on the curve, and where along it that was.
    var best = vec2<f32>(1e9, 0.0);
    var prev = curve_at(0.0);
    for (var i = 1; i <= PIECES; i++) {
        let u = f32(i) / f32(PIECES);
        let next = curve_at(u);
        let hit = sd_piece(p, prev, next);
        if hit.x < best.x {
            best = vec2<f32>(hit.x, (f32(i - 1) + hit.y) / f32(PIECES));
        }
        prev = next;
    }
    let d = best.x;
    let t = best.y;

    // The head is a triangle laid on the curve: as wide as `head_width` where
    // it starts and closing to nothing at the tip. Widening abruptly is what
    // makes it read as a head rather than as a shaft that got fat — an arrow
    // drawn with a smooth taper is a brush stroke.
    let base = 1.0 - params.head_frac;
    let along = clamp((t - base) / max(params.head_frac, 1e-4), 0.0, 1.0);
    let head = select(0.0, params.head_width * (1.0 - along), t >= base);
    let stroke = max(params.width, head);

    let aa = max(fwidth(d), 1e-5);
    let coverage = clamp(0.5 + (stroke - d) / aa, 0.0, 1.0);
    if coverage <= 0.0 {
        discard;
    }

    // The current, and it stops at the head. A dashed arrowhead is a broken
    // arrowhead: the head is the part that says which way this is read, so it
    // is the one part that is never allowed to be half there.
    let phase = fract(t * params.dashes - globals.time * params.motion * params.speed);
    let dash = smoothstep(0.0, 0.18, phase) - smoothstep(0.62, 0.92, phase);
    let lit = select(mix(0.45, 1.0, dash), 1.0, t >= base);

    return vec4<f32>(params.color.rgb, params.color.a * lit * coverage);
}
