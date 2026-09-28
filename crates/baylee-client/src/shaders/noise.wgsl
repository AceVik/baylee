// The grain every generated surface in this client is made of.
//
// One hash, and the value noise and fbm built on it. It is a module rather
// than a function copied into each shader for the reason `docs/legal.md` §2
// gives about there being no pictures here at all: if ornament is arithmetic,
// then the arithmetic is the thing that has to be consistent. Two surfaces
// that grained differently would be two authors.
//
// **No trigonometry in the hash, and no float arithmetic.** A `sin`-based
// hash differs between drivers, so the same window can grain differently on
// two machines — and the table's own felt already made the argument that
// everyone should see the same surface. `hash2` says why floats went too.
// The sky and the felt import it rather than keep a copy. `tabletop.rs` computes its buffers in Rust with a seeded
// value noise for exactly the same reason.

/// A hash in integers, 0..1.
///
/// **No float arithmetic in the hash either.** The one before this was
/// `fract` over a `dot` and two products. At the coordinates the sky's
/// clouds reach (hundreds of cells) it kept a few bits of the point, and
/// WGSL lets a compiler reassociate and fuse float arithmetic (§15.7), so
/// the two cells either side of an edge could hash their shared corner
/// differently. Value noise with corners that disagree is a field of
/// squares, which is what a Windows player on an RTX 4070 Ti under Vulkan
/// saw in the sky and in the table (report of 28 September); the Mac drew
/// the same code smoothly. Integer operations have one answer on every GPU,
/// and the mixing constants are `tabletop.rs`'s, whose Rust hash grains the
/// cloth's buffers.
///
/// `p` is split into its cell and its place in the cell. A lattice point
/// has no place (`fract` of an integer-valued float is 0, and a negative
/// zero is folded into it), so a corner hashes the same from either side;
/// a caller that adds a fraction to draw a second value from the same cell
/// (`id + 7.1`) still gets a different one.
fn hash2(p: vec2<f32>) -> f32 {
    let cell = bitcast<vec2<u32>>(vec2<i32>(floor(p)));
    let off = fract(p);
    let place = select(bitcast<vec2<u32>>(off), vec2<u32>(0u), off == vec2<f32>(0.0));
    var h = (cell.x * 0x27d4eb2du) ^ (cell.y * 0x165667b1u)
        ^ ((place.x ^ (place.y * 0x85ebca6bu)) * 0x9e3779b9u);
    h = h ^ (h >> 15u);
    h = h * 0x2c1b3c6du;
    h = h ^ (h >> 12u);
    h = h * 0x29745c65u;
    h = h ^ (h >> 15u);
    // 24 bits, the most an `f32` holds exactly.
    return f32(h >> 8u) * (1.0 / 16777216.0);
}

/// Value noise, smoothed with the usual quintic so the derivative is
/// continuous and the field has no visible cell edges.
fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

/// Four octaves. A fifth is not visible at these amplitudes and costs a
/// quarter of the shader.
fn fbm2(p: vec2<f32>) -> f32 {
    var sum = 0.0;
    var amp = 0.5;
    var at = p;
    for (var i = 0; i < 4; i = i + 1) {
        sum = sum + amp * noise2(at);
        at = at * 2.03 + vec2<f32>(17.0, 9.0);
        amp = amp * 0.5;
    }
    return sum;
}

/// The same field along one axis, off the same hash.
///
/// A surface whose structure runs one way wants a field that runs one way.
/// The cloth under the hand is the case that asked for it: a hanging fold is
/// vertical over its whole length, and an isotropic field in a strip 150
/// pixels tall and up to 3200 wide reads as a flat wash — there is no room
/// across it for a two-dimensional feature to be seen.
fn noise1(x: f32) -> f32 {
    let i = floor(x);
    let f = fract(x);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    return mix(hash2(vec2<f32>(i, 0.0)), hash2(vec2<f32>(i + 1.0, 0.0)), u);
}

/// Three octaves of [`noise1`], which is what a fold needs: the fourth is a
/// quarter of a pixel at this scale.
fn fbm1(x: f32) -> f32 {
    var sum = 0.0;
    var amp = 0.5;
    var at = x;
    for (var i = 0; i < 3; i = i + 1) {
        sum = sum + amp * noise1(at);
        at = at * 2.03 + 17.0;
        amp = amp * 0.5;
    }
    return sum;
}
