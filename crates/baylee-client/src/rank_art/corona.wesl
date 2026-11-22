import bevy_ui_render::ui_vertex_output::UiVertexOutput;
import bevy_render::globals::Globals;
@group(0) @binding(1) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> tint: vec4<f32>;
@group(1) @binding(1) var<uniform> motion: vec4<f32>;
@fragment fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = 2.0 * in.uv - 1.0;
    let r = length(p);
    let a = atan2(p.y,p.x);
    let aa = max(fwidth(r),0.002);
    let t = select(0.34, globals.time/5.0, motion.y > 0.5);
    let strength = motion.x;
    let line = 1.0-smoothstep(0.008,0.008+aa,abs(r-0.77));
    let diamond = pow(max(cos(4.0*a),0.0),24.0);
    let tail = 1.0 - smoothstep(0.78,0.87+0.07*diamond,r);
    let inner = smoothstep(0.62,0.73,r);
    var light = 0.12*inner*tail + 0.40*line;
    if strength > 0.5 {
        let arc = smoothstep(-0.24,0.1,cos(6.0*a-t));
        light += arc * (1.0-smoothstep(0.005,0.005+aa,abs(r-0.86))) * 0.64;
    }
    if strength > 1.5 {
        let rays = pow(max(cos(16.0*a),0.0),5.0);
        let gleam = 0.65+0.35*cos(2.0*a-t);
        light += 0.45*rays*smoothstep(0.80,0.84,r)*(1.0-smoothstep(0.87,0.99,r))*gleam;
    }
    return vec4<f32>(mix(tint.rgb,vec3<f32>(1.0,0.84,0.59),0.42),clamp(light,0.0,0.88));
}
