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

use baylee_client_core::cardrail;
use baylee_client_core::images::{FinishTreatment, ImageKey};
use baylee_core::ids::ObjectId;
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
    /// An armed deed is waiting on this card: the tap has been made, and one
    /// more sends it.
    ///
    /// Deliberately not drawn like [`ACTIVATABLE`], which is the light beside
    /// it in the same register: that one travels, because it is an invitation
    /// and the eye should find it across a whole board. This one holds still,
    /// because it is a commitment. A player has to be able to tell "you could"
    /// from "you are about to" at a glance — only one of the two is undone by
    /// looking away.
    pub const ARMED: u32 = 32;
    /// An armed mana run would tap this permanent to pay for its deed.
    ///
    /// The other half of the same statement: the card says what will happen,
    /// its sources say what it will cost. Drawn rather than written because
    /// "Tap 3, then cast" does not say *which* three, and which three is a
    /// plan the player never made and would otherwise have to trust blind.
    pub const WILL_TAP: u32 = 64;

    /// Where the keyword rail's eleven marks begin in the word.
    ///
    /// The rail is a *field* and not eleven more flags, because the shader
    /// has to walk it: which mark a fragment is inside is the k-th set bit,
    /// found in one loop bound at compile time. Slot order is
    /// `baylee_client_core::cardrail::MARK_ORDER`, and nothing on the GPU
    /// side ever sees the engine's keyword numbering.
    pub const MARK_SHIFT: u32 = 8;

    /// The eleven mark bits, in place.
    pub const MARK_MASK: u32 = 0x7ff << MARK_SHIFT;
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

/// Translates the view's keyword bitset into what the shader draws.
///
/// Two different things come out. The band bits are the three keywords the
/// border is a *material* for; the rail field is the eleven the card wears as
/// marks along its bottom edge. Which keyword goes where is not a matter of
/// taste: a material composes with at most one other material before it says
/// neither thing, and a creature can carry six combat keywords at once, so
/// those have to be countable rather than mixed.
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
    // The slot is counted in `u32` from the shift, rather than as an
    // `enumerate()` index cast to one, so this function has no panic in it at
    // all: eleven slots cannot overflow, but saying so with an `expect` would
    // put a panic in the path every card on the board takes every frame.
    for (slot, badge) in (glow::MARK_SHIFT..).zip(cardrail::MARK_ORDER) {
        if keywords & badge.bit() != 0 {
            bits |= 1 << slot;
        }
    }
    bits
}

/// What this client is offering to do with one card, right now.
///
/// Three claims about the *client's own state* rather than about the card, and
/// they travel together rather than as three arguments for the same reason
/// [`glow_of`] exists at all: they change with priority and with a tap, and a
/// caller that passed two of the three would have the same card saying two
/// different things in the hand and on the table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Offer {
    /// Something on this permanent can be activated right now.
    pub activatable: bool,
    /// This is the card [`crate::Duel::armed`] is holding.
    pub armed: bool,
    /// An armed mana run would tap this permanent to pay for its deed.
    pub will_tap: bool,
}

impl Offer {
    /// Nothing on offer, which is the whole board while this seat is not the
    /// one being asked.
    pub const NONE: Self = Self {
        activatable: false,
        armed: false,
        will_tap: false,
    };

    /// Only the offer the engine itself made — what a card outside the
    /// battlefield can ever have.
    #[must_use]
    pub const fn activatable(activatable: bool) -> Self {
        Self {
            activatable,
            armed: false,
            will_tap: false,
        }
    }

