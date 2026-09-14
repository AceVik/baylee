// The grain every generated surface in this client is made of.
//
// One hash, and the value noise and fbm built on it. It is a module rather
// than a function copied into each shader for the reason `docs/legal.md` §2
// gives about there being no pictures here at all: if ornament is arithmetic,
// then the arithmetic is the thing that has to be consistent. Two surfaces
// that grained differently would be two authors.
//
// **No trigonometry in the hash.** A `sin`-based hash differs between
// drivers, so the same window can grain differently on two machines — and the
// table's own felt already made the argument that everyone should see the
// same surface. `tabletop.rs` computes its buffers in Rust with a seeded
// value noise for exactly the same reason.

/// A hash with no trigonometry in it.
fn hash2(p: vec2<f32>) -> f32 {
    var h = dot(p, vec2<f32>(127.1, 311.7));
    h = fract(h * 0.1031);
    h *= h + 33.33;
    h *= h + h;
    return fract(h);
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
