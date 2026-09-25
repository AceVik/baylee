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

use baylee_client_core::images::{FinishTreatment, ImageKey};
use baylee_core::ids::ObjectId;
use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

/// What a card on the table is saying round itself, as one bitset.
///
/// Deliberately not the engine's keyword numbering: a shader reads a handful
/// of bits and the engine has more than a hundred keywords, so translating
/// once here is cheaper than sending a `u128` to the GPU.
///
/// Two kinds of claim ride in the word. The offers — [`ACTIVATABLE`],
/// [`REACHABLE`], [`ARMED`], [`WILL_TAP`] — are this client saying "you
/// could" or "you are about to", and are light on the felt round the card
/// (`floor_light`, #298). The three protection keywords are facts about the
/// card. They were the frame's paper until #298 took the frame away, and they
/// wait for the shells that replace it. Sickness and identity used to ride
/// here too, and are the strip's now: the moon and the crests
/// (`baylee_client_core::cardrail::Strip`). Nothing in this word is drawn on
/// the print (#274).
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
    /// This client would tap lands for this card and then cast it:
    /// `Duel::reachable`, drawn on a card lying in a pile (#242).
    ///
    /// [`ACTIVATABLE`]'s twin and deliberately its motion — the same light
    /// running round the card, because both are "you could" — in the
    /// hand's indigo rather than its amber, because the hand already says
    /// the two claims in those two hues: one is the engine's yes, the other
    /// is this client's offer to spend the mana first. Never set with it;
    /// [`super::glow_of`] picks one.
    ///
    /// Bit 23 because the bits below it were other claims once — sickness,
    /// the commander, the rail's twelve marks, provenance — and the ones that
    /// are left kept their numbers rather than moving, because
    /// `card_common.wgsl`'s `GLOW_*` constants are the other half of each.
    pub const REACHABLE: u32 = 1 << 23;

    /// The four offers: what the felt round a card is lit with.
    pub const OFFERS: u32 = ACTIVATABLE | ARMED | WILL_TAP | REACHABLE;
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

/// Translates the view's keyword bitset into the protection a card wears.
///
/// Three keywords come out, the three that were the frame's paper until #298
/// and are marks on the strip since (`cardrail::MARK_ORDER`'s last three):
/// the shells that stand round a card for them read them here.
///
/// Shroud swallows hexproof on the way through, because that is what the two
/// keywords do to each other: a permanent with both may be targeted by
/// nobody, including its controller, which is precisely shroud. Drawing them
/// as two sheaths would say the card is protected in two ways when it is
/// protected in one.
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

/// What this client is offering to do with one card, right now.
///
/// Four claims about the *client's own state* rather than about the card, and
/// they travel together rather than as four arguments for the same reason
/// [`glow_of`] exists at all: they change with priority and with a tap, and a
/// caller that passed some of them would have the same card saying two
/// different things in the hand and on the table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // four claims `glow_of` ranks; each keeps its own reader
pub struct Offer {
    /// Something on this permanent can be activated right now — or, on a card
    /// in a pile, the engine offers to play, cast or activate it
    /// ([`crate::Reach::Offered`]).
    pub activatable: bool,
    /// This client would tap lands for this pile card and then cast it
    /// ([`crate::Reach::Taps`]).
    pub reachable: bool,
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
        reachable: false,
        armed: false,
        will_tap: false,
    };

    /// Only the offer the engine itself made — what a card outside the
    /// battlefield can ever have.
    #[must_use]
    pub const fn activatable(activatable: bool) -> Self {
        Self {
            activatable,
            reachable: false,
            armed: false,
            will_tap: false,
        }
    }

    /// The same offer, with this client's own on top: a pile card it would
    /// tap lands for ([`crate::Reach::Taps`]).
    ///
    /// Separate from [`Self::on`] because only a pile card can be reached
    /// for — a hand card says it with a halo, and a permanent is never cast.
    #[must_use]
    pub const fn reaching(self, reachable: bool) -> Self {
        Self { reachable, ..self }
    }

    /// What one drawn card is being offered, given what this client has
    /// armed.
    ///
    /// One reader for the three surfaces that draw a card — the table, the
    /// hand zone and the own-board overlay — for the same reason [`glow_of`]
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
    pub fn on(proposing: crate::Proposing<'_>, members: &[ObjectId], activatable: bool) -> Self {
        let spends = |plan: &baylee_client_core::manaplan::Plan| {
            plan.steps.iter().any(|step| members.contains(&step.source))
        };
        match proposing {
            crate::Proposing::Nothing => Self::activatable(activatable),
            crate::Proposing::Armed(armed) => Self {
                activatable,
                reachable: false,
                armed: members.contains(&armed.object),
                will_tap: match &armed.deed {
                    // Whichever end the run has: a land the plan spends is a
                    // land the player has to see marked before the second
                    // click.
                    crate::Deed::Run { plan, .. } => spends(plan),
                    crate::Deed::Play | crate::Deed::Ability(_) | crate::Deed::Suspend => false,
                },
            },
            // The same light for a different claim, deliberately. `WILL_TAP`
            // means "the plan would spend this", and that is exactly as true
            // of a payment window as of an armed spell — what differs is what
            // happens on the next tap, and the next tap is the player's
            // either way. Nothing is `armed` here, because nothing was: a
            // mana ability is one tap and the arming contract exempts it.
            crate::Proposing::Owed(plan) => Self {
                activatable,
                reachable: false,
                armed: false,
                will_tap: spends(plan),
            },
        }
    }
}

/// Everything a permanent is saying round itself, in one word: its
/// protection, and what this client is offering to do with it (the
/// [`Offer`]). Gathered in one function so that no caller can assemble a
/// different subset than another.
#[must_use]
pub fn glow_of(object: Option<&baylee_view::PublicObject>, offer: Offer) -> u32 {
    let from_card = object.map_or(0, |o| glow_bits(o.keywords));
    // An armed card is not also inviting a tap: the invitation was accepted,
    // and drawing both would put a travelling light and a steady one round
    // the same card saying the same thing twice.
    //
    // The engine's yes beats this client's offer for the same reason: a card
    // the engine will take as it stands needs no lands tapped for it, and
    // `Duel::reachable` leaves such a card out anyway.
    let offered = if offer.armed {
        glow::ARMED
    } else if offer.activatable {
        glow::ACTIVATABLE
    } else if offer.reachable {
        glow::REACHABLE
    } else {
        0
    };
    from_card | offered | if offer.will_tap { glow::WILL_TAP } else { 0 }
}

/// What the shader needs to know about one card.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct CardParams {
    /// 0 plain, 1 foil, 2 etched, 3 holographic, 4 glitter, 5 galaxy.
    pub finish: u32,
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
    /// The text face a card with no artwork stands in its window, packed by
    /// [`baylee_client_core::textface::face_word`], or `0` for one drawn as
    /// its flat `tint` — a back, or a slab under a pile (#259).
    pub face: u32,
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
    /// Drop materials keyed by game-local print references.
    pub(crate) fn clear_game(&mut self) {
        self.made.clear();
    }

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
                // A printing in the picker is a print: it has art to show.
                face: 0,
                tint: Vec4::ONE,
            },
        });
        if self.previewed.len() >= 256 {
            self.previewed.clear();
        }
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
    /// The flat colour, quantised, for a card with no art. `0` when it has
    /// art — a colour is not part of the key then.
    pub tint: u32,
    /// The text face it stands in its window instead of the flat colour,
    /// [`CardParams::face`]; `0` for none.
    pub face: u32,
    /// The one-shot sheen this card is in the middle of, if any.
    ///
    /// In the key because it is in the material — but it is *transient*,
    /// so a look carrying one is deliberately
    /// **not** cached: see [`UiCardMaterials::get`]. Two cards that started
    /// sweeping on the same frame still share a material, which is the
    /// opening hand's whole seven.
    pub sweep: Option<crate::sheen::Sweep>,
}

