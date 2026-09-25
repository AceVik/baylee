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
#import "embedded://baylee_client/shaders/card_common.wgsl"::{print_finish, print_uv, print_cover, frame_layer, corner_sdf, sweep_amount, door_layer, DOOR_NONE, text_face, FACE_ON}

struct CardParams {
    /// 0 plain, 1 foil, 2 etched, 3 holographic, 4 glitter, 5 galaxy.
    finish: u32,
    /// What the rules have made this card, what it cannot do this turn, and
    /// what this client is offering to do with it — the bits are
    /// `cardmat::glow`. The keywords a card wears as marks are not here:
    /// they are the strip's, which is its own object (`marks.wgsl`).
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
    /// The clock every animated term below runs on: 1 normally, 0 for
    /// `Preferences::reduce_motion`.
    motion: f32,
    /// When this card's one-shot sheen began, on `globals.time`'s clock.
    sweep_at: f32,
    /// One over how long that sheen takes, or 0 for a card that is not
    /// sweeping — which is almost every card almost all of the time.
    sweep_rate: f32,
    /// Which of the five zone-change doors this sweep draws, or `DOOR_NONE`
    /// for the plain arrival. `cardmat::door` numbers them.
    sweep_door: u32,
    /// The text face a card with no art draws in its window, or 0 for its
    /// flat `tint` — a back, a slab. `textface::face_word`.
    face: u32,
    /// The flat colour a card with no art is drawn in.
    tint: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var art: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var art_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> params: CardParams;
// The glyph atlas the ledge's numerals and the identity crests are drawn
// from: one row of square distance fields, baked at startup by
// `markatlas.rs`. Every card material carries the same handle, so this is one
// texture for the whole table.
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var marks: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var marks_sampler: sampler;


/// The lamp over the table.
///
/// Not a finish — a *finish* is what a particular printing was made with, and
/// three cards in four are plain — and no longer a coating either: the
/// brushed floor every card used to be lacquered with lifted the print's
/// blacks, the artist's line and the copyright line included, and went with
/// #274. What is left is light passing over. Nothing on this table is lit —
/// the stage is deliberately `unlit` so scene lighting can never make a
/// colour identity unreadable — and a specular term against a *virtual* lamp
/// gives the surface back without putting a light anywhere near the art: it
/// is arithmetic on the view vector, the same move `felt.wgsl`'s `under_lamp`
/// makes for the cloth.
///
/// [`LAMP`] is a direction in world space, so the highlight is a broad pool
/// lying across the table rather than a spot on each card, and it travels as
/// the player orbits. Every card in the pool shares it, which is what makes
/// it read as one room and not as three hundred separate materials.
///
/// The numbers are deliberately small. A gloss loud enough to notice is a
/// gloss competing with the picture it is lying on, and the test for this one
/// is that a player who is not looking for it sees a table with a light over
/// it rather than a table of shiny cards. `METAL_GLOSS` is the arrival
/// sweep's weight, which is the same kind of light and passes just as fast.
const LAMP: vec3<f32> = vec3<f32>(-0.32, 0.86, -0.40);
const METAL_POWER: f32 = 48.0;
const METAL_SPECULAR: f32 = 0.045;
const METAL_GLOSS: f32 = 0.20;
const METAL_TONE: vec3<f32> = vec3<f32>(1.0, 0.975, 0.925);

/// What a card's corner is inked with once the scan's white is cut away: the
/// same near-black as the slab's edge wall, so the corner reads as the card
/// turning away rather than as a mark printed on it.
const EDGE_INK: vec3<f32> = vec3<f32>(0.035, 0.038, 0.045);

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let uv = mesh.uv;

    // The direction the card is being looked at from. A foil's whole
    // character is that it changes as the table moves, so the sheen is a
    // function of the view and not only of time.
    let to_view = normalize(view.world_position - mesh.world_position.xyz);
    let facing = dot(normalize(mesh.world_normal), to_view);

