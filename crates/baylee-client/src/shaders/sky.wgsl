// What is behind the table: a clouded sky by day, stars and a crescent moon
// by night, and a warm band across both at dawn and dusk.
//
// Ornament, and therefore arithmetic. `docs/legal.md` §2 is the reason there
// is no photograph here, and it is also the cheap answer: one pass over a
// quad, no texture to load, no bytes in the wasm bundle and nothing to fail
// to fetch in a browser.
//
// # Why it is drawn in screen space
//
// The quad is a child of the camera and covers the frustum, and everything
// below works from the **framebuffer** position rather than the mesh's uv.
// That is what makes the sky hold still when the player orbits: a sky mapped
// onto geometry would swing past, and at this camera — about fourteen degrees
// off vertical — a sky mapped onto anything *in* the world would be under the
// table rather than behind it.
//
// So the sun and the moon are placed on the screen, in the two upper corners.
// That is not a compromise, it is the only place they can be seen: the table
// is cut as a racetrack precisely so that the corners of the window are not
// felt, and the corners are therefore the whole of the visible sky at the
// shot a duel opens on.
//
// # WebGL2
//
// Uniforms only, no storage buffers, every loop bound at compile time. The
// star field counts to a literal.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::{globals, view}

struct SkyParams {
    /// 0 is full night, 1 is full day. `baylee_client_core::sky::phase`.
    day: f32,
    /// 0 at noon and at midnight, 1 in the middle of a dawn or a dusk.
    glow: f32,
    /// The clock everything moves on: 1 normally, 0 for reduce-motion.
    motion: f32,
    /// Padding to the 16-byte boundary a uniform needs.
    budget: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: SkyParams;

const TAU: f32 = 6.2831855;

// The day sky, display-referred like every colour this project writes down.
const DAY_HIGH: vec3<f32> = vec3<f32>(0.19, 0.34, 0.49);
const DAY_LOW: vec3<f32> = vec3<f32>(0.91, 0.73, 0.48);
const CLOUD_LIT: vec3<f32> = vec3<f32>(1.00, 0.90, 0.68);
const CLOUD_SHADE: vec3<f32> = vec3<f32>(0.40, 0.46, 0.55);
const SUN_CORE: vec3<f32> = vec3<f32>(1.000, 0.976, 0.855);

// Blue-black air matches the mineral table and the hand's dark ground.
// The grade lives behind those surfaces: it never touches the card art.
const NIGHT_HIGH: vec3<f32> = vec3<f32>(0.022, 0.031, 0.050);
const NIGHT_LOW: vec3<f32> = vec3<f32>(0.065, 0.094, 0.112);
const MOON_CORE: vec3<f32> = vec3<f32>(0.973, 0.965, 0.902);
const STAR_TINT: vec3<f32> = vec3<f32>(0.898, 0.925, 1.000);

/// The low sun. Only ever multiplied by `params.glow`, so a pinned sky never
/// sees it.
const DUSK_WARM: vec3<f32> = vec3<f32>(0.976, 0.510, 0.278);

/// Where the sun and the moon stand, in aspect-corrected screen space with
/// the origin at the top left.
///
/// Opposite upper corners so that neither is ever behind the table, and so
/// that a dusk has both of them in frame at once — which is the one moment
/// this sky has that neither of its ends does.
const SUN_AT: vec2<f32> = vec2<f32>(0.064, 0.082);
const MOON_AT: vec2<f32> = vec2<f32>(-0.060, 0.080);

// Small, because the wedge is small. Measured off a screenshot rather than
// chosen: the sky the player actually sees at the opening shot is about a
// tenth of the window high in each corner, so a body wider than that is a
// body half of which is behind the table.
const SUN_R: f32 = 0.025;
const MOON_R: f32 = 0.027;
/// How far the shadow disc is pushed off the moon to leave a crescent.
const MOON_BITE: vec2<f32> = vec2<f32>(0.017, -0.011);

/// How fast the cloud deck drifts, in screen widths per second.
///
/// Raised from 0.0065, where it was chosen so that a player reading a card
/// never caught it moving — and succeeded so completely that the owner asked
/// for clouds that move. A screen width every forty seconds is the compromise:
/// look at the sky and it is going somewhere, look at a card and the sky is
/// not what you notice. The two decks and the sun's rim are all scaled off
/// this, so the deck, its veil and the lit edges stay in step.
const DRIFT: f32 = 0.020;
/// How fast a star finishes one twinkle.
const TWINKLE: f32 = 1.7;

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    let lo = c / 12.92;
    return select(lo, hi, c > vec3<f32>(0.04045));
}

