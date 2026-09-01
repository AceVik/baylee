//! The card material: printed art, its physical finish, and the keywords the
//! rules have given it.
//!
//! # Why one material and not three
//!
//! A foil that is also indestructible is one card, not two draws. The three
//! effects compose on the same pixel, so they compose in one shader, and a
//! board of three hundred permanents costs one pipeline rather than three.
//!
//! # Why the material key is not the texture
//!
//! [`crate::table::SceneIndex`] used to key materials on [`ImageKey`] alone —
//! forty Islands were one material. They still are, but "the same card" now
//! means the same *art, finish and glow*: a foil Island and a plain one are
//! two materials, and an Island that gains indestructible for a turn is a
//! third until it loses it. That is the smallest key that draws correctly,
//! and it keeps the sharing that made the original one worth having.
//!
//! # WebGL2
//!
//! The browser build targets WebGL2, so the shader uses uniforms only — no
//! storage buffers, no texture arrays. The animation reads `globals.time`
//! from the view bind group, which means nothing here is written per frame:
//! a material is created once and never touched again while it is on screen.

use baylee_client_core::images::{FinishTreatment, ImageKey};
use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

/// Which keyword glows a card wears, as the shader's bitset.
///
/// Deliberately not the engine's keyword numbering: the shader reads three
/// bits and the engine has more than a hundred keywords, so translating once
/// here is cheaper than sending a `u128` to the GPU, and it makes adding a
/// fourth glow a one-line change on both sides.
pub mod glow {
    /// Indestructible — darksteel.
    pub const INDESTRUCTIBLE: u32 = 1;
    /// Hexproof.
    pub const HEXPROOF: u32 = 2;
    /// Shroud.
    pub const SHROUD: u32 = 4;
}

/// The engine's keyword bit for each glow, from `baylee-cards-dsl`.
///
/// Pinned by a test rather than trusted: the DSL numbering is generated, and
/// a card that silently glowed for the wrong keyword would be a rules lie a
/// player would believe.
const KEYWORD_BITS: [(u32, u32); 3] = [
    (6, glow::INDESTRUCTIBLE),
    (5, glow::HEXPROOF),
    (14, glow::SHROUD),
];

/// Translates the view's keyword bitset into the shader's three bits.
#[must_use]
pub fn glow_bits(keywords: u128) -> u32 {
    let mut bits = 0;
    for (keyword, flag) in KEYWORD_BITS {
        if keywords & (1u128 << keyword) != 0 {
            bits |= flag;
        }
    }
    bits
}

/// What the shader needs to know about one card.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct CardParams {
    /// 0 plain, 1 foil, 2 etched.
    pub finish: u32,
    /// Keyword glows, from [`glow_bits`].
    pub glow: u32,
    /// 1.0 when the material carries real artwork, 0.0 when the card is drawn
    /// as a flat `tint` — its constructed face, or its back.
    pub has_art: f32,
    /// How strongly the finish shows. One material can be dimmed without a
    /// second pipeline.
    pub strength: f32,
    /// The colour a card with no artwork is drawn in.
    pub tint: Vec4,
}

/// A card's surface.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct CardMaterial {
    /// The printed face. `None` for a card drawing its own text or its back,
    /// which is what `params.has_art` says.
    #[texture(0)]
    #[sampler(1)]
    pub art: Option<Handle<Image>>,
    /// Everything else.
    #[uniform(2)]
    pub params: CardParams,
}

impl Material for CardMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/card.wgsl".into()
    }

    /// Cards are opaque rectangles. The mesh's rounded corners are cut out of
    /// the geometry, not alpha-tested, so nothing here needs blending — and
    /// an opaque card sorts by depth rather than by draw order, which is what
    /// keeps a stack of four looking like a stack.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}

/// The key a material is shared on.
///
/// Two cards share a material when they show the same art with the same
/// finish and the same glow. Anything less would draw a foil as plain;
/// anything more would be a material per permanent.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CardLook {
    /// Which art, or `None` for a card drawing its own face.
    pub art: Option<ImageKey>,
    /// Its physical finish.
    pub finish: FinishTreatment,
    /// Its keyword glows.
    pub glow: u32,
    /// The flat colour, quantised, for a card with no art. `0` when it has
    /// art — a colour is not part of the key then.
    pub tint: u32,
}

impl CardLook {
    /// A card showing artwork.
    #[must_use]
    pub fn art(art: ImageKey, finish: FinishTreatment, glow: u32) -> Self {
        Self {
            art: Some(art),
            finish,
            glow,
            tint: 0,
        }
    }

    /// A card showing a flat colour: its constructed face, or its back.
    #[must_use]
    pub fn flat(color: Color, finish: FinishTreatment, glow: u32) -> Self {
        Self {
            art: None,
            finish,
            glow,
            tint: quantise(color),
        }
    }
}

/// A colour as a hashable key.
///
/// Eight bits a channel: two colours a player could not tell apart must not
/// cost two materials, and `f32` is not `Hash` anyway.
fn quantise(color: Color) -> u32 {
    let c = color.to_srgba();
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0) as u32;
    (byte(c.red) << 24) | (byte(c.green) << 16) | (byte(c.blue) << 8) | byte(c.alpha)
}

/// The finish as the shader's number.
#[must_use]
pub fn finish_code(finish: FinishTreatment) -> u32 {
    match finish {
        FinishTreatment::Plain => 0,
        FinishTreatment::Foil => 1,
        FinishTreatment::Etched => 2,
    }
}

