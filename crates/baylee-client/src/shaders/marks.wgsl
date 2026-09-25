// The keyword strip on a card lying on the table (#274), and since #298 the
// card's label: its counters' chip, its moon and its crests beside the marks.
//
// One quad per card with something to say, a child of the card lying just
// over its face, and this is all it draws: `card_common.wgsl`'s
// `label_strip`, blended over the card. The quad is `cardrail::quad_rect` —
// the largest strip with its shadow round it — so every strip is the same
// mesh and the only thing that differs between two of them is the words.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::globals
#import "embedded://baylee_client/shaders/card_common.wgsl"::{label_strip}

struct MarksParams {
    /// The marks, one bit each in `cardrail::MARK_ORDER`'s order.
    bits: u32,
    /// The clock every animated term runs on: 1 normally, 0 for
    /// `Preferences::reduce_motion`.
    motion: f32,
    /// The quad's size in card widths.
    quad: vec2<f32>,
    /// The chip, `cardplate::Corner::chip`; zero for none.
    swing: u32,
    /// The sleep moon and the crests, `cardrail::label`.
    label: u32,
    /// Sixteen-byte rows under the GL backend's `std140`.
    pad: vec2<u32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: MarksParams;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var marks: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var marks_sampler: sampler;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // The quad's UV has (0, 0) at its top-left, like a card's, so this is the
    // point in card widths from the quad's corner with `y` growing down.
    let q = mesh.uv * params.quad;
    // Taken here, in uniform control flow, before anything branches on which
    // mark a fragment is in.
    let aa = max(fwidth(q.x), 0.0015);
    let t = globals.time * params.motion;
    return label_strip(
        q,
        params.quad,
        params.bits,
        params.swing,
        params.label,
        t,
        aa,
        marks,
        marks_sampler,
    );
}
