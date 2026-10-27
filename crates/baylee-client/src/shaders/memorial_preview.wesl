// Full photographs remain untouched. This material decorates only their preview.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;
@group(1) @binding(0) var<uniform> focal: vec4<f32>;
@group(1) @binding(1) var photo: texture_2d<f32>;
@group(1) @binding(2) var photo_sampler: sampler;
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let dimensions = vec2<f32>(textureDimensions(photo));
    let aspect = dimensions.x / dimensions.y;
    let frame = in.size.x / max(in.size.y, 1.0);
    let span = min(vec2<f32>(frame / aspect, aspect / frame), vec2<f32>(1.0));
    // CSS object-position: anchor within the portion left after cover cropping.
    let uv = in.uv * span + (1.0 - span) * focal.xy;
    let pixel = textureSample(photo, photo_sampler, uv);
    let radius = min(focal.z, min(in.size.x, in.size.y) * 0.25);
    let q = abs((in.uv - 0.5) * in.size) - in.size * 0.5 + radius;
    let distance = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
    let alpha = 1.0 - smoothstep(-fwidth(distance), fwidth(distance), distance);
    let vignette = smoothstep(0.35, 1.0, length((in.uv - 0.5) * 1.4142));
    return vec4<f32>(pixel.rgb * (1.0 - vignette * focal.w), pixel.a * alpha);
}