/// Builds the material for a look.
#[must_use]
pub fn material(look: CardLook, art: Option<Handle<Image>>, tint: Color) -> CardMaterial {
    CardMaterial {
        art,
        params: CardParams {
            finish: finish_code(look.finish),
            glow: look.glow,
            has_art: if look.art.is_some() { 1.0 } else { 0.0 },
            strength: 1.0,
            tint: LinearRgba::from(tint).to_f32_array().into(),
        },
    }
}

/// Registers the material and ships its shader inside the binary.
///
/// Embedded rather than loaded from `assets/`: `index.html` copies only the
/// font directory to `dist/`, so a shader on disk would work natively and
/// silently fail to load in a browser — which is the one build that cannot be
/// debugged by looking at the filesystem.
pub struct CardMaterialPlugin;

impl Plugin for CardMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/card.wgsl");
        app.add_plugins(MaterialPlugin::<CardMaterial>::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shader reads three bits; the engine numbers more than a hundred
    /// keywords and generates that numbering. A card glowing for the wrong
    /// keyword would be a rules lie a player would believe.
    #[test]
    fn the_glow_bits_are_the_keywords_they_claim_to_be() {
        use baylee_cards_dsl::KeywordSet;
        assert_eq!(
            glow_bits(KeywordSet::INDESTRUCTIBLE.bits()),
            glow::INDESTRUCTIBLE
        );
        assert_eq!(glow_bits(KeywordSet::HEXPROOF.bits()), glow::HEXPROOF);
        assert_eq!(glow_bits(KeywordSet::SHROUD.bits()), glow::SHROUD);
        // And nothing else lights the border up.
        assert_eq!(glow_bits(KeywordSet::FLYING.bits()), 0);
        assert_eq!(glow_bits(0), 0);
    }

    /// Two keywords on one card are one border with both bits, not two draws.
    #[test]
    fn a_card_can_wear_more_than_one_glow() {
        use baylee_cards_dsl::KeywordSet;
        let both = KeywordSet::INDESTRUCTIBLE
            .union(KeywordSet::HEXPROOF)
            .bits();
        assert_eq!(
            glow_bits(both),
            glow::INDESTRUCTIBLE | glow::HEXPROOF,
            "an indestructible hexproof creature wears both"
        );
    }

    /// The material key has to separate what the shader draws differently and
    /// nothing else, or a board of forty Islands stops being one material.
    #[test]
    fn a_look_is_shared_by_exactly_what_looks_the_same() {
        use baylee_client_core::images::{ArtSize, Face};
        use baylee_core::ids::PrintRef;

        let key = ImageKey {
            print: PrintRef(0),
            face: Face::Front,
            size: ArtSize::Normal,
        };
        let plain = CardLook::art(key, FinishTreatment::Plain, 0);
        assert_eq!(plain, CardLook::art(key, FinishTreatment::Plain, 0));
        assert_ne!(
            plain,
            CardLook::art(key, FinishTreatment::Foil, 0),
            "a foil is not the same surface as a plain card"
        );
        assert_ne!(
            plain,
            CardLook::art(key, FinishTreatment::Plain, glow::INDESTRUCTIBLE),
            "and neither is one the rules have made indestructible"
        );
    }

    /// Colours that a player could not tell apart must not cost two
    /// materials.
    #[test]
    fn two_colours_a_player_cannot_tell_apart_share_a_material() {
        let a = Color::srgb(0.5, 0.25, 0.125);
        let b = Color::srgb(0.5 + 1.0 / 2048.0, 0.25, 0.125);
        assert_eq!(
            CardLook::flat(a, FinishTreatment::Plain, 0),
            CardLook::flat(b, FinishTreatment::Plain, 0)
        );
        assert_ne!(
            CardLook::flat(a, FinishTreatment::Plain, 0),
            CardLook::flat(Color::srgb(0.1, 0.2, 0.3), FinishTreatment::Plain, 0)
        );
    }
    /// The shader itself, parsed and validated.
    ///
    /// A WGSL error is otherwise found when a real pipeline is built — which
    /// on the web is the one environment that cannot be debugged by looking
    /// at a filesystem, and on native is a log line in a window that has
    /// already drawn a black table. Naga is the same front end wgpu uses, so
    /// what passes here compiles there.
    ///
    /// The two things naga cannot see are stripped first: `#import` lines,
    /// which `naga_oil` resolves against bevy's own modules, and
    /// `#{MATERIAL_BIND_GROUP}`, which the pipeline substitutes. Everything
    /// they bring in is stubbed below with the same shapes bevy declares, so
    /// a use that would not type-check against the real ones does not
    /// type-check here either.
    #[test]
    fn the_card_shader_compiles() {
        const SOURCE: &str = include_str!("shaders/card.wgsl");

        // Stand-ins for what the imports bring in. Same field names and same
        // types as `bevy_pbr::forward_io::VertexOutput` and the parts of the
        // view bindings the shader reads.
        let prelude = "\
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};
struct View { world_position: vec3<f32> };
struct Globals { time: f32 };
@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> globals: Globals;
";
        let body: String = SOURCE
            .lines()
            .filter(|line| !line.trim_start().starts_with("#import"))
            .collect::<Vec<_>>()
            .join("\n")
            .replace("#{MATERIAL_BIND_GROUP}", "3");
        let source = format!("{prelude}{body}");

        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|e| panic!("card.wgsl does not parse:\n{}", e.emit_to_string(&source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("card.wgsl does not validate: {e:?}"));
    }
}
