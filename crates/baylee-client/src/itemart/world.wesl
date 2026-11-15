// The same portrait on the table; ordinary cards never use this material.
import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::mesh_view_bindings::globals;
import baylee_client::itemart::common::{Params, draw};
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: Params;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var portrait: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var portrait_sampler: sampler;
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return draw(in.uv, params, globals.time, portrait, portrait_sampler);
}
