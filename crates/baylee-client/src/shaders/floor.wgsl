// The offer's light on the felt round a card lying on the table (#298).
//
// What this client offers to do with a card — the travelling invitation, the
// armed card, the tap a payment plan will make — lit the frame's rim until
// #298 took the frame away. Now it is light on the cloth: one quad per lit
// card, a child of the card lying under it just above its contact shadow,
// `floormat::REACH` past the card on every side. The card covers every part
// of it that would be under the print, and so does the next card of a fanned
// lane. This is all it draws: `card_common.wgsl`'s `floor_light`, added to
// whatever is under it.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::globals
#import "embedded://baylee_client/shaders/card_common.wgsl"::{floor_light, FLOOR_REACH, CARD_ASPECT}

struct FloorParams {
    /// The offers, `cardmat::glow::OFFERS` of the card's glow word.
    glow: u32,
    /// The clock every animated term runs on: 1 normally, 0 for
    /// `Preferences::reduce_motion`.
    motion: f32,
    /// Sixteen-byte rows under the GL backend's `std140`.
    pad: vec2<u32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: FloorParams;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // The quad's UV has (0, 0) at its top-left, like a card's; this is the
    // point in the card's own UV, running on past its edges by the reach —
    // which down the card is a smaller share, the card being taller.
    let reach = vec2<f32>(FLOOR_REACH, FLOOR_REACH * CARD_ASPECT);
    let uv = mix(-reach, vec2<f32>(1.0) + reach, mesh.uv);
    let m = params.motion;
    let light = floor_light(uv, params.glow, globals.time * m, m, globals.time);
    // `AlphaMode::Add` is a premultiplied blend, so a zero alpha keeps all of
    // the felt under the light and adds the light to it.
    return vec4<f32>(light, 0.0);
}
