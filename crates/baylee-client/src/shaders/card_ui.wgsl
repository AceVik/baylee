// The same card surface, in the 2D overlay.
//
// The hand, the preview and the printing picker draw cards as UI nodes, and a
// foil a player is holding has to look like the foil that will land on the
// table. So this is the table shader's twin, with one difference it cannot
// avoid: a UI node has no world position and no normal, so there is no view
// angle to drive the sheen with. Time and UV do it instead — the sweep runs
// on its own rather than answering the camera.
//
// Everything else is deliberately identical, down to the constants: two
// shaders that disagreed about what "foil" looks like would be worse than one
// that only ran on the table.

#import bevy_render::globals::Globals
#import bevy_ui::ui_vertex_output::UiVertexOutput

struct CardParams {
    /// 0 plain, 1 foil, 2 etched.
    finish: u32,
    /// Keyword bits: 1 indestructible, 2 hexproof, 4 shroud.
    glow: u32,
    /// 1.0 when `art` holds real artwork.
    has_art: f32,
    /// How strongly the finish is applied.
    strength: f32,
    /// The flat colour a card with no art is drawn in.
    tint: vec4<f32>,
}

@group(0) @binding(1) var<uniform> globals: Globals;

@group(1) @binding(0) var art: texture_2d<f32>;
@group(1) @binding(1) var art_sampler: sampler;
@group(1) @binding(2) var<uniform> params: CardParams;

const FINISH_FOIL: u32 = 1u;
const FINISH_ETCHED: u32 = 2u;

const GLOW_INDESTRUCTIBLE: u32 = 1u;
const GLOW_HEXPROOF: u32 = 2u;
const GLOW_SHROUD: u32 = 4u;

/// How far in from the edge the border treatment reaches, in UV.
const BORDER: f32 = 0.055;

fn hash21(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2<f32>(123.34, 456.21));
    q += dot(q, q + 45.32);
    return fract(q.x * q.y);
}

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

fn edge_distance(uv: vec2<f32>) -> f32 {
    let d = min(uv, vec2<f32>(1.0) - uv);
    return min(d.x, d.y);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let sampled = textureSample(art, art_sampler, uv);
    var color = mix(params.tint, sampled, params.has_art);

    // Stands in for the view angle the table shader has: a slow sweep across
    // the card, which is what a player tilting a foil in their hand sees.
    let t = globals.time;
    let tilt = sin(t * 0.55);

    if params.finish == FINISH_FOIL {
        let sweep = (uv.x + uv.y) * 1.6 + tilt * 2.4 + t * 0.10;
        let grain = noise(uv * 46.0) * 0.28;
        let sheen = spectrum(fract(sweep * 0.35 + grain));
        let glint = pow(1.0 - abs(tilt), 2.0);
        let amount = (0.16 + 0.42 * glint) * params.strength;
        color = vec4<f32>(
            color.rgb + sheen * amount * (0.55 + 0.45 * color.a),
            color.a,
        );
    } else if params.finish == FINISH_ETCHED {
        let lines = sin((uv.x - uv.y) * 220.0 + noise(uv * 9.0) * 6.0);
        let etch = smoothstep(0.55, 1.0, lines) * (0.5 + 0.5 * (1.0 - abs(tilt)));
        let metal = vec3<f32>(0.78, 0.74, 0.62);
        color = vec4<f32>(color.rgb + metal * etch * 0.22 * params.strength, color.a);
    }

    if params.glow != 0u {
        let band = 1.0 - smoothstep(0.0, BORDER, edge_distance(uv));
        if band > 0.0 {
            var glow = vec3<f32>(0.0);
            var weight = 0.0;
            if (params.glow & GLOW_INDESTRUCTIBLE) != 0u {
                let brush = noise(vec2<f32>(uv.x * 120.0, uv.y * 8.0));
                let spec = pow(smoothstep(0.35, 1.0, brush), 3.0);
                let steel = vec3<f32>(0.36, 0.42, 0.50) + vec3<f32>(0.55) * spec;
                glow += steel * (0.72 + 0.28 * sin(t * 0.8 + uv.y * 3.0));
                weight += 1.0;
            }
            if (params.glow & GLOW_HEXPROOF) != 0u {
                glow += vec3<f32>(0.28, 0.86, 0.48) * (0.70 + 0.30 * sin(t * 1.6));
                weight += 1.0;
            }
            if (params.glow & GLOW_SHROUD) != 0u {
                let haze = noise(uv * 14.0 + vec2<f32>(t * 0.30, -t * 0.22));
                glow += vec3<f32>(0.55, 0.62, 0.92) * (0.55 + 0.45 * haze);
                weight += 1.0;
            }
            glow /= max(weight, 1.0);
            color = vec4<f32>(mix(color.rgb, glow, band * 0.85), color.a);
        }
    }

    return color;
}
