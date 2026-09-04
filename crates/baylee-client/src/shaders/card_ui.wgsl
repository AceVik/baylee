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
#import "embedded://baylee_client/shaders/card_common.wgsl"::{mark_layer, plate_layer, chip_layer, corner_sdf, MARK_SHIFT, MARK_FIELD}

struct CardParams {
    /// 0 plain, 1 foil, 2 etched.
    finish: u32,
    /// What the rules have made this card, plus what this client is
    /// offering to do with it. The bits are `cardmat::glow`, never the
    /// engine's keyword numbering.
    glow: u32,
    /// What the reserved bottom-right corner says, packed by
    /// `cardplate::Plate::packed`: a creature's power, toughness and marked
    /// damage, or a planeswalker's loyalty.
    plate: u32,
    chips_a: u32,
    chips_b: u32,
    /// 1.0 when `art` holds real artwork.
    has_art: f32,
    /// How strongly the finish is applied.
    strength: f32,
    /// The clock every animated term below runs on: 1 normally, 0 for
    /// `Preferences::reduce_motion`.
    motion: f32,
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
const GLOW_ACTIVATABLE: u32 = 8u;
const GLOW_SUMMONING_SICK: u32 = 16u;
const GLOW_ARMED: u32 = 32u;
const GLOW_WILL_TAP: u32 = 64u;

/// How far in from the edge the border treatment reaches, in UV.
const BORDER: f32 = 0.055;

/// What the travelling activatable light averages to over its own circuit.
/// The table shader's twin of this constant; the derivation is at the use
/// site there.
const CHASE_STILL: f32 = 0.32;

/// The view angle a still foil is drawn at: the one whose glint equals the
/// average of the sweep it replaces.
const STILL_TILT: f32 = 0.524;

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

// Position around the card's border, 0..1, clockwise from the top-left
// corner. Continuous across all four corners, so a light travelling on it
// runs round the card instead of jumping at the edges.
fn perimeter(uv: vec2<f32>) -> f32 {
    let d = vec2<f32>(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    if d.x < d.y {
        if uv.x < 0.5 {
            return 0.75 + (1.0 - uv.y) * 0.25;
        }
        return 0.25 + uv.y * 0.25;
    }
    if uv.y < 0.5 {
        return uv.x * 0.25;
    }
    return 0.5 + (1.0 - uv.x) * 0.25;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let sampled = textureSample(art, art_sampler, uv);
    var color = mix(params.tint, sampled, params.has_art);

    // Reduce-motion is one multiplier on the clock, exactly as in the table
    // shader; see the comment there for why one number and what phase zero
    // has to mean. Every animated term runs on `t`, bar the three marked
    // below where stopping the clock would not leave an honest frame.
    let m = params.motion;
    let t = globals.time * m;

    // Stands in for the view angle the table shader has: a slow sweep across
    // the card, which is what a player tilting a foil in their hand sees.
    //
    // First exception, and the one only this shader has. `tilt` is not a
    // brightness but an *angle*, and the glint below is brightest where the
    // angle is zero — edge-on, where a real foil catches the light. Stopping
    // the clock at zero would therefore freeze a hand of foils at their
    // most garish, which is the opposite of what was asked for.
    // `STILL_TILT` is the angle whose glint equals the moving one's mean
    // (E[(1-|sin|)²] = 1.5 - 4/π ≈ 0.227, so |tilt| = 1 - √0.227).
    let tilt = mix(STILL_TILT, sin(t * 0.55), m);

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

    // Asleep: the same breath the table draws, over the face and never on the
    // border. A card in hand is never summoning sick, but the preview of one
    // on the battlefield is, and it must not disagree with the table.
    if (params.glow & GLOW_SUMMONING_SICK) != 0u {
        let breath = 0.5 + 0.5 * sin(t * 2.618);
        let luma = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        let asleep = mix(color.rgb, vec3<f32>(luma), 0.34) * (0.76 - 0.06 * breath);
        color = vec4<f32>(asleep, color.a);
    }

    if params.glow != 0u {
        let band = 1.0 - smoothstep(0.0, BORDER, edge_distance(uv));
        if band > 0.0 {
            // Base × film, exactly as the table composes it: the metal is
            // what the card is made of, the sheath is what lies over it, and
            // shroud replaces the hexproof film rather than mixing with it.
            var base = vec3<f32>(0.0);
            var has_base = 0.0;
            var film = vec3<f32>(0.0);
            var film_amount = 0.0;
            if (params.glow & GLOW_INDESTRUCTIBLE) != 0u {
                let brush = noise(vec2<f32>(uv.x * 120.0, uv.y * 8.0));
                let spec = pow(smoothstep(0.35, 1.0, brush), 3.0);
                let steel = vec3<f32>(0.36, 0.42, 0.50) + vec3<f32>(0.55) * spec;
                base = steel * (0.72 + 0.28 * sin(t * 0.8 + uv.y * 3.0));
                has_base = 1.0;
            }
            if (params.glow & GLOW_HEXPROOF) != 0u {
                film = vec3<f32>(0.28, 0.86, 0.48) * (0.70 + 0.30 * sin(t * 1.6));
                film_amount = 0.78;
            }
            if (params.glow & GLOW_SHROUD) != 0u {
                let haze = noise(uv * 14.0 + vec2<f32>(t * 0.30, -t * 0.22));
                film = vec3<f32>(0.55, 0.62, 0.92) * (0.55 + 0.45 * haze);
                film_amount = 0.88;
            }
            let painted = max(has_base, film_amount);
            if painted > 0.0 {
                var mark = mix(color.rgb, base, has_base);
                mark = mix(mark, film, film_amount);
                color = vec4<f32>(mix(color.rgb, mark, band * 0.85), color.a);
            }
            // Activatable is not a property of the card, so it must not read
            // like one: a warm light running round the border, which the eye
            // finds across a whole board and which no printed ability could
            // be mistaken for. It is added on top of any keyword sheath
            // rather than averaged into it — the two are saying different
            // things and both stay legible.
            if (params.glow & GLOW_ACTIVATABLE) != 0u {
                let head = fract(perimeter(uv) - t * 0.22);
                let chase = pow(1.0 - min(head, 1.0 - head) * 2.0, 5.0);
                let amber = vec3<f32>(0.99, 0.78, 0.34);
                // Second exception: the chase is a position, so a stopped
                // clock parks it rather than dimming it. A still card gets
                // the circuit's mean, which is an even ring.
                let amount = mix(CHASE_STILL, 0.22 + 0.60 * chase, m);
                color = vec4<f32>(color.rgb + amber * band * amount, color.a);
            }
            // Armed and its price, the same two the table draws and drawn the
            // same way: a steady ring for the deed, a cooler pulse a beat
            // behind it on whatever would pay. This is the twin that matters
            // most for `Deed::Play`, because the card being armed is in the
            // hand and the hand is drawn here.
            if (params.glow & GLOW_ARMED) != 0u {
                let hold = 0.86 + 0.14 * sin(t * 2.2);
                let ring = pow(band, 0.45);
                let gold = vec3<f32>(1.00, 0.87, 0.54);
                color = vec4<f32>(color.rgb + gold * ring * 0.52 * hold, color.a);
            }
            if (params.glow & GLOW_WILL_TAP) != 0u {
                // Third exception: the `- 0.9` that puts the price a beat
                // behind the deed also means phase zero is near the bottom
                // of the swing, not its middle. Scale the oscillation, not
                // the clock, so a still card is drawn at the mean.
                let pulse = 0.70 + 0.30 * sin(globals.time * 2.2 - 0.9) * m;
                let indigo = vec3<f32>(0.56, 0.60, 0.98);
                color = vec4<f32>(color.rgb + indigo * band * 0.40 * pulse, color.a);
            }
        }
    }

    // ---- the rail, identical to the table's, from the same file
    color = vec4<f32>(
        mark_layer(uv, (params.glow >> MARK_SHIFT) & MARK_FIELD, t, color.rgb),
        color.a,
    );

    // ---- the plate the rail reserved a corner for
    //
    // After the rail, because the two share a bottom edge and the plate is
    // what the rail stops short of; before the corner ink, for the same reason
    // the rail is — nothing may survive outside the card.
    color = vec4<f32>(plate_layer(uv, params.plate, color.rgb), color.a);
    // ---- and the counters standing above it
    color = vec4<f32>(
        chip_layer(uv, params.chips_a, params.chips_b, color.rgb), color.a);

    // ---- the corners the scanner saw and the card does not have
    //
    // A UI node has no mesh to round, so the corner is cut here, in alpha.
    // This is the one the player was actually looking at: the hand, the
    // preview and the printing picker all drew a Scryfall scan as a square,
    // white corners included, which is the single most obvious way for a card
    // to look like a photograph of a card.
    color.a *= 1.0 - smoothstep(-0.003, 0.003, corner_sdf(uv));

    return color;
}
