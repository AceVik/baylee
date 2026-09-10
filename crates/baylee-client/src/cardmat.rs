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
//! # The browser's budget
//!
//! The browser build renders through WebGPU, and this shader is nonetheless
//! written to the older WebGL2 budget: uniforms only, no storage buffers, no
//! texture arrays. That is a choice rather than a constraint now, and it is
//! worth keeping while it costs nothing — a card is drawn from a handful of
//! numbers, so nothing here would be simpler with a storage buffer, and the
//! GL backend stays one feature away for a browser that has no WebGPU.
//! The animation reads `globals.time` from the view bind group, which means
//! nothing here is written per frame: a material is created once and never
//! touched again while it is on screen.

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
    /// This card is one of its owner's commanders (CR 903.3).
    ///
    /// The odd one out in this word, and drawn nowhere near the rest of it.
    /// The four bits above are offers and a fact about *this turn*; the three
    /// below are materials on the border. This is an identity — true in every
    /// zone, for the whole game, before the first turn and after the card has
    /// died four times — so it is drawn as a still crest on the card's face
    /// and never touches the border register at all.
    ///
    /// It rides this word anyway because the word is what reaches every
    /// surface: table, hand bar, tray, own-board overlay and hover preview all
    /// key one [`CardLook`] on it. A commander drawn on the table and plain in
    /// the overlay would be the same card disagreeing with itself.
    pub const COMMANDER: u32 = 128;

    /// Where the keyword rail's twelve marks begin in the word.
    ///
    /// The rail is a *field* and not twelve more flags, because the shader
    /// has to walk it: which mark a fragment is inside is the k-th set bit,
    /// found in one loop bound at compile time. Slot order is
    /// `baylee_client_core::cardrail::MARK_ORDER`, and nothing on the GPU
    /// side ever sees the engine's keyword numbering.
    pub const MARK_SHIFT: u32 = 8;

    /// The twelve mark bits, in place.
    pub const MARK_MASK: u32 = 0xfff << MARK_SHIFT;

    /// This permanent has no card under it at all: a token (CR 111.1).
    ///
    /// The first of the two provenance bits, and they are the first thing in
    /// this word *above* the rail's field rather than below it — the eight
    /// low bits were full, and a bit that landed in [`MARK_MASK`] would grow
    /// a keyword mark on every token on the table.
    ///
    /// Provenance is the [`COMMANDER`] question asked a second way: not what
    /// a card can do but what it *is*, true in every zone and for the whole
    /// game, so it is drawn on the face beside the crest and never in the
    /// border. `baylee_client_core::board::Provenance` is where the two are
    /// decided, in one place, which is what makes them exclusive.
    pub const TOKEN: u32 = 1 << 20;

    /// This permanent's own card is one thing and the face it is showing is
    /// another: a copy effect is at work (CR 707.2).
    ///
    /// Never set together with [`TOKEN`]. A token that a copy effect made is
    /// a token — the chit is the whole truth about it and there is no
    /// original to go and look at — and that is settled in the model rather
    /// than here.
    pub const COPY: u32 = 1 << 21;
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
                // Whichever end the run has: a land the plan spends is a land
                // the player has to see marked before the second click.
                crate::Deed::Run { plan, .. } => {
                    plan.steps.iter().any(|step| members.contains(&step.source))
                }
                crate::Deed::Play | crate::Deed::Ability(_) | crate::Deed::Suspend => false,
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
/// Sickness is asked of creatures only. The host projects CR 302.6 now and
/// so answers this for creatures alone, but the field once meant "did this
/// permanent enter this turn" — a land played this turn came back `true` —
/// and the shape of the view did not change with its meaning, so no
/// `VIEW_VERSION` bump refuses a host from before the fix. The type test is
/// what keeps such a host from putting a whole opening board to sleep, and
/// it costs one bit compare.
#[must_use]
pub fn glow_of(object: Option<&baylee_view::PublicObject>, offer: Offer) -> u32 {
    let from_card = object.map_or(0, |o| {
        let sick = o.summoning_sick && o.types.contains(baylee_core::types::TypeSet::CREATURE);
        glow_bits(o.keywords)
            | if sick { glow::SUMMONING_SICK } else { 0 }
            | if o.commander { glow::COMMANDER } else { 0 }
            | provenance_bit(o)
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

/// The provenance mark for one object, as its bit in the glow word.
///
/// The registry is reached from inside [`glow_of`] rather than handed to it,
/// which is the opposite of what `BoardModel::from_view` does one crate down —
/// and deliberately. That seam exists because `baylee-client-core` does not
/// link `baylee-cards`; this crate does, and every one of `glow_of`'s three
/// callers would otherwise pass the same closure to get the same answer, which
/// is three chances for a card in the hand bar to disagree with the same card
/// on the table.
///
/// `board::provenance_of` is still where the judgement is made. Nothing is
/// decided here.
fn provenance_bit(object: &baylee_view::PublicObject) -> u32 {
    match baylee_client_core::board::provenance_of(object, crate::cardart::registry()) {
        baylee_client_core::board::Provenance::Printed => 0,
        baylee_client_core::board::Provenance::Token => glow::TOKEN,
        baylee_client_core::board::Provenance::Copy => glow::COPY,
    }
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
    /// toughness and damage, a planeswalker's loyalty, or a saga's chapter.
    pub plate: u32,
    /// The first two counter chips above the plate, packed by
    /// [`baylee_client_core::cardplate::Corner::packed`].
    pub chips_a: u32,
    /// The third chip and the one that counts what did not fit.
    pub chips_b: u32,
    /// 1.0 when the material carries real artwork, 0.0 when the card is drawn
    /// as a flat `tint` — its constructed face, or its back.
    pub has_art: f32,
    /// How strongly the finish shows. One material can be dimmed without a
    /// second pipeline.
    pub strength: f32,
    /// The clock every animated term on this card runs on: [`MOVING`] or
    /// [`STILL`].
    ///
    /// A number rather than a second shader, and on the material rather than
    /// in [`CardLook`], which is the part worth stating. `CardLook` is the
    /// *cache key*: putting a global preference in it would make every card
    /// on the table two entries instead of one, and nothing evicts the half
    /// that is no longer wanted. So the caches hold the setting once, beside
    /// their contents rather than inside the key, and a change rewrites this
    /// field on the materials already made — nothing is discarded, because
    /// every card entity still holds the handle either way.
    pub motion: f32,
    /// When this card's one-shot sheen began, on the same clock the shaders'
    /// `globals.time` runs on.
    ///
    /// Absolute rather than a phase, which is what lets the material be
    /// written once and left alone: the shader subtracts and divides, and the
    /// band crosses the card without anything touching the uniform again.
    pub sweep_at: f32,
    /// One over how long that sheen takes, in seconds, or `0.0` for a card
    /// that is not sweeping. See [`crate::sheen`].
    pub sweep_rate: f32,
    /// Which of the five zone-change doors this sweep is drawing, or
    /// [`door::NONE`] for the plain arrival every card wears most of the
    /// time.
    ///
    /// A code rather than a colour, so the palette is one table in the shader
    /// beside the figures it goes with, rather than five colours travelling
    /// through Rust to be looked at once. The two halves are paired by a test
    /// that reads the WGSL, the way the rail's are.
    pub sweep_door: u32,
    /// The colour a card with no artwork is drawn in.
    pub tint: Vec4,
}

/// The five doors a permanent goes through, as the shader numbers them.
///
/// The names are [`baylee_client_core::zones::Passage`]'s; these are the
/// wire between it and `card_common.wgsl`, and
/// `the_doors_are_numbered_the_same_in_both_languages` is what keeps the two
/// lists from drifting.
pub mod door {
    /// No door: the plain arrival — drawn, cast, previewed.
    pub const NONE: u32 = 0;
    /// Battlefield to hand.
    pub const BOUNCE: u32 = 1;
    /// Battlefield to exile.
    pub const EXILED: u32 = 2;
    /// Exile to battlefield.
    pub const FLICKERED: u32 = 3;
    /// Battlefield to a graveyard.
    pub const DESTROYED: u32 = 4;
    /// A graveyard to the battlefield.
    pub const RETURNED: u32 = 5;

    /// The code for a door, or [`NONE`] for a card that came through none.
    #[must_use]
    pub fn code(door: Option<baylee_client_core::zones::Passage>) -> u32 {
        use baylee_client_core::zones::Passage;
        match door {
            None => NONE,
            Some(Passage::Bounce) => BOUNCE,
            Some(Passage::Exiled) => EXILED,
            Some(Passage::Flickered) => FLICKERED,
            Some(Passage::Destroyed) => DESTROYED,
            Some(Passage::Returned) => RETURNED,
        }
    }
}

/// Writes a sweep on to params that already exist.
///
/// Split out of [`material`] because a departing card is dressed the other
/// way round: it is not looked up by a [`CardLook`] at all — it has left the
/// board model and nothing will ever ask for its material again — so its
/// exit is written on to the material it is already wearing. One function, so
/// the decision a still card makes is made once.
pub fn wear(params: &mut CardParams, sweep: Option<crate::sheen::Sweep>, motion: f32) {
    // A card holding still does not sweep. Not "sweeps at phase zero", which
    // is where every other term is stopped: phase zero of this one is the
    // band sitting off the card's bottom right corner, so the honest still
    // frame of a one-shot travel is the travel not having happened.
    params.sweep_at = sweep.map_or(0.0, crate::sheen::Sweep::at);
    params.sweep_rate = match sweep {
        Some(s) if motion > 0.0 => s.rate(),
        _ => 0.0,
    };
    params.sweep_door = match sweep {
        Some(s) if motion > 0.0 => door::code(s.door()),
        _ => door::NONE,
    };
}

/// A card whose animations run.
pub const MOVING: f32 = 1.0;

/// A card holding still, for [`Preferences::reduce_motion`].
///
/// Zero, and the shaders are written so that zero lands every term it stops
/// somewhere that term could have been. For a pure `a + b·sin(t·ω)` that
/// place is the mean; for a term carrying a *spatial* phase as well — the
/// indestructible border's `uv.y`, the rail's per-slot offset — it is an
/// honest frame of the animation rather than its average, which is equally
/// what is wanted. A still card is the moving one held still, not a different
/// drawing. The three terms where phase zero is neither are handled where
/// they appear.
///
/// [`Preferences::reduce_motion`]: baylee_client_core::prefs::Preferences::reduce_motion
pub const STILL: f32 = 0.0;

/// The clock a card animates on, from the preference.
#[must_use]
pub const fn motion_of(reduce_motion: bool) -> f32 {
    if reduce_motion { STILL } else { MOVING }
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
    /// Whether the materials made for a card's back have been given the
    /// printed picture yet — see [`Self::dress_the_backs`].
    ///
    /// It only ever goes one way. Once the picture has arrived, everything
    /// made afterwards is built with it, because the handle comes from
    /// `CardTextures::card_back`.
    back_dressed: bool,
    /// Whether what is cached was made to hold still.
    ///
    /// A `bool` and not the `f32` the shader wants, so that `Default` is the
    /// right answer: a client that has not read a preference yet animates,
    /// and `false` says so. Deriving the clock from it ([`motion_of`]) keeps
    /// the default honest in one place.
    still: bool,
}

impl UiCardMaterials {
    /// The clock the cards in this cache were made on.
    const fn motion(&self) -> f32 {
        motion_of(self.still)
    }

    /// Says whether cards should animate, and tells the ones already made.
    ///
    /// Called every frame from the preference, so the comparison is the
    /// point: only a change costs anything, and what it costs is one pass
    /// over a cache holding at most a hand's worth of materials.
    ///
    /// The clock is rewritten *in place* rather than the cache emptied. A
    /// cleared cache is only refilled by whatever draws the card next, and a
    /// hand nobody is touching is drawn once and left alone — the setting
    /// would appear to do nothing until the game moved. Rewriting keeps
    /// every handle the nodes are holding, so the cards change where they
    /// are.
    ///
    /// None of this is in [`CardLook`], deliberately: the look is the cache
    /// *key*, and a key carrying a global preference would keep both answers
    /// alive at once and evict neither.
    pub fn set_still(&mut self, still: bool, assets: &mut Assets<CardUiMaterial>) {
        if self.still == still {
            return;
        }
        self.still = still;
        let motion = self.motion();
        for handle in self.made.values().chain(self.previewed.values()) {
            if let Some(mut material) = assets.get_mut(handle) {
                material.params.motion = motion;
            }
        }
    }

    /// The material for a look, made once — unless it is sweeping.
    ///
    /// A [`CardLook::sweep`] is the one part of a key that is over in a
    /// second, so a sweeping look is built and handed out without being
    /// stored: this store has no eviction at all, and one entry per card per
    /// arrival is a leak the length of a game. Nothing is lost by it. The
    /// handle goes to the node that asked, the shader animates from what is
    /// baked in it, and when the card stops sweeping the next rebuild asks
    /// for the plain look and gets the shared material back.
    pub fn get(
        &mut self,
        look: CardLook,
        art: Option<Handle<Image>>,
        tint: Color,
        assets: &mut Assets<CardUiMaterial>,
    ) -> Handle<CardUiMaterial> {
        if look.sweep.is_some() {
            let made = material(look, art, tint, self.motion());
            return assets.add(CardUiMaterial {
                art: made.art,
                params: made.params,
            });
        }
        if let Some(handle) = self.made.get(&look) {
            return handle.clone();
        }
        let made = material(look, art, tint, self.motion());
        let handle = assets.add(CardUiMaterial {
            art: made.art,
            params: made.params,
        });
        self.made.insert(look, handle.clone());
        handle
    }

    /// Puts the printed card back on every material made for one.
    ///
    /// The same move [`Self::set_still`] makes, for the same reason: this
    /// cache hands out one material per look and never looks at the handle
    /// again, so a back drawn before the picture arrived would stay a flat
    /// colour for the rest of the session however many times the hand is
    /// rebuilt. Rewriting in place also reaches every node already holding
    /// one.
    ///
    /// Both halves, as everywhere else the back is dressed: `has_art` follows
    /// the handle, and a material given the texture without the flag draws
    /// exactly what it drew before.
    pub fn dress_the_backs(&mut self, art: &Handle<Image>, assets: &mut Assets<CardUiMaterial>) {
        self.back_dressed = true;
        for (look, handle) in &self.made {
            if !look.is_back() {
                continue;
            }
            if let Some(mut material) = assets.get_mut(handle) {
                material.art = Some(art.clone());
                material.params.has_art = 1.0;
            }
        }
    }

    /// Whether the printed back has already been put on what was made
    /// without it.
    #[must_use]
    pub const fn backs_are_dressed(&self) -> bool {
        self.back_dressed
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
        let motion = self.motion();
        let handle = assets.add(CardUiMaterial {
            art: Some(art),
            params: CardParams {
                finish: finish_code(finish),
                glow: 0,
                // A printing in the picker is a piece of cardboard, not a
                // permanent: it has no body and no counters on it.
                plate: 0,
                chips_a: 0,
                chips_b: 0,
                has_art: 1.0,
                strength: 1.0,
                // A picker card has no border to animate, but it can be a
                // foil, and a sweeping rainbow is exactly the kind of motion
                // the preference is about.
                motion,
                // The picker is a shelf of cardboard a player is choosing
                // between: nothing there has just arrived, and a sheen
                // travelling across a row of printings would be saying
                // something about them that is not true.
                sweep_at: 0.0,
                sweep_rate: 0.0,
                sweep_door: door::NONE,
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
    art.and_then(|key| statics.print(key.printing()?))
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
    /// Its counter chips, packed, for the same reason and with the same cost.
    pub chips: [u32; 2],
    /// The flat colour, quantised, for a card with no art. `0` when it has
    /// art — a colour is not part of the key then.
    pub tint: u32,
    /// The one-shot sheen this card is in the middle of, if any.
    ///
    /// In the key because it is in the material, like the plate — but unlike
    /// the plate it is *transient*, so a look carrying one is deliberately
    /// **not** cached: see [`UiCardMaterials::get`]. Two cards that started
    /// sweeping on the same frame still share a material, which is the
    /// opening hand's whole seven.
    pub sweep: Option<crate::sheen::Sweep>,
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
            chips: [0; 2],
            tint: 0,
            sweep: None,
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
            chips: [0; 2],
            tint: quantise(color),
            sweep: None,
        }
    }

    /// The same look with its reserved corner filled in.
    ///
    /// A builder rather than a sixth argument on all three constructors: a
    /// card in hand, a card in a browser and a card in the printing picker
    /// have no body to show, and only the two board surfaces ever call this.
    #[must_use]
    pub fn with_corner(mut self, corner: baylee_client_core::cardplate::Corner) -> Self {
        let [plate, a, b] = corner.packed();
        self.plate = plate;
        self.chips = [a, b];
        self
    }

    /// The same look catching the light once.
    ///
    /// `None` is the ordinary answer and the one every card gives a second
    /// after it arrived, which is the point of the whole mechanism: the
    /// sheen is what marks a card out as *new*, so a card that has been on
    /// the table for a turn must give the same key as one that has been
    /// there for ten.
    #[must_use]
    pub fn with_sweep(mut self, sweep: Option<crate::sheen::Sweep>) -> Self {
        self.sweep = sweep;
        self
    }

    /// A card showing the back.
    ///
    /// No `ImageKey`, because the back belongs to no printing: it is one
    /// picture the whole game shares, held under a key of its own
    /// ([`baylee_client_core::images::ImageKey::card_back`]) and handed out
    /// by [`crate::textures::CardTextures::card_back`], which answers with a
    /// flat colour until it has arrived. And no tint, which is what separates
    /// it from [`CardLook::flat`]: both have no `ImageKey`, and a back that
    /// collided with a constructed face would draw one as the other.
    #[must_use]
    pub fn back(finish: FinishTreatment, glow: u32) -> Self {
        Self {
            art: None,
            finish,
            glow,
            plate: 0,
            chips: [0; 2],
            tint: 0,
            sweep: None,
        }
    }

    /// Whether this look is a card seen from behind.
    ///
    /// Read off the key rather than carried as a flag, because the key
    /// already says it: no art and no tint is what [`CardLook::back`] means
    /// and the one thing no other constructor produces.
    #[must_use]
    pub const fn is_back(&self) -> bool {
        self.art.is_none() && self.tint == 0
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
/// `has_art` follows the *handle*, not the key: a card wearing the printed
/// back is drawn from a texture that belongs to no printing, and it still has
/// to be sampled rather than replaced by a tint. The back is fetched like any
/// other picture and arrives late, so the material that wears it is built
/// without one and dressed afterwards — `table::dress_in_the_back` sets both
/// halves, and there is a test that it produces this same material.
#[must_use]
pub fn material(
    look: CardLook,
    art: Option<Handle<Image>>,
    tint: Color,
    motion: f32,
) -> CardMaterial {
    let has_art = if art.is_some() { 1.0 } else { 0.0 };
    let mut made = CardMaterial {
        art,
        params: CardParams {
            finish: finish_code(look.finish),
            glow: look.glow,
            plate: look.plate,
            chips_a: look.chips[0],
            chips_b: look.chips[1],
            has_art,
            strength: 1.0,
            motion,
            // Written by `wear` below rather than here, so the one decision a
            // still card makes about a one-shot travel is made in one place —
            // a departing card is dressed the same way and never passes
            // through a `CardLook` at all.
            sweep_at: 0.0,
            sweep_rate: 0.0,
            sweep_door: door::NONE,
            tint: LinearRgba::from(tint).to_f32_array().into(),
        },
    };
    wear(&mut made.params, look.sweep, motion);
    made
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
            .init_resource::<UiCardMaterials>()
            .add_systems(Update, (track_motion, dress_the_card_backs));
    }
}

/// Carries [`Preferences::reduce_motion`] to the UI cards.
///
/// The table's half of this is in `sync_scene`, which owns the 3D cache and
/// is already reading the preferences. There is no shared place for both: the
/// two caches are a `Resource` and a field of another `Resource`, and joining
/// them would put the deck builder's printing picker behind the duel's scene
/// index.
///
/// `Prefs` is optional because a headless app can install the materials
/// without installing the account's settings, and an app with no preference
/// to read has no reason to hold still.
///
/// [`Preferences::reduce_motion`]: baylee_client_core::prefs::Preferences::reduce_motion
fn track_motion(
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut cache: ResMut<UiCardMaterials>,
    mut assets: ResMut<Assets<CardUiMaterial>>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    if cache.still != still {
        cache.set_still(still, &mut assets);
    }
}

/// Carries the printed card back to the UI cards, once it has arrived.
///
/// The table does the same to its own back material in `table::sync_scene`.
/// Both are needed and neither covers the other: the two caches are a
/// `Resource` and a field of another one, and the 3D and 2D materials are
/// different assets.
///
/// The texture cache is optional for the same reason the preferences are
/// above: this plugin is installed by the deck builder too, which draws
/// cardboard and has no duel behind it.
fn dress_the_card_backs(
    textures: Option<Res<crate::textures::CardTextures>>,
    mut cache: ResMut<UiCardMaterials>,
    mut assets: ResMut<Assets<CardUiMaterial>>,
) {
    if cache.backs_are_dressed() {
        return;
    }
    let Some(textures) = textures else {
        return;
    };
    if textures.card_back_is_printed() {
        cache.dress_the_backs(&textures.card_back(), &mut assets);
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
            // A card, and it matters: `card: None` is a *token*, and every
            // test below that reads a keyword or an offer would be handed a
            // token mark along with it. The name is one no card is printed
            // with, so the registry answers nothing and this is an ordinary
            // permanent — which is what all of them meant by "a permanent".
            card: Some(baylee_view::CardIdentity {
                index: baylee_core::ids::CardIndex::new(0),
                print: baylee_core::ids::PrintRef::new(0),
                face: 0,
            }),
            name: "Test".to_string(),
            controller: PlayerId::new(0),
            owner: PlayerId::new(0),
            commander: false,
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
            granted_mana: None,
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

    /// Sickness is drawn for creatures and nothing else, whatever a host
    /// says. Only a creature is stopped by it (CR 302.6) — a land played
    /// this turn taps perfectly well, and a board where every fresh
    /// permanent breathed would be teaching a player something false. The
    /// view carries the narrower fact today; this is what holds if it ever
    /// carries the wider one again.
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
        use baylee_client_core::images::ArtSize;
        use baylee_core::ids::PrintRef;

        let key = ImageKey::new(PrintRef(0), 0, ArtSize::Normal);
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

    /// Holding still changes the cards already made, and makes no new ones.
    ///
    /// Two claims, and both are the kind that prose cannot hold. The first is
    /// that the setting is not in [`CardLook`]: if it ever became part of the
    /// key, the same look would mint a second material and the first would
    /// live on with nothing to evict it — so the handle has to come back
    /// *identical*. The second is that the change is written into the
    /// materials rather than the cache being emptied: a cleared cache is only
    /// refilled by whatever draws the card next, and a hand nobody is
    /// touching is drawn once and left alone, so the preference would appear
    /// to do nothing until the game moved.
    ///
    /// `Assets` needs no render plugins, which is why this can be a unit test
    /// at all.
    #[test]
    fn holding_still_rewrites_the_cards_already_made() {
        let mut cache = UiCardMaterials::default();
        let mut assets = Assets::<CardUiMaterial>::default();
        let look = CardLook::flat(Color::WHITE, FinishTreatment::Foil, glow::ACTIVATABLE);

        // The clock is only ever one of two exact values, but `float_cmp` is
        // right in general and the file already has the idiom.
        let clock = |assets: &Assets<CardUiMaterial>, handle: &Handle<CardUiMaterial>| {
            assets.get(handle).expect("the material").params.motion
        };

        let handle = cache.get(look, None, Color::WHITE, &mut assets);
        let made = clock(&assets, &handle);
        assert!(
            (made - MOVING).abs() < f32::EPSILON,
            "a card starts out animating, not at {made}"
        );

        cache.set_still(true, &mut assets);
        let told = clock(&assets, &handle);
        assert!(
            (told - STILL).abs() < f32::EPSILON,
            "the material already made is still at {told}: it was discarded \
             rather than told"
        );

        let again = cache.get(look, None, Color::WHITE, &mut assets);
        assert_eq!(
            again, handle,
            "the same look gave a second material: the setting has leaked \
             into the cache key"
        );
        assert_eq!(assets.len(), 1, "one look, one material");
    }

    /// The field names of a WGSL struct, in declaration order.
    fn wgsl_fields(source: &str, name: &str) -> Vec<String> {
        let head = format!("struct {name} {{");
        let body = source
            .split_once(&head)
            .unwrap_or_else(|| panic!("no `struct {name}` in the shader"))
            .1
            .split_once('}')
            .expect("an unterminated struct")
            .0;
        body.lines()
            .map(str::trim)
            .filter(|line| !line.starts_with("///") && !line.starts_with("//"))
            .filter_map(|line| Some(line.split_once(':')?.0.trim().to_string()))
            .collect()
    }

    /// `CardParams` is one struct written three times.
    ///
    /// A uniform is bytes: nothing checks that the Rust field order and the
    /// two WGSL declarations agree, and a mismatch has neither an error nor a
    /// crash. Swapping the last two would feed `tint`'s red channel in as the
    /// clock and the clock in as a colour — every card on the table drawn in
    /// a wrong flat colour, animating at a speed that depends on how blue it
    /// is.
    ///
    /// What this pins is the pair of shaders against each other and against
    /// the order written out below, which is the Rust struct's. Adding a
    /// field to `CardParams` and forgetting either shader fails here; the one
    /// hole left is changing all three of these and none of the Rust, which
    /// no plausible edit does.
    #[test]
    fn card_params_is_the_same_struct_in_all_three_files() {
        // The order `#[derive(ShaderType)]` writes the bytes in.
        let ours = [
            "finish",
            "glow",
            "plate",
            "chips_a",
            "chips_b",
            "has_art",
            "strength",
            "motion",
            "sweep_at",
            "sweep_rate",
            "sweep_door",
            "tint",
        ];
        for (which, src) in [
            ("card.wgsl", include_str!("shaders/card.wgsl")),
            ("card_ui.wgsl", include_str!("shaders/card_ui.wgsl")),
        ] {
            let theirs = wgsl_fields(src, "CardParams");
            assert_eq!(theirs, ours, "{which} disagrees about `CardParams`");
        }
    }

    /// The sheen crosses a card once, the right way, and only when a card
    /// has been given one.
    ///
    /// Read out of the WGSL for the reason the sleep test is: the direction
    /// and the one-shot-ness live in the shader, nothing in Rust can observe
    /// them, and the fault they replace was invisible in exactly that way —
    /// a six-second loop on every card in the hand, which a player reads as
    /// the cards flickering at rest and which no test noticed for months.
    #[test]
    fn the_sheen_crosses_once_from_the_bottom_right() {
        let common = include_str!("shaders/card_common.wgsl");
        // `uv` is 0 at the top left and 1 at the bottom right on both axes,
        // so this diagonal is the travel and the band is across it.
        assert!(
            common.contains("let along = (uv.x + uv.y) * 0.5;"),
            "the band no longer runs along the card's diagonal"
        );
        // 1 → 0 as the phase advances: bottom right to top left, with a
        // margin at each end so the band starts and finishes off the card.
        assert!(
            common.contains("mix(1.0 + SWEEP_MARGIN, -SWEEP_MARGIN, phase)"),
            "the band no longer travels bottom right to top left"
        );
        // And it is a *shot*, not a cycle: outside its phase there is
        // nothing there at all.
        assert!(
            common.contains("if phase < 0.0 || phase > 1.0 {"),
            "the sweep is no longer bounded to one pass"
        );

        for (which, src) in [
            ("card.wgsl", include_str!("shaders/card.wgsl")),
            ("card_ui.wgsl", include_str!("shaders/card_ui.wgsl")),
        ] {
            assert!(
                src.contains(
                    "let travel = select(0.0, sweep_amount(uv, phase), params.sweep_rate > 0.0);"
                ),
                "{which} draws a sheen on cards that were given none"
            );
            // The metal is not the sheen. A card that is not sweeping still
            // has a coating on it, which is what makes card stock read as
            // card stock, and the owner asked for exactly that to stay.
            assert!(
                src.contains("METAL_FLOOR"),
                "{which} lost the coating along with the animation"
            );
        }
    }

    /// The five doors are the same five numbers on both sides of the wire.
    ///
    /// Nothing in either compiler can notice that they are not: a
    /// `sweep_door` of 4 is a valid `u32` whatever the shader thinks 4 means,
    /// so a card being destroyed would simply be drawn as a bounce and the
    /// build would stay green. This is the same pairing the rail gets, for
    /// the same reason.
    #[test]
    fn the_doors_are_numbered_the_same_in_both_languages() {
        use baylee_client_core::zones::Passage;
        let src = include_str!("shaders/card_common.wgsl");
        for (name, ours) in [
            ("DOOR_NONE", door::NONE),
            ("DOOR_BOUNCE", door::BOUNCE),
            ("DOOR_EXILED", door::EXILED),
            ("DOOR_FLICKERED", door::FLICKERED),
            ("DOOR_DESTROYED", door::DESTROYED),
            ("DOOR_RETURNED", door::RETURNED),
        ] {
            let theirs = wgsl_const(src, name);
            assert!(
                (theirs - ours as f32).abs() < f32::EPSILON,
                "{name}: {ours} here, {theirs} in the shader"
            );
        }
        // And the Rust half maps every door to one of them. Written out
        // rather than swept, so a sixth `Passage` fails to compile here
        // instead of quietly mapping to `NONE` and drawing nothing.
        assert_eq!(door::code(None), door::NONE);
        assert_eq!(door::code(Some(Passage::Bounce)), door::BOUNCE);
        assert_eq!(door::code(Some(Passage::Exiled)), door::EXILED);
        assert_eq!(door::code(Some(Passage::Flickered)), door::FLICKERED);
        assert_eq!(door::code(Some(Passage::Destroyed)), door::DESTROYED);
        assert_eq!(door::code(Some(Passage::Returned)), door::RETURNED);
    }

    /// A door and the plain arrival are never drawn at once, and the pairs
    /// are one figure reversed rather than two.
    ///
    /// All of it read out of the WGSL, because none of it is observable from
    /// Rust — and the failure it guards against is the quiet kind: a card
    /// wearing both a white band and a violet ring reads as neither, and
    /// looks in a screenshot exactly like a card wearing one of them.
    #[test]
    fn a_door_replaces_the_arrival_band_rather_than_joining_it() {
        for (which, src) in [
            ("card.wgsl", include_str!("shaders/card.wgsl")),
            ("card_ui.wgsl", include_str!("shaders/card_ui.wgsl")),
        ] {
            assert!(
                src.contains("let plain = select(0.0, travel, door == DOOR_NONE);"),
                "{which} lays the arrival band under a door"
            );
            assert!(
                src.contains(
                    "let door = select(DOOR_NONE, params.sweep_door, params.sweep_rate > 0.0);"
                ),
                "{which} draws a door on a card that was given no sweep"
            );
        }
        let common = include_str!("shaders/card_common.wgsl");
        // The two exile doors are one ring, run in opposite directions — the
        // owner's "the same effect reversed" in one line each.
        assert!(
            common.contains("door_ring(uv, mix(DOOR_REACH, 0.0, phase))"),
            "exile no longer closes"
        );
        assert!(
            common.contains("door_ring(uv, mix(0.0, DOOR_REACH, phase))"),
            "a flicker no longer opens"
        );
        // And the two graveyard doors are one band, the same way.
        assert!(
            common.contains("door_band(uv.y, phase, true)"),
            "a destruction no longer falls down the card"
        );
        assert!(
            common.contains("door_band(uv.y, phase, false)"),
            "a resurrection no longer rises up it"
        );
        // A door outside its one pass draws nothing at all, which is what
        // keeps every card on the table paying one compare and no more.
        assert!(
            common.contains("if door == DOOR_NONE || phase < 0.0 || phase > 1.0 {"),
            "a door is no longer bounded to one pass"
        );
    }

    /// A card shader may only call what it has asked for by name.
    ///
    /// `card_common.wgsl` is imported with an explicit item list, so adding a
    /// function to it and calling it from a card shader is *two* edits and
    /// the second one is invisible: the file parses, every test that reads
    /// the WGSL as text passes, and the composition fails at run time with
    /// both card shaders dead — which draws the whole board wrong on the
    /// first frame and nowhere else. `sweep_amount` shipped that way and was
    /// caught by reading the import line, not by anything that ran.
    #[test]
    fn a_card_shader_imports_every_shared_helper_it_calls() {
        let common = include_str!("shaders/card_common.wgsl");
        let shared: Vec<&str> = common
            .lines()
            .filter_map(|line| line.strip_prefix("fn "))
            .filter_map(|rest| rest.split('(').next())
            .collect();
        assert!(
            shared.contains(&"sweep_amount"),
            "the shared file no longer defines the helpers this reads"
        );

        for (which, src) in [
            ("card.wgsl", include_str!("shaders/card.wgsl")),
            ("card_ui.wgsl", include_str!("shaders/card_ui.wgsl")),
        ] {
            let line = src
                .lines()
                .find(|line| line.starts_with("#import") && line.contains("card_common.wgsl"))
                .unwrap_or_else(|| panic!("{which} no longer imports the shared file"));
            let asked = line
                .split_once('{')
                .and_then(|(_, rest)| rest.split_once('}'))
                .expect("the import is no longer an item list")
                .0;
            let asked: Vec<&str> = asked.split(',').map(str::trim).collect();
            // The body is everything below the imports, so an import line
            // naming a function is not itself read as a call to it.
            let body = src.split_once("\nfn ").map_or(src, |(_, rest)| rest);
            for name in &shared {
                if body.contains(&format!("{name}(")) {
                    assert!(
                        asked.contains(name),
                        "{which} calls {name} without importing it"
                    );
                }
            }
        }
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
            ("GLOW_COMMANDER", glow::COMMANDER),
            ("GLOW_TOKEN", glow::TOKEN),
            ("GLOW_COPY", glow::COPY),
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

    /// The two provenance bits stand alone in the word, and never together on
    /// one card.
    ///
    /// Two claims, and the second is the one worth a test. A bit that
    /// collided would draw a token mark on something that is not a token,
    /// which a player has no way to check; a card carrying both would draw
    /// one glyph and leave the other silently unsaid, which is worse — the
    /// mark would be *there*, so it would be believed. `provenance_of` is
    /// what makes that impossible, by answering with one value of three
    /// instead of two booleans, and this is what says the packing kept it so.
    #[test]
    fn a_card_is_marked_a_token_or_a_copy_and_never_both() {
        let others = glow::INDESTRUCTIBLE
            | glow::HEXPROOF
            | glow::SHROUD
            | glow::ACTIVATABLE
            | glow::SUMMONING_SICK
            | glow::ARMED
            | glow::WILL_TAP
            | glow::COMMANDER
            | glow::MARK_MASK;
        assert_eq!(glow::TOKEN & others, 0, "the token bit is somebody else's");
        assert_eq!(glow::COPY & others, 0, "the copy bit is somebody else's");
        assert_eq!(glow::TOKEN & glow::COPY, 0, "and they are not each other");

        // A Clone wearing another card's face, and a token wearing the same
        // one. Both are drawn from the same registry answer and exactly one
        // bit comes back each time.
        let elves = |card: Option<baylee_view::CardIdentity>| {
            let mut o = baylee_client_core::test_support::token(1, 0, "Llanowar Elves", 1, 1);
            o.card = card;
            crate::cardmat::glow_of(Some(&o), Offer::NONE)
        };
        let (real, _) = crate::cardart::wearing("Llanowar Elves").expect("the pool has it");
        let identity = |index: baylee_core::ids::CardIndex| baylee_view::CardIdentity {
            index,
            print: baylee_core::ids::PrintRef::new(1),
            face: 0,
        };
        assert_eq!(
            elves(Some(identity(baylee_core::ids::CardIndex::new(
                real.get() + 1
            )))),
            glow::COPY
        );
        assert_eq!(elves(None), glow::TOKEN);
        assert_eq!(
            elves(Some(identity(real))),
            0,
            "and a Llanowar Elves that is one wears no mark at all"
        );
    }

    /// The night a sick creature lies under is written out twice, and it has
    /// to be the same night.
    ///
    /// A card picked up off the table keeps the sleep it was lying in, and
    /// nothing in either compiler can notice when one copy drifts: the
    /// permanent on the felt would draw one thing and its own hover preview
    /// another, and both would look deliberate. So the constants are compared
    /// as the lines they are — the block is UV and the clock only, which is
    /// exactly what lets the UI twin run it unchanged.
    #[test]
    fn both_shaders_lay_the_card_down_under_the_same_night() {
        fn night(src: &str) -> Vec<&str> {
            src.lines()
                .map(str::trim)
                .filter(|line| line.starts_with("const SLEEP_"))
                .collect()
        }
        let table = include_str!("shaders/card.wgsl");
        let ui = include_str!("shaders/card_ui.wgsl");
        let theirs = night(table);
        assert_eq!(theirs.len(), 19, "the table shader lost a sleep constant");
        assert_eq!(theirs, night(ui), "the two shaders sleep differently");

        // And the breath runs on the constant that names its period, with
        // phase zero — where reduce-motion stops the clock — in the *middle*
        // of the sway rather than at an end of it.
        let seconds = wgsl_const(table, "SLEEP_SECONDS");
        assert!(
            (seconds - 5.0).abs() < f32::EPSILON,
            "the sleep period moved to {seconds}"
        );
        for (which, src) in [("card.wgsl", table), ("card_ui.wgsl", ui)] {
            assert!(
                src.contains("sin(t * 6.2831855 / SLEEP_SECONDS)"),
                "{which} does not breathe on SLEEP_SECONDS"
            );
            assert!(
                src.contains("ring_r * SLEEP_RING_COUNT - t / SLEEP_RING_SECONDS"),
                "{which} does not send its rings out on SLEEP_RING_SECONDS"
            );
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
            deed: crate::Deed::Run {
                plan: Plan {
                    steps: vec![Step {
                        source: forests[1],
                        tap: Tap::Intrinsic,
                        color: None,
                    }],
                    ..default()
                },
                then: crate::RunEnd::Cast,
            },
        };
        let offer = Offer::on(Some(&run), &forests, false);
        assert!(offer.will_tap, "one of the three is being spent");
        assert!(!offer.armed, "the spell is not one of the lands");

        // And a plan that touches none of them leaves the stack dark.
        let other = crate::Armed {
            object: elsewhere,
            deed: crate::Deed::Run {
                plan: Plan {
                    steps: vec![Step {
                        source: elsewhere,
                        tap: Tap::Intrinsic,
                        color: None,
                    }],
                    ..default()
                },
                then: crate::RunEnd::Cast,
            },
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
        // Prowess is bit 23 of the engine's word and slot 11 of the rail,
        // which is the whole reason the two numberings are pinned rather
        // than assumed to be the same list.
        assert_eq!(slot(KeywordSet::PROWESS), 1 << 11);
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
            ("PLATE_LORE", plate::KIND_LORE),
            ("GLYPH_W", plate::GLYPH_W),
            ("GLYPH_H", plate::GLYPH_H),
            ("ROMAN_MAX", u32::from(plate::ROMAN_MAX)),
            ("CHIP_BITS", plate::CHIP_BITS),
            ("CHIP_TINT_MASK", plate::CHIP_TINT_MASK),
            ("CHIP_COUNT_SHIFT", plate::CHIP_COUNT_SHIFT),
            ("CHIP_COUNT_MASK", plate::CHIP_COUNT_MASK),
            ("PIP_MAX", u32::from(plate::PIP_MAX)),
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

        // The stencils. Every word is under 2^24, so an `f32` carries it
        // exactly and this comparison is not an approximation.
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
            "GLYPH_PLUS",
            "GLYPH_I",
            "GLYPH_V",
        ]
        .into_iter()
        .enumerate()
        {
            assert!(
                (wgsl_const(src, name) - plate::GLYPHS[i] as f32).abs() < 0.5,
                "{name} draws a different picture in the shader"
            );
        }

        // The chip column: its geometry, and the six die faces.
        for (name, ours) in [
            ("CHIP_D", plate::CHIP_D),
            ("CHIP_GAP", plate::CHIP_GAP),
            ("CHIP_X", plate::CHIP_X),
            ("CHIP_BASE", plate::CHIP_BASE),
        ] {
            let theirs = wgsl_const(src, name);
            assert!(
                (theirs - ours).abs() < 1e-5,
                "{name}: {ours} here, {theirs} in the shader"
            );
        }
        for (i, name) in ["PIP_1", "PIP_2", "PIP_3", "PIP_4", "PIP_5", "PIP_6"]
            .into_iter()
            .enumerate()
        {
            assert!(
                (wgsl_const(src, name) - plate::PIPS[i] as f32).abs() < 0.5,
                "{name} is a different die face in the shader"
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

    /// The crest is the rail's alphabet, on the edge the rail does not use.
    ///
    /// Two claims, and both are geometry rather than taste. It borrows the
    /// rail's slot and inset so that a commander's crown is the same size and
    /// sits the same distance in as a flying chevron — that is what makes it
    /// read as one more glyph instead of a second design language. And it
    /// lives on the *top* edge, which is the one region of the card nothing
    /// else claims: the rail and the plate share the bottom, the chips run up
    /// the right. A crest that drifted down into either would be two marks
    /// overlapping with no error and no crash, which is exactly the failure
    /// the flag test above exists to catch on the other axis.
    #[test]
    fn the_crest_shares_the_rails_metrics_and_none_of_its_edge() {
        let src = include_str!("shaders/card_common.wgsl");
        for name in ["SLOT", "INSET"] {
            let crest = wgsl_const(src, &format!("CREST_{name}"));
            let rail = wgsl_const(src, &format!("RAIL_{name}"));
            assert!(
                (crest - rail).abs() < f32::EPSILON,
                "CREST_{name} is {crest} and RAIL_{name} is {rail}"
            );
        }

        // Both edges measured in width-units, the way the shader measures
        // them, because the card is taller than it is wide and a bound
        // compared across that would be off by the aspect.
        let aspect = wgsl_const(src, "CARD_ASPECT");
        let height = 1.0 / aspect;
        let crest_bottom = wgsl_const(src, "CREST_INSET") + wgsl_const(src, "CREST_SLOT");
        let rail_top = height - wgsl_const(src, "RAIL_INSET") - wgsl_const(src, "RAIL_SLOT");
        assert!(
            crest_bottom < rail_top,
            "the crest reaches {crest_bottom} and the rail starts at {rail_top}"
        );

        // The chips climb the right edge; the crest is centred, so what has to
        // hold is that it never reaches their column.
        let crest_right = 0.5 + wgsl_const(src, "CREST_SLOT") * 0.5 + 0.014;
        let chip_left = wgsl_const(src, "CHIP_X") - wgsl_const(src, "CHIP_D") * 0.5;
        assert!(
            crest_right < chip_left,
            "the crest reaches {crest_right} and the chips start at {chip_left}"
        );
    }

    /// Every triangle in the shader is wound the way `sd_tri` needs.
    ///
    /// `sd_tri` takes the outermost of three edge half-planes and builds each
    /// normal as `(edge.y, -edge.x)`. That is only "outside the triangle" for
    /// one winding; give it the other and every point lands outside all three
    /// edges, so the shape **silently never draws**. No error, no crash, no
    /// pink card — just a pictogram that is quietly missing part of itself.
    ///
    /// This is not hypothetical. The commander crest shipped its first frame
    /// as a plain white bar: the circlet drew, all three of the crown's points
    /// were wound backwards, and every existing test stayed green, including
    /// the one right above that checks where the crest *is*. Placement tests
    /// cannot see shape — the same lesson the card quad taught when it shipped
    /// as a bowtie.
    ///
    /// Checked for the whole file rather than for the crest, because the trap
    /// is in the helper and catches the next mark just as easily as it caught
    /// this one.
    #[test]
    fn every_triangle_in_the_shader_is_wound_so_it_actually_draws() {
        let src = include_str!("shaders/card_common.wgsl");

        // The last three `vec2<f32>` literals of a call are its vertices; an
        // earlier one belongs to the point expression, the way `mark_trample`
        // writes `p - vec2<f32>(0.0, stomp)`.
        let vertices = |call: &str| -> Vec<(f32, f32)> {
            call.match_indices("vec2<f32>(")
                .filter_map(|(at, _)| {
                    let open = at + "vec2<f32>(".len();
                    let close = call[open..].find(')')? + open;
                    let (x, y) = call[open..close].split_once(',')?;
                    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
                })
                .collect()
        };

        let mut checked = 0;
        for (at, _) in src
            .match_indices("sd_tri(")
            // The helper's own declaration matches too, and its parameter
            // list spells `vec2<f32>` without a paren after it, so it parses
            // to no vertices at all rather than to bad ones.
            .filter(|(at, _)| !src[..*at].ends_with("fn "))
        {
            let rest = &src[at..];
            let end = rest.find(");").expect("a call ends");
            let verts = vertices(&rest[..end]);
            let n = verts.len();
            assert!(n >= 3, "sd_tri call with {n} vertex literals");
            let [a, b, c] = [verts[n - 3], verts[n - 2], verts[n - 1]];

            // Twice the signed area. Positive is the winding `sd_tri` treats
            // as inside; the sign is what matters, not the magnitude.
            let cross = (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0);
            assert!(
                cross > 0.0,
                "a triangle at byte {at} is wound backwards ({cross}) and will \
                 not draw: {a:?} {b:?} {c:?}"
            );
            checked += 1;
        }

        // A parser that quietly matched nothing would make every assertion
        // above vacuous, which is the way a test like this really fails.
        assert!(
            checked >= 6,
            "only {checked} triangles found — the parser has drifted from the file"
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

    /// This cache hands out one material per look and never looks at the
    /// handle again, so a card back drawn before the picture arrived would
    /// stay a flat colour for the rest of the session — the hand can be
    /// rebuilt a hundred times and get the same material every time. The
    /// picture is therefore written into what is already hanging there.
    #[test]
    fn a_back_made_before_the_picture_arrived_is_dressed_where_it_hangs() {
        let mut images = Assets::<Image>::default();
        let mut assets = Assets::<CardUiMaterial>::default();
        let mut cache = UiCardMaterials::default();

        // Two shapes, both legitimate: the hand builds a back around the
        // stand-in texture, and a caller with nothing to hand it builds one
        // around no picture at all. The second is the one that needs the
        // flag as well as the handle.
        let flat = images.add(Image::default());
        let back = cache.get(
            CardLook::back(FinishTreatment::Plain, 0),
            Some(flat.clone()),
            Color::BLACK,
            &mut assets,
        );
        let bare = cache.get(
            CardLook::back(FinishTreatment::Plain, 1),
            None,
            Color::BLACK,
            &mut assets,
        );
        // A card drawing its own text is not a back, and must not be given
        // the picture: both have no `ImageKey`, and the tint is what tells
        // them apart.
        let face = cache.get(
            CardLook::flat(Color::srgb(0.2, 0.3, 0.4), FinishTreatment::Plain, 0),
            None,
            Color::srgb(0.2, 0.3, 0.4),
            &mut assets,
        );

        let printed = images.add(Image::default());
        assert!(!cache.backs_are_dressed());
        cache.dress_the_backs(&printed, &mut assets);
        assert!(cache.backs_are_dressed());

        let dressed = assets.get(&back).expect("the back material");
        assert_eq!(dressed.art.as_ref(), Some(&printed), "still the stand-in");
        let bare = assets.get(&bare).expect("the bare back material");
        assert_eq!(bare.art.as_ref(), Some(&printed));
        assert!(
            (bare.params.has_art - 1.0).abs() < f32::EPSILON,
            "given the picture and not the flag, it draws the tint as before"
        );
        assert_ne!(
            assets.get(&face).expect("the face material").art.as_ref(),
            Some(&printed),
            "a constructed face was dressed as a card back"
        );
    }
}
