// Original lathe-turned portrait bezels. All distances are fractions of the disc radius.
// Keep the portrait and the transparent outside live, including on a moving portrait.
const TAU: f32 = 6.283185307;
fn fill(d: f32, aa: f32) -> f32 {
    return 1.0 - smoothstep(-aa, aa, d);
}
fn rim(d: f32, width: f32, aa: f32) -> f32 {
    return fill(abs(d) - width, aa);
}

// Chased feather with pointed ends and two polished faces.
fn leaf(p: vec2<f32>, root: vec2<f32>, tip: vec2<f32>, half_width: f32, aa: f32) -> vec2<f32> {
    let v = tip - root;
    let axis = normalize(v);
    let along = dot(p-root,axis);
    let t = clamp(along / length(v),0.0,1.0);
    let across = dot(p-root,vec2<f32>(-axis.y,axis.x));
    let d = max(max(-along,along-length(v)),abs(across)-half_width*(1.0-abs(2.0*t-1.0)));
    return vec2<f32>(fill(d,aa),smoothstep(-aa,aa,across));
}

fn frame(uv: vec2<f32>, kind: vec4<u32>, metal: vec3<f32>, jewel: vec3<f32>, phase: f32) -> vec4<f32> {
    let p = 2.0 * uv - 1.0;
    // The deck's almost-square shoulders echo the backs of its cards.
    var r = length(p);
    if kind.w == 3u {
        let squared = p*p;
        r = sqrt(sqrt(dot(squared,squared)));
    }
    let aa = max(fwidth(r) * 0.55, 0.001);
    // No chased element reaches the central two-thirds or the square corners.
    if r < 0.68 || r > 1.0+aa { return vec4<f32>(0.0); }
    let angle = atan2(p.x, -p.y);
    if kind.x == 11u {
        let base = rim(r - 0.952, 0.023, aa);
        let etched = rim(r - 0.947, 0.005, aa);
        return vec4<f32>(metal * (0.65 + 0.55 * etched), base);
    }
    let level = f32(kind.x);
    let step = f32(kind.y);
    let podium = select(0.0, 4.0 - f32(kind.z), kind.z > 0u);
    let width = 0.028 + level * 0.0021 + step * 0.0018;
    let relief = cos(angle * 4.0);
    let seat = 0.951 - width + 0.008 * relief - 0.031*smoothstep(6.0,10.0,level);
    let band = r - seat;
    let strength = clamp(1.0 - abs(band / width), 0.0, 1.0);
    let shine = 0.5 + 0.5 * cos(angle + 1.0);
    let grain = 0.5 + 0.5 * sin(4.0 * angle + 160.0 * r);
    let lacquer = select(1.0, 0.84 + 0.16 * grain, kind.x == 1u);
    let groove = 0.5 + 0.5 * cos(3.8 * band / width);
    var color = metal * (0.26 + 0.45 * strength + 0.3 * shine) * lacquer;
    color += vec3<f32>(0.26,0.24,0.21) * groove * shine * strength;
    var alpha = rim(band, width, aa);

    // Wire rims and chasing are progressively worked into the cast ring.
    let wire = rim(band + width * 0.61, 0.006, aa);
    color += metal * wire * (0.15 + 0.18 * shine);
    if kind.x >= 3u {
        let engraving = rim(band - width * 0.24, 0.005, aa)
            * smoothstep(-0.45, 0.50, cos(angle * (16.0 + 4.0 * step)));
        color *= 1.0 - engraving * 0.33;
    }
    let glint = pow(max(cos(angle - TAU * phase),0.0), 18.0);
    color += metal * glint * (0.09 + 0.028 * level) * strength;

    // Crystal facets alternate across the outer lip at Diamond. The elite
    // settings grow a second rail and progressively longer pairs of temple leaves.
    if kind.x >= 7u {
        let facet = 0.5 + 0.5 * cos(angle * 28.0);
        color = mix(color,jewel*(0.31 + 0.60*facet),rim(band-width*0.56,0.012,aa)*0.72);
    }
    if kind.x >= 8u {
        let extent = level - 8.0;
        let wire2 = rim(band+width+0.026,0.010,aa)
            * smoothstep(-0.75,-0.23,cos(4.0*angle));
        color = mix(color, metal*(0.68 + 0.43*shine),wire2);
        alpha = max(alpha,wire2);
        let mirror = vec2<f32>(abs(p.x),p.y);
        for (var i=0u; i<4u; i+=1u) {
            if i >= kind.x-6u { break; }
            let t = f32(i);
            let start = vec2<f32>(0.26+0.12*t,-0.78+0.063*t);
            let tip = vec2<f32>(0.41+0.12*t,-0.83+0.077*t);
            let feather = leaf(mirror,start,tip,0.032+0.004*extent,aa);
            let enamel = select(metal,jewel,kind.x == 9u && i == 2u);
            color = mix(color,enamel*(0.48+0.58*feather.y)+0.06*glint,feather.x);
            alpha = max(alpha,feather.x);
        }
        if kind.x == 10u {
            let side = abs(p.x);
            let y = p.y+0.86;
            let arch = abs(y - 0.16*side + 0.030*cos(27.0*side));
            let comb = fill(arch-0.021,aa)*fill(side-0.33,aa);
            color = mix(color,metal*(0.59+0.50*shine),comb);
            alpha = max(alpha,comb);
        }
    }

    // A paired leaf for each earned division, set around the foot of the disc.
    let foot = abs(abs(angle) - TAU * 0.5);
    let leaf_index = floor(foot / 0.166);
    if leaf_index < step {
        let along = fract(foot / 0.166) - 0.5;
        let across = r - (seat - width - 0.006 + 0.040 * along);
        let leaf = fill(abs(across) - 0.020 * (1.0 - abs(2.0 * along)), aa);
        color = mix(color, mix(metal * 0.65, metal + 0.12, 0.5 + along), leaf);
        alpha = max(alpha, leaf);
    }

    // From Gold the setting holds a gemstone. The elite bezels gain side stones,
    // then a rose kite, a stepped sapphire and a brilliant with a pavé halo.
    if kind.x >= 5u {
        let count = select(1.0, select(3.0, 4.0, kind.x >= 9u), kind.x >= 7u);
        let sector = TAU / count;
        let local = (fract(angle / sector + 0.5) - 0.5) * sector;
        let top = abs(angle) < 0.4;
        let size = (0.043 + 0.0041 * level + 0.0016 * step) * select(0.67, 1.0, top)
            + select(0.0, 0.019 * podium, top);
        let v = vec2<f32>(local * seat, r - (seat - 0.021));
        var cut = abs(v.x) + 1.15 * abs(v.y);
        if kind.z == 2u && top {
            cut = max(abs(v.x), 1.12 * abs(v.y)) + 0.40 * min(abs(v.x),abs(v.y));
        }
        let collar = fill(cut - size, aa);
        let gem = fill(cut - size * 0.77, aa);
        let face = clamp(0.55 + (0.31 * v.x - 0.41 * v.y) / size, 0.1, 1.0);
        let fire = pow(max(cos(5.0 * atan2(v.x, v.y) + phase * TAU), 0.0), 4.0);
        color = mix(color, metal * (0.43 + 0.64 * face), collar);
        color = mix(color, jewel * (0.38 + 0.67 * face) + vec3<f32>(0.26) * fire, gem);
        alpha = max(alpha, collar);
        if kind.z > 0u && top {
            let shoulder = vec2<f32>(abs(v.x)-size*1.18, v.y+0.004);
            let satellite = abs(shoulder.x)+abs(shoulder.y)*select(1.8,1.18,kind.z == 3u);
            let sides = fill(satellite-size*0.35,aa);
            color = mix(color,mix(jewel,metal,0.34)*(0.8+0.12*fire),sides);
            alpha = max(alpha,sides);
        }
        if kind.z == 1u && top {
            let bead_angle = atan2(v.y,v.x);
            let bead = rim(length(v * vec2<f32>(1.0,1.10)) - size * 0.85, 0.009, aa)
                * smoothstep(-0.2,0.65,cos(bead_angle * 14.0));
            color = mix(color, mix(jewel,vec3<f32>(1.0),0.5), bead);
            alpha = max(alpha, bead);
        }
    }
    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), alpha);
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
