// Original Baylee rank paintings. Input and output are straight-alpha sRGB in
// unorm textures/targets. Lighting is applied in linear RGB; coverage is copied.
struct Finish {
    phase: f32,       // normalized loop, held fixed for reduced motion
    tier: u32,        // Papier = 0 ... Champion = 10
    deck: u32,        // 0 = helmet, 1 = card fan
    division: u32,    // 0 = league badge, 1 = first division, then 2..4
    place: u32,       // 1..3 only on the corresponding Champion player painting
    enabled: u32,
    pad0: u32,
    pad1: u32,
}
@group(0) @binding(0) var<uniform> fx: Finish;
@group(0) @binding(1) var art: texture_2d<f32>;
@group(0) @binding(2) var art_sampler: sampler;

struct Vertex { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs(@builtin(vertex_index) i: u32) -> Vertex {
    let xy = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return Vertex(vec4<f32>(xy * 2.0 - 1.0, 0.0, 1.0), vec2<f32>(xy.x, 1.0 - xy.y));
}

const TAU: f32 = 6.2831853;
fn band(x: f32, phase: f32, width: f32) -> f32 {
    let distance = abs(fract(x - phase + 0.5) - 0.5);
    return 1.0 - smoothstep(0.0, width, distance);
}
fn spot(uv: vec2<f32>, center: vec2<f32>, radii: vec2<f32>) -> f32 {
    return 1.0 - smoothstep(0.40, 1.0, length((uv - center) / radii));
}
fn linear(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}
fn srgb(c: vec3<f32>) -> vec3<f32> {
    return select(1.055 * pow(c, vec3<f32>(1.0/2.4)) - 0.055, c * 12.92, c <= vec3<f32>(0.0031308));
}

// The front card and the two fanned cards catch light on different planes.
// A helm curves symmetrically away from its polished central ridge.
fn light_path(uv: vec2<f32>) -> f32 {
    let x = uv.x - 0.5;
    if fx.deck == 1u {
        let side = smoothstep(0.13, 0.25, abs(x)) * sign(x);
        return 0.69 * uv.x + (0.29 + 0.22 * side) * uv.y + 0.075 * side;
    }
    return 0.78 * uv.x + 0.26 * uv.y + 0.70 * x*x;
}

fn finish_light(uv: vec2<f32>, c: vec3<f32>, path: f32, tone: f32) -> vec3<f32> {
    let t = fx.phase;
    let relief = smoothstep(0.18, 0.71, tone);
    let blue = smoothstep(0.03, 0.32, c.b - c.r);
    let red = smoothstep(0.08, 0.37, c.r - c.g);
    let violet = smoothstep(0.04, 0.24, min(c.r, c.b) - c.g);
    let silver = vec3<f32>(0.70, 0.84, 1.0);
    let gold = vec3<f32>(1.0, 0.60, 0.22);
    switch fx.tier {
        case 0u: { // Stationary side lighting on the existing paper folds.
            return vec3<f32>(0.036, 0.021, 0.006) * (0.70 - path) * relief;
        }
        case 1u: { // A fixed wax sheen with the grain; the carving remains matte.
            let grain = 0.55 + 0.45*cos(29.0*uv.x + 3.5*sin(6.0*uv.y));
            return vec3<f32>(0.049, 0.025, 0.008) * grain * relief;
        }
        case 2u: { // Soft broken reflection on brushed iron.
            let brush = 0.76 + 0.24 * cos(81.0*uv.y + 3.0*uv.x);
            return silver * 0.105 * relief * brush * band(path, t, 0.21);
        }
        case 3u: { // Copper catches on raised bronze; patina rests in recesses.
            return vec3<f32>(1.0,0.43,0.18)*0.19*relief*band(path,t,0.18)
                + vec3<f32>(0.0,0.013,0.010)*(1.0-relief)*red;
        }
        case 4u: { // A bright hairline followed by a softer silver reflection.
            return silver * relief * (0.28*band(path,t,0.085) + 0.08*band(path,t+0.105,0.055));
        }
        case 5u: { // A broad sun-gold sweep across the engraved sunrays.
            let rays = 0.76 + 0.24*cos(10.0*atan2(uv.y-0.49,uv.x-0.5));
            return gold * 0.38 * relief * rays * band(path,t,0.20);
        }
        case 6u: { // Two cool banks of light along the mint enamel.
            let filigree = smoothstep(0.0,0.22,c.g-c.r);
            return silver * 0.23 * relief * band(path,t,0.10)
                + vec3<f32>(0.21,0.90,0.82)*0.22*(0.24+filigree)*band(path,t+0.17,0.09);
        }
        case 7u: { // Angular fronts break up differently on neighbouring facets.
            let facets = floor(uv.x*8.0) + floor((uv.y+uv.x*0.44)*9.0);
            let facet = path + 0.10*cos(facets*2.17);
            return vec3<f32>(0.24,0.78,1.0)*0.46*blue*band(facet,t,0.14)
                + silver*0.18*relief*band(facet,t+0.13,0.045);
        }
        case 8u: { // Amethyst light wells up inside the purple stone.
            let core = 0.5+0.5*cos(TAU*(t - 0.62*length((uv-0.5)*1.7)));
            return vec3<f32>(0.56,0.15,0.96)*0.30*violet*core
                + gold*0.13*relief*band(path,t,0.12);
        }
        case 9u: { // Two ruby caustics rise through the stone and its gold setting.
            let arc = 0.76*uv.y + 0.54*abs(uv.x-0.5);
            return vec3<f32>(1.0,0.12,0.16)*red*(0.34*band(arc,t,0.19)+0.18*band(arc,t+0.25,0.11))
                + gold*0.18*relief*band(path,t+0.12,0.09);
        }
        default: { // Mother of pearl on the light shell, honey on the gold rim.
            let pearl = smoothstep(0.42,0.82,min(c.r,min(c.g,c.b)));
            let iridescence = 0.52 + 0.48*cos(TAU*(path-t)+vec3<f32>(0.2,2.3,4.4));
            return iridescence * pearl * 0.29 + gold*0.20*relief*band(path,t,0.12);
        }
    }
}

fn podium_light(uv: vec2<f32>, c: vec3<f32>) -> vec3<f32> {
    // Centers measured after the paintings have been trimmed to the game canvas.
    let brow = vec2<f32>(0.50, select(0.427, 0.390, fx.place == 3u));
    let p = uv - brow;
    let mirror = vec2<f32>(abs(p.x),p.y);
    let t = fx.phase;
    switch fx.place {
        case 3u: { // Rose diamond and chased copper laurel.
            let gem = spot(uv,brow,vec2<f32>(0.047,0.074));
            let leaves = spot(mirror,vec2<f32>(0.103,0.008),vec2<f32>(0.081,0.055));
            return vec3<f32>(0.48,0.19,0.22)*gem*band(0.36+4.0*p.x-2.0*p.y,t,0.13)
                + vec3<f32>(0.33,0.14,0.07)*leaves*band(abs(p.x)*3.2-p.y,t,0.18);
        }
        case 2u: { // A stepped blue cut, followed by both side stones and silver feathers.
            let wings = spot(mirror,vec2<f32>(0.146,-0.014),vec2<f32>(0.117,0.086));
            let sides = spot(mirror,vec2<f32>(0.098,0.034),vec2<f32>(0.033,0.023));
            let gem = spot(uv,brow,vec2<f32>(0.078,0.078));
            let cut = max(abs(p.x),0.83*abs(p.y))+0.15*min(abs(p.x),abs(p.y));
            return vec3<f32>(0.08,0.33,0.55)*gem*band(0.18+5.4*cut,t,0.135)
                + vec3<f32>(0.29,0.42,0.50)*(wings*band(abs(p.x)*3.0-p.y,t,0.18)+sides*band(0.59,t,0.17));
        }
        case 1u: { // Prismatic fire breaks over the brilliant before it reaches the pavé halo.
            let cut = abs(p.x)+0.98*abs(p.y);
            let gem = 1.0-smoothstep(0.066,0.086,cut);
            let setting = (1.0-smoothstep(0.009,0.023,abs(cut-0.097)));
            let rays = spot(uv,brow-vec2<f32>(0.0,0.045),vec2<f32>(0.19,0.112));
            let fire = 0.55+0.45*cos(TAU*(t+2.4*p.x)+vec3<f32>(0.0,2.1,4.2));
            let facet = band(0.35+3.9*p.x-2.8*p.y+0.13*sign(p.x*p.y),t,0.18);
            return fire*0.53*gem*facet + vec3<f32>(0.34,0.30,0.24)*setting*band(0.5+p.x*3.2,t,0.14)
                + vec3<f32>(0.36,0.23,0.09)*rays*band(length(p)*3.8,t+0.13,0.14)*c.r;
        }
        default: {return vec3<f32>(0.0);}
    }
}

@fragment fn fs(in: Vertex) -> @location(0) vec4<f32> {
    let src = textureSample(art, art_sampler, in.uv);
    if fx.enabled == 0u || src.a == 0.0 {return src;}
    let uv = in.uv;
    let path = light_path(uv);
    let tone = dot(src.rgb, vec3<f32>(0.2126,0.7152,0.0722));
    var light = finish_light(uv,src.rgb,path,tone);
    if fx.tier == 10u && fx.deck == 0u {light += podium_light(uv,src.rgb);}
    if fx.division > 0u {
        // Keep the Roman numeral and its enamel bed exactly as painted.
        if abs(uv.x-0.5) < 0.112 && uv.y > 0.79 && uv.y < 0.907 {return src;}
        let clasp = smoothstep(0.75,0.81,uv.y)*(1.0-smoothstep(0.19,0.26,abs(uv.x-0.5)));
        let phase = select(fx.phase,0.46,fx.tier < 2u);
        light += vec3<f32>(1.0,0.86,0.66)*0.025*f32(fx.division)*clasp*band(path,phase,0.14)*tone;
    }
    // Dark recesses, including the black visor, retain depth.
    let result = clamp(linear(src.rgb) + light*smoothstep(0.035,0.30,tone),vec3<f32>(0.0),vec3<f32>(1.0));
    return vec4<f32>(srgb(result),src.a);
}
