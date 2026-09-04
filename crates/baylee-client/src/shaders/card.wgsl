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
#import "embedded://baylee_client/shaders/card_common.wgsl"::{mark_layer, plate_layer, chip_layer, corner_sdf, MARK_SHIFT, MARK_FIELD}

struct CardParams {
    /// 0 plain, 1 foil, 2 etched.
    finish: u32,
    /// What the rules have made this card, what it cannot do this turn, and
    /// what this client is offering to do with it — the bits are
    /// `cardmat::glow`, and the eleven above `MARK_SHIFT` are the rail.
    glow: u32,
    /// What the reserved bottom-right corner says, packed by
    /// `cardplate::Plate::packed`: a creature's power, toughness and marked
    /// damage, or a planeswalker's loyalty.
    plate: u32,
    chips_a: u32,
    chips_b: u32,
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
const GLOW_ACTIVATABLE: u32 = 8u;
const GLOW_SUMMONING_SICK: u32 = 16u;
const GLOW_ARMED: u32 = 32u;
const GLOW_WILL_TAP: u32 = 64u;

/// How far in from the edge the border treatment reaches, in UV.
const BORDER: f32 = 0.055;

/// What a card's corner is inked with once the scan's white is cut away: the
/// same near-black as the slab's edge wall, so the corner reads as the card
/// turning away rather than as a mark printed on it.
const EDGE_INK: vec3<f32> = vec3<f32>(0.035, 0.038, 0.045);

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

    // ---- the face, when the card cannot do anything yet
    //
    // Summoning sickness is drawn *over the art* and never on the border,
    // and that separation is the whole grammar: the border says what the card
    // is, the face says what it can do, and a player can read both at once
    // only while they stay in different places. A slow breath, desaturated
    // and dimmed — the card is asleep, not disabled.
    if (params.glow & GLOW_SUMMONING_SICK) != 0u {
        let breath = 0.5 + 0.5 * sin(t * 2.618);
        let luma = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        let asleep = mix(color.rgb, vec3<f32>(luma), 0.34) * (0.76 - 0.06 * breath);
        color = vec4<f32>(asleep, color.a);
    }

    // ---- the border, when the rules have made the card something
    //
    // Drawn inside the card's own printed border rather than outside the
    // quad: the mesh is exactly the card, and a glow that needed room around
    // it would need every layout in the client to leave room for it.
    //
    // The band is a *material*, composed as base × film rather than as an
    // average. Indestructible is what the card is made of; hexproof and
    // shroud are what lies over it. Averaging them turned an indestructible
    // hexproof creature into a third colour that said neither thing — this
    // way it is a green sheath on metal, and both are still legible.
    if params.glow != 0u {
        let d = edge_distance(uv);
        let band = 1.0 - smoothstep(0.0, BORDER, d);
        if band > 0.0 {
            var base = vec3<f32>(0.0);
            var has_base = 0.0;
            var film = vec3<f32>(0.0);
            var film_amount = 0.0;

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
                base = steel * turn;
                has_base = 1.0;
            }
            // Hexproof: a protective sheath, green and steady.
            if (params.glow & GLOW_HEXPROOF) != 0u {
                let pulse = 0.70 + 0.30 * sin(t * 1.6);
                film = vec3<f32>(0.28, 0.86, 0.48) * pulse;
                film_amount = 0.78;
            }
            // Shroud: the same idea taken further — nothing may target it,
            // including its controller — so it is colder and hazier. It
            // *replaces* the hexproof film rather than mixing with it, which
            // is also what the rules do to a card carrying both. `glow_bits`
            // already drops hexproof in that case; this ordering is the
            // second lock.
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
                color = vec4<f32>(color.rgb + amber * band * (0.22 + 0.60 * chase), color.a);
            }
            // Armed: the tap has been made, and one more sends it. The same
            // register as the offer above and deliberately the opposite
            // motion — no chase, a bright ring pulled in tight against the
            // printed edge, breathing slowly in place. The card has stopped
            // inviting anything; it is waiting, and a light that still
            // travelled would say it was still a suggestion.
            if (params.glow & GLOW_ARMED) != 0u {
                let hold = 0.86 + 0.14 * sin(t * 2.2);
                // Concentrated toward the edge rather than spread across the
                // band, so it is a ring and not a wash — an armed card and a
                // hovered one must not read the same.
                let ring = pow(band, 0.45);
                let gold = vec3<f32>(1.00, 0.87, 0.54);
                color = vec4<f32>(color.rgb + gold * ring * 0.52 * hold, color.a);
            }
            // What it will cost: the sources an armed mana run would tap.
            // Cool against the deed's warm, and a beat behind it, because the
            // two are one sentence and the price follows the verb.
            if (params.glow & GLOW_WILL_TAP) != 0u {
                let pulse = 0.70 + 0.30 * sin(t * 2.2 - 0.9);
                let indigo = vec3<f32>(0.56, 0.60, 0.98);
                color = vec4<f32>(color.rgb + indigo * band * 0.40 * pulse, color.a);
            }
        }
    }

    // ---- the rail, for the keywords a border cannot count
    //
    // Drawn after the travelling light so that an *offer* passes under the
    // facts and never washes one out, and before the corner ink so that a
    // mark can never survive outside the card. `card_common.wgsl` is shared
    // with the UI twin: a creature in the own-board overlay is the same
    // creature, and two hundred lines of pictogram kept in step by hand would
    // not stay in step.
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
    // The mesh is rounded at exactly this radius, so the geometry has already
    // taken the white away; what is left is the sliver of pixels the mesh
    // edge antialiases through. Those are inked to the same colour as the
    // card's edge wall rather than cut, because the mesh is opaque and a hole
    // in it would show the felt through the card. The same ink lands as a
    // hairline along the straight edges, which is what a printed card has.
    let outside = smoothstep(-0.004, 0.004, corner_sdf(uv));
    color = vec4<f32>(mix(color.rgb, EDGE_INK, outside), color.a);

    return color;
}
