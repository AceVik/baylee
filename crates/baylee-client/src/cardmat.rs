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
/// [`REACHABLE`], [`ARMED`], [`WILL_TAP`], [`CHOOSABLE`], [`SELECTED`] — are
/// this client saying "you could", "you are about to" or "you chose", and are
/// light on the felt round the card
/// (`floor_light`, #298). The three protection keywords are facts about the
/// card. They were the frame's paper until #298 took the frame away, and they
/// wait for the shells that replace it. Sickness and identity used to ride
/// here too: identity is the strip's crests now
/// (`baylee_client_core::cardrail::Strip`), and sickness a shell's wave. Nothing in this word is drawn on
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
    /// `card_common.wesl`'s `GLOW_*` constants are the other half of each.
    pub const REACHABLE: u32 = 1 << 23;
    /// This card is one the question this seat is answering will take: a
    /// legal target, or a card it may choose (report 09).
    ///
    /// Light on the felt round the ones it will take; the rest are dimmed
    /// for as long as the question is open ([`super::veil::DIM`], the
    /// owner's ruling of 13.10.2026 that a passing state may lie on the
    /// print, `docs/legal.md` §3) and take no press
    /// (`input::table_click`'s choice gate).
    pub const CHOOSABLE: u32 = 1 << 24;
    /// This card belongs to the draft answer, whether or not it is still a
    /// legal thing to click. A steady, strong light independent of `choosable`.
    pub const SELECTED: u32 = 1 << 25;

    /// The six offers: what the felt round a card is lit with.
    pub const OFFERS: u32 = ACTIVATABLE | ARMED | WILL_TAP | REACHABLE | CHOOSABLE | SELECTED;
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
/// Five claims about the *client's own state* rather than about the card, and
/// they travel together rather than as five arguments for the same reason
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
    /// The question this seat is answering will take this card ([`glow::CHOOSABLE`]).
    pub choosable: bool,
}

impl Offer {
    /// Nothing on offer, which is the whole board while this seat is not the
    /// one being asked.
    pub const NONE: Self = Self {
        activatable: false,
        reachable: false,
        armed: false,
        will_tap: false,
        choosable: false,
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
            choosable: false,
        }
    }

    /// Add this client's mana plan to the engine's offer: a pile card it
    /// would cast or a permanent it would activate after making mana
    /// ([`crate::Reach::Taps`]). Hand cards use the same state for their halo.
    #[must_use]
    pub const fn reaching(self, reachable: bool) -> Self {
        Self { reachable, ..self }
    }

    /// Add whether the question this seat is answering will take this card.
    #[must_use]
    pub const fn choosing(self, choosable: bool) -> Self {
        Self { choosable, ..self }
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
                choosable: false,
                will_tap: match &armed.deed {
                    // Whichever end the run has: a land the plan spends is a
                    // land the player has to see marked before the second
                    // click.
                    crate::Deed::Run { plan, .. } => spends(plan),
                    crate::Deed::Play
                    | crate::Deed::Ability(_)
                    | crate::Deed::Suspend
                    | crate::Deed::Foretell => false,
                },
            },
            // The same light for a different claim, deliberately. `WILL_TAP`
            // means "the plan would spend this", and that is exactly as true
            // of a payment window as of an armed spell — what differs is what
            // happens on the next tap, and the next tap is the player's
            // either way. Nothing is `armed` here, because nothing was: a
            // mana ability is one tap and the arming contract exempts it.
            // A staged permanent is lit too — it taps when the player says Go
            // — and is told apart from the suggestion by lying turned
            // (`Proposal::Staged`).
            crate::Proposing::Owed { plan, staged } => Self {
                activatable,
                reachable: false,
                armed: false,
                will_tap: plan.is_some_and(spends)
                    || staged.iter().any(|step| members.contains(&step.source)),
                choosable: false,
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
    glow_of_with_selection(object, offer, false)
}

/// [`glow_of`], plus the selection held by the table placement rather than by
/// the offer. Selection is persistent and therefore never ranked against the
/// invitations to act.
#[must_use]
pub fn glow_of_with_selection(
    object: Option<&baylee_view::PublicObject>,
    offer: Offer,
    selected: bool,
) -> u32 {
    let from_card = object.map_or(0, |o| glow_bits(o.keywords));
    // An armed card is not also inviting a tap: the invitation was accepted,
    // and drawing both would put a travelling light and a steady one round
    // the same card saying the same thing twice.
    //
    // The engine's yes beats this client's offer for the same reason: a card
    // the engine will take as it stands needs no lands tapped for it, and
    // `Duel::reachable` leaves such a card out anyway.
    // A card the question will take is what the player is looking for
    // while it is asked, and nothing is activated in the middle of one.
    let offered = if offer.armed {
        glow::ARMED
    } else if offer.choosable {
        glow::CHOOSABLE
    } else if offer.activatable {
        glow::ACTIVATABLE
    } else if offer.reachable {
        glow::REACHABLE
    } else {
        0
    };
    from_card
        | offered
        | if offer.will_tap { glow::WILL_TAP } else { 0 }
        | if selected { glow::SELECTED } else { 0 }
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
    /// Its passing states ([`veil`]): the ones it is going to in the low
    /// byte, the ones it comes from in the next ([`Veil::code`]). Zero for
    /// a card in none, which then draws exactly as it did before them.
    pub veil: u32,
    /// When they set out, on the clock [`Self::sweep_at`] is on.
    pub veil_at: f32,
    /// One over how long they take, or `0.0` once they have arrived.
    pub veil_rate: f32,
}

