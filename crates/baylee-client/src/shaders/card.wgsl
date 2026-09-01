// The card surface: printed art, its physical finish, and whatever the rules
// have made it right now.
//
// One shader for all three because they compose on the same pixel — an
// indestructible foil is a foil that also glows, not a third case — and
// because a board of three hundred permanents can afford one pipeline and not
// three.
//
// # WebGL2
//
// The browser build targets WebGL2 (`webgl2` in bevy's feature list), which
// means: uniforms only, no storage buffers, no texture arrays, and every loop
// bound at compile time. Nothing below reaches for any of them. `globals.time`
// comes from the view bind group, so the sheen animates without the CPU
// touching a material asset per frame.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::{view, globals}

struct CardParams {
    /// 0 plain, 1 foil, 2 etched.
    finish: u32,
    /// Keyword bits: 1 indestructible, 2 hexproof, 4 shroud.
    glow: u32,
    /// 1.0 when `art` holds real artwork, 0.0 when the card draws as `tint`.
    has_art: f32,
    /// How strongly the finish is applied. Lets one material be dimmed
    /// without a second pipeline.
    strength: f32,
    /// The flat colour a card with no art is drawn in.
    tint: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var art: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var art_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> params: CardParams;

const FINISH_FOIL: u32 = 1u;
const FINISH_ETCHED: u32 = 2u;

const GLOW_INDESTRUCTIBLE: u32 = 1u;
const GLOW_HEXPROOF: u32 = 2u;
const GLOW_SHROUD: u32 = 4u;

/// How far in from the edge the border treatment reaches, in UV.
const BORDER: f32 = 0.055;

/// A cheap value-noise hash. Deterministic, and the same on every backend —
/// two clients looking at the same foil see the same foil.
fn hash21(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2<f32>(123.34, 456.21));
    q += dot(q, q + 45.32);
    return fract(q.x * q.y);
}

/// Smooth noise over a UV, for the grain a real foil has under its sheen.
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

/// Hue → RGB, the part of HSV a rainbow sheen actually needs.
fn spectrum(h: f32) -> vec3<f32> {
    let k = fract(h) * 6.0;
    return clamp(
        vec3<f32>(
            abs(k - 3.0) - 1.0,
            2.0 - abs(k - 2.0),
            2.0 - abs(k - 4.0),
        ),
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
}

/// Distance from the nearest edge of the card, in UV, 0 at the edge.
fn edge_distance(uv: vec2<f32>) -> f32 {
    let d = min(uv, vec2<f32>(1.0) - uv);
    return min(d.x, d.y);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let uv = mesh.uv;
    // A card with no artwork is a flat colour, and gets the same finish and
    // the same glow: a face-down foil is still a foil.
    let sampled = textureSample(art, art_sampler, uv);
    var color = mix(params.tint, sampled, params.has_art);

    // The direction the card is being looked at from. A foil's whole
    // character is that it changes as the table moves, so the sheen is a
    // function of the view and not only of time.
    let to_view = normalize(view.world_position - mesh.world_position.xyz);
    let facing = dot(normalize(mesh.world_normal), to_view);
    let t = globals.time;

    if params.finish == FINISH_FOIL {
        // A broad diagonal band that sweeps as the angle changes, plus fine
        // grain so it reads as foil rather than as a gradient.
        let sweep = (uv.x + uv.y) * 1.6 + facing * 2.4 + t * 0.10;
        let grain = noise(uv * 46.0) * 0.28;
        let hue = fract(sweep * 0.35 + grain);
        let sheen = spectrum(hue);
        // Brightest when the card is seen edge-on, which is when a real foil
        // catches the light.
        let glint = pow(1.0 - abs(facing), 2.0);
        let amount = (0.16 + 0.42 * glint) * params.strength;
        color = vec4<f32>(
            color.rgb + sheen * amount * (0.55 + 0.45 * color.a),
            color.a,
        );
    } else if params.finish == FINISH_ETCHED {
        // Etched foil is engraved rather than laminated: no rainbow, a cooler
        // metal, and the pattern is in the surface, so it moves with the card
        // instead of with the light.
        let lines = sin((uv.x - uv.y) * 220.0 + noise(uv * 9.0) * 6.0);
        let etch = smoothstep(0.55, 1.0, lines) * (0.5 + 0.5 * (1.0 - abs(facing)));
        let metal = vec3<f32>(0.78, 0.74, 0.62);
        color = vec4<f32>(color.rgb + metal * etch * 0.22 * params.strength, color.a);
    }

    // ---- the border, when the rules have made the card something
    //
    // Drawn inside the card's own printed border rather than outside the
    // quad: the mesh is exactly the card, and a glow that needed room around
    // it would need every layout in the client to leave room for it.
    if params.glow != 0u {
        let d = edge_distance(uv);
        let band = 1.0 - smoothstep(0.0, BORDER, d);
        if band > 0.0 {
            var glow = vec3<f32>(0.0);
            var weight = 0.0;

            // Indestructible is darksteel: a hard, dark blue-grey metal with
            // a bright specular line, not a coloured light. It is the card
            // *itself* that is made of something.
            if (params.glow & GLOW_INDESTRUCTIBLE) != 0u {
                let brush = noise(vec2<f32>(uv.x * 120.0, uv.y * 8.0));
                let spec = pow(smoothstep(0.35, 1.0, brush), 3.0);
                let steel = vec3<f32>(0.36, 0.42, 0.50) + vec3<f32>(0.55) * spec;
                // Slow, so it reads as metal catching the light rather than
                // as something switched on.
                let turn = 0.72 + 0.28 * sin(t * 0.8 + uv.y * 3.0);
                glow += steel * turn;
                weight += 1.0;
            }
            // Hexproof: a protective sheath, green and steady.
            if (params.glow & GLOW_HEXPROOF) != 0u {
                let pulse = 0.70 + 0.30 * sin(t * 1.6);
                glow += vec3<f32>(0.28, 0.86, 0.48) * pulse;
                weight += 1.0;
            }
            // Shroud: the same idea taken further — nothing may target it,
            // including its controller — so it is colder and hazier.
            if (params.glow & GLOW_SHROUD) != 0u {
                let haze = noise(uv * 14.0 + vec2<f32>(t * 0.30, -t * 0.22));
                glow += vec3<f32>(0.55, 0.62, 0.92) * (0.55 + 0.45 * haze);
                weight += 1.0;
            }
            // Two keywords on one card share the border rather than stacking
            // to white; the point is to be readable at a glance across a
            // board, not to be bright.
            glow /= max(weight, 1.0);
            color = vec4<f32>(mix(color.rgb, glow, band * 0.85), color.a);
        }
    }

    return color;
}
