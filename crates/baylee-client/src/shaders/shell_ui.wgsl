// The shells round a permanent on its preview (the PM, 25.09): the table
// shader's twin, as `plate_ui.wgsl` is `plate.wgsl`'s. A card held up to the
// light wears what it wears on the felt: its keywords' border and summoning
// sickness's wave.
//
// The preview is the card laid flat on the screen, so there is no camera to
// follow a ray from. Each shell is drawn as it stands, seen from straight
// over the card, which is what lines it up with the flat print. It always
// stands: the preview has air round it, so nothing lies down as a ring.
// What both shaders draw with is `shell_common.wgsl`'s, so the layers and
// their patterns are the table's.
//
// Nothing but the wave is drawn over the print. From straight over the card
// a point's ray meets the face right under it, so the mask is the card's own
// outline (`clear`), and every colour returned but the wave's carries it; a
// test reads every `return` below to make sure it still does.

#import bevy_render::globals::Globals
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import "embedded://baylee_client/shaders/card_common.wgsl"::{perimeter, CARD_ASPECT}
#import "embedded://baylee_client/shaders/shell_common.wgsl"::{SHELL_WAVE, WAVE_PERIOD, MOONLIGHT, CARD_HALF_W, CARD_HALF_H, MASK_FEATHER, KEY, card_sdf, wave_along, wave_glow, ward_border}

struct ShellUiParams {
    /// The node, `[x0, y0, x1, y1]` in card widths from the card's top-left
    /// corner, `y` down it: `shellui::PREVIEW_QUAD`.
    quad: vec4<f32>,
    /// `shellmat::ShellKind`.
    kind: u32,
    /// The clock every animated term runs on: 1 normally, 0 for
    /// `Preferences::reduce_motion`.
    motion: f32,
    /// The keywords a border says (`shellmat::Ward`), and how many layers
    /// they make.
    ward: u32,
    layers: u32,
    /// How far past the card's edge the border's foot stands
    /// (`shellmat::border_margin`), and nothing.
    margin: f32,
    spare: f32,
    rest: vec2<f32>,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> params: ShellUiParams;

/// How a duel's seat sees a card of its own: from this far off vertical, as
/// a tangent, towards the card's foot. `table::DUEL_LEAN`.
const LEAN: f32 = 0.62;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // The point, in card widths from the card's top-left corner, `y` down
    // it; then in the shell's own space. The preview's card is
    // 1/`CARD_ASPECT` card widths tall and the table's `2 * CARD_HALF_H`,
    // which differ in the fourth place: the print's outline is laid where
    // the preview draws it.
    let q = mix(params.quad.xy, params.quad.zw, in.uv);
    let p = vec2<f32>(q.x - CARD_HALF_W, CARD_HALF_H - q.y * (2.0 * CARD_HALF_H * CARD_ASPECT));
    // The card's own UV, running on past its edges: the border's patterns
    // go round by it.
    let uv = vec2<f32>(q.x, q.y * CARD_ASPECT);
    let d = card_sdf(p);
    // Nothing of the border over the print.
    let clear = smoothstep(0.0, MASK_FEATHER, d);
    // From straight over it, a standing border's slope runs out from the
    // card's edge to its foot. Taken here, in uniform control flow, before
    // anything branches.
    let across = d / max(params.margin, 1e-4);
    let s = perimeter(uv);
    let aa = fwidth(across);
    let aa_s = min(fwidth(s), 0.01);
    let m = params.motion;
    let t = globals.time * m;

    if params.kind == SHELL_WAVE {
        // The table's wave over the print, seen from straight over it, at
        // the preview's own moment: its sheet is the card's face, which is
        // where `p` already lies. Towards the eye, and the light, in the
        // card's own space: a duel's seat looking at a card of its own.
        let v = normalize(vec3<f32>(0.0, -LEAN, 1.0));
        let key = vec3<f32>(KEY.x, -KEY.z, KEY.y);
        let glow = wave_glow(p, wave_along(globals.time * m / WAVE_PERIOD, m), v, key);
        return vec4<f32>(MOONLIGHT, glow);
    }
    let lit = ward_border(params.ward, params.layers, across, s, t, aa, aa_s);
    let inside = 1.0 - smoothstep(1.0 - aa, 1.0 + aa, across);
    return vec4<f32>(lit.rgb, lit.a * inside * clear);
}
