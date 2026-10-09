// The light on the shell's primary buttons (owner, 09.10.2026): a quiet
// glow from the top edge at rest, and while the pointer is on the button a
// soft band of light that sweeps across it, left to right, every 2.4 s.
//
// Uniforms only (the GL budget, CLAUDE.md): one small block per button. At
// rest (`hover` 0) nothing here reads the clock, so a still screen draws the
// same pixels every frame and asks for none; `sweep` 0 (reduce_motion) holds
// the band in the middle instead of moving it. Drawn over the button's own
// ground and under its words, as light: alpha only, the colour is the
// light's.

#import bevy_render::globals::Globals
#import bevy_ui::ui_vertex_output::UiVertexOutput

struct SheenParams {
    /// The light's colour (linear); its alpha is unused.
    light: vec4<f32>,
    /// How far the pointer is on the button, 0 to 1, eased.
    hover: f32,
    /// The resting glow's strength.
    glow: f32,
    /// 1 moves the band, 0 holds it still (reduce_motion).
    sweep: f32,
    /// Padding to the 16-byte boundary a uniform needs.
    pad: f32,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> params: SheenParams;

const PERIOD: f32 = 2.4;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let size = max(in.size, vec2<f32>(1.0, 1.0));
    // The node's rounded corners: outside them, nothing.
    let p = (in.uv - vec2<f32>(0.5, 0.5)) * size;
    let r = min(in.border_radius.x, min(size.x, size.y) * 0.5);
    let q = abs(p) - size * 0.5 + vec2<f32>(r, r);
    let d = length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
    let inside = clamp(0.5 - d, 0.0, 1.0);

    // At rest: light falling from the top edge, a quarter of the way down.
    let fall = 1.0 - in.uv.y;
    var light = fall * fall * params.glow;

    if params.hover > 0.001 {
        // Measured in the button's heights, so a wide button and a narrow
        // one sweep at the same pace and the band keeps its shape.
        let aspect = size.x / size.y;
        let u = in.uv.x * aspect + (1.0 - in.uv.y) * 0.45;
        let travel = aspect + 1.6;
        let phase = fract(globals.time / PERIOD) * params.sweep + 0.5 * (1.0 - params.sweep);
        let at = phase * travel - 0.8;
        let band = exp(-pow((u - at) / 0.32, 2.0));
        light += params.hover * (0.05 + 0.20 * band);
    }
    return vec4<f32>(params.light.rgb, clamp(light * inside, 0.0, 1.0));
}
