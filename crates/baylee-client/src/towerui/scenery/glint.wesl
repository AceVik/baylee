// Original fine cabinet inlay (motion.y < 0.5) and frozen-shelf rime frost
// overlay (motion.y >= 0.5): constant pixel geometry, GPU-clock animated.
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
    let energy = motion.x;
    let t = globals.time * energy;

    if motion.y >= 0.5 {
        // Frozen shelf rime overlay: crystalline frost creeping in from the wood
        // rails and deeper at the four corners, leaving the center clear.
        let corner_d = length(edge);
        if d > 40.0 && corner_d > 78.0 {
            discard;
        }
        let facets = abs(sin(p.x * 0.14 + sin(p.y * 0.09) * 1.5))
            * abs(cos(p.y * 0.13 - sin(p.x * 0.08) * 1.5));
        let corner_swell = smoothstep(78.0, 14.0, corner_d);
        let reach = 22.0 + 11.0 * facets + 18.0 * corner_swell;
        let rime = smoothstep(reach, 4.0, d);
        let vein = exp(-pow((d - 14.5 - 3.5 * facets) / 1.8, 2.0));
        let sweep = pow(max(0.0, cos((p.x - p.y) * 0.011 - t * 0.52)), 14.0) * energy;

        let cell = floor(p / 14.0);
        let h = fract(sin(dot(cell, vec2<f32>(12.9898, 78.233))) * 43758.5453);
        let star_d = length(fract(p / 14.0) - 0.5);
        let twinkle = pow(max(0.0, sin(t * (0.8 + h * 0.7) + h * 6.2831)), 12.0) * energy;
        let crystal = smoothstep(0.18, 0.0, star_d) * step(0.60, h) * twinkle * smoothstep(38.0, 6.0, d);

        let ice_deep = vec3<f32>(0.10, 0.38, 0.72);
        let ice_bright = vec3<f32>(0.58, 0.88, 1.0);
        let col = mix(ice_deep, ice_bright, 0.38 + 0.45 * facets + 0.25 * sweep) + ice_bright * crystal * 0.65;
        let alpha = clamp(rime * 0.42 + vein * (0.18 + 0.26 * sweep) + crystal * 0.55, 0.0, 0.78);
        if alpha < 0.004 {
            discard;
        }
        return vec4<f32>(col, alpha);
    }

    // Empty cabinet and ornaments remain untouched; no texture or trigonometry
    // is evaluated for the large transparent interior.
    if d < 4.0 || d > 14.0 || (edge.x < 24.0 && edge.y < 24.0) {
        discard;
    }
    let rail = exp(-pow((d - 8.9) / 1.3, 2.0));
    let sweep = pow(max(0.0, cos((p.x + p.y) * 0.013 - t * 0.45)), 20.0);
    let alpha = rail * (0.08 + sweep * 0.38 * energy);
    return vec4<f32>(vec3<f32>(1.0, 0.64, 0.24), alpha);
}
