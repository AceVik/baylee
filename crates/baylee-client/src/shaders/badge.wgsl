// The count badge on a card lying on the table (#261, #274).
//
// One quad per merged card, a child of the card lying just over its face and
// hanging off a corner of it, and this is all it draws:
// `card_common.wgsl`'s `count_badge`, blended over the card and whatever lies
// beside it. The quad is `cardplate::badge_quad_rect` — the widest badge with
// its shadow round it — so every badge is the same mesh and the only thing
// that differs between two of them is the count.

#import bevy_pbr::forward_io::VertexOutput
#import "embedded://baylee_client/shaders/card_common.wgsl"::{count_badge}

struct BadgeParams {
    /// The quad, `[x0, y0, x1, y1]` in card widths from the card's top-left
    /// corner, `y` down the card.
    quad: vec4<f32>,
    /// How many permanents the card stands for. `cardplate::count_word`.
    count: u32,
    /// Where the body's right end and top stand, in card widths:
    /// `cardplate::BADGE_RIGHT` and `BADGE_TOP`.
    right: f32,
    top: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: BadgeParams;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var marks: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var marks_sampler: sampler;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // The quad's UV has (0, 0) at its top-left, like a card's, so this is the
    // point on the card in card widths, off it to the left of the edge.
    let p = mix(params.quad.xy, params.quad.zw, mesh.uv);
    // Taken here, in uniform control flow, before anything branches on
    // where a fragment is.
    let aa = max(fwidth(p.x), 0.0015);
    return count_badge(p, params.count, params.right, params.top, aa, marks, marks_sampler);
}
