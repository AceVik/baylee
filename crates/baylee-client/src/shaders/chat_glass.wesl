// The chat's light (owner, 29.10.2026: "make the chat input and the chat
// window more attractive, with light shader animations").
//
// Two looks on one small block, drawn as light (alpha only) over what is
// under it and never over the middle of what is read:
//
// - the window (`kind` 0): a rim of light just inside the panel's rounded
//   edge, brightest along the top, and, for 0.9 s after a chat line
//   arrives, a band that climbs the two sides from the foot, where new
//   lines appear, to the head, washing a little way in;
// - the field (`kind` 1): while the composer has the caret, its rim lights
//   and a glint travels round it every 3.2 s; without the caret, a faint
//   rim and nothing that moves.
//
// Uniforms only (the GL budget, docs/dev/commands.md). With no arrival
// running and no caret, nothing here depends on the clock, so a still
// screen draws the same pixels every frame. `motion` 0 (reduce_motion)
// holds the glint where it starts and draws no band.

import bevy_render::globals::Globals;
import bevy_ui_render::ui_vertex_output::UiVertexOutput;

struct GlassParams {
    /// The light's colour (linear); its alpha is unused.
    light: vec4<f32>,
    /// The field has the caret: 0 or 1.
    focus: f32,
    /// `globals.time` when the last line arrived; far in the past for none.
    arrived: f32,
    /// 1 moves, 0 holds still (reduce_motion).
    motion: f32,
    /// 0 the window, 1 the field.
    kind: f32,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> params: GlassParams;

const ARRIVAL: f32 = 0.9;
const GLINT_PERIOD: f32 = 3.2;
const TAU: f32 = 6.2831853;

/// The signed distance to the node's rounded edge, in pixels: negative
/// inside.
fn edge(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let size = max(in.size, vec2<f32>(1.0, 1.0));
    let half = size * 0.5;
    let p = (in.uv - vec2<f32>(0.5, 0.5)) * size;
    let r = min(in.border_radius_x.x, min(half.x, half.y));
    let d = edge(p, half, r);
    let inside = clamp(0.5 - d, 0.0, 1.0);
    // How far in from the edge, in pixels.
    let depth = max(-d, 0.0);
    // The rim: light a pixel or three inside the edge. Laid on the edge
    // itself it fell under the panel's and the field's 1 px border and was
    // gone (measured 30.10.2026: the glint's peak was 18 levels).
    let rim = exp(-pow(max(depth - 1.0, 0.0), 2.0) / 5.0);

    var light = 0.0;
    if params.kind < 0.5 {
        // The window: the rim, stronger along the top, and a breath of
        // light falling a little way in from the top edge.
        let top = 1.0 - in.uv.y;
        light = rim * (0.08 + 0.20 * top * top) + exp(-depth / 22.0) * 0.035 * top;
        let t = (globals.time - params.arrived) / ARRIVAL;
        if params.motion > 0.5 && t > 0.0 && t < 1.0 {
            // A band climbing from the foot to the head, fading as it goes,
            // that lights the rim and washes the panel in from its sides.
            let at = 1.0 - t;
            let band = exp(-pow((in.uv.y - at) / 0.10, 2.0)) * (1.0 - t);
            light += band * (0.70 * rim + 0.20 * exp(-depth / 26.0));
        }
    } else {
        // The field: a faint rim without the caret; with it, a lit rim, a
        // glow rising from the foot, and the glint going round.
        light = rim * (0.08 + 0.30 * params.focus)
            + exp(-depth / 10.0) * 0.06 * params.focus * in.uv.y;
        if params.focus > 0.5 {
            // Round the edge by angle, measured in the field's own shape so
            // the glint keeps its pace on a long, low box.
            let a = atan2(p.y / half.y, p.x / half.x) / TAU + 0.5;
            let turn = fract(globals.time / GLINT_PERIOD) * params.motion;
            var off = abs(a - turn);
            off = min(off, 1.0 - off);
            let glint = exp(-pow(off / 0.05, 2.0));
            light += glint * (0.85 * rim + 0.30 * exp(-depth / 6.0));
        }
    }
    return vec4<f32>(params.light.rgb, clamp(light * inside, 0.0, 1.0));
}
