// The approved portrait is sampled unchanged. Light and botanical engraving
// are independent layers, never baked over the approved source.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;
import bevy_render::globals::Globals;
@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> feel: vec4<f32>;
@group(1) @binding(1) var portrait: texture_2d<f32>;
@group(1) @binding(2) var portrait_sampler: sampler;
const TAU: f32 = 6.2831853;

fn gaussian(x: f32) -> f32 { return exp(-x * x); }

fn sample_portrait(uv: vec2<f32>) -> vec4<f32> {
    let inside = step(0.0, uv.x) * step(uv.x, 1.0) * step(0.0, uv.y) * step(uv.y, 1.0);
    return textureSampleLevel(portrait, portrait_sampler, clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0) * inside;
}
@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let hover = feel.x;
    let t = globals.time;
    // Keep fine fur and flower details anchored to the pixel grid. Subpixel
    // UV motion at a heavily minified portrait caused temporal aliasing.
    let uv = (in.uv - 0.5) / 0.92 + 0.5;
    let cat = sample_portrait(uv);
    var halo = 0.0;
    for (var i = 0; i < 8; i += 1) {
        let angle = f32(i) * TAU / 8.0;
        halo += sample_portrait(uv + vec2<f32>(cos(angle), sin(angle) * 1.5) * 0.012).a / 8.0;
    }
    halo = max(halo - cat.a, 0.0) * hover;
    let p = (in.uv - vec2<f32>(0.5, 0.57)) * vec2<f32>(1.0, 0.6667);
    let q = p / vec2<f32>(0.44, 0.23);
    let r = length(q);
    let a = atan2(q.y, q.x);
    let aa = max(fwidth(r) * 1.25, 0.008);
    let edge = gaussian((r - 1.0) / aa);
    let band = gaussian((r - 1.035) / max(aa, 0.01)) * 0.45;
    // Small paired leaves decorate the lower arc of the light oval.
    let leaves = pow(max(0.0, cos(a * 26.0)), 12.0)
        * gaussian((r - 1.06 - sin(a * 26.0) * 0.035) / max(aa, 0.014));
    let engraving = (edge + band + leaves * 0.7) * smoothstep(0.02, 0.20, p.y) * hover;
    let warm = vec3<f32>(1.0, 0.74, 0.35);
    let glow = exp(-dot(q, q) * 1.4) * hover * (0.065 + sin(t * 0.45) * 0.005 * feel.z);
    let light_alpha = clamp(halo * 0.8 + engraving * 0.8 + glow, 0.0, 0.85);
    let alpha = cat.a + light_alpha * (1.0 - cat.a);
    let rgb = (cat.rgb * (1.0 + hover * 0.045) * cat.a + warm * light_alpha * (1.0 - cat.a)) / max(alpha, 0.0001);
    return vec4<f32>(rgb, alpha);
}
