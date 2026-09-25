// The count badge on a card drawn as a UI node: the preview (#261, #274).
//
// The table shader's twin, as `marks_ui.wgsl` is `marks.wgsl`'s: a pile held
// up to the light says how many it is the way it says so on the felt, and
// the one drawing both is `card_common.wgsl`'s `count_badge`. The node is
// laid over the card at `cardplate::badge_quad_rect`, so its UV is the
// quad's.

#import bevy_ui::ui_vertex_output::UiVertexOutput
#import "embedded://baylee_client/shaders/card_common.wgsl"::{count_badge}

struct BadgeParams {
    quad: vec4<f32>,
    count: u32,
    right: f32,
    top: f32,
}

@group(1) @binding(0) var<uniform> params: BadgeParams;
@group(1) @binding(1) var marks: texture_2d<f32>;
@group(1) @binding(2) var marks_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = mix(params.quad.xy, params.quad.zw, in.uv);
    let aa = max(fwidth(p.x), 0.0015);
    return count_badge(p, params.count, params.right, params.top, aa, marks, marks_sampler);
}