/// A hash with no trigonometry in it.
///
/// `sin`-based hashes differ between drivers — the same page can grain
/// differently on two machines, and the table's own cloth already made the
/// argument that everyone should see the same surface.
fn hash2(p: vec2<f32>) -> f32 {
    var h = dot(p, vec2<f32>(127.1, 311.7));
    h = fract(h * 0.1031);
    h *= h + 33.33;
    h *= h + h;
    return fract(h);
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

/// Three octaves, normalised back to 0..1. One more than the table's cloth
/// gets: a cloud is read at the size of the whole window and wants the
/// coarse end, and the fine end is what stops the edges reading as blobs.
fn fbm(p: vec2<f32>) -> f32 {
    var sum = 0.0;
    var amplitude = 0.5;
    var total = 0.0;
    var at = p;
    for (var i = 0; i < 5; i = i + 1) {
        sum = sum + amplitude * vnoise(at);
        total = total + amplitude;
        at = at * 2.07;
        amplitude = amplitude * 0.5;
    }
    return sum / total;
}

/// A round glow that falls off fast enough to be a light and not a wash.
fn halo(d: f32, radius: f32, reach: f32) -> f32 {
    let t = clamp(1.0 - (d - radius) / reach, 0.0, 1.0);
    return t * t * t;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Framebuffer position over the viewport's size: screen space, whatever
    // the quad is and however the camera is pointed. `view.viewport` is
    // `(x, y, w, h)`.
    let size = max(view.viewport.zw, vec2<f32>(1.0, 1.0));
    let uv = (in.position.xy - view.viewport.xy) / size;
    let aspect = size.x / size.y;
    // Aspect-corrected, and measured from whichever side the body is on so
    // that the sun and the moon keep their distance from *their* corner on a
    // window of any shape. A negative x in the constant means the right edge.
    let p = vec2<f32>(uv.x * aspect, uv.y);
    let sun = vec2<f32>(select(SUN_AT.x, aspect + SUN_AT.x, SUN_AT.x < 0.0), SUN_AT.y + (1.0 - params.day) * 0.28);
    let moon = vec2<f32>(select(MOON_AT.x, aspect + MOON_AT.x, MOON_AT.x < 0.0), MOON_AT.y + params.day * 0.24);

    let t = globals.time * params.motion;

    // Golden-hour daylight: honey highlights above cool slate cloud shadows.
    // ---- day ----------------------------------------------------------
    var lit = mix(DAY_HIGH, DAY_LOW, smoothstep(0.0, 1.0, uv.y));

    if params.day > 0.001 {
    // Two decks, the upper one thinner and faster, so the sky has a depth to
    // it rather than one sheet of noise sliding past.
    let deck = fbm(p * 4.8 + vec2<f32>(t * DRIFT, t * DRIFT * 0.30));
    let veil = fbm(p * 9.1 + vec2<f32>(t * DRIFT * 2.1, -t * DRIFT * 0.4) + 21.7);
    // Shaped hard: `smoothstep` on the field is what turns noise into cloud
    // with sky between it, and a linear field is the flat grey overcast that
    // every first attempt at this produces.
    let body = smoothstep(0.38, 0.67, deck * 0.75 + veil * 0.25);
    // The rim: cloud is brightest on the side the sun is on, and the cheap
    // way to say so without a gradient is to sample the field again a little
    // way towards the sun and take the difference.
    let towards = normalize(sun - p + vec2<f32>(1e-4));
    let ahead = smoothstep(
        0.46,
        0.72,
        fbm((p + towards * 0.035) * 4.8 + vec2<f32>(t * DRIFT, t * DRIFT * 0.30))
    );
    let rim = clamp(body - ahead, 0.0, 1.0);
    let cloud = mix(CLOUD_SHADE, CLOUD_LIT, clamp(0.36 + body * 0.38 + veil * 0.25 + rim * 3.5, 0.0, 1.0));
    lit = mix(lit, cloud, body);

    // The sun: a hard core, a corona, and a wide bloom that the cloud in
    // front of it is allowed to sit on top of.
    let ds = distance(p, sun);
    let disc = 1.0 - smoothstep(SUN_R * 0.92, SUN_R, ds);
    // Measured off a screenshot and pulled in twice: the first draft reached
    // 0.55 of the screen at 0.30 and washed out the tab strip in the corner
    // it stands in. A sun is allowed to be the brightest thing on screen and
    // is not allowed to make the interface in front of it unreadable.
    let corona = halo(ds, SUN_R, 0.13) * 0.60 + halo(ds, SUN_R, 0.38) * 0.24;
    lit += SUN_CORE * corona * 0.45 * (1.0 - body * 0.65);
    lit = mix(lit, SUN_CORE, disc * (1.0 - body * 0.45));

    let ray_angle = atan2(p.y - sun.y, p.x - sun.x);
    let rays = pow(max(0.0, sin(ray_angle * 11.0 + 0.08 * sin(t * 0.12))), 8.0);
    lit += SUN_CORE * rays * halo(ds, SUN_R, 0.38) * 0.032
        * smoothstep(SUN_R, SUN_R * 2.5, ds) * (1.0 - body);
    }

    // ---- night --------------------------------------------------------
    var dark = mix(NIGHT_HIGH, NIGHT_LOW, smoothstep(0.0, 1.0, uv.y));

    if params.day < 0.999 {
    // A faint band of something further away, so the night is not a flat
    // fill between the stars.
    let deep = fbm(p * 3.3 + vec2<f32>(t * DRIFT * 0.25, 0.0) + 61.3);
    dark += vec3<f32>(0.035, 0.065, 0.075) * smoothstep(0.42, 0.82, deep);

    // Stars: one candidate per cell of a grid, kept or dropped by its own
    // hash, so the field is even without being a lattice.
    let cell = p * 46.0;
    let id = floor(cell);
    let seed = hash2(id);
    var star = 0.0;
    if (seed > 0.86) {
        let at = vec2<f32>(hash2(id + 7.1), hash2(id + 19.4));
        let d = distance(fract(cell), at);
        let size = mix(0.055, 0.115, hash2(id + 3.7));
        // Twinkling is a *brightness*, not a size: a star that changed size
        // would crawl, and one that goes out entirely reads as a dead pixel.
        let beat = 0.55 + 0.45 * sin(t * TWINKLE * mix(0.6, 1.6, seed) + seed * TAU);
        star = (1.0 - smoothstep(0.0, size, d)) * beat;
    }
    dark += STAR_TINT * star * 0.55;

    // A second, denser layer of much fainter ones, so the gaps between the
    // bright stars are not a solid colour. It has to be *discs* like the
    // layer above and not a hash of the cell: a per-cell constant paints the
    // whole cell, and the first version of this line put a field of tiny
    // grey squares behind the table.
    let fine = p * 118.0;
    let fid = floor(fine);
    let fseed = hash2(fid + 91.7);
    if (fseed > 0.90) {
        let at = vec2<f32>(hash2(fid + 5.3), hash2(fid + 23.9));
        let d = distance(fract(fine), at);
        dark += STAR_TINT * (1.0 - smoothstep(0.0, 0.14, d)) * 0.12;
    }

    // The moon: a disc with a second disc taken out of it, and a glow that
    // belongs to the whole body rather than to the crescent — the dark limb
    // of a real crescent moon is still lit by the sky around it.
    let dm = distance(p, moon);
    let full = 1.0 - smoothstep(MOON_R * 0.94, MOON_R, dm);
    let bite = 1.0 - smoothstep(MOON_R * 0.92, MOON_R * 0.99, distance(p, moon + MOON_BITE));
    let crescent = clamp(full - bite, 0.0, 1.0);
    // A little relief on the lit face, so it is a body and not a sticker.
    var seas = 1.0;
    if dm < MOON_R { seas = 0.88 + 0.12 * vnoise((p - moon) * 180.0); }
    dark += MOON_CORE * (crescent * seas * 0.85 + halo(dm, MOON_R, 0.20) * 0.18);

    // Fine curtains of aurora, visible at the open shoulders of the table.
    let curtain = p.y + 0.065 * sin(p.x * 3.2 + t * 0.08)
        + 0.025 * sin(p.x * 8.0 - t * 0.11);
    let gauze = exp(-pow((curtain - 0.23) * 16.0, 2.0));
    let filaments = 0.65 + 0.35 * sin(p.x * 38.0 + deep * 6.0 + t * 0.15);
    dark += mix(vec3<f32>(0.075, 0.20, 0.17), vec3<f32>(0.11, 0.10, 0.23), uv.x)
        * gauze * filaments * 1.15 * params.budget;
    }

    // Drifting sparks belong to the night sky, not to the lands in play.
    let dust_cell = p * 24.0 + vec2<f32>(t * 0.025, t * 0.045);
    let dust_id = floor(dust_cell);
    let dust_seed = hash2(dust_id + 83.0);
    let dust = 1.0 - smoothstep(0.012, 0.065, length(fract(dust_cell) - 0.5));
    dark += vec3<f32>(0.25, 0.60, 0.68) * dust * step(0.91, dust_seed)
        * (0.55 + 0.45 * sin(t * 0.7 + dust_seed * TAU)) * params.budget;

    // ---- the two of them, and the light between them -------------------
    var sky = mix(dark, lit, params.day);
    // Dawn and dusk: a warm band low in the frame, strongest where the light
    // is coming from. Added rather than mixed, because what it is is light.
    let low = smoothstep(0.15, 1.0, uv.y);
    let side = 1.0 - smoothstep(0.0, aspect * 0.9, distance(p, vec2<f32>(sun.x, 1.0)));
    sky += DUSK_WARM * params.glow * (low * 0.16 + side * 0.14);

    // Distant blue-green mist picks up the inlay's colours. Broad, dim and slow:
    // a room around the table, never a luminous HUD competing with the hand.
    let ribbon = p.y + 0.055 * sin(p.x * 3.4 + t * 0.075)  + 0.025 * sin(p.x * 8.0 - t * 0.09);
    let aurora = exp(-pow((ribbon - 0.72) * 9.0, 2.0));
    let sheen = 0.7 + 0.3 * sin(p.x * 4.0 - t * 0.11);
    var air_colour = mix(vec3<f32>(0.16, 0.22, 0.24), vec3<f32>(0.12, 0.24, 0.20), uv.x);
    air_colour += vec3<f32>(0.14, 0.075, 0.04) * pow(abs(uv.x * 2.0 - 1.0), 3.0);
    sky += air_colour * aurora * sheen * mix(0.32, 0.10, params.day);
    // Hazy mountain ridges ground the sky. Analytic silhouettes keep their
    // detail quiet and cost no texture fetches or additional geometry.
    for (var layer = 0; layer < 3; layer = layer + 1) {
        let z = f32(layer);
        let ridge = 0.79 + z * 0.064
            + 0.046 * sin(p.x * (3.1 + z) + z * 7.0)
            + 0.020 * sin(p.x * (9.0 + z * 2.0) + z * 3.0);
        let silhouette = smoothstep(ridge - 0.002, ridge + 0.002, p.y);
        let distant = mix(vec3<f32>(0.036, 0.065, 0.082), vec3<f32>(0.39, 0.52, 0.54), params.day);
        sky = mix(sky, distant * (1.0 - z * 0.16), silhouette * (0.48 + z * 0.15));
    }
    // Lens-like framing, not a post-process: the table and the artwork keep
    // their own exposure. Both daytime and night retain readable shadows.
    let vignette = smoothstep(0.30, 0.85, length((uv - 0.5) * vec2<f32>(0.85, 1.0)));
    sky *= 1.0 - vignette * mix(0.30, 0.12, params.day);

    return vec4<f32>(to_linear(sky), 1.0);
}
