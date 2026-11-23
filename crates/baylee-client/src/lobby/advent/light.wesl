import bevy_ui_render::ui_vertex_output::UiVertexOutput;

@group(1) @binding(0) var<uniform> params: vec4<f32>;

fn segment(p:vec2<f32>, a:vec2<f32>, b:vec2<f32>) -> f32 {
    let line = b - a;
    return length(p - a - line * clamp(dot(p - a,line) / dot(line,line),0.0,1.0));
}

// Six engraved arms with paired branches, measured in units of plaque height.
fn snowflake(p:vec2<f32>, radius:f32, aa:f32) -> f32 {
    var d = radius;
    for(var i=0u; i<6u; i=i+1u) {
        let angle = f32(i)*1.047198 - 1.570796;
        let ray = vec2<f32>(cos(angle),sin(angle));
        let arm = vec2<f32>(dot(p,ray),abs(dot(p,vec2<f32>(-ray.y,ray.x))));
        d = min(d,segment(arm,vec2<f32>(0.0),vec2<f32>(radius,0.0)));
        d = min(d,segment(arm,radius*vec2<f32>(0.47,0.0),radius*vec2<f32>(0.75,0.25)));
        d = min(d,segment(arm,radius*vec2<f32>(0.70,0.0),radius*vec2<f32>(0.89,0.16)));
    }
    return 1.0 - smoothstep(0.006,0.010+aa,d);
}

@fragment
fn fragment(in:UiVertexOutput)->@location(0) vec4<f32> {
    let dp = max(fwidth(in.uv),vec2<f32>(0.00001));
    let aspect = dp.y/dp.x;
    let p = vec2<f32>(in.uv.x*aspect,in.uv.y);
    let aa = dp.y;
    let gold = vec3<f32>(0.89,0.62,0.30);
    let ice = vec3<f32>(0.77,0.92,0.86);
    let vignette = smoothstep(0.0,0.9,1.0-length((in.uv-vec2<f32>(0.5))*vec2<f32>(1.24,0.78)));
    var color = mix(vec3<f32>(0.018,0.041,0.083),vec3<f32>(0.034,0.177,0.161),vignette);
    let edges = vec2<f32>(min(p.x,aspect-p.x),min(p.y,1.0-p.y));
    let edge = min(edges.x,edges.y);
    let rail = 1.0-smoothstep(0.003,0.007+aa,abs(edge-0.055));
    let end = min(p.x,aspect-p.x);
    let medallion = vec2<f32>(end-0.53,p.y-0.5);
    let ring = 1.0-smoothstep(0.003,0.008+aa,abs(length(medallion)-0.30));
    let rays = snowflake(medallion,0.225,aa);
    let diamond = 1.0-smoothstep(0.005,0.012+aa,
        abs(abs(end-0.105)+abs(p.y-0.5)-0.050));
    let glint = 0.5+0.5*cos(6.283185*(in.uv.x*1.5-params.x/5.5));
    let shine = params.y*glint;
    color += vec3<f32>(0.10,0.065,0.018)*(0.45*params.w+params.z);
    color = mix(color,gold,clamp(0.60*rail + 0.45*ring + 0.90*diamond + shine*0.4*rail,0.0,1.0));
    color = mix(color,mix(gold,ice,0.65+shine*0.3),rays*(0.76+0.21*params.z));
    return vec4<f32>(color,1.0);
}