    // Reduce-motion is one multiplier on the clock, not a second pipeline: a
    // board of three hundred permanents cannot afford a variant, and the two
    // drawings would drift apart the first time either was edited.
    //
    // Every animated term runs on `t`, so a zero clock stops all of them at
    // phase zero, and each is written so that phase zero is a place it could
    // have been — which is what makes a still card the moving one held still
    // rather than a different picture. The terms where phase zero is neither
    // are marked where they appear and reach for `globals.time` instead.
    let m = params.motion;
    let t = globals.time * m;

    // ---- the print, and the one thing ever drawn on it
    //
    // The artwork, sampled through the window, with its own finish: a foil is
    // what that printing is. A card with no artwork is a flat colour and gets
    // the same finish — a face-down foil is still a foil. Nothing below this
    // block writes to `print`.
    let at = print_uv(uv);
    let sampled = textureSample(art, art_sampler, at);
    // A card with no art stands its text face in the window where it has
    // one (#259): drawn only where there is no print to cover.
    let flat = select(params.tint, vec4<f32>(text_face(uv, params.face), 1.0), (params.face & FACE_ON) != 0u);
    var print = mix(flat, sampled, params.has_art);
    print = print_finish(print, at, facing, t, params.finish, params.strength);

    // ---- the frame, where everything this client says about the card lives
    let frame = frame_layer(
        uv,
        params.glow,
        params.plate,
        params.chips_a,
        params.chips_b,
        t,
        m,
        globals.time,
        marks,
        marks_sampler,
    );
    let inside = print_cover(uv);
    var color = vec4<f32>(mix(frame, print.rgb, inside), mix(1.0, print.a, inside));

    // ---- light passing over the whole card, print and frame alike
    //
    // The lamp's pool, and the one-shot sheen for a card that has just
    // arrived — a spell resolving on to the battlefield is the whole of what
    // this shader ever sweeps. Both are light that leaves nothing behind:
    // the pool moves with the camera and the sweep is gone in a second.
    //
    // The sweep runs on `globals.time` and not on `t`: its start is an
    // absolute moment, so it has to be read against the clock it was written
    // from, and a still card is given no sweep at all rather than one on a
    // stopped clock.
    let lamp = normalize(LAMP);
    let half_way = normalize(to_view + lamp);
    let spec = pow(max(dot(normalize(mesh.world_normal), half_way), 0.0), METAL_POWER);
    let phase = (globals.time - params.sweep_at) * params.sweep_rate;
    let travel = select(0.0, sweep_amount(uv, phase), params.sweep_rate > 0.0);
    // A door is the same one-shot on the same clock, drawn as a different
    // figure in a colour of its own, so the plain band is drawn only when the
    // card came through no door — otherwise the two would be laid over each
    // other and read as neither.
    let door = select(DOOR_NONE, params.sweep_door, params.sweep_rate > 0.0);
    let plain = select(0.0, travel, door == DOOR_NONE);
    color = vec4<f32>(
        color.rgb + METAL_TONE * (METAL_SPECULAR * spec + METAL_GLOSS * plain),
        color.a,
    );
    color = vec4<f32>(color.rgb + door_layer(uv, phase, door), color.a);

    // ---- the card's own corners
    //
    // The mesh is rounded at the printed radius, so what is left is the
    // sliver of pixels the mesh edge antialiases through. Those are inked to
    // the same colour as the card's edge wall rather than cut, because the
    // mesh is opaque and a hole in it would show the felt through the card.
    // The same ink lands as a hairline along the straight edges, which is
    // what a real card has. The scan's own white corners never get this far:
    // they are outside the window, and the frame's paper is drawn there.
    let outside = smoothstep(-0.004, 0.004, corner_sdf(uv));
    color = vec4<f32>(mix(color.rgb, EDGE_INK, outside), color.a);

    return color;
}
