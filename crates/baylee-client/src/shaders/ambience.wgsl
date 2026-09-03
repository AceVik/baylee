// The moving ground the lobby and the loading screen sit on.
//
// Ornament, and therefore arithmetic: `docs/legal.md` §2 is the reason there
// is no picture here. Everything below is value noise over a drifting domain,
// which borrows nothing from anyone and costs one pass over the node.
//
// Three layers, in the order they read: a slow warped field that gives the
// surface its clouds, a set of thin bands lensed by that same field (the
// "aurora", which is what actually reads as motion), and a vignette that
// keeps the middle of the screen the brightest thing on it so text stays
// legible on top. `params.energy` scales the second and third; at 0 the
// surface is still there and simply stops moving, which is what
// `Preferences::reduce_motion` asks for.

#import bevy_render::globals::Globals
#import bevy_ui::ui_vertex_output::UiVertexOutput

struct AmbienceParams {
    /// The ground, and what the bands are drawn in.
    low: vec4<f32>,
    high: vec4<f32>,
    /// How far the bands travel and how bright they get. 0 stills the whole
    /// surface without flattening it.
    energy: f32,
    /// Separates two surfaces on screen at once, so the loading panel does
    /// not repeat the backdrop behind it pixel for pixel.
    seed: f32,
    /// The node's aspect, so the field is not stretched on a wide window.
    aspect: f32,
    /// Padding to the 16-byte boundary a uniform needs.
    pad: f32,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> params: AmbienceParams;

/// A hash with no trigonometry in it.
///
/// `sin`-based hashes differ between drivers — the same page can grain
/// differently on two machines, and the table's own felt already made the
/// argument that everyone should see the same surface.
fn hash2(p: vec2<f32>) -> f32 {
    var h = dot(p, vec2<f32>(127.1, 311.7));
    h = fract(h * 0.1031);
    h *= h + 33.33;
    h *= h + h;
    return fract(h);
}

/// Value noise, smoothed with the usual quintic so the derivative is
/// continuous and the field has no visible cell edges.
fn noise(p: vec2<f32>) -> f32 {
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
fn fbm(p: vec2<f32>) -> f32 {
    var sum = 0.0;
    var amp = 0.5;
    var at = p;
    for (var i = 0; i < 4; i = i + 1) {
        sum = sum + amp * noise(at);
        at = at * 2.03 + vec2<f32>(17.0, 9.0);
        amp = amp * 0.5;
    }
    return sum;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let t = globals.time * params.energy;
    // Aspect-corrected so a circle stays a circle on a 21:9 monitor.
    var uv = in.uv * vec2<f32>(params.aspect, 1.0) + vec2<f32>(params.seed, params.seed * 0.7);

    // The field, warped by a slower copy of itself. Domain warping is what
    // stops fbm from reading as fog and starts it reading as current.
    let warp = vec2<f32>(
        fbm(uv * 1.6 + vec2<f32>(t * 0.05, t * -0.03)),
        fbm(uv * 1.6 + vec2<f32>(-t * 0.04, t * 0.06) + 4.0),
    );
    let field = fbm(uv * 2.2 + warp * 1.4 + vec2<f32>(0.0, t * 0.02));

    // Bands, lensed through the same warp so they bend with the field
    // instead of sliding across it.
    let ribbon = sin((uv.y * 6.0 + warp.x * 3.0 - t * 0.35) * 3.14159);
    let bands = pow(max(ribbon, 0.0), 6.0) * (0.35 + 0.65 * field) * params.energy;

    // Brightest in the middle, so whatever is written on top keeps its
    // contrast at the edges of a wide window.
    let centred = (in.uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(params.aspect, 1.0);
    let vignette = 1.0 - smoothstep(0.15, 0.95, length(centred));

    // A hint of the accent, not a wash of it. The first version mixed more
    // than half way and the sign-in panel sat on a wall of teal: this is the
    // ground *behind* a form somebody is reading, and the brightest thing on
    // the screen has to stay the thing they are typing into.
    let base = mix(params.low.rgb, params.high.rgb, field * 0.14 + 0.02);
    let lit = base + params.high.rgb * bands * 0.14;
    let rgb = lit * (0.65 + 0.35 * vignette);
    return vec4<f32>(rgb, params.low.a);
}
