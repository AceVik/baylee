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
#import "embedded://baylee_client/shaders/card_common.wgsl"::{print_finish, corner_sdf, sweep_amount, door_layer, DOOR_NONE, text_face, FACE_ON}

struct CardParams {
    /// 0 plain, 1 foil, 2 etched, 3 holographic, 4 glitter, 5 galaxy.
    finish: u32,
    /// 1.0 when `art` holds real artwork.
    has_art: f32,
    /// How strongly the finish is applied.
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

@group(0) @binding(1) var<uniform> globals: Globals;

@group(1) @binding(0) var art: texture_2d<f32>;
@group(1) @binding(1) var art_sampler: sampler;
@group(1) @binding(2) var<uniform> params: CardParams;


/// The view angle a still foil is drawn at: the one whose glint equals the
/// average of the sweep it replaces.
const STILL_TILT: f32 = 0.524;

/// The arrival sweep's weight and colour — the table shader's twins, so a card
/// picked up off the table keeps the light it caught. A UI node has no world
/// position and no normal, so there is no lamp here to answer; the coating
/// floor that used to be the rest of this block lifted the print's blacks and
/// went with #274.
const METAL_GLOSS: f32 = 0.20;
const METAL_TONE: vec3<f32> = vec3<f32>(1.0, 0.975, 0.925);

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;

    // Reduce-motion is one multiplier on the clock, exactly as in the table
    // shader; see the comment there for why one number and what phase zero
    // has to mean. Every animated term runs on `t`, bar the ones marked where
    // stopping the clock would not leave an honest frame.
    let m = params.motion;
    let t = globals.time * m;

    // Stands in for the view angle the table shader has: a slow sweep across
    // the card, which is what a player tilting a foil in their hand sees.
    //
    // The exception only this shader has. `tilt` is not a brightness but an
    // *angle*, and the glint is brightest where the angle is zero — edge-on,
    // where a real foil catches the light. Stopping the clock at zero would
    // therefore freeze a hand of foils at their most garish, which is the
    // opposite of what was asked for. `STILL_TILT` is the angle whose glint
    // equals the moving one's mean (E[(1-|sin|)²] = 1.5 - 4/π ≈ 0.227, so
    // |tilt| = 1 - √0.227).
    let tilt = mix(STILL_TILT, sin(t * 0.55), m);

    // ---- the print, which is the whole card (#298)
    //
    // The artwork, edge to edge, with its own finish: a foil is what that
    // printing is. A card with no artwork is a flat colour and gets the same
    // finish — a face-down foil is still a foil. Nothing this client says
    // about the card is drawn here: the strip, the badge and the offer's
    // light on the felt are objects of their own.
    let sampled = textureSample(art, art_sampler, uv);
    // A card with no art stands its text face in the window where it has
    // one (#259).
    let flat = select(params.tint, vec4<f32>(text_face(uv, params.face), 1.0), (params.face & FACE_ON) != 0u);
    var color = mix(flat, sampled, params.has_art);
    color = print_finish(color, uv, tilt, t, params.finish, params.strength);

    // ---- light passing over the whole card: the arrival, or a door
    //
    // Not on `t`: the sweep is anchored to an absolute moment the material
    // was given, so it has to be compared against the same clock that moment
    // was read from. A card that is holding still does not sweep at all —
    // `sweep_rate` is left at zero — rather than sweeping on a stopped clock.
    // A door replaces the plain band rather than joining it, exactly as on
    // the table.
    let phase = (globals.time - params.sweep_at) * params.sweep_rate;
    let travel = select(0.0, sweep_amount(uv, phase), params.sweep_rate > 0.0);
    let door = select(DOOR_NONE, params.sweep_door, params.sweep_rate > 0.0);
    let plain = select(0.0, travel, door == DOOR_NONE);
    color = vec4<f32>(color.rgb + METAL_TONE * METAL_GLOSS * plain, color.a);
    color = vec4<f32>(color.rgb + door_layer(uv, phase, door), color.a);

    // ---- the card's own corners
    //
    // A UI node has no mesh to round, so the corner is cut here, in alpha,
    // and the scan's own corners go with it.
    color.a *= 1.0 - smoothstep(-0.003, 0.003, corner_sdf(uv));

    return color;
}
