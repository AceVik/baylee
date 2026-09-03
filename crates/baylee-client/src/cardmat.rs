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

/// What a card's border is saying, as the shader's bitset.
///
/// Deliberately not the engine's keyword numbering: the shader reads a
/// handful of bits and the engine has more than a hundred keywords, so
/// translating once here is cheaper than sending a `u128` to the GPU, and it
/// makes adding a glow a one-line change on both sides.
///
/// Two different kinds of claim ride in the same word, and the shader draws
/// them differently on purpose. The keyword bits are facts about the card —
/// steady sheaths, the card *is* that. [`ACTIVATABLE`] is this client saying
/// "you could do something here", which is an offer, and reads as a moving
/// light rather than a material.
pub mod glow {
    /// Indestructible — darksteel.
    pub const INDESTRUCTIBLE: u32 = 1;
    /// Hexproof.
    pub const HEXPROOF: u32 = 2;
    /// Shroud.
    pub const SHROUD: u32 = 4;
    /// Something on this permanent can be activated right now.
    ///
    /// Not a keyword, so it is not in `KEYWORD_BITS`: it comes from
    /// `LegalActions`, changes with priority, and would be a rules lie if it
    /// were ever mistaken for a printed ability.
    pub const ACTIVATABLE: u32 = 8;
    /// This creature came under its controller's command too recently to
    /// attack or to tap (CR 302.6).
    ///
    /// Also not a keyword: it is a fact about *this turn*, not about the
    /// card, which is why the shader draws it over the card's face rather
    /// than on its border. The border says what a card is; the face says
    /// what it can do.
    pub const SUMMONING_SICK: u32 = 16;
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
///
/// Shroud swallows hexproof on the way through, because that is what the two
/// keywords do to each other: a permanent with both may be targeted by
/// nobody, including its controller, which is precisely shroud. Drawing them
/// as two sheaths would say the card is protected in two ways when it is
/// protected in one, and would cost a second material for a border no player
/// could tell from the first.
#[must_use]
pub fn glow_bits(keywords: u128) -> u32 {
    let mut bits = 0;
    for (keyword, flag) in KEYWORD_BITS {
        if keywords & (1u128 << keyword) != 0 {
            bits |= flag;
        }
    }
    if bits & glow::SHROUD != 0 {
        bits &= !glow::HEXPROOF;
    }
    bits
}

/// Everything a permanent's surface is saying about it, in one word.
///
/// Three different kinds of claim ride here and the shader draws each in its
/// own place: the keywords are what the rules have *made* the card (the
/// border), summoning sickness is what it cannot do *this turn* (a veil over
/// the face), and `activatable` is what this client is *offering* (a light
/// running round the edge). They are gathered in one function so that no
/// caller can assemble a different subset than another — a card in hand, in
/// the overlay and on the table must agree about what it is.
///
/// Sickness is asked of creatures only. The view reports it for every
/// permanent that entered this turn, but only a creature is stopped by it
/// (CR 302.6); a land played this turn taps perfectly well, and a board where
/// every fresh permanent breathed would be teaching the player something
/// false.
#[must_use]
pub fn glow_of(object: Option<&baylee_view::PublicObject>, activatable: bool) -> u32 {
    let from_card = object.map_or(0, |o| {
        let sick = o.summoning_sick && o.types.contains(baylee_core::types::TypeSet::CREATURE);
        glow_bits(o.keywords) | if sick { glow::SUMMONING_SICK } else { 0 }
    });
    from_card | if activatable { glow::ACTIVATABLE } else { 0 }
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

/// The same surface, for a card drawn as a UI node.
///
/// The hand, the preview and the printing picker are 2D, and a foil a player
/// is holding has to look like the foil that will land on the table — so this
/// carries the identical [`CardParams`] and its shader is the table shader's
/// twin. The one difference it cannot avoid is that a UI node has no world
/// position and no normal, so the sheen runs on time rather than answering
/// the camera.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct CardUiMaterial {
    /// The printed face, or a 1×1 white pixel when the card draws flat.
    #[texture(0)]
    #[sampler(1)]
    pub art: Option<Handle<Image>>,
    /// Everything else.
    #[uniform(2)]
    pub params: CardParams,
}

impl UiMaterial for CardUiMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/card_ui.wgsl".into()
    }
}

