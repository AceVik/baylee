// The plate on a card lying on the table (the owner, 25.09).
//
// One quad per card whose plate shows, a child of the card, standing at its
// bottom right: beside the printed power and toughness box over the black
// border where the row leaves the room, on the card's own foot where the
// next card reaches that far, and upright under a tapped card. This is all
// it draws: `card_common.wgsl`'s `plate_object`, blended over the card and
// whatever lies beside it. The quad is `cardplate::plate_quad` — the widest
// plate with its shadow round it — so every plate is the same mesh and the
// only thing that differs between two of them is the words.

#import bevy_pbr::forward_io::VertexOutput
#import "embedded://baylee_client/shaders/card_common.wgsl"::{plate_object}

struct PlateParams {
    /// The quad's size in card widths.
    quad: vec2<f32>,
    /// The plate, `cardplate::Plate::packed`.
    word: u32,
    /// Its ink: the tone and the night bit, `cardplate::Corner::plate_words`.
    ink: u32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: PlateParams;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var marks: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var marks_sampler: sampler;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // The quad's UV has (0, 0) at its top-left, like a card's, so this is the
    // point in card widths from the quad's corner with `y` growing down.
    let p = mesh.uv * params.quad;
    // Taken here, in uniform control flow, before anything branches on
    // where a fragment is.
    let aa = max(fwidth(p.x), 0.0015);
    return plate_object(p, params.quad, params.word, params.ink, aa, marks, marks_sampler);
}
