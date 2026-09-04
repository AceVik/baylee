// The table the game is played on: a slab of polished granite with runes
// burning in it.
//
// Two textures and one number. The stone is opaque and static — it is a
// picture of a rock and there is nothing for it to do — and everything that
// moves is in the rune layer, which is why they are not one image. Baking
// the glow into the stone would make the whole slab a new texture every time
// it breathed.
//
// # WebGL2
//
// The browser build targets WebGL2: uniforms only, no storage buffers, no
// texture arrays, every loop bound at compile time. Nothing below reaches for
// any of them, and `globals.time` comes from the view bind group, so the
// runes breathe without the CPU touching a material asset per frame.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::globals

struct TableParams {
    /// The clock the runes breathe on: 1 normally, 0 for
    /// `Preferences::reduce_motion`.
    motion: f32,
    /// How bright a rune burns at the top of its breath.
    gain: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var stone: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var stone_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var runes: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var runes_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<uniform> params: TableParams;

/// Radians per second: about seven seconds to a full breath, which is slow
/// enough that a player reading a card never catches it moving.
const BREATH: f32 = 0.9;

/// How far down a rune goes at the bottom of its breath, as a fraction of
/// its peak. Never to nothing: a rune that goes out reads as a bug, and the
/// runes that are *supposed* to be dark are the ones this texture never drew.
const EMBER: f32 = 0.30;

const TAU: f32 = 6.2831855;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(stone, stone_sampler, in.uv).rgb;

    // `rgb` is the sigil's colour already multiplied by its coverage, so it
    // is black wherever there is no rune and this adds nothing; `a` is that
    // rune's own phase. `tabletop::runes` says why the phase rides in alpha
    // and not in a colour channel.
    let rune = textureSample(runes, runes_sampler, in.uv);

    // A spatial phase, so a still table is an honest frame of the animation
    // rather than its mean — some runes bright, some low, none moving. That
    // is the same rule the card shader's indestructible border follows, and
    // it is what a frozen phase is allowed to be.
    let t = globals.time * params.motion;
    let breath = mix(EMBER, 1.0, 0.5 + 0.5 * sin(t * BREATH + rune.a * TAU));

    return vec4<f32>(color + rune.rgb * breath * params.gain, 1.0);
}