    /// What one drawn card is being offered, given what this client has
    /// armed.
    ///
    /// One reader for the three surfaces that draw a card — the table, the
    /// hand bar and the own-board overlay — for the same reason [`glow_of`]
    /// is one function: they draw the same cards, and an armed spell that lit
    /// up in the hand but not on the table would be worse than not drawing it
    /// at all.
    ///
    /// `members` is every permanent the drawn card stands for
    /// (`CardGroup::members`), which is one id for a card in hand and may be
    /// four for a stack of Forests. Both answers here are **any**, where
    /// `CardGroup::activatable` is deliberately *all*, and the difference is
    /// not an inconsistency: `activatable` invites a click, so a stack where
    /// only one member could act would be inviting one that gets refused.
    /// These two invite nothing — they announce what an armed deed is and
    /// what it will spend — and a stack of three Forests two of which are
    /// about to tap is better drawn lit than dark.
    ///
    /// The plan is walked rather than indexed. It holds at most a handful of
    /// steps — a spell nobody can pay for has no plan at all — and a set
    /// built per frame to answer six questions costs more than the answers.
    #[must_use]
    pub fn on(armed: Option<&crate::Armed>, members: &[ObjectId], activatable: bool) -> Self {
        let Some(armed) = armed else {
            return Self::activatable(activatable);
        };
        Self {
            activatable,
            armed: members.contains(&armed.object),
            will_tap: match &armed.deed {
                crate::Deed::Run(plan) => {
                    plan.steps.iter().any(|step| members.contains(&step.source))
                }
                crate::Deed::Play | crate::Deed::Ability(_) => false,
            },
        }
    }
}

/// Everything a permanent's surface is saying about it, in one word.
///
/// Three different kinds of claim ride here and the shader draws each in its
/// own place: the keywords are what the rules have *made* the card (the
/// border), summoning sickness is what it cannot do *this turn* (a veil over
/// the face), and the [`Offer`] is what this client is proposing (the lights
/// in the border's outer register). They are gathered in one function so that
/// no caller can assemble a different subset than another — a card in hand, in
/// the overlay and on the table must agree about what it is.
///
/// Sickness is asked of creatures only. The view reports it for every
/// permanent that entered this turn, but only a creature is stopped by it
/// (CR 302.6); a land played this turn taps perfectly well, and a board where
/// every fresh permanent breathed would be teaching the player something
/// false.
#[must_use]
pub fn glow_of(object: Option<&baylee_view::PublicObject>, offer: Offer) -> u32 {
    let from_card = object.map_or(0, |o| {
        let sick = o.summoning_sick && o.types.contains(baylee_core::types::TypeSet::CREATURE);
        glow_bits(o.keywords) | if sick { glow::SUMMONING_SICK } else { 0 }
    });
    // An armed card is not also inviting a tap: the invitation was accepted,
    // and drawing both would put a travelling light and a steady one on the
    // same border saying the same thing twice.
    let offered = if offer.armed {
        glow::ARMED
    } else if offer.activatable {
        glow::ACTIVATABLE
    } else {
        0
    };
    from_card | offered | if offer.will_tap { glow::WILL_TAP } else { 0 }
}

