// Baylee's World (front door) & an enchanted tavern (lobby interior).
// Each scene is a 16:9 still (`sanctuary`, 3840 × 2160) with a moving picture
// of our own over it: an AVIF loop at a third of the still's width, handed in
// as its three 8-bit planes (`vista::loops`).
// All textures are original; live effects never sample card art.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;

struct VistaParams {
    panel: vec4<f32>, home: vec4<f32>, view: vec4<f32>, hour: vec4<f32>,
    gate_a: vec4<f32>, gate_b: vec4<f32>, air: vec4<f32>, portal: vec4<f32>,
    moving: vec4<f32>,
}
@group(1) @binding(0) var<uniform> params: VistaParams;
@group(1) @binding(1) var sanctuary: texture_2d<f32>;
@group(1) @binding(2) var sanctuary_sampler: sampler;
@group(1) @binding(3) var loop_y: texture_2d<f32>;
@group(1) @binding(4) var loop_u: texture_2d<f32>;
@group(1) @binding(5) var loop_v: texture_2d<f32>;
@group(1) @binding(6) var first_y: texture_2d<f32>;
@group(1) @binding(7) var first_u: texture_2d<f32>;
@group(1) @binding(8) var first_v: texture_2d<f32>;

// How far a frame may differ from the loop's first before it counts as
// moving, in linear light: below MOVE_FROM it is the codec's own grain,
// above MOVE_TO it is the picture's.
const MOVE_FROM: f32 = 0.010;
const MOVE_TO: f32 = 0.055;

// 8-bit limited-range BT.709 Y′CbCr (what the shipped loops hold, and what
// `loops::work` refuses anything else for) to linear light.
fn linear_of(y: f32, u: f32, v: f32) -> vec3<f32> {
    let luma = (y - 16.0 / 255.0) * (255.0 / 219.0);
    let cb = (u - 128.0 / 255.0) * (255.0 / 224.0);
    let cr = (v - 128.0 / 255.0) * (255.0 / 224.0);
    let encoded = clamp(
        vec3<f32>(luma + 1.5748 * cr, luma - 0.18733 * cb - 0.46812 * cr, luma + 1.8556 * cb),
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
    // The still is an sRGB texture, so the loop is read by the same curve.
    let low = encoded / 12.92;
    let high = pow((encoded + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, encoded <= vec3<f32>(0.04045));
}

// The moving picture over its still. Where the running frame agrees with the
// loop's first frame nothing is moving, and the still's own detail (the still
// less that first frame, which is the still at the loop's size) is drawn over
// it: the scene stays as sharp as its 3840 × 2160 painting wherever it holds
// still, and is the loop only where the loop moves.
fn moving(still: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let now = linear_of(
        textureSample(loop_y, sanctuary_sampler, uv).r,
        textureSample(loop_u, sanctuary_sampler, uv).r,
        textureSample(loop_v, sanctuary_sampler, uv).r,
    );
    let first = linear_of(
        textureSample(first_y, sanctuary_sampler, uv).r,
        textureSample(first_u, sanctuary_sampler, uv).r,
        textureSample(first_v, sanctuary_sampler, uv).r,
    );
    let change = abs(now - first);
    let holds = 1.0 - smoothstep(MOVE_FROM, MOVE_TO, max(change.r, max(change.g, change.b)));
    let drawn = max(now + (still - first) * holds, vec3<f32>(0.0));
    return mix(still, drawn, clamp(params.moving.x, 0.0, 1.0));
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let aspect = max(params.view.z, 0.01);
    let energy = params.view.w;
    let interior = params.gate_b.w;
    let flight = params.portal.x * params.portal.y;
    let image_aspect = 16.0 / 9.0;
    let cover = vec2<f32>(min(aspect / image_aspect, 1.0), min(image_aspect / aspect, 1.0));
    let zoom = 1.015 + params.gate_a.w * 0.018 + smoothstep(0.0, 1.0, flight) * 0.06;
    let uv = vec2<f32>(0.5)
        + (in.uv - 0.5) * cover / zoom
        + params.view.xy * vec2<f32>(0.0014, 0.0009) * energy;
    let sample_uv = clamp(uv, vec2<f32>(0.001), vec2<f32>(0.999));
    var rgb = moving(textureSample(sanctuary, sanctuary_sampler, sample_uv).rgb, sample_uv);

    // Soft reading contrast & passage haze on the front door login form:
    let p = in.uv * vec2<f32>(aspect, 1.0);
    if interior <= 0.5 {
        let panel_q = abs(p - params.panel.xy) - params.panel.zw;
        let quiet = 1.0 - smoothstep(0.0, 0.12, length(max(panel_q, vec2<f32>(0.0))));
        rgb *= 1.0 - quiet * 0.18 * (1.0 - params.portal.y);
        let vignette = smoothstep(0.42, 0.85, length(in.uv - 0.5));
        rgb *= 1.0 - vignette * 0.05;
        rgb = mix(rgb, vec3<f32>(0.91, 0.88, 0.79), params.hour.y * 0.12);
    }

    var alpha = params.air.z;
    if params.portal.y > 0.5 {
        let radius = length(p - vec2<f32>(aspect * 0.5, 0.48));
        let reveal = smoothstep(0.35, 1.0, flight) * (length(vec2<f32>(aspect, 1.0)) + 0.3);
        alpha *= smoothstep(reveal - 0.3, reveal + 0.1, radius);
    }
    return vec4<f32>(rgb, alpha);
}