impl CardLook {
    /// A card showing artwork.
    #[must_use]
    pub fn art(art: ImageKey, finish: FinishTreatment) -> Self {
        Self {
            art: Some(art),
            finish,
            tint: 0,
            face: 0,
            sweep: None,
        }
    }

    /// A card showing a flat colour: its constructed face, or an empty slot.
    #[must_use]
    pub fn flat(color: Color, finish: FinishTreatment) -> Self {
        Self {
            art: None,
            finish,
            tint: quantise(color),
            face: 0,
            sweep: None,
        }
    }

    /// The same look standing its text face in the window (#259): the word
    /// is [`baylee_client_core::textface::face_word`]'s.
    #[must_use]
    pub fn with_face(mut self, face: u32) -> Self {
        self.face = face;
        self
    }

    /// This look with its text face standing in the window in place of its
    /// art (#259): no picture, `tint` under the face, and the face's `word`.
    /// Its finish and sweep are the card's still, as they are a print's.
    #[must_use]
    pub fn faced(self, tint: Color, word: u32) -> Self {
        Self {
            art: None,
            tint: quantise(tint),
            face: word,
            ..self
        }
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
    pub fn back(finish: FinishTreatment) -> Self {
        Self {
            art: None,
            finish,
            tint: 0,
            face: 0,
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
        FinishTreatment::Holographic => 3,
        FinishTreatment::Glitter => 4,
        FinishTreatment::Galaxy => 5,
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
            face: look.face,
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
        // The file both of them import, and the keyword strip's two shaders
        // (`marksmat`) besides. Registered here rather than loaded on demand
        // because it is not a shader in its own right: nothing sets it on a
        // pipeline, and the four that import it name it by this path.
        embedded_asset!(app, "shaders/card_common.wgsl");
        app.add_plugins(MaterialPlugin::<CardMaterial>::default())
            .add_plugins(UiMaterialPlugin::<CardUiMaterial>::default())
            .init_resource::<UiCardMaterials>()
            .add_systems(
                Update,
                (
                    track_motion,
                    dress_the_card_backs,
                    crate::card_loading::images,
                    crate::card_loading::spin,
                ),
            );
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
    use baylee_client_core::cardrail;

    #[test]
    fn game_materials_do_not_reuse_previous_games_print_references() {
        let mut cache = UiCardMaterials::default();
        let look = CardLook::art(
            ImageKey::new(
                baylee_core::ids::PrintRef::new(0),
                0,
                baylee_client_core::images::ArtSize::Small,
            ),
            FinishTreatment::Foil,
        );
        cache.made.insert(look, Handle::default());
        cache
            .previewed
            .insert(("url".into(), FinishTreatment::Foil), Handle::default());
        cache.clear_game();
        assert!(cache.made.is_empty());
        assert_eq!(
            cache.previewed.len(),
            1,
            "URL-based builder previews are independent of games"
        );
    }

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
            rules: Some(baylee_view::RulesFace {
                card: baylee_core::ids::CardIndex::new(0),
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
            base_power: Some(2),
            base_toughness: Some(2),
            loyalty: None,
            damage: 0,
            counters: Vec::new(),
            attached_to: None,
            targets: Vec::new(),
            stack_item: None,
            summoning_sick: false,
            granted_mana: None,
            board_mana: None,
            flashback: None,
            grants: Vec::new(),
        }
    }

    /// The protection word speaks for three keywords; the engine numbers
    /// over a hundred and generates that numbering. A card shelled for the
    /// wrong keyword would be a rules lie a player would believe.
    #[test]
    fn the_glow_bits_are_the_keywords_they_claim_to_be() {
        use baylee_cards_dsl::KeywordSet;
        assert_eq!(
            glow_bits(KeywordSet::INDESTRUCTIBLE.bits()),
            glow::INDESTRUCTIBLE
        );
        assert_eq!(glow_bits(KeywordSet::HEXPROOF.bits()), glow::HEXPROOF);
        assert_eq!(glow_bits(KeywordSet::SHROUD.bits()), glow::SHROUD);
        // And nothing else reaches it: flying is a mark on the strip and
        // nothing more, and a keyword in this word would be claiming a
        // protection the card does not have.
        assert_eq!(glow_bits(KeywordSet::FLYING.bits()), 0);
        assert_eq!(glow_bits(0), 0);
    }

    /// Two protections on one card are one word with both bits.
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

    /// A defender is a mark on the strip and nothing more.
    ///
    /// It used to be drawn twice — the mark, and a brick wall crossing the
    /// card's face — and the wall went with #274, because the face is the
    /// print and nothing of ours is painted on it. What is pinned is that a
    /// defender says nothing round itself, since the mark is the strip's.
    #[test]
    fn a_defender_is_a_mark_on_the_strip_and_nothing_more() {
        use baylee_cards_dsl::KeywordSet;
        use baylee_client_core::board::KeywordBadge;
        use baylee_core::types::TypeSet;
        let slot = cardrail::slot_of(KeywordBadge::Defender).expect("defender rides the strip");

        let mut wall = permanent(TypeSet::CREATURE);
        wall.keywords = KeywordSet::DEFENDER.bits();
        assert_eq!(
            glow_of(Some(&wall), Offer::NONE),
            0,
            "a defender is its mark, and no second drawing"
        );
        assert_eq!(
            cardrail::mark_bits(wall.keywords),
            1 << slot,
            "and the mark is on the strip"
        );
    }

    /// Protection and an offer are separate bits, and a card that is both
    /// wears both: they are drawn in different places on purpose.
    #[test]
    fn a_card_can_be_protected_and_useful_at_once() {
        use baylee_cards_dsl::KeywordSet;
        use baylee_core::types::TypeSet;
        let mut obj = permanent(TypeSet::CREATURE);
        obj.keywords = KeywordSet::HEXPROOF
            .union(KeywordSet::INDESTRUCTIBLE)
            .bits();
        assert_eq!(
            glow_of(Some(&obj), Offer::activatable(true)),
            glow::HEXPROOF | glow::INDESTRUCTIBLE | glow::ACTIVATABLE
        );
    }

    /// The material key has to separate what the shader draws differently and
    /// nothing else, or a board of forty Islands stops being one material.
    #[test]
    fn a_look_is_shared_by_exactly_what_looks_the_same() {
        use baylee_client_core::images::ArtSize;
        use baylee_core::ids::PrintRef;

        let key = ImageKey::new(PrintRef(0), 0, ArtSize::Normal);
        let plain = CardLook::art(key, FinishTreatment::Plain);
        assert_eq!(plain, CardLook::art(key, FinishTreatment::Plain));
        assert_ne!(
            plain,
            CardLook::art(key, FinishTreatment::Foil),
            "a foil is not the same surface as a plain card"
        );
    }

    /// Colours that a player could not tell apart must not cost two
    /// materials.
    #[test]
    fn two_colours_a_player_cannot_tell_apart_share_a_material() {
        let a = Color::srgb(0.5, 0.25, 0.125);
        let b = Color::srgb(0.5 + 1.0 / 2048.0, 0.25, 0.125);
        assert_eq!(
            CardLook::flat(a, FinishTreatment::Plain),
            CardLook::flat(b, FinishTreatment::Plain)
        );
        assert_ne!(
            CardLook::flat(a, FinishTreatment::Plain),
            CardLook::flat(Color::srgb(0.1, 0.2, 0.3), FinishTreatment::Plain)
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

    /// The same reading for a colour, which [`wgsl_const`] cannot do: its
    /// answer is one `f32` and every constant here is three.
    ///
    /// Written as a second function rather than as a general one returning a
    /// slice, because the two are asked different questions — a scalar that
    /// is secretly a triple is a bug and a triple read as a scalar is a
    /// panic, and keeping them apart means neither can quietly become the
    /// other. It accepts only the fully-spelled form
    /// `vec3<f32>(a, b, c)`; a shorthand `vec3(…)` or a splat panics here
    /// instead of reading two of the three channels as zero.
    pub(crate) fn wgsl_vec3(source: &str, name: &str) -> [f32; 3] {
        let head = format!("const {name}:");
        let line = source
            .lines()
            .map(str::trim_start)
            .find(|line| line.starts_with(&head))
            .unwrap_or_else(|| panic!("no `{name}` in the shader"));
        let body = line
            .rsplit_once('=')
            .and_then(|(_, rhs)| rhs.trim().strip_suffix(';'))
            .and_then(|rhs| rhs.trim().strip_prefix("vec3<f32>("))
            .and_then(|rhs| rhs.strip_suffix(')'))
            .unwrap_or_else(|| panic!("cannot read `{line}`"));
        let mut out = [0.0; 3];
        let mut seen = 0;
        for (slot, text) in body.split(',').enumerate() {
            assert!(slot < 3, "`{name}` has more than three channels");
            out[slot] = text
                .trim()
                .parse()
                .unwrap_or_else(|_| panic!("`{text}` in `{name}` is not a number"));
            seen += 1;
        }
        assert_eq!(seen, 3, "`{name}` has {seen} channels, not three");
        out
    }

    /// The strip is laid out twice — once in Rust so the pointer can
    /// hit-test a mark and the table can place the quad, once in WGSL so the
    /// GPU can draw one — and the two have to be the same strip. Nothing in
    /// either compiler can notice that they are.
    #[test]
    fn the_strip_is_in_the_same_place_in_both_languages() {
        let src = include_str!("shaders/card_common.wgsl");
        for (name, ours) in [
            ("STRIP_PAD", cardrail::STRIP_PAD),
            ("MARK", cardrail::MARK),
            ("MARK_GAP", cardrail::MARK_GAP),
            ("SHADOW_MARGIN", cardrail::SHADOW_MARGIN),
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
            "the shader draws a different number of marks than the strip has"
        );
        assert!(
            (wgsl_const(src, "ROW_MAX") - cardrail::ROW_MAX).abs() < 1e-6,
            "the shader opens a row at a different length than the hit test"
        );
        for (name, ours) in [
            ("LABEL_MOON", cardrail::label::MOON),
            ("LABEL_CREST_SHIFT", cardrail::label::CREST_SHIFT),
            ("LABEL_CREST_BITS", cardrail::label::CREST_BITS),
        ] {
            assert_eq!(
                wgsl_const(src, name) as u32,
                ours,
                "{name}: the shader reads the label word differently"
            );
        }
        assert_eq!(
            wgsl_const(src, "LABEL_ITEMS") as usize,
            2 + cardrail::MARK_ORDER.len() + baylee_client_core::cardcrest::MAX_CRESTS,
            "the chip, the moon, every mark and the crests"
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
        let look = CardLook::flat(Color::WHITE, FinishTreatment::Foil);

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
            "has_art",
            "strength",
            "motion",
            "sweep_at",
            "sweep_rate",
            "sweep_door",
            "face",
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
            // And the sheen is all that is left of the metal. The coating
            // every card used to wear lifted the print's blacks, the artist's
            // line and the copyright line included, and the owner took it off
            // entirely (#274): light may pass over the print, but nothing may
            // stay on it.
            for gone in ["METAL_FLOOR", "METAL_GRAIN", "let brushed"] {
                assert!(!src.contains(gone), "{which} coats the print again: {gone}");
            }
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

    /// Every flag is a bit of its own, and every offer the same number on
    /// both sides.
    ///
    /// Two copies of the same table — one Rust, one WGSL — and nothing in
    /// either compiler can notice when one of them moves. A wrong number here
    /// has no error and no crash: the felt lights a card for an offer it does
    /// not have, or two flags share a bit and one offer draws as another.
    /// `floor_light` in `card_common.wgsl` is the one reader (#298), and no
    /// other shader may keep a table of its own that could drift from it.
    #[test]
    fn the_glow_flags_are_the_same_number_on_both_sides() {
        let common = include_str!("shaders/card_common.wgsl");
        let mut taken = 0u32;
        for (name, ours, drawn) in [
            ("GLOW_INDESTRUCTIBLE", glow::INDESTRUCTIBLE, false),
            ("GLOW_HEXPROOF", glow::HEXPROOF, false),
            ("GLOW_SHROUD", glow::SHROUD, false),
            ("GLOW_ACTIVATABLE", glow::ACTIVATABLE, true),
            ("GLOW_ARMED", glow::ARMED, true),
            ("GLOW_WILL_TAP", glow::WILL_TAP, true),
            ("GLOW_REACHABLE", glow::REACHABLE, true),
        ] {
            if drawn {
                let theirs = wgsl_const(common, name);
                assert!(
                    (theirs - ours as f32).abs() < f32::EPSILON,
                    "{name}: {ours} here, {theirs} in card_common.wgsl"
                );
                assert_ne!(
                    ours & glow::OFFERS,
                    0,
                    "{name} is an offer the felt is not lit for"
                );
            }
            assert_eq!(ours.count_ones(), 1, "{name} is not one bit");
            assert_eq!(ours & taken, 0, "{name} shares a bit with another flag");
            taken |= ours;
        }
        assert_eq!(
            glow::OFFERS.count_ones(),
            4,
            "the felt is lit for four offers, and the protection is not one"
        );
        for (which, src) in [
            ("card.wgsl", include_str!("shaders/card.wgsl")),
            ("card_ui.wgsl", include_str!("shaders/card_ui.wgsl")),
            ("floor.wgsl", include_str!("shaders/floor.wgsl")),
        ] {
            assert!(
                !src.contains("const GLOW_"),
                "{which} keeps a glow table of its own beside the shared one"
            );
        }
    }

    /// Nothing is drawn on the print but its own finish, and light that
    /// passes over (#274, #298).
    ///
    /// The owner's rule, and Scryfall's: a card image is not covered, tinted,
    /// dimmed or stamped. Each card shader samples the print edge to edge
    /// into `color` and gives it its finish; after that the colour may be
    /// touched only by the lamp and the arrival sweep (`METAL_TONE`), a door
    /// (`door_layer`) and the card's own corner — all of them light that
    /// leaves nothing behind, or ink outside the card. Everything else this
    /// client says about a card stands on an object of its own: the strip,
    /// the badge, the light on the felt.
    ///
    /// Read as text, because what it holds is a composition and not a
    /// number: a rail or a night laid on the colour after the finish draws
    /// perfectly well, compiles, and is exactly the fault. The live half —
    /// the same card rendered with every state on and off, diffed over the
    /// print — is in `docs/client.md` §"The print fills the card".
    #[test]
    fn nothing_but_the_finish_is_drawn_on_the_print() {
        for (which, src, angle) in [
            ("card.wgsl", include_str!("shaders/card.wgsl"), "facing"),
            ("card_ui.wgsl", include_str!("shaders/card_ui.wgsl"), "tilt"),
        ] {
            let body = src
                .split_once("fn fragment(")
                .unwrap_or_else(|| panic!("{which} has no fragment"))
                .1;
            let lines: Vec<&str> = body.lines().map(str::trim).collect();
            let writes = |from: &[&'static str]| -> Vec<&'static str> {
                from.iter()
                    .copied()
                    .filter(|l| l.starts_with("color") || l.starts_with("var color"))
                    .collect()
            };

            assert!(
                lines.contains(&"let sampled = textureSample(art, art_sampler, uv);"),
                "{which} samples its art somewhere other than edge to edge"
            );
            // What stands in for a print where there is none (#259): the flat
            // colour, or the text face. It is weighed by `1 - has_art`, so it
            // never lies on a print.
            let flats: Vec<&str> = lines
                .iter()
                .copied()
                .filter(|l| l.starts_with("let flat") || l.starts_with("flat ="))
                .collect();
            assert_eq!(
                flats,
                [
                    "let flat = select(params.tint, vec4<f32>(text_face(uv, params.face), 1.0), (params.face & FACE_ON) != 0u);"
                ],
                "{which} stands something else in for a print"
            );

            // The print: sampled, finished, and nothing more.
            let finish = format!(
                "color = print_finish(color, uv, {angle}, t, params.finish, params.strength);"
            );
            let finished = lines
                .iter()
                .position(|l| *l == finish)
                .unwrap_or_else(|| panic!("{which} no longer finishes the print"));
            assert_eq!(
                writes(&lines[..finished]),
                ["var color = mix(flat, sampled, params.has_art);"],
                "{which} writes to the print before its finish"
            );
            let after = writes(&lines[finished + 1..]);
            assert!(
                after.len() >= 3,
                "only {} writes after the finish in {which} — the scan has gone blind",
                after.len()
            );
            // A statement split over lines is read by its lines: the opening
            // `color = vec4<f32>(` and the closing `color.a,` say nothing, and
            // the line between them has to name its light.
            for line in after {
                assert!(
                    ["color = vec4<f32>(", "color.a,"].contains(&line)
                        || ["METAL_TONE", "door_layer(", "corner_sdf(uv)", "EDGE_INK"]
                            .iter()
                            .any(|allowed| line.contains(allowed)),
                    "{which} draws on the print after its finish: {line}"
                );
            }

            // And the frame's inputs are gone from the card with the frame.
            for field in ["params.glow", "params.plate", "params.chips", "print_cover"] {
                assert!(
                    !src.contains(field),
                    "{which} reads {field}: the card says that on an object of its own"
                );
            }
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
            reachable: false,
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
            reachable: false,
            armed: false,
            will_tap: true,
        };
        assert_eq!(
            glow_of(Some(&obj), paying),
            glow::ACTIVATABLE | glow::WILL_TAP
        );
    }

    /// This client's offer is drawn only where nothing stronger is: the
    /// engine's yes wins over it, and so does the deed it becomes once armed.
    ///
    /// Three answers from one card, because each is a way to draw two lights
    /// on one border — a chase in two hues at once, or a chase still running
    /// round a card that has stopped inviting anything.
    #[test]
    fn a_reachable_card_is_lit_only_where_nothing_stronger_is() {
        use baylee_core::types::TypeSet;
        let obj = permanent(TypeSet::INSTANT);
        let reached = Offer::NONE.reaching(true);
        assert_eq!(glow_of(Some(&obj), reached), glow::REACHABLE);
        assert_eq!(
            glow_of(Some(&obj), Offer::activatable(true).reaching(true)),
            glow::ACTIVATABLE,
        );
        let armed = Offer {
            armed: true,
            ..reached
        };
        assert_eq!(glow_of(Some(&obj), armed), glow::ARMED);
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
        let offer = Offer::on(crate::Proposing::Armed(&run), &forests, false);
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
            Offer::on(crate::Proposing::Armed(&other), &forests, true),
            Offer::activatable(true)
        );

        // A deed that is not a run spends nothing, whatever it is aimed at.
        let play = crate::Armed {
            object: forests[0],
            deed: crate::Deed::Play,
        };
        let offer = Offer::on(crate::Proposing::Armed(&play), &forests, false);
        assert!(offer.armed && !offer.will_tap);
    }

    /// The same question of an **owed** plan rather than an armed one, which
    /// is the fourth corner of a square that had three.
    ///
    /// `Armed` over a stack is the test above; `Owed` over a single land is
    /// `owed_tests`; `Owed` over a card standing for several was held by
    /// nothing. The two arms share one `spends` closure, so this is cheap —
    /// and "cheap and untested" is exactly the shape #105 is about, so it is
    /// written rather than argued.
    #[test]
    fn a_card_standing_for_several_lands_lights_when_an_owed_plan_taps_one() {
        use baylee_client_core::manaplan::{Plan, Step, Tap};
        let forests: Vec<ObjectId> = (1..=3).map(|i| ObjectId::new(i, 0)).collect();
        let plan = Plan {
            steps: vec![Step {
                source: forests[2],
                tap: Tap::Intrinsic,
                color: None,
            }],
            ..default()
        };
        let offer = Offer::on(crate::Proposing::Owed(&plan), &forests, false);
        assert!(offer.will_tap, "one of the three is named by the plan");
        assert!(!offer.armed, "an open window is not a commitment");

        let elsewhere = [ObjectId::new(9, 0)];
        assert_eq!(
            Offer::on(crate::Proposing::Owed(&plan), &elsewhere, false),
            Offer::activatable(false),
            "a card the plan does not name is drawn dark"
        );
    }

    /// Every mark the strip carries is the keyword it claims to be, the
    /// three protections are marks too since #298 took the paper away, and
    /// none of the combat marks reaches the protection word.
    #[test]
    fn the_strip_carries_the_keywords_it_says_it_does() {
        use baylee_cards_dsl::KeywordSet;
        let slot = |set: KeywordSet| cardrail::mark_bits(set.bits());
        assert_eq!(slot(KeywordSet::FLYING), 1 << 0);
        assert_eq!(slot(KeywordSet::DEATHTOUCH), 1 << 3);
        assert_eq!(slot(KeywordSet::DEFENDER), 1 << 10);
        // Prowess is bit 23 of the engine's word and slot 11 of the strip,
        // which is the whole reason the two numberings are pinned rather
        // than assumed to be the same list.
        assert_eq!(slot(KeywordSet::PROWESS), 1 << 11);
        // The paper's three were appended when the paper went (#298): the
        // index is the wire and the atlas cell, so they could not go in
        // front.
        assert_eq!(slot(KeywordSet::HEXPROOF), 1 << 12);
        assert_eq!(slot(KeywordSet::INDESTRUCTIBLE), 1 << 13);
        assert_eq!(slot(KeywordSet::SHROUD), 1 << 14);
        // And a creature wearing six combat keywords is one word with six
        // bits in it, and no protection.
        let six = KeywordSet::FLYING
            .union(KeywordSet::TRAMPLE)
            .union(KeywordSet::LIFELINK)
            .union(KeywordSet::VIGILANCE)
            .union(KeywordSet::HASTE)
            .union(KeywordSet::MENACE);
        assert_eq!(slot(six).count_ones(), 6, "six keywords, six marks");
        assert_eq!(glow_bits(six.bits()), 0);
    }

    /// A mark that takes the phase and never uses it has to say why.
    ///
    /// Three did, when each mark was its own function: first strike, double
    /// strike and defender all took `ph` and ignored it, and nothing in the
    /// file said whether that was a decision or an omission — two were
    /// omissions, the third is the design, and no reader could tell them
    /// apart. The rail is the one place in this client where "it does not
    /// animate" is both a legitimate answer and the signature of unfinished
    /// work.
    ///
    /// The twelve functions are gone and the question is not: `mark_pulse`
    /// is one switch with an arm per mark, and an arm that returns a constant is
    /// the same silence in one line instead of thirty. So a still arm
    /// declares itself with `STILL` in the comment above it. This is the
    /// build-time half of a claim that otherwise needs a camera: it cannot
    /// say a motion is *visible*, which is what the measurements in that
    /// file's header are for, but it can say that nobody added a mark and
    /// quietly left the phase on the floor.
    #[test]
    fn the_rail_declares_every_mark_that_does_not_move() {
        let src = include_str!("shaders/card_common.wgsl");
        let open = src.find("fn mark_pulse(").expect("the pulse table");
        let body = &src[open..];
        let body = &body[..body.find("\n}").expect("a brace at column zero")];

        let mut seen = 0;
        let mut still = Vec::new();
        for (at, _) in body.match_indices("\n        case ") {
            let arm = &body[at + 1..];
            let slot: usize = arm["        case ".len()..]
                .split('u')
                .next()
                .expect("a slot number")
                .parse()
                .expect("a slot number");
            let end = arm
                .find("\n        }")
                .unwrap_or_else(|| arm.find('\n').expect("an arm ends on its own line"));
            let arm = &arm[..end];
            seen += 1;
            // `ph` as a token and not as a substring: `graph` is not a phase.
            let moves = arm.match_indices("ph").any(|(i, _)| {
                let before = arm[..i].chars().next_back();
                let after = arm[i + 2..].chars().next();
                let word = |c: char| c.is_alphanumeric() || c == '_';
                !before.is_some_and(word) && !after.is_some_and(word)
            });
            if moves {
                continue;
            }
            // The comment is the run of `//` lines above the arm. Measured
            // from the newline the arm starts on, not from the arm itself:
            // a slice that ends in `\n` splits to an empty last line and the
            // run stops before it has read anything.
            let note = body[..at]
                .rsplit('\n')
                .take_while(|line| line.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                note.contains("STILL"),
                "slot {slot} ignores its phase and does not say why"
            );
            still.push(slot);
        }

        assert_eq!(
            seen,
            cardrail::MARK_ORDER.len(),
            "a mark on the strip, an arm"
        );
        assert_eq!(
            still,
            [10, 12, 13, 14],
            "which marks hold still is a decision, and this is the list of it"
        );
        assert_eq!(
            baylee_client_core::cardrail::MARK_ORDER[10],
            baylee_client_core::board::KeywordBadge::Defender,
            "slot 10 is the shield, and the stillness is drawn for a shield"
        );
    }

    /// Between choosing a mark and drawing its ink, nothing asks which mark
    /// it is.
    ///
    /// This is the half of the stillness claim that had no test, and #102 is
    /// what it cost. `mark_pulse`'s slot 10 arm returns a flat `1.0` and
    /// declares `STILL`, and
    /// [`the_rail_declares_every_mark_that_does_not_move`] holds that —
    /// honestly, because it is a test about a scale. But the ink under every
    /// mark is `mix(INK, accent, 0.70) * (0.95 + 0.05 * sin(phase))` with no
    /// `which` in it, so defender breathed with the rest of the row while two
    /// comments and a test said it was the one thing on the rail that did
    /// not move.
    ///
    /// The number is why the answer was to correct the claim rather than add
    /// the guard: that breath is 10.8 display levels peak to peak on slot
    /// 10's warm stone, under a floor of 20. It is below what a player can
    /// read, which makes it a false sentence and not a visible defect — and
    /// makes a guard a change nobody could see.
    ///
    /// So what is pinned here is the **absence**. `which` is read exactly
    /// twice between the mark being chosen and the ink — the glyph and the
    /// accent — and a third use is somebody making the drawing depend on the
    /// mark's identity. That is a legitimate thing to want (#23 is open, and
    /// #24's lane wave rides the same `phase`), which is precisely why it
    /// should not be able to land quietly: this test going red *is* the
    /// event, and whoever turns it green owes `mark_pulse`'s note the
    /// sentence that is true afterwards.
    #[test]
    fn the_ink_below_a_mark_is_not_told_which_mark_it_is() {
        let src = include_str!("shaders/card_common.wgsl");
        let open = src.find("fn label_strip(").expect("the strip");
        let body = &src[open..];
        let body = &body[..body.find("\n}").expect("a brace at column zero")];

        // The window: after the line that names the mark the point is in, up
        // to and including the ink. Everything before it is *choosing* the
        // mark, and reading which one there is the point.
        let bail = body.find("let which = hit - 2u;").expect("the mark chosen");
        let after = body[bail..].find(';').expect("the line ends") + bail + 1;
        let ink = body.find("let ink =").expect("the ink");
        let ink = body[ink..].find(';').expect("the ink ends") + ink + 1;
        assert!(after < ink, "the mark is chosen before it is inked");

        // WGSL has no string literals, so a line is code up to its `//`.
        let code = body[after..ink]
            .lines()
            .map(|line| line.split("//").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");

        let uses = code
            .match_indices("which")
            .filter(|(i, _)| {
                let word = |c: char| c.is_alphanumeric() || c == '_';
                let before = code[..*i].chars().next_back();
                let after = code[i + "which".len()..].chars().next();
                !before.is_some_and(word) && !after.is_some_and(word)
            })
            .count();

        assert_eq!(
            uses, 2,
            "between the choice and the ink, `which` should be read exactly \
             twice — `mark_sdf(which, ..)` for the glyph and `mark_color(which)` \
             for the accent. {uses} means the ink now depends on which mark it \
             is drawing, which is #23's decision and #102's claim: say so in \
             `mark_pulse`'s note before making this number agree with you."
        );
    }

    /// The shader reads the atlas the baker writes.
    ///
    /// Two numbers with no compiler between them, and each fails in its own
    /// quiet way. A cell size out of step makes the half-texel inset the
    /// wrong width, so a mark fetches a sliver of its neighbour along one
    /// wall — which at ten pixels looks like a smudge and not like a bug.
    /// A range out of step rescales every distance the rail has: the ink
    /// edge moves, the halo changes size, and nothing anywhere errors.
    #[test]
    fn the_shader_and_the_baker_agree_about_the_atlas() {
        let src = include_str!("shaders/card_common.wgsl");
        assert!(
            (wgsl_const(src, "MARK_CELL") - crate::markatlas::CELL as f32).abs() < 1e-5,
            "one mark's square, in texels"
        );
        assert!(
            (wgsl_const(src, "MARK_RANGE") - crate::markatlas::RANGE).abs() < 1e-5,
            "how far either side of the outline the atlas encodes"
        );
        // And the atlas is exactly as many cells wide as the shader
        // indexes: it divides by `ATLAS_CELLS` to find a column, and the
        // corner's characters start at `TEXT_BASE` — so a cell added to
        // either half without the other number moving hands every glyph a
        // slice of its neighbour.
        let (wide, _) = crate::markatlas::atlas_size();
        assert_eq!(
            wide,
            wgsl_const(src, "ATLAS_CELLS") as u32 * crate::markatlas::CELL as u32
        );
        assert_eq!(
            wgsl_const(src, "TEXT_BASE") as usize + baylee_client_core::cardplate::TEXT_CHARS.len(),
            wgsl_const(src, "CREST_BASE") as usize,
            "the text half does not end where the identity column starts"
        );
        assert_eq!(
            wgsl_const(src, "CREST_BASE") as usize + baylee_client_core::cardcrest::GLYPH_COUNT,
            crate::markatlas::CELLS,
            "the identity column does not end where the row does"
        );
        assert_eq!(
            wgsl_const(src, "CREST_BASE") as usize,
            crate::markatlas::CREST_BASE,
            "the shader and the baker start the identity column in different cells"
        );
        assert!(
            (wgsl_const(src, "TEXT_RANGE") - baylee_client_core::cardplate::TEXT_RANGE).abs()
                < 1e-5,
            "how far either side of an outline a text cell encodes"
        );
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
    /// constant that drifts puts the plate off its quad. The
    /// packing is checked from the other side by
    /// `cardplate::tests::every_number_survives_the_packing`; this is the
    /// half that checks the shader agrees about where the bits are.
    #[test]
    fn the_plate_is_the_same_plate_in_both_languages() {
        use baylee_client_core::cardplate as plate;
        let src = include_str!("shaders/card_common.wgsl");

        for (name, ours) in [
            ("PLATE_W", plate::PLATE_W),
            ("PLATE_H", plate::PLATE_H),
            ("PLATE_PAD", plate::PLATE_PAD),
            ("PLATE_CAP", plate::PLATE_CAP),
            ("CHIP_GAP", plate::CHIP_GAP),
            ("CHIP_W", plate::CHIP_W),
            ("BADGE_H", plate::BADGE_H),
            ("BADGE_CORNER", plate::BADGE_CORNER),
            ("BADGE_DROP_X", plate::BADGE_DROP[0]),
            ("BADGE_DROP_Y", plate::BADGE_DROP[1]),
            ("BADGE_BLUR", plate::BADGE_BLUR),
            ("BADGE_W", plate::BADGE_W),
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
            ("ROMAN_MAX", u32::from(plate::ROMAN_MAX)),
            ("TEXT_BASE", crate::markatlas::TEXT_BASE as u32),
            ("TEXT_COUNT", plate::TEXT_CHARS.len() as u32),
            ("ATLAS_CELLS", crate::markatlas::CELLS as u32),
            ("GLYPH_MINUS", plate::GLYPH_MINUS as u32),
            ("GLYPH_SLASH", plate::GLYPH_SLASH as u32),
            ("GLYPH_PLUS", plate::GLYPH_PLUS as u32),
            ("GLYPH_I", plate::GLYPH_I as u32),
            ("GLYPH_V", plate::GLYPH_V as u32),
            ("GLYPH_TIMES", plate::GLYPH_TIMES as u32),
            ("COUNT_MIN", plate::COUNT_MIN),
            ("COUNT_MAX", plate::COUNT_MAX),
            ("SWING_SET", plate::SWING_SET),
            ("TONE_SHIFT", plate::TONE_SHIFT),
            ("TONE_PLAIN", plate::TONE_PLAIN),
            ("TONE_DEADLY", plate::TONE_DEADLY),
            ("TONE_TOXIC", plate::TONE_TOXIC),
            ("PLATE_NIGHT", plate::PLATE_NIGHT),
            ("PLATE_TURNED", plate::PLATE_TURNED),
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

        // The corner's typesetting: what the shader has to know about the
        // face to lay a line of it out. An advance that drifted here would
        // not fail to draw — it would draw the same characters a fraction
        // out of step with one another, which is precisely the complaint
        // this whole change answers, so it is worth a mirror of its own.
        for (name, ours) in [
            ("TEXT_RANGE", plate::TEXT_RANGE),
            ("TEXT_CAP", plate::TEXT_CAP),
            ("TEXT_ADV_DIGIT", plate::TEXT_ADV[0]),
            ("TEXT_ADV_MINUS", plate::TEXT_ADV[plate::GLYPH_MINUS]),
            ("TEXT_ADV_SLASH", plate::TEXT_ADV[plate::GLYPH_SLASH]),
            ("TEXT_ADV_PLUS", plate::TEXT_ADV[plate::GLYPH_PLUS]),
            ("TEXT_ADV_I", plate::TEXT_ADV[plate::GLYPH_I]),
            ("TEXT_ADV_V", plate::TEXT_ADV[plate::GLYPH_V]),
            ("TEXT_ADV_TIMES", plate::TEXT_ADV[plate::GLYPH_TIMES]),
        ] {
            let theirs = wgsl_const(src, name);
            assert!(
                (theirs - ours).abs() < 1e-5,
                "{name}: {ours} here, {theirs} in the shader"
            );
        }
    }

    /// Parchment is one material, and the card and the interface draw it from
    /// one number.
    ///
    /// A saga's chapter page is parchment on a card; a prompt slip and the
    /// ability sheet are parchment in the interface. They were two colours —
    /// #E0D4B0 in the shader and #EDE3CC in the palette — which is a drift
    /// nothing could see, because the two are never touching on screen. What
    /// makes it worth a test is that they answer the same question: a player
    /// who has learned what paper looks like in this game has learned one
    /// thing, and two papers is a fact they have to hold twice.
    ///
    /// Read out of the WGSL rather than restated here, the way the rail and
    /// the plate are already held — a constant copied into a test agrees with
    /// whatever it was copied from, including a mistake.
    #[test]
    fn the_parchment_is_the_same_paper_in_both_languages() {
        let src = include_str!("shaders/card_common.wgsl");
        let page = wgsl_rgb(src, "PARCHMENT");
        let sheet = crate::hud::palette::PARCHMENT.to_srgba();
        for (channel, (shader, ui)) in [
            (page[0], sheet.red),
            (page[1], sheet.green),
            (page[2], sheet.blue),
        ]
        .into_iter()
        .enumerate()
        {
            assert!(
                (shader - ui).abs() < 1e-6,
                "channel {channel}: the card draws parchment at {shader} and \
                 the interface paints it at {ui}"
            );
        }
        // And the generated sheet the interface stretches over that fill is
        // the same paper again in its middle, which is the seam a flat
        // border round a grained middle used to show.
        let grain = baylee_client_core::tabletop::parchment(64);
        let middle = grain.pixel(32, 32);
        for (channel, (fill, drawn)) in [
            (page[0], middle[0]),
            (page[1], middle[1]),
            (page[2], middle[2]),
        ]
        .into_iter()
        .enumerate()
        {
            assert!(
                (fill - drawn).abs() < 0.06,
                "channel {channel}: the fill is {fill} and the sheet over it \
                 is {drawn}"
            );
        }
    }

    /// One `vec3<f32>` literal out of the shader.
    fn wgsl_rgb(source: &str, name: &str) -> [f32; 3] {
        let line = source
            .lines()
            .map(str::trim_start)
            .find(|line| line.starts_with(&format!("const {name}:")))
            .unwrap_or_else(|| panic!("the shader has no {name}"));
        let Some((inside, _)) = line
            .rsplit_once("vec3<f32>(")
            .and_then(|(_, tail)| tail.split_once(')'))
        else {
            panic!("{name} is not a vec3 literal: {line}")
        };
        let parts: Vec<f32> = inside
            .split(',')
            .map(|part| part.trim().parse().expect("a number"))
            .collect();
        [parts[0], parts[1], parts[2]]
    }

    /// Rec. 709 relative luminance — how bright a colour is, which is the
    /// question "which of these two is the figure" actually asks.
    fn luma(c: [f32; 3]) -> f32 {
        0.2126f32.mul_add(c[0], 0.7152f32.mul_add(c[1], 0.0722 * c[2]))
    }

    /// Hue in degrees, for the one comparison that is about colour rather
    /// than brightness: a damaged creature must not be read as a
    /// planeswalker.
    fn hue(c: [f32; 3]) -> f32 {
        let high = c[0].max(c[1]).max(c[2]);
        let low = c[0].min(c[1]).min(c[2]);
        let span = high - low;
        assert!(span > 0.0, "a grey has no hue");
        let raw = if (high - c[0]).abs() < f32::EPSILON {
            (c[1] - c[2]) / span
        } else if (high - c[1]).abs() < f32::EPSILON {
            (c[2] - c[0]) / span + 2.0
        } else {
            (c[0] - c[1]) / span + 4.0
        };
        (raw * 60.0).rem_euclid(360.0)
    }

    /// How wide a card is drawn, in physical pixels, at the two distances
    /// the plate has to work at.
    ///
    /// Measured on this machine rather than derived: `/state` reports a
    /// permanent on a duel's battlefield at about 47 logical pixels across,
    /// and the hover preview asks for 308 before its own fit, both on a
    /// display at scale 2. They are the whole reason the constants below
    /// are what they are, so they are written down beside them.
    const TABLE_CARD_PX: f32 = 94.0;
    const PREVIEW_CARD_PX: f32 = 616.0;

    /// Marked damage is drawn as the **figure** on the plate and the
    /// numerals standing in it as its ground, and that is the whole of the
    /// fix — not a louder colour.
    ///
    /// It was a rising fill of `EMBER` at 58% *behind* near-white numerals,
    /// which composites to a luminance of about 0.25 against the body's
    /// 0.05 and the ink's 0.96: the brightest thing on the plate was the
    /// numerals, so the numerals were what the eye read and the damage was
    /// a slightly warmer dark behind them. The owner played whole games
    /// without seeing it, which is the correct reading of that picture.
    ///
    /// So the three claims here are about the ordering of brightnesses and
    /// not about any one of them, and each is bounded on both sides: a band
    /// far brighter than the body it covers, a digit inside it still well
    /// clear of the band, and a hue nothing else in this corner owns.
    #[test]
    fn the_damage_band_is_the_figure_and_its_numerals_are_the_ground() {
        let src = include_str!("shaders/card_common.wgsl");
        let heat = wgsl_rgb(src, "HEAT");
        let plate = wgsl_rgb(src, "PLATE");
        let ink = wgsl_rgb(src, "INK");
        let gilt = wgsl_rgb(src, "GILT");

        // One number read twice, because a digit inside the band is drawn
        // in the plate's own colour: it is the band against the body it
        // covers, and it is the contrast that digit keeps standing in it.
        // Seven is the bound and these colours give nine — against the
        // nineteen a white digit keeps on the bare plate, and the four an
        // ink digit kept over the old 58% fill.
        let lit = luma(heat) / luma(plate);
        assert!(
            lit >= 7.0,
            "the band is only {lit} times the brightness of the plate, so \
             neither it nor the digits standing in it read"
        );

        // The other half of the inversion, and the one that says the
        // ordering rather than the ratio: the ink stays the brightest thing
        // the plate can draw, so a digit *outside* the band is still the
        // figure there. A band brighter than the ink would swap the two
        // back the other way round.
        assert!(
            luma(ink) > luma(heat),
            "the band is brighter than the ink it is read against"
        );

        // And laid on whole. The old fill was this colour at 58%, which is
        // the single edit that would put the damage back behind the
        // numerals while every constant above still passed.
        assert!(
            src.contains("out = mix(out, HEAT, band);"),
            "the band is no longer laid on at the band mask alone"
        );

        // Red of the gilt a planeswalker's plate is rimmed with, by enough
        // that the two are never one glance apart.
        let (hot, gold) = (hue(heat), hue(gilt));
        assert!(
            hot + 20.0 < gold,
            "the band sits at {hot}° against the rim's {gold}°"
        );
    }

    /// A point of damage is drawn at the table, and how *many* points is
    /// answered where there is room to answer it.
    ///
    /// Both halves are bounded on both sides, because both fail in two
    /// directions: a floor too small says nothing and a floor too big makes
    /// one damage on a twelve look like one damage on a two; a tick
    /// threshold too high rules a plate eighteen pixels wide into invisible
    /// rows and one too low never rules the preview at all.
    #[test]
    fn the_band_says_marked_at_the_table_and_how_much_in_the_preview() {
        use baylee_client_core::cardplate as plate;
        let src = include_str!("shaders/card_common.wgsl");
        let floor = wgsl_const(src, "DAMAGE_FLOOR");
        let tick_aa = wgsl_const(src, "TICK_AA");
        let tick_max = wgsl_const(src, "TICK_MAX");

        let pixels = floor * TABLE_CARD_PX;
        assert!(
            pixels >= 2.0,
            "one point of damage is {pixels} pixels tall at the table"
        );
        assert!(
            floor < plate::PLATE_H / 2.0,
            "the floor is {floor} of a {} plate, so a point of damage on a \
             big creature looks like a point on a small one",
            plate::PLATE_H
        );

        // `aa` is the pixel size in card widths, so the threshold is read
        // straight against the two distances.
        assert!(
            tick_aa < 1.0 / TABLE_CARD_PX,
            "the rules would be drawn on the table, {tick_aa} against {}",
            1.0 / TABLE_CARD_PX
        );
        assert!(
            tick_aa > 1.0 / PREVIEW_CARD_PX,
            "the preview is never ruled, {tick_aa} against {}",
            1.0 / PREVIEW_CARD_PX
        );

        // And the rows a ruled band can actually be counted in.
        let row = plate::PLATE_H / tick_max * PREVIEW_CARD_PX;
        assert!(
            row >= 3.0,
            "the tallest ruled creature has rows {row} pixels apart"
        );
    }

    /// The text face is the same face in both languages (#259).
    ///
    /// The shader draws the bars and the table's `Text2d` lines stand on
    /// them, placed by `textface`: a bar that drifted a hundredth in either
    /// file puts a name half off its bar, on a machine where nothing fails.
    /// The hues are read out of `face_hue`'s switch by their code, so a
    /// shader that swapped two draws no blue card red.
    #[test]
    fn the_text_face_is_the_same_face_in_both_languages() {
        use baylee_client_core::textface::{self as face, Hue};
        let src = include_str!("shaders/card_common.wgsl");
        for (name, ours) in [
            ("TEXT_BORDER", face::BORDER),
            ("TEXT_SEAM", face::seam()),
            ("DEPTH_STEP", face::DEPTH_STEP),
            ("TEXT_PINLINE", face::PINLINE),
            ("TEXT_BOX_GAP", face::BOX_GAP),
            ("TEXT_FOOT", face::TEXT_FOOT),
            ("BAR_CORNER", face::BAR_CORNER),
            ("PAPER_MIX", face::PAPER_MIX),
            ("ART_TOP", face::ART_TOP),
            ("ART_FOOT", face::ART_FOOT),
            ("CLOTH", face::CLOTH),
            ("BEVEL", face::BEVEL),
        ] {
            let theirs = wgsl_const(src, name);
            assert!(
                (theirs - ours).abs() < 1e-6,
                "{name}: {ours} here, {theirs} in the shader"
            );
        }
        #[allow(clippy::cast_precision_loss)] // four small words
        for (name, ours) in [
            ("FACE_ON", face::FACE_ON),
            ("FACE_BARS_SHIFT", face::FACE_BARS_SHIFT),
            ("FACE_NAME_SHIFT", face::FACE_NAME_SHIFT),
            ("FACE_TYPE_SHIFT", face::FACE_TYPE_SHIFT),
        ] {
            let theirs = wgsl_const(src, name);
            assert!(
                (theirs - ours as f32).abs() < 1e-6,
                "{name}: {ours} here, {theirs} in the shader"
            );
        }
        let hues = [
            (Hue::White, "FACE_WHITE"),
            (Hue::Blue, "FACE_BLUE"),
            (Hue::Black, "FACE_BLACK"),
            (Hue::Red, "FACE_RED"),
            (Hue::Green, "FACE_GREEN"),
            (Hue::Gold, "FACE_GOLD"),
            (Hue::Grey, "FACE_GREY"),
        ];
        let tones = hues.iter().map(|(hue, name)| (*name, hue.tone())).chain([
            ("PAPER_WHITE", face::PAPER_WHITE),
            ("BORDER_INK", face::BORDER_INK),
        ]);
        for (name, ours) in tones {
            let theirs = wgsl_vec3(src, name);
            for c in 0..3 {
                assert!(
                    (theirs[c] - ours[c]).abs() < 1e-5,
                    "{name}: {ours:?} here, {theirs:?} in the shader"
                );
            }
        }
        // Each bar is drawn as deep as its own byte says, the byte the text
        // was placed by.
        for line in [
            "let name_end = top + f32((word >> FACE_NAME_SHIFT) & 0xffu) * DEPTH_STEP;",
            "let type_end = TEXT_SEAM + f32((word >> FACE_TYPE_SHIFT) & 0xffu) * DEPTH_STEP;",
        ] {
            assert!(src.contains(line), "`text_face` no longer says `{line}`");
        }
        // Grey is the switch's default, so a code the table does not know
        // is drawn as no colour rather than as white.
        for (hue, name) in hues {
            let arm = if hue == Hue::Grey {
                format!("default: {{ return {name}; }}")
            } else {
                format!("case {}u: {{ return {name}; }}", hue as u32)
            };
            assert!(src.contains(&arm), "`face_hue` has no `{arm}`");
        }
    }

    /// The face word a look carries reaches the shader's uniform, and only a
    /// look that asked for the face carries one: a print and a back leave
    /// the window to their picture (#259).
    #[test]
    fn a_face_look_carries_its_word_to_the_uniform() {
        use baylee_client_core::images::ArtSize;
        use baylee_client_core::textface::{FACE_NAME_SHIFT, FACE_ON};
        use baylee_core::ids::PrintRef;

        let tint = Color::srgb(0.2, 0.3, 0.4);
        let word = FACE_ON | 5 << 4 | 133 << FACE_NAME_SHIFT;
        let face = CardLook::flat(tint, FinishTreatment::Plain).with_face(word);
        assert_eq!(material(face, None, tint, 1.0).params.face, word);
        assert_ne!(face, CardLook::flat(tint, FinishTreatment::Plain));

        let key = ImageKey::new(PrintRef(0), 0, ArtSize::Normal);
        for look in [
            CardLook::flat(tint, FinishTreatment::Plain),
            CardLook::art(key, FinishTreatment::Plain),
            CardLook::back(FinishTreatment::Plain),
        ] {
            assert_eq!(material(look, None, tint, 1.0).params.face, 0, "{look:?}");
        }
    }

    /// The crests' papers and ink are the same in both languages, and the
    /// crests are where the atlas put them.
    ///
    /// The papers are keyed by the glyph index rather than written out in
    /// order, so a shader that swapped two of them fails here rather than
    /// shipping a copy on verdigris.
    #[test]
    fn the_identity_papers_are_the_same_in_both_languages() {
        use baylee_client_core::cardcrest as crest;

        let src = include_str!("shaders/card_common.wgsl");
        for (name, ours) in [
            ("CREST_BASE", crate::markatlas::CREST_BASE),
            ("CREST_TOKEN", crest::GLYPH_TOKEN),
            ("CREST_COPY", crest::GLYPH_COPY),
            ("CREST_COMMANDER", crest::GLYPH_COMMANDER),
        ] {
            assert!(
                (wgsl_const(src, name) - ours as f32).abs() < 0.5,
                "{name} differs between the two files"
            );
        }
        for (name, ours) in [
            ("PAPER_TOKEN", crest::PAPER[crest::GLYPH_TOKEN]),
            ("PAPER_COPY", crest::PAPER[crest::GLYPH_COPY]),
            ("PAPER_COMMANDER", crest::PAPER[crest::GLYPH_COMMANDER]),
            ("CREST_INK", crest::CREST_INK),
        ] {
            let theirs = wgsl_vec3(src, name);
            for c in 0..3 {
                assert!(
                    (theirs[c] - ours[c]).abs() < 1e-5,
                    "{name}: {ours:?} here, {theirs:?} in the shader"
                );
            }
        }
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
            CardLook::back(FinishTreatment::Plain),
            Some(flat.clone()),
            Color::BLACK,
            &mut assets,
        );
        let bare = cache.get(
            CardLook::back(FinishTreatment::Plain),
            None,
            Color::BLACK,
            &mut assets,
        );
        // A card drawing its own text is not a back, and must not be given
        // the picture: both have no `ImageKey`, and the tint is what tells
        // them apart.
        let face = cache.get(
            CardLook::flat(Color::srgb(0.2, 0.3, 0.4), FinishTreatment::Plain),
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