/// A card's passing states, as `card_common.wesl` numbers them: what comes
/// and goes with the game and may lie on the print while it lasts (the
/// owner, 13.10.2026; `docs/legal.md` §3). A keyword's border lasts as long
/// as the keyword and stays off the print ([`crate::shellmat`]).
pub mod veil {
    /// The question this seat is answering leaves the card out
    /// ([`baylee_client_core::interaction::Interaction::leaves_out`]): dimmed
    /// until it is answered.
    pub const DIM: u32 = 1;
    /// Phased out (CR 702.26b): grey and faded until it phases in.
    pub const PHASED: u32 = 2;
    /// Summoning sick (CR 302.6): moonlight breathing in from its edge.
    pub const ASLEEP: u32 = 4;
    /// How long dimming takes to come on, in seconds. It goes at once: the
    /// board is the player's again the moment the question is answered.
    pub const DIM_FADE: f32 = 0.18;
    /// How long phasing and sleep take to come or go.
    pub const SLOW_FADE: f32 = 0.6;
}

/// Where a card's passing states stand: the ones it is going to, the ones it
/// came from, and when and how fast it set out. A part of [`CardLook`], so
/// it is plain bits; [`Veil::settled`] once it has arrived, which is the one
/// look every card in those states shares.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Veil {
    /// The states it is going to.
    pub to: u32,
    /// The states it comes from.
    pub from: u32,
    /// When it set out, as `f32` bits; zero once it has arrived.
    pub at: u32,
    /// One over how long it takes, as `f32` bits; zero once it has arrived.
    pub rate: u32,
}

impl Veil {
    /// No state at all: every card almost all of the time.
    pub const NONE: Self = Self {
        to: 0,
        from: 0,
        at: 0,
        rate: 0,
    };

    /// `to`, arrived.
    #[must_use]
    pub const fn settled(to: u32) -> Self {
        Self {
            to,
            from: to,
            at: 0,
            rate: 0,
        }
    }

    /// The shader's `veil` word.
    #[must_use]
    pub const fn code(self) -> u32 {
        self.to | (self.from << 8)
    }
}

/// One card's passing states over time: what [`crate::table`] keeps per
/// permanent so a change fades rather than jumps.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct VeilTrack {
    /// The states it is going to.
    pub to: u32,
    /// The states it comes from.
    pub from: u32,
    /// When it set out, on `Time::elapsed_secs_wrapped`.
    pub at: f32,
    /// One over how long it takes; zero for a change made at once.
    pub rate: f32,
}

impl VeilTrack {
    /// A card first drawn in `states`: already in them, since nothing on
    /// screen changed.
    #[must_use]
    pub const fn new(states: u32) -> Self {
        Self {
            to: states,
            from: states,
            at: 0.0,
            rate: 0.0,
        }
    }

    /// Where the card stands once it is wanted in `states` at `now`. A
    /// change sets out from what it was going to; dimming goes at once, and
    /// nothing fades while motion is off.
    #[must_use]
    pub fn toward(self, states: u32, now: f32, moving: bool) -> Self {
        if states == self.to {
            return self;
        }
        // Dimming leaves at once: the question was answered.
        let from = self.to & !(veil::DIM & !states);
        let changed = from ^ states;
        if !moving || changed == 0 {
            return Self::new(states);
        }
        let fade = if changed & (veil::PHASED | veil::ASLEEP) == 0 {
            veil::DIM_FADE
        } else {
            veil::SLOW_FADE
        };
        Self {
            to: states,
            from,
            at: now,
            rate: 1.0 / fade,
        }
    }

    /// The look's part of it at `now`: on its way, or [`Veil::settled`] once
    /// it has arrived, so a card in a state for a turn shares its look with
    /// every other.
    #[must_use]
    pub fn veil(self, now: f32) -> Veil {
        if self.rate <= 0.0 || (now - self.at) * self.rate >= 1.0 {
            Veil::settled(self.to)
        } else {
            Veil {
                to: self.to,
                from: self.from,
                at: self.at.to_bits(),
                rate: self.rate.to_bits(),
            }
        }
    }
}

/// The five doors a permanent goes through, as the shader numbers them.
///
/// The names are [`baylee_client_core::zones::Passage`]'s; these are the
/// wire between it and `card_common.wesl`, and
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
        "embedded://baylee_client/shaders/card.wesl".into()
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
        "embedded://baylee_client/shaders/card_ui.wesl".into()
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
                // And it is in no game, so in no passing state.
                veil: 0,
                veil_at: 0.0,
                veil_rate: 0.0,
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
    /// Its passing states ([`veil`]), on their way or arrived. Only the
    /// table's permanents are ever in one; on its way the look is
    /// transient as a sweep is, and swept out of the cache once it has
    /// arrived.
    pub veil: Veil,
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
            veil: Veil::NONE,
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
            veil: Veil::NONE,
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

    /// The same look in its passing states.
    #[must_use]
    pub fn with_veil(mut self, veil: Veil) -> Self {
        self.veil = veil;
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
            veil: Veil::NONE,
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
            veil: look.veil.code(),
            veil_at: f32::from_bits(look.veil.at),
            veil_rate: f32::from_bits(look.veil.rate),
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
        embedded_asset!(app, "shaders/card.wesl");
        embedded_asset!(app, "shaders/card_ui.wesl");
        // The file both of them import, and the keyword strip's two shaders
        // (`marksmat`) besides. It is not a shader in its own right: nothing
        // sets it on a pipeline. Since bevy 0.20 an import names a module
        // (`baylee_client::shaders::card_common`), and a module resolves only
        // once its file is a loaded `Shader` — embedding it is not enough
        // ("module not found … no shader registered"), so it is loaded as a
        // library, the way bevy loads its own.
        bevy::shader::load_shader_library!(app, "shaders/card_common.wesl");
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
pub(crate) mod tests;