/// UI card materials, shared on the same key as the table's.
///
/// The overlay is rebuilt whenever the game state moves, and a material minted
/// per rebuild would grow `Assets` for as long as the duel lasted. Sharing on
/// [`CardLook`] makes a hand of seven cards at most seven materials, however
/// many times it is redrawn.
#[derive(Resource, Default)]
pub struct UiCardMaterials {
    made: bevy::platform::collections::HashMap<CardLook, Handle<CardUiMaterial>>,
    /// Cards that are not in a game: the deck builder's printing picker,
    /// keyed by CDN url and finish because it has no `PrintRef` to key on.
    previewed:
        bevy::platform::collections::HashMap<(String, FinishTreatment), Handle<CardUiMaterial>>,
}

impl UiCardMaterials {
    /// The material for a look, made once.
    pub fn get(
        &mut self,
        look: CardLook,
        art: Option<Handle<Image>>,
        tint: Color,
        assets: &mut Assets<CardUiMaterial>,
    ) -> Handle<CardUiMaterial> {
        if let Some(handle) = self.made.get(&look) {
            return handle.clone();
        }
        let made = material(look, art, tint);
        let handle = assets.add(CardUiMaterial {
            art: made.art,
            params: made.params,
        });
        self.made.insert(look, handle.clone());
        handle
    }

    /// Forgets every material, when the duel closes.
    ///
    /// Held handles are what keeps the assets alive, so dropping them here is
    /// what actually frees them; a duel that ended must not leave a hand's
    /// worth of materials behind for the next one.
    pub fn clear(&mut self) {
        self.made.clear();
        self.previewed.clear();
    }

    /// The material for a card that is not in any game.
    ///
    /// The deck builder's printing picker shows cardboard a player is
    /// choosing between, so it has no `PrintRef` and no print table to look
    /// one up in — it has a CDN URL and the finish the dialog is offering.
    /// Keyed on exactly those two, because that is what makes two previews
    /// the same picture.
    pub fn preview(
        &mut self,
        url: &str,
        finish: FinishTreatment,
        art: Handle<Image>,
        assets: &mut Assets<CardUiMaterial>,
    ) -> Handle<CardUiMaterial> {
        let key = (url.to_string(), finish);
        if let Some(handle) = self.previewed.get(&key) {
            return handle.clone();
        }
        let handle = assets.add(CardUiMaterial {
            art: Some(art),
            params: CardParams {
                finish: finish_code(finish),
                glow: 0,
                has_art: 1.0,
                strength: 1.0,
                tint: Vec4::ONE,
            },
        });
        self.previewed.insert(key, handle.clone());
        handle
    }
}

/// The two things making a UI card material needs: the shared cache, and the
/// asset store to mint into.
///
/// They always travel together, so they travel as one — and as *one optional*
/// argument, which is the point: a headless app has no render plugins and
/// therefore no `Assets<CardUiMaterial>`, and every drawing function falls
/// back to a plain image rather than growing a second code path.
pub struct UiCards<'a> {
    /// Materials already made.
    pub cache: &'a mut UiCardMaterials,
    /// Where new ones go.
    pub assets: &'a mut Assets<CardUiMaterial>,
}

impl UiCards<'_> {
    /// The material for a look, made once.
    pub fn get(&mut self, look: CardLook, art: Option<Handle<Image>>) -> Handle<CardUiMaterial> {
        self.cache.get(look, art, Color::WHITE, self.assets)
    }

    /// The material for a card that is not in any game.
    pub fn preview(
        &mut self,
        url: &str,
        finish: FinishTreatment,
        art: Handle<Image>,
    ) -> Handle<CardUiMaterial> {
        self.cache.preview(url, finish, art, self.assets)
    }
}

