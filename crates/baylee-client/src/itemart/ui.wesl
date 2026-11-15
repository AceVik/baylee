// Shared original art, both in the market and inside a card preview.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;
import bevy_render::globals::Globals;
import baylee_client::itemart::common::{Params, draw};
@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> params: Params;
@group(1) @binding(1) var portrait: texture_2d<f32>;
@group(1) @binding(2) var portrait_sampler: sampler;
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    return draw(in.uv, params, globals.time, portrait, portrait_sampler);
}
