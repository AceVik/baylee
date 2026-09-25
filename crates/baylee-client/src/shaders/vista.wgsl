// The front door's scene: the geode (#295).
//
// The screen is the cut face of a dark mineral, and the panel stands in its
// cavity, through which a world is seen at first light. Drawn as arithmetic
// for the reason `docs/legal.md` §2 gives about every surface in this client
// (ornament is the easiest thing to borrow by accident, and value noise,
// distance fields and hashed points borrow nothing), and §10 says what came
// from the login screen the owner pointed at: a mood and a job, nothing else.
//
// Back to front, each layer moving with the pointer by its own share (the
// nearer, the more), which is the whole of the depth:
//
// - the world: a blue-hour sky with the first light low in it, clouds lit
//   from below and rays from the light;
// - a far ridge of crystal fins, one skyline from two noise samples, low in
//   the middle where the light rises over the panel;
// - a floor of dark resin that mirrors the sky, with the light's reflection
//   running towards the viewer as a path of glitter;
// - motes in the middle air, gold near the light;
// - the frame: dark mineral with seams of light in it (the table's own
//   material), lit by the opening; a broken rim with a seam of gold along it,
//   and crystal teeth lining the cavity, pointing in;
// - the near air: motes, grain, and a dither against banding.
//
// The panel's rect and the ring round it are held dark whatever is lit, so
// the form reads against the brightest scene.
//
// Passing through (choosing a gateway) is two frames: `gate_a` is the cavity
// the viewer stands before and walks through, `gate_b` the one they arrive
// in. `vista.rs` schedules the uniforms; this file only draws a frame of them.

#import bevy_render::globals::Globals
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import "embedded://baylee_client/shaders/noise.wgsl"::{noise2, hash2}

