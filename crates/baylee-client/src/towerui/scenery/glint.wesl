// Original fine inlay: constant pixel width, independent of cabinet proportions.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;
import bevy_render::globals::Globals;
@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> motion: vec4<f32>;
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let size = max(in.size, vec2<f32>(1.0));
    let p = in.uv * size;
    let edge = min(p, size - p);
    let d = min(edge.x, edge.y);
    // Empty cabinet and ornaments remain untouched; no texture or trigonometry
    // is evaluated for the large transparent interior.
    if d < 8.0 || d > 19.0 || (edge.x < 34.0 && edge.y < 34.0) {
        discard;
    }
    let rail = exp(-pow((d - 13.5) / 1.6, 2.0));
    let sweep = pow(max(0.0, cos((p.x + p.y) * 0.013 - globals.time * motion.x * 0.45)), 20.0);
    let alpha = rail * (0.08 + sweep * 0.38 * motion.x);
    return vec4<f32>(vec3<f32>(1.0, 0.64, 0.24), alpha);
}
