// The table the game is played on: a slab of dark timber with a channel of
// resin poured through it.
//
// Two textures and a handful of numbers. `timber` is a picture of wood and
// has nothing to do; everything that moves is in the channel, and the channel
// is not a picture at all but a *field* — `baylee_client_core::tabletop::
// channel` writes how deep the resin is, which way it runs and where its
// shore is, computed from the seating. So the water is wherever a card never
// lies, at two seats and at eight, without this shader knowing what a seat is.
//
// What the resin is *not*: it is not lava, and it is not water. It is a dark,
// nearly colourless glass, and its colour comes from `params.wash` — the same
// four lamps the table has always been lit by, which say where in the turn
// the game is. A permanently orange channel would say "it is combat" in every
// step of every turn, and in a four-player game it would quietly overwrite
// the red seat's own rim. So the channel is water at untap and molten at
// combat because *the turn* is, not because the table was painted that way.
//
// # WebGL2
//
// The browser build targets WebGL2: uniforms only, no storage buffers, no
// texture arrays, every loop bound at compile time. The one loop below counts
// to four literally. `globals.time` comes from the view bind group, so the
// current runs without the CPU touching a material asset per frame.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::globals

struct RiverParams {
    /// The phase lamp: `rgb` its colour, `a` how much of it there is.
    wash: vec4<f32>,
    /// Where the light enters the channel — `xy` a point on the active
    /// seat's shore in table space, `zw` the direction it travels from there.
    source: vec4<f32>,
    /// The slab's world size, which is what turns a point on the table into a
    /// point in the two fields above.
    span: vec2<f32>,
    /// The clock the current runs on: 1 normally, 0 for reduce-motion.
    motion: f32,
    /// How hard the wash burns at full energy. Above 1 on purpose, so
    /// combat blooms and the cards — which are unlit — do not.
    gain: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var channel: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var channel_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> params: RiverParams;

// The timber, drawn here rather than sampled from an image.
//
// It was a texture, and a texture cannot win this. The slab is about
// thirty-five units across; at the distance the camera now sits, a card is
// roughly 114 physical pixels, so drawing the wood sharply would need some
// 4000 texels across — and measured in a debug build, generating 2048 already
// costs 1.6 seconds every time the table is re-cut, while still being half as
// sharp as needed. Arithmetic has no resolution.
//
// `baylee_client_core::tabletop::timber` remains the reference: it is the same
// arithmetic on the CPU, where a test can block the image at card size and
// measure that the grain survives, and check that every sample keeps red above
// blue. `shader_tests::the_shader_and_the_generator_agree_about_the_wood`
// reads the numbers below out of this file and fails if the two drift apart.
const WOOD_DEEP: vec3<f32> = vec3<f32>(0.085, 0.058, 0.044);
const WOOD_BASE: vec3<f32> = vec3<f32>(0.170, 0.118, 0.084);
const WOOD_PALE: vec3<f32> = vec3<f32>(0.255, 0.185, 0.132);
const GRAIN_LINES: f32 = 0.62;
const GRAIN_WANDER: f32 = 1.8;
const TAU: f32 = 6.2831855;


/// The resin with no light in it: a dark, warm, almost colourless glass.
///
/// Warm — red above blue — for the same reason the timber is. A cold black
/// eats the identity of black and blue cards, and this is the surface
/// directly beside them. Darker than the wood, so the channel reads as an
/// inlay even in a step where the lamp is nearly out.
///
/// **Display-referred**, like every other colour this project writes down,
/// and therefore run through `to_linear` before it is used. That conversion
/// is not a detail: the first version of this shader added these numbers
/// straight into a linear render target, and a channel meant to sit at 0.22
/// measured 0.45 on screen — a pale grey tube where dark glass belonged. The
/// timber escaped it only because a texture in an sRGB format is decoded on
/// the way in, which is exactly the round trip a raw constant does not get.
const RESIN: vec3<f32> = vec3<f32>(0.060, 0.052, 0.050);

/// How fast the current travels, in uv per second. Slow: a player reading a
/// card must never catch the table moving out of the corner of their eye.
const CURRENT: f32 = 0.014;

/// Swirls per uv — the size of one eddy. Kept coarse for the same reason the
/// timber's grain is: detail finer than a card competes with the card. At 9
/// over a slab about thirty units across, an eddy is roughly three cards and
/// the finest octave is about one; the first draft's 3.5 put a single eddy
/// across ten cards, which does not read as moving water at all.
const EDDY: f32 = 9.0;

/// How far the deep layer lags the surface one. This is the whole illusion of
/// a pour with a bottom to it, and it is a *fixed* offset rather than one
/// taken from the view: a highlight that tracked the camera would drag along
/// behind every card whenever the table was turned.
const PARALLAX: f32 = 0.055;

/// The painted shimmer, in **linear** light. A fixed window, not a moving one.
const SHEEN: f32 = 0.030;

/// The meniscus, in linear light: the line where resin has climbed the timber
/// it was poured against. Most of what says "poured" rather than "painted".
const MENISCUS: f32 = 0.035;

/// The light left in the resin when no step is calling for any.
///
/// Measured, and it is the correction to the squaring below. A main phase is
/// graded at 0.16, which squares to 0.026, and at that the channel rendered
/// (43, 36, 26) against timber at (42, 29, 21) — the same surface. The curve
/// was tuned at the combat end and the quiet end fell through it, which is the
/// end the game spends nearly all of its time at.
///
/// So the resin keeps a floor: enough that it always reads as lit glass rather
/// than as a darker board, and low enough that combat still arrives as a
/// change. At 0.055 the quiet channel measures about (58, 48, 34) at the far
/// shore and (84, 72, 50) at the near one — an amber inlay in dark wood.
const RESTING: f32 = 0.055;

/// sRGB to linear, componentwise.
///
/// Everything this project writes down as a colour — the pie, the phase
/// lamps, the resin above — is what the table should *look* like. A fragment
/// shader's output is light. These are not the same numbers.
fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    let lo = c / 12.92;
    return select(lo, hi, c > vec3<f32>(0.04045));
}

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