struct VistaParams {
    /// The panel the scene frames: centre xy and half extent zw, in the
    /// field's units (the height is 1, the width the aspect; y runs down).
    panel: vec4<f32>,
    /// Pointer xy (-1 to 1, low-passed on the CPU), aspect, energy (0 stills
    /// the scene: no time and no pointer).
    view: vec4<f32>,
    /// The hour (0 first light on the gateway's side, 1 the light broadened
    /// on the far side), haze, glow, quality (0 drops what a phone's tiler
    /// pays most for).
    hour: vec4<f32>,
    /// The frame walked through: opening, alpha, zoom, dolly (0 to 1).
    gate_a: vec4<f32>,
    /// The frame arrived in: opening, alpha, zoom, unused.
    gate_b: vec4<f32>,
    /// The glitter path's brightness, the ember rate, the scene's own alpha,
    /// the floor's zoom.
    air: vec4<f32>,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> params: VistaParams;

// The palette, linear, with the sRGB each came from beside it.
const MINERAL: vec3<f32> = vec3<f32>(0.0030, 0.0027, 0.0137); // (0.06, 0.055, 0.12)
const AGATE_DEEP: vec3<f32> = vec3<f32>(0.0040, 0.0033, 0.0176); // (0.06, 0.05, 0.14)
const AGATE_VIOLET: vec3<f32> = vec3<f32>(0.0176, 0.0080, 0.0561); // (0.14, 0.09, 0.26)
const AGATE_PALE: vec3<f32> = vec3<f32>(0.0561, 0.0513, 0.1195); // (0.26, 0.25, 0.38)
const AMETHYST: vec3<f32> = vec3<f32>(0.2623, 0.1329, 0.6867); // (0.55, 0.40, 0.85)
const VIOLET: vec3<f32> = vec3<f32>(0.1473, 0.0732, 0.4770); // (0.42, 0.30, 0.72)
const CHAMPAGNE: vec3<f32> = vec3<f32>(0.5705, 0.3931, 0.1626); // (0.78, 0.66, 0.44)
const RIM_GOLD: vec3<f32> = vec3<f32>(0.9560, 0.6290, 0.2270); // (0.98, 0.81, 0.51)
const FIRST: vec3<f32> = vec3<f32>(0.9780, 0.6120, 0.3020); // (0.99, 0.80, 0.58)
const SKY_LOW: vec3<f32> = vec3<f32>(0.2623, 0.2140, 0.4480); // (0.55, 0.50, 0.70)
const SKY_HIGH: vec3<f32> = vec3<f32>(0.0180, 0.0610, 0.3250); // (0.16, 0.28, 0.60)
const SKY_TOP: vec3<f32> = vec3<f32>(0.0048, 0.0080, 0.0513); // (0.06, 0.09, 0.25)
const CRYSTAL: vec3<f32> = vec3<f32>(0.1730, 0.2180, 0.6990); // (0.45, 0.50, 0.85)
const FLOOR: vec3<f32> = vec3<f32>(0.0040, 0.0040, 0.0080); // (0.03, 0.03, 0.045)
const AURORA: vec3<f32> = vec3<f32>(0.0637, 0.2049, 0.7874); // (0.28, 0.49, 0.90)
const RIDGE: vec3<f32> = vec3<f32>(0.0039, 0.0060, 0.0220); // (0.05, 0.07, 0.16)
const TIPS: vec3<f32> = vec3<f32>(0.0134, 0.0220, 0.0732); // (0.12, 0.16, 0.30)
/// The passage's haze: the accent warmed by the light, never white.
const HAZE: vec3<f32> = vec3<f32>(0.3837, 0.3474, 0.6175);

/// One bar of the lobby's music (6/8 at a dotted quarter of 63), in seconds:
/// everything that breathes does so over whole bars.
const BAR: f32 = 0.952;
const TAU: f32 = 6.28318;
/// The brightest the panel and the band round it may be, linear luminance.
const PANEL_CAP: f32 = 0.12;
/// How many crystal teeth line the cavity.
const TEETH: f32 = 36.0;
/// How far above the top of the screen the cavity reaches, so the frame is
/// an arch over the page and the title stands in the sky.
const OVERHEAD: f32 = 0.04;

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

/// Signed distance to a box of half extent `b` with corners of radius `r`.
fn rounded_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

/// 0 on a tall screen, 1 on a wide one.
fn landscape(aspect: f32) -> f32 {
    return smoothstep(0.8, 1.25, aspect);
}

/// How far the cavity stands off the panel at its sides and below it, per
/// unit of opening. Above, it reaches past the top of the screen
/// ([`OVERHEAD`]). On a tall screen the sides run off the screen too, and
/// the frame is a sill.
fn margin(aspect: f32) -> vec2<f32> {
    return mix(vec2<f32>(0.035, 0.11), vec2<f32>(0.30, 0.11), landscape(aspect));
}

/// The skyline's three samples at `x`: the broad noise, the fine noise and
/// the fin.
struct Skyline {
    broad: f32,
    fine: f32,
    fin: f32,
}

fn skyline(x: f32, quality: f32) -> Skyline {
    var s: Skyline;
    s.broad = noise2(vec2<f32>(x * 3.1, 7.0));
    s.fine = 0.5;
    if quality > 0.5 {
        s.fine = noise2(vec2<f32>(x * 11.0, 9.0));
    }
    // A triangle wave: 0 at a fin's point, 1 between two.
    s.fin = abs(fract(x * 9.0 + s.fine) - 0.5) * 2.0;
    return s;
}

/// The world's light at a point: the sky's gradient, the first light and
/// its halo. The floor's mirror reads it too, so only exponentials are in it.
fn light_of(p: vec2<f32>, y_h: f32, sx: f32, hour: f32, breath: f32) -> vec3<f32> {
    let low = SKY_LOW * (1.0 + 0.2 * hour);
    var rgb = mix(SKY_TOP, SKY_HIGH, smoothstep(0.0, y_h - 0.12, p.y));
    rgb = mix(rgb, low, smoothstep(y_h - 0.12, y_h, p.y));
    let dy = p.y - y_h;
    let dx = p.x - sx;
    let sun = exp(-(dy * dy) / 0.0012) * exp(-(dx * dx) / 0.05);
    let halo = exp(-(dy * dy) / 0.006) * exp(-(dx * dx) / 0.2);
    rgb = rgb + FIRST * (0.8 + 0.5 * hour) * sun * breath;
    rgb = rgb + FIRST * 0.35 * (1.0 + 0.6 * hour) * halo;
    return rgb;
}

/// Where a point stands against a cavity: the point about the panel's
/// centre, the direction round the cavity and its angle, and the signed
/// distance to the cavity's broken edge (negative inside).
struct Cavity {
    g: vec2<f32>,
    c: vec2<f32>,
    an: f32,
    d: f32,
}

/// The cavity for a point `at` (its own parallax applied), of `opening`,
/// seen at `zoom`.
fn cavity(at: vec2<f32>, opening: f32, zoom: f32) -> Cavity {
    let centre = params.panel.xy;
    let room = margin(params.view.z);
    var out: Cavity;
    // Scaled about the panel's centre: walking through is the frame growing
    // past the viewer while the panel stays where it is.
    out.g = (at - centre) / zoom;
    // From past the top of the screen to the margin below the panel: the
    // cavity's middle sits over the panel's. The sill does not widen with
    // the opening: below the panel is the small print, which the rim must
    // never reach.
    let overhead = (centre.y - params.panel.w + OVERHEAD) * opening;
    let lift = 0.5 * (overhead - room.y);
    let half = params.panel.zw + vec2<f32>(room.x * opening, 0.5 * (overhead + room.y));
    let b = out.g + vec2<f32>(0.0, lift);
    // A fat superellipse, nowhere a circle.
    let corner = 0.55 * min(half.x, half.y);
    // The angle normalised to the box, so the teeth are as dense on the long
    // sides as on the short ones and nothing jumps at the corners.
    out.an = atan2(b.y * half.x, b.x * half.y);
    out.c = vec2<f32>(cos(out.an), sin(out.an));
    // A broken edge: noise round the unit circle is periodic by construction.
    var rough = 0.05 * (noise2(out.c * 1.6 + 3.0) - 0.5);
    if params.hour.w > 0.5 {
        rough = rough + 0.015 * (noise2(out.c * 5.5 + 7.0) - 0.5);
    }
    // Distances back in screen units, so edges keep their width at any zoom.
    out.d = (rounded_box(b, half, corner) + rough) * zoom;
    return out;
}

/// The frame over `world` at a point of `cave`: its mineral and crystal, with
/// its light added. `lining` scales the crystal teeth, `shimmer` the light
/// running along them.
fn frame(
    cave: Cavity,
    at: vec2<f32>,
    world: vec3<f32>,
    opening: f32,
    zoom: f32,
    lining: f32,
    shimmer: f32,
    field: f32,
    warp: vec2<f32>,
    t: f32,
    px: f32,
) -> vec3<f32> {
    let aspect = params.view.z;
    let quality = params.hour.w;
    let glow = params.hour.z;
    let room = margin(aspect);
    let g = cave.g;
    let c = cave.c;
    let an = cave.an;
    let d = cave.d;
    let outside = max(d, 0.0);
    let stone = smoothstep(-px, px, d);

    // The breath of the rim: eight bars at rest, four while the world is
    // being reached (a wait, a passage), crossfaded rather than switched.
    let slow = sin(t * TAU / (8.0 * BAR));
    let quick = sin(t * TAU / (4.0 * BAR));
    let breath = 1.0 + 0.15 * mix(slow, quick, smoothstep(1.1, 1.3, glow));

    // The mineral, as a geode's cut face is: bands of agate following the
    // cavity, thin and pale next to it and wider and darker away from it,
    // fading into rough dark rock. Each band is its own tone.
    var mineral = vec3<f32>(0.0);
    if stone > 0.0 {
        var wobble = 0.03 * noise2(g * 3.0 + warp * 0.4);
        if quality > 0.5 {
            wobble = wobble + 0.012 * noise2(g * 9.0 + 5.0);
        }
        let layer = sqrt(max(outside + wobble, 0.0)) * 16.0;
        let stratum = floor(layer);
        let within = fract(layer);
        let tone = hash2(vec2<f32>(stratum, 3.0));
        let edge = smoothstep(0.0, 0.12, within) * (1.0 - smoothstep(0.88, 1.0, within));
        var agate = mix(AGATE_DEEP, AGATE_VIOLET, tone);
        // Now and then a pale band of chalcedony.
        let pale = step(0.88, hash2(vec2<f32>(stratum, 11.0)));
        agate = mix(agate, AGATE_PALE, pale) * (0.7 + 0.3 * edge);
        let rock = MINERAL * (0.7 + 0.6 * field);
        mineral = mix(agate, rock, smoothstep(0.18, 0.40, outside + wobble));
        // Light travelling along the bands, one swell every sixteen bars.
        mineral = mineral * (1.0 + 0.15 * sin(t * TAU / (16.0 * BAR) - 3.0 * (g.x + g.y)));
        let flicker = 0.6 + 0.4 * noise2(c * 1.8 + vec2<f32>(t * 0.03, 2.0));
        mineral = mineral + FIRST * 0.04 * exp(-outside / 0.10) * flicker;
        mineral = mineral + CHAMPAGNE * 0.5 * (1.0 - smoothstep(0.0015, 0.008, outside));
    }

    // The crystal teeth, pointing in, longest where the gap is widest and
    // never reaching the panel or the light over it.
    var tooth = 0.0;
    var crystal = vec3<f32>(0.0);
    if d < 0.0 {
        let across = smoothstep(-0.15, 0.15, abs(c.x) - abs(c.y));
        let wide = mix(abs(c.y), abs(c.x), landscape(aspect));
        let s = TEETH * (an / TAU + 0.5);
        let k = floor(s);
        let f = fract(s);
        // A second, shorter row between the first, as a druse grows.
        let s2 = TEETH * 1.6 * (an / TAU + 0.5) + 0.37;
        let f2 = fract(s2);
        var reach = mix(0.02, 0.13, wide) * (0.5 + 0.5 * noise2(c * 2.2 + 11.0));
        reach = reach * (0.45 + 0.55 * hash2(vec2<f32>(k, 5.0)));
        // Up is negative y: over the panel the teeth stop short of the horizon.
        let up = (1.0 - across) * (1.0 - smoothstep(-0.1, 0.1, c.y));
        let gap = mix(room.y, room.x * opening, across) - 0.03;
        let free = mix(gap, 0.55 * room.y * opening - 0.03, up);
        reach = min(reach, max(free, 0.0)) * lining;
        // 0 on a tooth's point line, 1 between two teeth.
        let depth = reach * (1.0 - abs(f - 0.5) * 2.0) * zoom;
        let reach2 = reach * (0.35 + 0.3 * hash2(vec2<f32>(floor(s2), 8.0)));
        let depth2 = reach2 * (1.0 - abs(f2 - 0.5) * 2.0) * zoom;
        let front = 1.0 - smoothstep(depth - px, depth + px, -d);
        let back = 1.0 - smoothstep(depth2 - px, depth2 + px, -d);
        tooth = max(front, back);
        // 0 at a tooth's root, 1 at its point: the points let the world
        // through and catch its light.
        let is_front = front > 0.5;
        let tip = clamp(-d / max(select(depth2, depth, is_front), 1.0e-4), 0.0, 1.0);
        let own_f = select(f2, f, is_front);
        let facet = select(0.55, 1.0, own_f > 0.5);
        let hue = mix(CRYSTAL, AMETHYST, 0.5 + 0.5 * sin(k * 1.3));
        // Dark in body, the world's light coming through it violet towards
        // the point, and a lit ridge down its middle where two faces meet.
        crystal = hue * 0.10 * facet + world * hue * 0.35 * tip * tip;
        let ridge_line = 1.0 - smoothstep(0.0, 0.05, abs(own_f - 0.5));
        crystal = crystal + (hue * 0.10 + RIM_GOLD * 0.30 * tip) * ridge_line * select(0.5, 1.0, is_front);
        let wave = pow(0.5 + 0.5 * sin(s * 1.7 - t * TAU / (2.0 * BAR)), 6.0);
        crystal = crystal + RIM_GOLD * 0.35 * wave * shimmer;
    }

    var rgb = mix(mix(world, mineral, stone), crystal, tooth);
    // The seam of gold where frame and world meet: tight over the stone,
    // softer into the opening, and held back over a tooth, which catches
    // the light at its point and not along its body.
    let rim = select(0.45 * exp(d / 0.04) * (1.0 - tooth), exp(-d / 0.012), d > 0.0);
    rgb = rgb + RIM_GOLD * 0.7 * glow * breath * rim;

    // Embers on the band just outside the rim, rising and burning out.
    let band = smoothstep(0.0, 0.004, d) * (1.0 - smoothstep(0.02, 0.03, d));
    if band > 0.0 {
        let rising = (at + vec2<f32>(0.0, t * 0.06)) * 60.0;
        let cell = floor(rising);
        let seed = hash2(cell);
        let lit = step(1.0 - 0.06 * params.air.y, seed);
        let point = fract(rising) - vec2<f32>(seed, fract(seed * 7.3));
        let life = fract(t * 0.7 + seed * 13.1);
        let ember = (1.0 - smoothstep(0.03, 0.14, length(point))) * sin(3.14159 * life);
        rgb = rgb + RIM_GOLD * ember * lit * band;
    }
    return rgb;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let energy = params.view.w;
    let t = globals.time * energy;
    let aspect = params.view.z;
    let quality = params.hour.w;
    let hour = params.hour.x;
    let centre = params.panel.xy;
    let px = 1.0 / max(in.size.y, 1.0);
    let uv = in.uv * vec2<f32>(aspect, 1.0);
    let pointer = params.view.xy * energy;
    // Two slow drifts that never line up, so nothing repeats within a minute.
    let drift = vec2<f32>(sin(t * 0.126), cos(t * 0.188) * 0.7);
    let dolly = params.gate_a.w;

    // Where the light rises: over the panel, in the one gap every screen
    // has, and a little to the right of its middle.
    let y_h = centre.y - params.panel.w - 0.045;
    let sx = centre.x + 0.08;
    let first_breath = 1.0 + 0.08 * sin(t * TAU / (12.0 * BAR));

    // Where the frame walked through stands; under its solid stone, when it
    // is the only frame, none of the world is seen and none is drawn.
    let frame_at = uv + pointer * 0.035;
    let cave_a = cavity(frame_at, params.gate_a.x, params.gate_a.z);
    let buried = params.gate_a.y >= 1.0 && params.gate_b.y <= 0.0 && cave_a.d > 4.0 * px;

    // The sky, which barely moves.
    let sky_at = uv + pointer * 0.004 + drift * 0.003;
    let warp = vec2<f32>(
        noise2(sky_at * 1.6 + vec2<f32>(t * 0.05, t * -0.03)),
        noise2(sky_at * 1.6 + vec2<f32>(-t * 0.04, t * 0.06) + 4.0),
    );
    let field = noise2(sky_at * 2.2 + warp * 1.4 + vec2<f32>(0.0, t * 0.02));
    var rgb = light_of(sky_at, y_h, sx, hour, first_breath);
    if !buried {
        // Clouds lit from underneath, drifting.
        var puff = field;
        if quality > 0.5 {
            puff = noise2(vec2<f32>(sky_at.x * 2.4 + t * 0.012, sky_at.y * 7.0) + warp * 0.5);
        }
        let cloud = smoothstep(0.52, 0.72, puff) * (1.0 - smoothstep(y_h - 0.05, y_h - 0.01, sky_at.y));
        let lit_from_below = clamp((y_h - sky_at.y) / 0.3, 0.0, 1.0);
        rgb = mix(rgb, mix(FIRST * 0.55, SKY_HIGH * 0.45, lit_from_below), 0.6 * cloud);

        // Rays from the light, and stars high up while it is still first light.
        if quality > 0.5 && sky_at.y < y_h {
            let toward = sky_at - vec2<f32>(sx, y_h);
            let ray = pow(noise2(vec2<f32>(atan2(toward.y, toward.x) * 6.0 + t * 0.02, 1.0)), 3.0);
            rgb = rgb + FIRST * 0.12 * ray * exp(-length(toward) / 0.5);
        }
        if sky_at.y < y_h - 0.30 && hour < 1.0 {
            let cells = sky_at * 90.0;
            let seed = hash2(floor(cells));
            let point = fract(cells) - vec2<f32>(seed, fract(seed * 7.3));
            let star = (1.0 - smoothstep(0.02, 0.07, length(point))) * step(0.992, seed);
            let twinkle = 0.6 + 0.4 * sin(t * 1.3 + seed * 40.0);
            rgb = rgb + vec3<f32>(0.6, 0.7, 1.0) * star * twinkle * (1.0 - hour) * 0.5;
        }

        // The ridge, low in the middle where the light rises. On the far side it
        // is another stretch of the same line.
        var ridge_at = uv + pointer * 0.012 + drift * 0.006;
        ridge_at = centre + (ridge_at - centre) / (1.0 + 0.06 * dolly);
        let ridge_x = select(ridge_at.x, 3.7 - ridge_at.x, hour > 0.5);
        let line = skyline(ridge_x, quality);
        let valley = smoothstep(0.15, 0.6, abs(ridge_at.x - centre.x) / max(params.panel.z, 1.0e-3));
        let skyline_y = mix(y_h + 0.075, y_h + 0.09 * line.broad + 0.03 * line.fin, valley);
        let ground = smoothstep(skyline_y - px, skyline_y + px, ridge_at.y);
        let crest = 1.0 - smoothstep(0.0, 6.0 * px, ridge_at.y - skyline_y);
        let body = mix(TIPS, RIDGE, smoothstep(skyline_y, skyline_y + 0.08, ridge_at.y));
        let backlit = exp(-pow((ridge_at.x - sx) / 0.5, 2.0));
        rgb = mix(rgb, body + FIRST * 0.5 * (0.5 + 0.5 * backlit) * crest, ground);

        // The floor: dark resin from the ridge's foot, mirroring the sky, with
        // the light's reflection running towards the viewer.
        var floor_at = uv + pointer * 0.020 + drift * 0.010;
        floor_at = centre + (floor_at - centre) / params.air.w;
        let shore = y_h + 0.12;
        if floor_at.y > shore - 2.0 * px {
            let below = floor_at.y - shore;
            let ripple = 0.02 * below * (noise2(vec2<f32>(floor_at.x * 9.0, floor_at.y * 30.0 - t * 0.5)) - 0.5);
            let mirrored = vec2<f32>(floor_at.x, 2.0 * shore - floor_at.y + ripple);
            let fade = mix(0.22, 0.06, smoothstep(shore, shore + 0.4, floor_at.y));
            var water = FLOOR + light_of(mirrored, y_h, sx, hour, first_breath) * fade;
            // The ridge upside down in it, where the mirrored point is behind it.
            let under_ridge = smoothstep(skyline_y - px, skyline_y + px, mirrored.y);
            water = mix(water, FLOOR + RIDGE * 0.6, under_ridge);
            let width = mix(0.05, 0.30, smoothstep(shore, shore + 0.6, floor_at.y));
            let off = abs(floor_at.x - sx) / width;
            // The light's path: short bright strokes across the water, as the
            // sun glitters on ripples, thinning out towards its banks.
            let path = exp(-off * off) * (1.0 - smoothstep(0.9, 1.4, off));
            if path > 0.0 {
                // In the water's own plane: across and away, so the strokes are
                // wide near the viewer and fine at the horizon.
                let away = 1.0 / (below + 0.03);
                let plane = vec2<f32>((floor_at.x - sx) * away * 25.0, away * 40.0 - t * 0.35);
                var glint = smoothstep(0.7, 0.92, noise2(plane));
                if quality > 0.5 {
                    glint = glint + 0.6 * pow(noise2(floor_at * 40.0 + warp * 2.0 - vec2<f32>(0.0, t * 0.3)), 6.0);
                }
                // Fading before the foot of the screen, where the small print is.
                let foot = 1.0 - 0.6 * smoothstep(0.78, 1.0, uv.y);
                water = water + FIRST * params.air.x * path * (0.05 + 0.8 * glint) * foot;
            }
            rgb = mix(rgb, water, smoothstep(shore - px, shore + px, floor_at.y));
        }

        // Motes in the middle air, rising, gold near the light.
        if quality > 0.5 {
            let mid_at = uv + pointer * 0.030 + drift * 0.010;
            let cells = (mid_at + vec2<f32>(0.0, t * 0.03)) * 28.0;
            let seed = hash2(floor(cells) + 3.0);
            let point = fract(cells) - vec2<f32>(seed, fract(seed * 5.1));
            let size = mix(0.02, 0.07, fract(seed * 31.7));
            let mote = (1.0 - smoothstep(size * 0.3, size, length(point))) * step(0.988, seed);
            let near_light = 1.0 - smoothstep(0.0, 0.35, length(mid_at - vec2<f32>(sx, y_h)));
            rgb = rgb + mix(AURORA, RIM_GOLD, near_light) * 0.25 * mote;
        }
    }

    // The frame: the cavity walked through, and the one arrived in.
    if params.gate_a.y > 0.0 {
        let a = frame(cave_a, frame_at, rgb, params.gate_a.x, params.gate_a.z, 1.0, 1.0, field, warp, t, px);
        rgb = mix(rgb, a, params.gate_a.y);
    }
    if params.gate_b.y > 0.0 {
        let cave_b = cavity(frame_at, params.gate_b.x, params.gate_b.z);
        let b = frame(cave_b, frame_at, rgb, params.gate_b.x, params.gate_b.z, 0.6, 0.5, field, warp, t, px);
        rgb = mix(rgb, b, params.gate_b.y);
    }

    // Brightest round the panel, so the edges of a wide window stay quiet.
    let vignette = 1.0 - smoothstep(0.15, 0.95, length(uv - centre));
    rgb = rgb * (0.65 + 0.35 * vignette);

    // The haze of the passage.
    let haze = params.hour.y;
    if haze > 0.0 {
        let fog = noise2(uv * 2.0 + vec2<f32>(t * 0.4, t * 0.4)) * haze;
        rgb = mix(rgb, HAZE, clamp(fog, 0.0, 1.0));
    }

    // The panel and the ring round it stay dark whatever is lit, so the
    // form reads at every frame of the passage.
    // Softly, so the hold reads as the panel's shade on the scene and not
    // as a plate behind it.
    let off_panel = rounded_box(uv - centre, params.panel.zw, 0.02);
    let held = 1.0 - smoothstep(0.0, 0.04, off_panel);
    let capped = rgb * min(1.0, PANEL_CAP / max(luminance(rgb), 1.0e-4));
    rgb = mix(rgb, capped, held);

    // The near air: motes, never over the panel.
    var air_at = uv + pointer * 0.060 + drift * 0.02;
    air_at = centre + (air_at - centre) / params.gate_a.z;
    let motes_at = air_at * 42.0 + vec2<f32>(t * 0.07, -t * 0.12);
    let mote_seed = hash2(floor(motes_at));
    let mote_point = fract(motes_at) - vec2<f32>(mote_seed, fract(mote_seed * 7.3));
    let mote = (1.0 - smoothstep(0.015, 0.065, length(mote_point))) * step(0.985, mote_seed);
    rgb = rgb + AURORA * 0.22 * mote * step(0.0, off_panel);

    // Grain, then a triangular dither of one step of the 8-bit target, in
    // sRGB where the steps are: long dark ramps band otherwise.
    let grain = (hash2(floor(in.position.xy)) - 0.5) * 0.004;
    let dither = hash2(in.position.xy) + hash2(in.position.xy + vec2<f32>(17.3, 5.9)) - 1.0;
    let encoded = pow(max(rgb + grain, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2)) + dither / 255.0;
    rgb = pow(max(encoded, vec3<f32>(0.0)), vec3<f32>(2.2));
    return vec4<f32>(rgb, params.air.z);
}
