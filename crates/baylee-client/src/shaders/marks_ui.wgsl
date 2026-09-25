// The keyword strip on a card drawn as a UI node: the preview (#274).
//
// The table shader's twin, as `card_ui.wgsl` is `card.wgsl`'s: a creature
// held up to the light has to wear the marks it wears on the felt, and the
// one drawing both is `card_common.wgsl`'s `label_strip`. The node is laid
// over the card at `cardrail::quad_rect`, so its UV is the quad's.

#import bevy_render::globals::Globals
#import bevy_ui::ui_vertex_output::UiVertexOutput
#import "embedded://baylee_client/shaders/card_common.wgsl"::{label_strip}

struct MarksParams {
    bits: u32,
    motion: f32,
    quad: vec2<f32>,
    plate: u32,
    swing: u32,
    label: u32,
    pad: u32,
}

@group(0) @binding(1) var<uniform> globals: Globals;

@group(1) @binding(0) var<uniform> params: MarksParams;
@group(1) @binding(1) var marks: texture_2d<f32>;
@group(1) @binding(2) var marks_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let q = in.uv * params.quad;
    let aa = max(fwidth(q.x), 0.0015);
    let t = globals.time * params.motion;
    return label_strip(
        q,
        params.quad,
        params.bits,
        params.plate,
        params.swing,
        params.label,
        t,
        aa,
        marks,
        marks_sampler,
    );
}