/// What the shader needs to know about one card.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct CardParams {
    /// 0 plain, 1 foil, 2 etched.
    pub finish: u32,
    /// Keyword glows, from [`glow_bits`].
    pub glow: u32,
    /// What the reserved bottom-right corner says, packed by
    /// [`baylee_client_core::cardplate::Plate::packed`]: a creature's power,
    /// toughness and damage, or a planeswalker's loyalty.
    pub plate: u32,
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
                plate: 0,
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
    /// What its corner plate says, packed. Part of the key because it is part
    /// of the material: two 2/2s share a plate and a material, a 2/2 and a
    /// 3/3 share neither.
    pub plate: u32,
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
            plate: 0,
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
            plate: 0,
            tint: quantise(color),
        }
    }

    /// The same look with a corner plate on it.
    ///
    /// A builder rather than a sixth argument on all three constructors: a
    /// card in hand, a card in a browser and a card in the printing picker
    /// have no body to show, and only the two board surfaces ever call this.
    #[must_use]
    pub const fn with_plate(mut self, plate: u32) -> Self {
        self.plate = plate;
        self
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
            plate: 0,
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
            plate: look.plate,
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
        // The rail both of them import. Registered here rather than loaded on
        // demand because it is not a shader in its own right: nothing sets it
        // on a pipeline, and the two that import it name it by this path.
        embedded_asset!(app, "shaders/card_common.wgsl");
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

    /// The border speaks for three keywords and the rail for eleven more;
    /// the engine numbers over a hundred and generates that numbering. A card
    /// glowing for the wrong keyword would be a rules lie a player would
    /// believe.
    #[test]
    fn the_glow_bits_are_the_keywords_they_claim_to_be() {
        use baylee_cards_dsl::KeywordSet;
        assert_eq!(
            glow_bits(KeywordSet::INDESTRUCTIBLE.bits()),
            glow::INDESTRUCTIBLE
        );
        assert_eq!(glow_bits(KeywordSet::HEXPROOF.bits()), glow::HEXPROOF);
        assert_eq!(glow_bits(KeywordSet::SHROUD.bits()), glow::SHROUD);
        // And nothing else lights the *border* up: flying is a mark on the
        // rail, and a keyword that turned the border green would be claiming
        // a protection the card does not have.
        assert_eq!(glow_bits(KeywordSet::FLYING.bits()) & !glow::MARK_MASK, 0);
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
        assert_eq!(glow_of(Some(&creature), Offer::NONE), glow::SUMMONING_SICK);

        let mut land = permanent(TypeSet::LAND);
        land.summoning_sick = true;
        assert_eq!(glow_of(Some(&land), Offer::NONE), 0);
        assert_eq!(
            glow_of(Some(&land), Offer::activatable(true)),
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
            glow_of(Some(&obj), Offer::activatable(true)),
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

    /// The value of a `const` declared in a WGSL file.
    ///
    /// Reading the shader's own text is the only way these constants can be
    /// checked at all: one side is a Rust `const` the compiler knows about,
    /// the other is a number in a `.wgsl` file that nothing but the GPU ever
    /// parses. A mismatch is silent, and looks like a rail drawn in the wrong
    /// place on a machine nobody is testing on.
    pub(crate) fn wgsl_const(source: &str, name: &str) -> f32 {
        let head = format!("const {name}:");
        let line = source
            .lines()
            .map(str::trim_start)
            .find(|line| line.starts_with(&head))
            .unwrap_or_else(|| panic!("no `{name}` in the shader"));
        let value = line
            .rsplit_once('=')
            .and_then(|(_, rhs)| rhs.trim().strip_suffix(';'))
            .unwrap_or_else(|| panic!("cannot read `{line}`"));
        // A ratio stays written as one — `63.0 / 88.0` says what it is and
        // `0.7159091` does not — so the one operator that appears in these
        // constants is evaluated here rather than banned from the shader.
        let number = |text: &str| -> f32 {
            let text = text.trim().trim_end_matches('u');
            // A mask is written as a mask — `0x7ff` says eleven bits and
            // `2047` says nothing — so the one other literal form these
            // constants use is read here too.
            if let Some(digits) = text.strip_prefix("0x") {
                return u32::from_str_radix(digits, 16)
                    .unwrap_or_else(|_| panic!("`{text}` is not a number"))
                    as f32;
            }
            text.parse()
                .unwrap_or_else(|_| panic!("`{text}` is not a number"))
        };
        match value.split_once('/') {
            Some((num, den)) => number(num) / number(den),
            None => number(value),
        }
    }

    /// The rail is laid out twice — once in Rust so the pointer can hit-test
    /// a mark, once in WGSL so the GPU can draw one — and the two have to be
    /// the same rail. Nothing in either compiler can notice that they are.
    #[test]
    fn the_rail_is_in_the_same_place_in_both_languages() {
        let src = include_str!("shaders/card_common.wgsl");
        for (name, ours) in [
            ("RAIL_INSET", cardrail::RAIL_INSET),
            ("RAIL_SLOT", cardrail::RAIL_SLOT),
            ("RAIL_SPAN", cardrail::RAIL_SPAN),
            ("CARD_ASPECT", cardrail::CARD_ASPECT),
        ] {
            let theirs = wgsl_const(src, name);
            assert!(
                (ours - theirs).abs() < 1e-6,
                "{name}: {ours} here, {theirs} in the shader"
            );
        }
        assert_eq!(
            wgsl_const(src, "MARK_COUNT") as usize,
            cardrail::MARK_ORDER.len(),
            "the shader draws a different number of marks than the rail has"
        );
        assert!(
            (wgsl_const(src, "MARK_SHIFT") - glow::MARK_SHIFT as f32).abs() < f32::EPSILON,
            "the marks are shifted differently on the two sides"
        );
        // The shift alone is not enough: a mask one bit short would drop the
        // eleventh keyword silently, and defender is the eleventh.
        assert!(
            (wgsl_const(src, "MARK_FIELD") - (glow::MARK_MASK >> glow::MARK_SHIFT) as f32).abs()
                < f32::EPSILON,
            "the shader reads a different number of mark bits than the mask holds"
        );
    }

    /// Every flag below the rail is the same number on both sides, in *both*
    /// shaders.
    ///
    /// Three copies of the same table — one Rust, two WGSL — and nothing in
    /// either compiler can notice when one of them moves. A wrong number here
    /// has no error and no crash: the card in the hand draws one thing and
    /// the same card on the table draws another, or a bit lands in the rail's
    /// field and a permanent grows a keyword mark it does not have.
    #[test]
    fn the_glow_flags_are_the_same_number_in_all_three_files() {
        let table = include_str!("shaders/card.wgsl");
        let ui = include_str!("shaders/card_ui.wgsl");
        for (name, ours) in [
            ("GLOW_INDESTRUCTIBLE", glow::INDESTRUCTIBLE),
            ("GLOW_HEXPROOF", glow::HEXPROOF),
            ("GLOW_SHROUD", glow::SHROUD),
            ("GLOW_ACTIVATABLE", glow::ACTIVATABLE),
            ("GLOW_SUMMONING_SICK", glow::SUMMONING_SICK),
            ("GLOW_ARMED", glow::ARMED),
            ("GLOW_WILL_TAP", glow::WILL_TAP),
        ] {
            for (which, src) in [("card.wgsl", table), ("card_ui.wgsl", ui)] {
                let theirs = wgsl_const(src, name);
                assert!(
                    (theirs - ours as f32).abs() < f32::EPSILON,
                    "{name}: {ours} here, {theirs} in {which}"
                );
            }
            // And none of them may reach into the rail, which would draw a
            // keyword mark for something that is not a keyword.
            assert_eq!(ours & glow::MARK_MASK, 0, "{name} overlaps the rail");
        }
    }

    /// An armed card is not also inviting a tap.
    ///
    /// Both lights live in the same register on the border, and the whole
    /// point of the pair is that one travels and one holds still. Drawing
    /// both would put a chase and a steady ring on the same edge saying the
    /// same thing twice, and a player would have nothing left to read the
    /// difference from.
    #[test]
    fn arming_a_card_replaces_the_offer_it_accepted() {
        use baylee_core::types::TypeSet;
        let obj = permanent(TypeSet::CREATURE);
        let offer = Offer {
            activatable: true,
            armed: true,
            will_tap: false,
        };
        assert_eq!(glow_of(Some(&obj), offer), glow::ARMED);
        assert_eq!(
            glow_of(Some(&obj), Offer::activatable(true)),
            glow::ACTIVATABLE
        );
        // The price is a separate claim and rides alongside either of them:
        // the land being spent is not the card being cast.
        let paying = Offer {
            activatable: true,
            armed: false,
            will_tap: true,
        };
        assert_eq!(
            glow_of(Some(&obj), paying),
            glow::ACTIVATABLE | glow::WILL_TAP
        );
    }

    /// A card standing for four permanents lights up when the deed touches
    /// *any* of them.
    ///
    /// The opposite of `CardGroup::activatable`, which is deliberately *all*,
    /// and for a reason that does not transfer: that one invites a click, so
    /// lighting a stack where only one member could act would invite a click
    /// that gets refused. These two invite nothing — they announce what is
    /// about to happen — and a stack of three Forests two of which are about
    /// to tap is better drawn lit than dark.
    #[test]
    fn a_stack_is_lit_by_whichever_of_it_the_deed_touches() {
        use baylee_client_core::manaplan::{Plan, Step, Tap};
        let forests: Vec<ObjectId> = (1..=3).map(|i| ObjectId::new(i, 0)).collect();
        let elsewhere = ObjectId::new(9, 0);

        let run = crate::Armed {
            object: elsewhere,
            deed: crate::Deed::Run(Plan {
                steps: vec![Step {
                    source: forests[1],
                    tap: Tap::Intrinsic,
                    color: None,
                }],
            }),
        };
        let offer = Offer::on(Some(&run), &forests, false);
        assert!(offer.will_tap, "one of the three is being spent");
        assert!(!offer.armed, "the spell is not one of the lands");

        // And a plan that touches none of them leaves the stack dark.
        let other = crate::Armed {
            object: elsewhere,
            deed: crate::Deed::Run(Plan {
                steps: vec![Step {
                    source: elsewhere,
                    tap: Tap::Intrinsic,
                    color: None,
                }],
            }),
        };
        assert_eq!(
            Offer::on(Some(&other), &forests, true),
            Offer::activatable(true)
        );

        // A deed that is not a run spends nothing, whatever it is aimed at.
        let play = crate::Armed {
            object: forests[0],
            deed: crate::Deed::Play,
        };
        let offer = Offer::on(Some(&play), &forests, false);
        assert!(offer.armed && !offer.will_tap);
    }

    /// Every mark the rail carries is the keyword it claims to be, and the
    /// three the border speaks for are not on it twice.
    #[test]
    fn the_rail_carries_the_keywords_it_says_it_does() {
        use baylee_cards_dsl::KeywordSet;
        let slot = |set: KeywordSet| glow_bits(set.bits()) >> glow::MARK_SHIFT;
        assert_eq!(slot(KeywordSet::FLYING), 1 << 0);
        assert_eq!(slot(KeywordSet::DEATHTOUCH), 1 << 3);
        assert_eq!(slot(KeywordSet::DEFENDER), 1 << 10);
        // The band's three keep the band and stay off the rail.
        assert_eq!(slot(KeywordSet::HEXPROOF), 0);
        assert_eq!(slot(KeywordSet::INDESTRUCTIBLE), 0);
        assert_eq!(slot(KeywordSet::SHROUD), 0);
        // And a creature wearing six of them is one word with six bits in it.
        let six = KeywordSet::FLYING
            .union(KeywordSet::TRAMPLE)
            .union(KeywordSet::LIFELINK)
            .union(KeywordSet::VIGILANCE)
            .union(KeywordSet::HASTE)
            .union(KeywordSet::MENACE);
        assert_eq!(
            glow_bits(six.bits()).count_ones(),
            6,
            "six keywords, six marks"
        );
        assert_eq!(glow_bits(six.bits()) & !glow::MARK_MASK, 0);
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
        check_wgsl(
            include_str!("shaders/card.wgsl"),
            &format!("{prelude}{}", include_str!("shaders/card_common.wgsl")),
        );
    }

    /// The plate is the same plate in Rust and in WGSL.
    ///
    /// Thirty numbers with no compiler between them, and every one of them
    /// fails silently: a slot boundary a bit out draws a 4/4 as a 0/16, a
    /// glyph word off by a copy-paste draws every 6 as an 8, and a geometry
    /// constant that drifts puts the plate over the keyword rail. The
    /// packing is checked from the other side by
    /// `cardplate::tests::every_number_survives_the_packing`; this is the
    /// half that checks the shader agrees about where the bits are.
    #[test]
    fn the_plate_is_the_same_plate_in_both_languages() {
        use baylee_client_core::cardplate as plate;
        let src = include_str!("shaders/card_common.wgsl");

        for (name, ours) in [
            ("PLATE_INSET", plate::PLATE_INSET),
            ("PLATE_W", plate::PLATE_W),
            ("PLATE_H", plate::PLATE_H),
            ("PLATE_PAD", plate::PLATE_PAD),
        ] {
            let theirs = wgsl_const(src, name);
            assert!(
                (theirs - ours).abs() < 1e-5,
                "{name}: {ours} here, {theirs} in the shader"
            );
        }

        for (name, ours) in [
            ("PLATE_KIND_SHIFT", plate::KIND_SHIFT),
            ("PLATE_SLOT_BITS", plate::SLOT_BITS),
            ("PLATE_SLOT_MASK", plate::SLOT_MASK),
            #[allow(clippy::cast_sign_loss)] // the bias is positive by construction
            ("PLATE_BIAS", plate::BIAS as u32),
            ("PLATE_NONE", plate::KIND_NONE),
            ("PLATE_FIGHT", plate::KIND_FIGHT),
            ("PLATE_LOYALTY", plate::KIND_LOYALTY),
            ("GLYPH_W", plate::GLYPH_W),
            ("GLYPH_H", plate::GLYPH_H),
        ] {
            // Half a unit, not an epsilon: these are whole numbers, so an
            // agreement is exactly zero apart and a disagreement is at least
            // one — and `f32::EPSILON` next to a seven-digit glyph word would
            // be asking for a precision no `f32` has up there.
            assert!(
                (wgsl_const(src, name) - ours as f32).abs() < 0.5,
                "{name} differs between the two files"
            );
        }

        // The twelve stencils. Every word is under 2^24, so an `f32` carries
        // it exactly and this comparison is not an approximation.
        for (i, name) in [
            "GLYPH_0",
            "GLYPH_1",
            "GLYPH_2",
            "GLYPH_3",
            "GLYPH_4",
            "GLYPH_5",
            "GLYPH_6",
            "GLYPH_7",
            "GLYPH_8",
            "GLYPH_9",
            "GLYPH_MINUS",
            "GLYPH_SLASH",
        ]
        .into_iter()
        .enumerate()
        {
            assert!(
                (wgsl_const(src, name) - plate::GLYPHS[i] as f32).abs() < 0.5,
                "{name} draws a different picture in the shader"
            );
        }

        // And the plate starts where the rail stops. Asserted against the
        // shader's own numbers rather than against Rust's, because these two
        // constants are what reserved the corner and they live in both files.
        let rail_end = wgsl_const(src, "RAIL_INSET") + wgsl_const(src, "RAIL_SPAN");
        let plate_start = 1.0 - wgsl_const(src, "PLATE_INSET") - wgsl_const(src, "PLATE_W");
        assert!(
            (rail_end - plate_start).abs() < 1e-5,
            "the rail ends at {rail_end} and the plate starts at {plate_start}"
        );
    }

    /// The rail's own file, which is plain WGSL and needs nothing stubbed.
    ///
    /// It is checked alone as well as inside the two that import it, because
    /// alone is how bevy compiles it: a module that failed only in isolation
    /// would take both card shaders down with it and blame whichever pipeline
    /// happened to be built first.
    #[test]
    fn the_marks_module_compiles() {
        check_wgsl(include_str!("shaders/card_common.wgsl"), "");
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
        check_wgsl(
            include_str!("shaders/card_ui.wgsl"),
            &format!("{prelude}{}", include_str!("shaders/card_common.wgsl")),
        );
    }
}