/// Four octaves, normalised back to 0..1 so callers can centre it on a half.
fn fbm2(p: vec2<f32>) -> f32 {
    var sum = 0.0;
    var amplitude = 0.5;
    var total = 0.0;
    var at = p;
    for (var i = 0; i < 4; i = i + 1) {
        sum = sum + amplitude * vnoise(at);
        total = total + amplitude;
        at = at * 2.03;
        amplitude = amplitude * 0.5;
    }
    return sum / total;
}

/// The wood at a point of table, in display-referred colour.
fn timber_at(p: vec2<f32>) -> vec3<f32> {
    // Stretched hard along x: a feature is many times longer than it is wide,
    // which is the whole difference between grain and noise.
    let drift = fbm2(vec2<f32>(p.x * 0.055, p.y * 0.16) + 11.7) - 0.5;
    let figure = fbm2(vec2<f32>(p.x * 0.10, p.y * 0.42) + 53.1) - 0.5;

    // `sin` rather than a sawtooth, so a line has two soft shoulders instead
    // of one hard step; squared, because real figure is mostly pale wood with
    // narrow dark lines in it and not an even wave.
    let phase = (p.y + drift * GRAIN_WANDER * 2.0) * GRAIN_LINES * TAU;
    let ring = 0.5 + 0.5 * sin(phase);
    let shaped = ring * ring;

    let base = mix(WOOD_DEEP, WOOD_BASE, shaped);
    return mix(base, WOOD_PALE, (figure + 0.5) * shaped * 0.55);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Table space out of the world position, rather than the quad's own uv.
    // The mesh's uv origin is a convention of whichever primitive built it,
    // and a wrong guess about it mirrors the whole field — invisible at two
    // seats, because a duel is symmetric about both axes, and wrong at three.
    // `to_world` is `(x, height, -y)`, so this is exactly its inverse.
    let table = vec2<f32>(in.world_position.x, -in.world_position.z);
    let uv = table / params.span + 0.5;

    // Display-referred, like everything this project writes down as a colour,
    // so it needs the same decode the sRGB texture used to do on the way in.
    let wood = to_linear(timber_at(table));

    // `r` how deep, `gb` which way the current runs, `a` resin or wood with a
    // shore two texels wide so the waterline is a line and not a staircase.
    let field = textureSample(channel, channel_sampler, uv);
    let depth = field.r;
    let flow = field.gb * 2.0 - 1.0;
    let wet = field.a;

    let t = globals.time * params.motion;
    let drift = flow * t * CURRENT;

    // Two layers, the lower one lagging: the surface where the light catches
    // it, and something further down that the surface is seen through.
    let surface = fbm2(uv * EDDY - drift);
    let under = fbm2((uv + flow * depth * PARALLAX) * EDDY * 0.62 - drift * 0.55 + 7.3);
    let swirl = mix(under, surface, 0.55);

    // Deeper resin is darker, and the shallows at the shore catch what light
    // there is — which is what makes a poured edge read as a thickness rather
    // than as a painted line.
    let body = to_linear(RESIN) * mix(1.75, 0.60, depth);

    // The lamp. It enters at the active seat's shore and runs out across the
    // channel, so combat begins on the attacker's side of the table and
    // reaches the defender — the seam between the two ends is the moment
    // itself, not an ornament that is always there.
    let along = dot(table - params.source.xy, params.source.zw) / max(params.span.y, 1.0);
    let near_bank = 1.0 - smoothstep(-0.15, 0.85, along);
    // The far shore keeps nearly half. At 0.30 the gradient was doing the
    // squaring's job a second time, and the end of the channel the defender
    // sits at went out altogether — a river with a dry end is not a river.
    let reach = mix(0.45, 1.0, near_bank);

    // Energy squared, and that is the whole reason combat can bloom while a
    // main phase stays a quiet amber inlay. `phase_light` grades its lamps
    // the way a colour is graded — by eye, on a screen — so the step from a
    // main phase (0.16) to combat damage (1.0) is a factor of six in a
    // display-referred number and nearer forty in light. Squaring is that
    // transfer, near enough, and it is what makes one gain serve both ends.
    let energy = max(params.wash.a * params.wash.a, RESTING);
    let lamp = to_linear(params.wash.rgb);
    let lit = lamp * energy * params.gain * reach * depth * (0.45 + 0.85 * swirl);

    // Painted shimmer, with no direction in it at all — the crests of the
    // swirl and nothing else. A specular lobe would need a light, and the
    // stage deliberately has none.
    let sheen = pow(smoothstep(0.58, 1.0, swirl), 3.0) * SHEEN;

    // The meniscus: resin climbs the timber it was poured against, and that
    // thin bright line is most of what says "poured" rather than "painted".
    let shore = 1.0 - smoothstep(0.0, 0.09, depth);
    let rim = shore * wet * MENISCUS * (1.0 + 3.0 * energy);

    let resin = body + lit + sheen + rim * lamp;
    return vec4<f32>(mix(wood, resin, wet), 1.0);
}
