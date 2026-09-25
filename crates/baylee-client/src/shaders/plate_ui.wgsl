// The plate on a card drawn as a UI node: the preview (the owner, 25.09).
//
// The table shader's twin, as `badge_ui.wgsl` is `badge.wgsl`'s: a creature
// held up to the light says its numbers where it says them on the felt, and
// the one drawing both is `card_common.wgsl`'s `plate_object`. The node is
// laid over the card at `cardplate::plate_quad`, so its UV is the quad's.

#import bevy_ui::ui_vertex_output::UiVertexOutput
#import "embedded://baylee_client/shaders/card_common.wgsl"::{plate_object}

struct PlateParams {
    quad: vec2<f32>,
    word: u32,
    ink: u32,
}

@group(1) @binding(0) var<uniform> params: PlateParams;
@group(1) @binding(1) var marks: texture_2d<f32>;
@group(1) @binding(2) var marks_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = in.uv * params.quad;
    let aa = max(fwidth(p.x), 0.0015);
    return plate_object(p, params.quad, params.word, params.ink, aa, marks, marks_sampler);
}