/// The finish of a printing, as the seat is entitled to see it.
///
/// The print table is per seat: a printing this seat has not earned resolves
/// to `None` and reads as plain. That is the whole reason the finish is
/// looked up here rather than carried on the card — a foil is a property of
/// the piece of cardboard, and the piece of cardboard is exactly the thing
/// `GameStatic.prints` withholds.
#[must_use]
pub fn finish_of(statics: &baylee_view::GameStatic, art: Option<ImageKey>) -> FinishTreatment {
    art.and_then(|key| statics.print(key.print))
        .map_or(FinishTreatment::Plain, |entry| entry.finish.into())
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

    /// A card showing a flat colour: its constructed face, or an empty slot.
    #[must_use]
    pub fn flat(color: Color, finish: FinishTreatment, glow: u32) -> Self {
        Self {
            art: None,
            finish,
            glow,
            tint: quantise(color),
        }
    }

    /// A card showing the back.
    ///
    /// No `ImageKey`, because the back is one texture the client owns rather
    /// than a printing it fetched — and no tint, which is what separates it
    /// from [`CardLook::flat`]: both have no `ImageKey`, and a back that
    /// collided with a constructed face would draw one as the other.
    #[must_use]
    pub fn back(finish: FinishTreatment, glow: u32) -> Self {
        Self {
            art: None,
            finish,
            glow,
            tint: 0,
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
///
/// `has_art` follows the *handle*, not the key: the card back is a real
/// texture the client owns rather than a printing it fetched, so it has no
/// `ImageKey` and still has to be sampled rather than replaced by a tint.
#[must_use]
pub fn material(look: CardLook, art: Option<Handle<Image>>, tint: Color) -> CardMaterial {
    let has_art = if art.is_some() { 1.0 } else { 0.0 };
    CardMaterial {
        art,
        params: CardParams {
            finish: finish_code(look.finish),
            glow: look.glow,
            has_art,
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
        embedded_asset!(app, "shaders/card_ui.wgsl");
        app.add_plugins(MaterialPlugin::<CardMaterial>::default())
            .add_plugins(UiMaterialPlugin::<CardUiMaterial>::default())
            .init_resource::<UiCardMaterials>();
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A permanent of a given type, with nothing else on it.
    ///
    /// Spelled out here rather than borrowed from `client-core`'s builders,
    /// which are `pub(crate)` there: what these tests need is three fields,
    /// and a view is not what they are about.
    fn permanent(types: baylee_core::types::TypeSet) -> baylee_view::PublicObject {
        use baylee_core::color::ColorSet;
        use baylee_core::ids::{ObjectId, PlayerId};
        use baylee_core::types::{SubtypeSet, SupertypeSet};
        baylee_view::PublicObject {
            mana_value: 0,
            id: ObjectId::new(1, 0),
            card: None,
            name: "Test".to_string(),
            controller: PlayerId::new(0),
            owner: PlayerId::new(0),
            status: baylee_view::ObjectStatus::NONE,
            types,
            supertypes: SupertypeSet::EMPTY,
            subtypes: SubtypeSet::EMPTY,
            token: None,
            colors: ColorSet::default(),
            keywords: 0,
            power: Some(2),
            toughness: Some(2),
            loyalty: None,
            damage: 0,
            counters: Vec::new(),
            attached_to: None,
            targets: Vec::new(),
            stack_item: None,
            summoning_sick: false,
        }
    }

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

    /// Shroud and hexproof on one card is shroud: nobody may target it, its
    /// controller included, which is exactly what shroud says. Two sheaths
    /// would claim two protections where there is one — and would cost a
    /// second material for a border no player could tell from the first.
    #[test]
    fn shroud_swallows_hexproof() {
        use baylee_cards_dsl::KeywordSet;
        let both = KeywordSet::SHROUD.union(KeywordSet::HEXPROOF).bits();
        assert_eq!(glow_bits(both), glow::SHROUD);
        // And it takes nothing else with it: an indestructible shrouded
        // creature is still made of metal.
        let all = KeywordSet::SHROUD
            .union(KeywordSet::HEXPROOF)
            .union(KeywordSet::INDESTRUCTIBLE)
            .bits();
        assert_eq!(glow_bits(all), glow::SHROUD | glow::INDESTRUCTIBLE);
    }

    /// Sickness is drawn for creatures and nothing else. The view reports it
    /// for every permanent that entered this turn, but only a creature is
    /// stopped by it (CR 302.6) — a land played this turn taps perfectly
    /// well, and a board where every fresh permanent breathed would be
    /// teaching a player something false.
    #[test]
    fn only_a_creature_is_drawn_asleep() {
        use baylee_core::types::TypeSet;
        let mut creature = permanent(TypeSet::CREATURE);
        creature.summoning_sick = true;
        assert_eq!(glow_of(Some(&creature), false), glow::SUMMONING_SICK);

        let mut land = permanent(TypeSet::LAND);
        land.summoning_sick = true;
        assert_eq!(glow_of(Some(&land), false), 0);
        assert_eq!(
            glow_of(Some(&land), true),
            glow::ACTIVATABLE,
            "a land that entered this turn still offers its mana ability"
        );
    }

    /// The three claims are three bits, and a card that is all three wears
    /// all three: they are drawn in three different places on purpose.
    #[test]
    fn a_card_can_be_protected_asleep_and_useful_at_once() {
        use baylee_cards_dsl::KeywordSet;
        use baylee_core::types::TypeSet;
        let mut obj = permanent(TypeSet::CREATURE);
        obj.summoning_sick = true;
        obj.keywords = KeywordSet::HEXPROOF
            .union(KeywordSet::INDESTRUCTIBLE)
            .bits();
        assert_eq!(
            glow_of(Some(&obj), true),
            glow::HEXPROOF | glow::INDESTRUCTIBLE | glow::SUMMONING_SICK | glow::ACTIVATABLE
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
    /// Parses and validates a shader the way `wgpu` will.
    ///
    /// The two things naga cannot see are stripped first: `#import` lines,
    /// which `naga_oil` resolves against bevy's own modules, and
    /// `#{MATERIAL_BIND_GROUP}`, which the pipeline substitutes. What they
    /// bring in is stubbed by the caller with the same shapes bevy declares,
    /// so a use that would not type-check against the real ones does not
    /// type-check here either.
    pub(crate) fn check_wgsl(source: &str, prelude: &str) {
        let body: String = source
            .lines()
            .filter(|line| !line.trim_start().starts_with("#import"))
            .collect::<Vec<_>>()
            .join("\n")
            .replace("#{MATERIAL_BIND_GROUP}", "3");
        let full = format!("{prelude}{body}");

        let module = naga::front::wgsl::parse_str(&full)
            .unwrap_or_else(|e| panic!("does not parse:\n{}", e.emit_to_string(&full)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("does not validate: {e:?}"));
    }

    /// The table shader, parsed and validated.
    ///
    /// A WGSL error is otherwise found when a real pipeline is built — which
    /// on the web is the one environment that cannot be debugged by looking
    /// at a filesystem, and on native is a log line in a window that has
    /// already drawn a black table. Naga is the same front end wgpu uses, so
    /// what passes here compiles there.
    #[test]
    fn the_card_shader_compiles() {
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
@group(0) @binding(11) var<uniform> globals: Globals;
";
        check_wgsl(include_str!("shaders/card.wgsl"), prelude);
    }
    /// The UI twin, held to the same standard. Its bind group is group 1 —
    /// `bevy_ui_render` puts the view layout first — and it reads `globals`
    /// out of group 0, so the stubs are shaped for that.
    #[test]
    fn the_ui_card_shader_compiles() {
        let prelude = "\
struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
    @location(2) border_radius: vec4<f32>,
    @location(3) @interpolate(flat) size: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
struct Globals { time: f32 };
";
        check_wgsl(include_str!("shaders/card_ui.wgsl"), prelude);
    }
}
