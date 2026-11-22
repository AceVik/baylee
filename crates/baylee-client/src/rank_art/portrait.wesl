// The placement frame is the unadorned rim of a blank. Ranked frames have
// true transparent cutouts and their own engraved, material-specific relief.
fn fill(d: f32, aa: f32) -> f32 {
    return 1.0 - smoothstep(-aa, aa, d);
}
fn rim(d: f32, width: f32, aa: f32) -> f32 {
    return fill(abs(d) - width, aa);
}
fn neutral_frame(uv: vec2<f32>, deck: bool, metal: vec3<f32>) -> vec4<f32> {
    let p = 2.0 * uv - 1.0;
    let squared = p * p;
    let r = select(length(p), sqrt(sqrt(dot(squared,squared))), deck);
    let aa = max(fwidth(r) * 0.55, 0.001);
    return vec4<f32>(metal * (0.65 + 0.55 * rim(r-0.947,0.005,aa)),rim(r-0.952,0.023,aa));
}
fn engraving(uv: vec2<f32>, src: vec4<f32>, kind: vec4<u32>, metal: vec3<f32>, jewel: vec3<f32>, phase: f32) -> vec4<f32> {
    if kind.x < 2u { return src; }
    let axis = uv.x * 0.63 + uv.y * 0.37;
    let sweep = 1.0 - smoothstep(0.0, 0.070, abs(axis - (phase * 1.36 - 0.18)));
    let value = max(src.r, max(src.g,src.b));
    let polished = smoothstep(0.085,0.72,value);
    let enamel = smoothstep(0.07,0.55,value - min(src.r,min(src.g,src.b)));
    let metal_light = mix(metal,vec3<f32>(1.0),0.38) * (0.10 + f32(kind.x)*0.013);
    let twinkle = 0.5 + 0.5 * cos(6.2831853 * (phase + uv.y * 0.55));
    let refracted = select(vec3<f32>(0.0),jewel * enamel * twinkle * 0.10,kind.x >= 6u);
    let crown = max(0.0,1.0-length((uv-vec2<f32>(0.50,0.12))*vec2<f32>(10.0,8.0)));
    let fire = select(vec3<f32>(0.0),jewel*crown*pow(twinkle,8.0)*0.22,kind.z>0u);
    return vec4<f32>(src.rgb + polished * (sweep * metal_light + refracted + fire),src.a);
}
fn rounded_plate(p: vec2<f32>, sides: vec2<f32>) -> f32 {
    let v = abs(p) - sides;
    return length(max(v,vec2<f32>(0.0))) + min(max(v.x,v.y),0.0) - 0.05;
}

// A plain pewter helmet study or a pair of slate card backs during placements.
fn unranked(uv: vec2<f32>, deck: bool, metal: vec3<f32>) -> vec4<f32> {
    let p = (uv - 0.5) * 2.0;
    let aa = max(fwidth(p.x) * 0.6, 0.001);
    var coverage = 0.0;
    var bevel = 0.0;
    if deck {
        let q = p + vec2<f32>(0.20,0.04);
        let back = rounded_plate(vec2<f32>(q.x*0.95-q.y*0.31, q.x*0.31+q.y*0.95), vec2<f32>(0.33,0.52));
        let front = rounded_plate(p-vec2<f32>(0.12,0.04),vec2<f32>(0.34,0.51));
        coverage = max(fill(back,aa),fill(front,aa));
        bevel = max(rim(back+0.052,0.026,aa) * (1.0-fill(front,aa)),rim(front+0.062,0.029,aa));
    } else {
        let crown = length((p + vec2<f32>(0.0,0.12)) / vec2<f32>(0.56,0.60)) - 1.0;
        let cheek = abs(p.x) - (0.54 - 0.39*clamp(p.y+0.08,0.0,1.0));
        let edge = max(cheek,max(-p.y-0.18, p.y - 0.66 + abs(p.x)*0.45));
        let shape = min(crown*0.54,edge);
        let visor = max(abs(p.y + 0.04 + abs(p.x)*0.12)-0.084, abs(p.x)-0.50);
        let seam = max(abs(p.x)-0.044,max(-p.y-0.04,p.y-0.55));
        coverage = fill(max(shape,-min(visor,seam)),aa);
        bevel = rim(shape+0.045,0.025,aa);
    }
    let highlight = 0.17 * clamp(1.0-uv.y,0.0,1.0);
    return vec4<f32>(metal*(0.58 + highlight + 0.63*bevel),coverage);
}
