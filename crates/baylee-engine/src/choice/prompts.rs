//! The prompts a question carries: cast modes, choices, numbers, targets and yes-or-no questions.

use super::AnswerFault;
use baylee_core::ids::{ObjectId, PlayerId, TargetRef};

/// One legal way to cast a spell (CR 601.2b).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CastModeDesc {
    /// Option index (answered via `PlayerAction::ChooseMode`).
    pub index: u32,
    /// Which kind of cast this is.
    pub kind: CastModeKind,
    /// The mana part to pay with this option.
    pub cost: baylee_core::mana::ManaCost,
}

/// The kind of a cast option.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum CastModeKind {
    /// Choose a zone permission supplied by this source (none means unrestricted).
    Permission(Option<ObjectId>),
    /// A counter kind chosen during resolution, not a spell mode.
    Counter(baylee_cards_dsl::CounterKind),
    /// A combination of colors chosen during effect resolution, not a cast mode.
    Colors(baylee_core::color::ColorSet),
    /// Printed cost.
    Normal,
    /// Printed cost plus kicker, chosen before its replacement targets.
    Kicked,
    /// An alternative cost (pitch, evoke, …).
    Alternative(usize),
    /// A spell mode (overload and friends).
    Mode(usize),
    /// Several modes of a spell that chooses more than one (CR 700.2a):
    /// bit `i` is mode `i`. Announced as one set, and carried out in the
    /// order the modes are printed (CR 608.2c). The option's cost is the
    /// spell's plus every chosen mode's own (spree, CR 702.172a).
    Modes(u8),
    /// Cast a non-front face for its own printed cost — an MDFC's back
    /// (CR 712.11b), an adventure (CR 715), a disturb back (CR 702.146).
    ///
    /// It used to name The True Scriptures, which is none of those: a
    /// *transformed* back is reached by turning the card over and never by
    /// paying (CR 712.2), and it was on this list only because its `FaceDef`
    /// carried a cost the printing does not have. Swift Spiral, on the back
    /// of Twining Twins, is the pool's real example.
    Face(usize),
    /// Play a specific land face of an MDFC (pathways; CR 712.12).
    PlayLandFace(usize),
    /// Cast with prototype characteristics (CR 718).
    Prototype,
    /// Cast face down as a 2/2 creature with ward {2}.
    Disguise,
    /// Miracle cast (CR 702.94).
    Miracle,
    /// Cast a foretold card from exile for its foretell cost after the turn
    /// it was exiled (CR 702.143a). This is an alternative cost.
    Foretell,
    /// Cast from the graveyard for the card's printed flashback cost
    /// (CR 702.34a). A *granted* flashback is paid with the mana cost and is
    /// offered as [`Self::Normal`].
    Flashback,
    /// Beam me up's graveyard cost and returning a controlled creature.
    BeamMeUp,
    /// Cast for the card's dash cost rather than its mana cost
    /// (CR 702.109a): the permanent it becomes has haste and returns to its
    /// owner's hand at the beginning of the next end step.
    Dash,
    /// Cast for the card's mutate cost rather than its mana cost (CR 702.140a),
    /// targeting a non-Human creature its owner owns to merge with.
    Mutate,
    /// Resolving a mutating spell: true puts it on top, false underneath.
    MutatePosition(bool),
    /// Choose the dungeon to enter.
    Dungeon(baylee_core::dungeon::Dungeon),
    /// Follow a printed arrow into this room.
    DungeonRoom {
        /// Dungeon containing the room.
        dungeon: baylee_core::dungeon::Dungeon,
        /// Zero-based room index.
        room: u8,
    },
    /// Cast from the owner's graveyard for the card's escape cost
    /// (CR 702.138a): its mana, and as many other cards from that graveyard
    /// exiled, asked as the cast's cost (`ChoicePrompt::CostExile`).
    Escape,
    /// Escape granted at a face's mana cost, with its announced spell modes.
    EscapeGranted {
        /// Use the prototype characteristics and mana cost.
        prototype: bool,
        /// Whether this escape pays the optional kicker cost.
        kicked: bool,
        /// Prospective face, including an Adventure.
        face: usize,
        /// Chosen modal bit set; zero for a nonmodal spell.
        modes: u8,
        /// Other graveyard cards required by the chosen grant.
        exile: u8,
    },
}

/// Why a [`Pending::ChooseCards`] is presented (UI hint).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum ChoicePrompt {
    /// Choose which incoming object’s entry replacements to process first.
    EntryOrder,
    /// Choose which source’s entry replacement applies next.
    EntryReplacement,
    /// Non-targeted counter recipients from the objects tapped by this effect.
    TappedForCounters {
        /// One counter of this kind goes on each chosen permanent.
        kind: baylee_cards_dsl::CounterKind,
    },
    /// Choose permanents to receive another counter of every kind already present.
    Proliferate,
    /// Optional creature from hand, cast face down under the resolving effect.
    CastFaceDown {
        /// Announced X; only this much of the paid mana can justify the card.
        x: u32,
        /// Actual activation payment, in WUBRG/colorless order.
        paid: [u32; 6],
        /// Fixed activation cost that the same receipt must also cover.
        fixed_cost: baylee_core::mana::ManaCost,
    },
    /// Choose the card a controlled player must play if able.
    CommandCard,
    /// Choose an Aura host as it enters or moves (Kudzu). Not a target.
    /// Relocation may allow none to leave the Aura where it is.
    NewHost {
        /// The Aura that moves.
        aura: baylee_core::ids::ObjectId,
    },
    /// Choose a permanent that this player sacrifices; the chooser may be
    /// an opponent, and the choice does not target the permanent.
    SacrificeFor {
        /// The player who will sacrifice the selected permanent.
        player: PlayerId,
    },
    /// Choose a land from which this ability removes all linked counters.
    RemoveLandCounters,
    /// Privately inspect the offered hand, then acknowledge without selecting.
    LookAtHand,
    /// Privately inspect library cards, then confirm without selecting any.
    LookAtLibrary,
    /// Choose a matching permanent you control or reveal a matching hand card.
    Behold,
    /// Of cards already assigned to the library, reveal these through Nephalia
    /// Academy; unselected cards use Library of Leng and stay hidden.
    RevealDiscard,
    /// Library search (tutor/fetch).
    SearchLibrary,
    /// Put cards from your hand on top of your library (chosen order).
    PutBackOnTop,
    /// A wish: cards from outside the game, or face-up in your exile.
    Wish,
    /// Dredge (CR 702.52a): the cards in the drawing player's graveyard
    /// whose dredge could replace the draw at hand. Naming one mills its
    /// count and returns it to their hand instead of drawing; naming none
    /// leaves the draw to be made (`min` 0, `max` 1).
    Dredge,
    /// Which replacement applies to the draw at hand next, when more than
    /// one could and the order matters (CR 616.1, 616.1e). The menu holds
    /// each one's source: a dredge card in the graveyard, a Chains of
    /// Mephistopheles on the battlefield, the Aladdin's Lamp or Ring of
    /// Ma'rûf whose effect waits for the draw. Exactly one is named; the
    /// draw it leaves is asked about again (CR 616.1f).
    DrawReplacement,
    /// Delve: exile cards from your graveyard, each paying for {1}
    /// (CR 702.66). Not a search and not a discard — the pile is offered so
    /// the caster can spend it.
    ///
    /// The first of the six prompts in this enum that are part of a *cost*,
    /// and the only one that asks for a heap rather than one card, which is
    /// what makes it worth telling apart: `options` is the whole graveyard, but
    /// `max` is the generic mana in the spell's total cost (CR 702.66a), and
    /// answering below it leaves a cast that cannot pay. The house AI reads
    /// this variant for exactly that reason.
    Delve,
    /// "Sacrifice a creature" in an activation cost (CR 701.21a).
    ///
    /// The second of the prompts here that are part of a *cost* rather than
    /// an effect, for the reason [`Self::Delve`] gives: a question a
    /// player is being asked in order to pay is a different question from a
    /// search, and a client that cannot tell them apart asks somebody to
    /// "choose a card" while what it means is "which one are you giving up".
    ///
    /// Not a target (CR 115.1). `Pending::ChooseCards` and not
    /// `ChooseTargets` is what says so, and it matters on the board: a
    /// creature with hexproof can be sacrificed to its own controller's
    /// outlet, and nothing "becomes the target of" an ability by being eaten
    /// by one.
    CostSacrifice,
    /// "Discard a card" in an activation cost (CR 701.9a).
    CostDiscard,
    /// Keep these hand cards; the unchosen ones will be discarded.
    Keep,
    /// Keep these lands; the unchosen ones will be sacrificed.
    KeepLands,
    /// Keep these creatures; the unchosen ones will be sacrificed.
    KeepCreatures,
    /// Keep these other matching permanents; the rest will be sacrificed.
    KeepPermanents,
    /// "Tap an untapped creature you control" in an activation cost.
    ///
    /// The first cost prompt whose answer is not destroyed, which is why it
    /// needs its own word rather than sharing the sacrifice's: a player told
    /// "choose a permanent to sacrifice" over a menu of their own creatures
    /// would decline a cost that only taps one. CR 118.3 supplies the
    /// "untapped"; nothing in the rules supplies "you control", so the card
    /// prints it and `cost_wizard::options` draws the line anyway.
    CostTap,
    /// "Return a Forest you control to its owner's hand" in an activation
    /// cost (Quirion Ranger).
    ///
    /// The second whose answer survives, and the word matters for the same
    /// reason [`Self::CostTap`] earned its own: a player shown "which one
    /// are you giving up" over their own lands would decline a cost that
    /// hands the land back. What the two do not share is which permanents
    /// are on the menu — a tap wants an untapped one (CR 118.3), a return
    /// takes either, and tapping the Forest for mana *first* is the play.
    CostReturn,
    /// "Exile a creature card from your graveyard" in an activation cost
    /// (Moorland Haunt, Mines of Moria), and escape's "exile five other
    /// cards from your graveyard" (CR 702.138a), asked once with `min` and
    /// `max` both the count.
    ///
    /// Not [`Self::CostDiscard`], though both answers end up somewhere a
    /// card is not played from: the noun has to say which pile the card
    /// leaves, and a player told "card to discard" over their graveyard
    /// would look for it in their hand. And not [`Self::Delve`], which
    /// exiles from the same pile: delve's `max` is the generic mana in a
    /// spell's cost and the house AI answers it with `max`, where this asks
    /// for exactly one card per part.
    CostExile,
    /// "Which of these do you want to leave tapped?" — the untap step's own
    /// determination (CR 502.3), on the permanents that print
    /// "you may choose not to untap".
    ///
    /// The only prompt here that is neither a cost nor an effect: it is a
    /// turn-based action asking the question its own rule gives the active
    /// player. It is phrased as what stays tapped rather than what untaps
    /// because the menu holds only the permanents with a second answer —
    /// listing everything the player controls would ask them to re-confirm
    /// the whole board every turn.
    LeaveTapped,
    /// "Which of these untap?" — the same determination (CR 502.3) under a
    /// limit on how many may untap (`Modifier::UntapAtMost`: Smoke, Winter
    /// Orb). The menu holds what may still untap and the answer is what
    /// untaps, so it is the opposite of [`Self::LeaveTapped`]: naming more
    /// is the better answer. Only the limit keeps a permanent tapped, so the
    /// question is asked again until no limit has room for anything on it.
    Untap,
    /// "Which attackers are in a band with this one?" (CR 508.1e,
    /// 702.22c), asked of each attacking creature with banding once the
    /// attack is declared and before anything triggers on it. The menu
    /// holds the other attackers aimed at the same player or planeswalker
    /// that are in no band yet (CR 702.22d); at most one of the answer may
    /// lack banding, and naming nothing leaves `with` out of any band.
    Band {
        /// The attacker with banding the band forms around.
        with: ObjectId,
    },
    /// "You may have it block an attacking creature of your choice" (False
    /// Orders): the menu holds the attackers `blocker` could be blocking,
    /// and naming none leaves it out of combat.
    BlockWith {
        /// The creature that will block what is chosen.
        blocker: ObjectId,
    },
    /// Raging River: "divides all creatures without flying they control
    /// into a 'left' pile and a 'right' pile". The menu holds the defending
    /// player's creatures without flying; the ones chosen are the left
    /// pile, the rest the right.
    LeftPile,
    /// Camouflage, instead of declaring blockers: the creatures for pile
    /// `pile` of `of` (one pile per creature attacking this player). A
    /// creature goes into one pile; the piles are then assigned to the
    /// attackers at random.
    CamouflagePile {
        /// Which pile, from 1.
        pile: u8,
        /// How many piles.
        of: u8,
    },
    /// Revealed cards of one card type, one of which may be put into the
    /// hand (Atraxa, Grand Unifier: "for each card type, you may put a card
    /// of that type … into your hand"). Asked once per type, and the type
    /// is the question: the menu holds only that type's cards. Taking one is
    /// never worse than leaving it, since the rest go to the bottom, so the
    /// house AI takes the best.
    OneOfType {
        /// The card type asked about.
        card_type: baylee_core::types::TypeSet,
    },
    /// "You may reveal a matching card from your hand; if you don't, this
    /// land enters tapped."
    ///
    /// A prompt of its own rather than [`Self::Generic`] because declining
    /// is never right: nothing is spent, the card stays in hand (CR 701.20b)
    /// and the only thing given up is the information. The house AI keys on
    /// exactly that — its `ChooseCards` rule answers `min` for a prompt it
    /// does not recognise, so a generic one would have every AI reveal land
    /// enter tapped for the rest of the game.
    RevealOrEnterTapped,
    /// Looked-at cards, one or more of which go into the hand (Expressive
    /// Iteration's first question, a dig's keep). The answer is what the
    /// player keeps, so the house AI takes the best of them.
    PutIntoHand,
    /// Looked-at cards, the chosen one of which goes on the bottom of the
    /// library (Expressive Iteration's second question): the answer is what
    /// the player gives up.
    PutOnBottom,
    /// Exiled cards, the chosen one of which the player may play this turn
    /// (Dauthi Voidwalker).
    PlayFromExile,
    /// A looked-at card the player may put onto the battlefield, and which
    /// goes elsewhere if they name nothing (Risen Reef's land). The answer
    /// is what the player puts down, so the house AI puts it down.
    PutOntoBattlefield,
    /// Cards in a graveyard, one of which may be put where the effect says;
    /// naming none searches the library instead (Finale of Devastation's
    /// "search your library and/or graveyard"). The house AI takes the best.
    FromGraveyard,
    /// Cards in the hand, the chosen ones of which are discarded — not as a
    /// price but as the effect itself ("you may discard up to two cards; if
    /// you do, draw that many"). The house AI lets its least valuable card
    /// go.
    Discard,
    /// Another player's found cards, the chosen ones of which go to their
    /// owner's graveyard and the rest to that player's hand (Realms
    /// Uncharted's opponent). The chooser is denying the other player, so
    /// the house AI sends the best of them away.
    PutIntoGraveyard,
    /// Revealed cards to separate into two piles (Fact or Fiction's
    /// opponent): the chosen ones are the first pile and the rest the
    /// second, and either may be empty. The other player then takes one
    /// pile into their hand, so the house AI puts the best card alone.
    FirstPile,
    /// Creatures to tap to saddle a permanent (CR 702.171a), with total
    /// effective power meeting the question's [`CardTotal`].
    CostSaddle {
        /// The N of saddle N.
        power: u8,
    },
    /// Creatures to tap to crew a Vehicle (CR 702.122a), with total effective
    /// power meeting the question's [`CardTotal`]. An insufficient answer
    /// is refused with [`AnswerFault::TotalTooLow`].
    CostCrew {
        /// The N of "Crew N".
        power: u8,
    },
    /// Generic selection.
    Generic,
    /// Choose the Army that receives amass counters and the named subtype.
    Amass,
    /// Optional simultaneous tap payment: decline or select exactly this many.
    CostTapExactlyOrNone {
        /// Exact number of distinct permanents needed to pay.
        count: u32,
    },
}

/// What a [`Pending::ChooseNumber`] counts (UI hint).
///
/// Three questions share the variant because each is a bounded number the
/// player announces as the spell or ability is put on the stack (CR 601.2b,
/// 601.2d), and the answer is [`PlayerAction::ChooseNumber`] every time;
/// what they *mean* is this field, for the reason [`TargetPrompt`] gives
/// about convoke. Without it a
/// player casting Lose Focus with mana to spare was asked to "choose a
/// number (0–2)" and nothing said what for.
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Debug, Default, serde::Serialize, serde::Deserialize,
)]
pub enum NumberPrompt {
    /// Optional energy payment before choosing counter recipients.
    EnergyPayment {
        /// Number of permanents actually tapped by the instruction; overpayment is legal.
        candidates: u32,
    },
    /// The twenty distinct ordered pairs in a five-word vocabulary.
    TextReplacement {
        /// Printed color words or basic land type words.
        kind: baylee_cards_dsl::TextWordKind,
        /// Exact target whose rules words are being changed.
        target: baylee_core::ids::DamageSourceRef,
    },
    /// Spend any chosen amount of floating mana to prevent damage.
    ManaPayment {
        /// Damage this payment can prevent; overpayment is legal.
        preventable_damage: u32,
    },
    /// How many counters to put on an object while an effect resolves.
    Counters {
        /// Object receiving the counters.
        target: ObjectId,
        /// Kind of counters to put.
        kind: baylee_cards_dsl::CounterKind,
    },
    /// The value of X (CR 107.3): a spell's printed `{X}`, an activation's,
    /// or the X of a counter cost.
    #[default]
    X,
    /// How many times to pay a spell's replicate cost (CR 702.56a). The
    /// maximum is the most the caster's floating mana pays for, and the
    /// spell is copied once for each.
    Replicate {
        /// The cost paid each time.
        cost: baylee_core::mana::ManaCost,
    },
    /// "Damage divided as you choose" (CR 601.2d): how much of what is left
    /// goes to one target, asked target by target in the order they were
    /// chosen. The last target takes the rest and is not asked, and each
    /// target is given at least 1, which is what `min` and `max` say.
    DivideDamage {
        /// The exact object or player this share goes to.
        target: TargetRef,
        /// Its place among the targets, from 0.
        index: u8,
        /// How many targets share the damage.
        of: u8,
        /// The damage not yet given to a target.
        left: u32,
    },
    /// One creature's share of another creature's combat damage divided
    /// among two or more: an attacker's by its controller (CR 510.1c), a
    /// blocker's by its own (510.1d), or either by the player banding
    /// names (702.22j–k). How much of what is left goes to `recipient`,
    /// asked creature by creature in declaration order. The last takes the
    /// rest and is not asked, and a share may be 0.
    CombatDamage {
        /// The creature whose damage is divided.
        source: ObjectId,
        /// The creature this share goes to.
        recipient: ObjectId,
        /// Its place among the recipients, from 0.
        index: u8,
        /// How many creatures share the damage.
        of: u8,
        /// The damage not yet given to a creature.
        left: u32,
    },
    /// How many eligible permanents to sacrifice as an object enters.
    Sacrifices,
    /// A bounded number chosen by a resolving or entry instruction.
    Choice,
}

/// Why a [`Pending::ChooseTargets`] is presented (UI hint).
///
/// The convoke question is not targeting, and the only thing that ever said
/// so was the name of the variant it arrives in: a player casting a waterbend
/// spell was asked for "up to 99 targets" when what was wanted was "tap what
/// you like to help pay". Both are a bounded selection over permanents, which
/// is why they share a variant; what they *mean* is this field.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum TargetPrompt {
    /// The targets of a spell or ability (CR 115).
    #[default]
    Targets,
    /// Targets must have distinct mana values; pairs are sorted by object handle.
    DistinctManaValues(Vec<(ObjectId, u32)>),
    /// Change one existing target. An empty answer retains that target when
    /// `min` is zero; a required answer changes it to a legal new target.
    Retarget {
        /// The target this slot currently retains unless it is changed.
        current: TargetRef,
        /// Zero-based position in the spell or ability's target sequence.
        index: u32,
        /// Total number of targets retained by the spell or ability.
        of: u32,
    },
    /// Tap permanents to help pay, each paying for {1}: creatures for
    /// convoke (CR 702.51a), artifacts and creatures for a paid waterbend
    /// (CR 701.67a). `options` says which.
    Convoke,
    /// Choose what a permanent enters as a copy of (CR 614.12a):
    /// a choice, not a target (Vesuvan Doppelganger's "any creature").
    CopyOf,
}

impl TargetPrompt {
    /// Additional constraints shared by the engine and every answer builder.
    #[must_use]
    pub fn selection_fault(&self, objects: &[ObjectId]) -> Option<AnswerFault> {
        let Self::DistinctManaValues(values) = self else {
            return None;
        };
        let mut selected = std::collections::BTreeSet::new();
        for object in objects {
            let Ok(index) = values.binary_search_by_key(object, |(id, _)| *id) else {
                return Some(AnswerFault::NotOffered);
            };
            if !selected.insert(values[index].1) {
                return Some(AnswerFault::SameManaValue);
            }
        }
        None
    }
}

/// What a [`Pending::YesNo`] asks.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum YesNoPrompt {
    /// Select one replacement for a permanent that would untap.
    RemoveCountersInsteadOfUntapping {
        /// Exact batch member represented by this visible permanent.
        permanent: ObjectId,
        /// Counters removed instead of untapping.
        kind: baylee_cards_dsl::CounterKind,
        /// Number of counters removed by this replacement.
        count: u16,
    },
    /// Clear marked damage instead of applying another destruction replacement.
    UseDamageClearingReplacement {
        /// Permanent facing destruction.
        permanent: ObjectId,
    },
    /// Use an intrinsic shield counter instead of regeneration.
    UseShieldCounter {
        /// Permanent facing destruction.
        permanent: ObjectId,
    },
    /// Apply this additive counter replacement before other applicable replacements.
    ApplyAdditionalCountersNext {
        /// Permanent receiving counters.
        permanent: ObjectId,
        /// Additional counters of each kind in this placement.
        additional: u16,
    },
    /// Use a one-shot shield rather than a standing regeneration replacement.
    UseRegenerationShield {
        /// Permanent being regenerated.
        permanent: ObjectId,
    },
    /// Use one North Star permission for this spell’s mana cost.
    ManaAnyType,
    /// Optionally enter tapped and receive the displayed player counters.
    EnterTappedForCounters {
        /// Player counter kind.
        kind: baylee_cards_dsl::CounterKind,
        /// Counters received when accepted.
        amount: u16,
    },
    /// Add another counter of each kind already on this player.
    ProliferatePlayer {
        /// Counter recipient, not the deciding player.
        player: PlayerId,
    },
    /// Pay energy counters.
    PayEnergy {
        /// Number of counters.
        amount: u16,
    },
    /// "You may pay N life; if you don't, this enters tapped" (shocklands).
    PayLifeOrEnterTapped {
        /// Life to pay.
        amount: u16,
    },
    /// Kicker/additional cost yes-or-no at cast time.
    Kicker,
    /// "Pay {N}?" for a tax trigger (Rhystic Study & co.).
    PayTax {
        /// Generic mana to pay.
        mana: u16,
    },
    /// "Pay {U}?" for a tax or price with colour in it
    /// (`Effect::PlayerMayPayManaOr`, `Effect::PlayerMayPayManaThen`):
    /// Phantasmal Forces' upkeep, Force of Nature's.
    PayMana {
        /// The printed price.
        cost: baylee_core::mana::ManaCost,
    },
    /// Pay one complete mana-and-life price, or decline both parts.
    PayManaAndLife {
        /// The full mana component.
        cost: baylee_core::mana::ManaCost,
        /// The full life component.
        life: u32,
    },
    /// Make mana for a pact's debt. Declining loses the game.
    PayPact {
        /// The full, possibly colored cost.
        cost: baylee_core::mana::ManaCost,
    },
    /// Pay life for a resolving ability (including ward).
    PayLife {
        /// Life to pay without truncating computed cumulative costs.
        amount: u32,
    },
    /// "Reveal and cast for its miracle cost?" (CR 702.94).
    Miracle {
        /// The drawn card.
        card: baylee_core::ids::ObjectId,
    },
    /// "A draw was offered. Accept?" (CR 104.4i). Everyone still in the
    /// game has to say yes; one no and play continues where it left off.
    DrawOffer {
        /// The player who offered.
        proposer: PlayerId,
    },
    /// "Your commander is in a graveyard or in exile — put it into the
    /// command zone instead?" (CR 903.9a).
    CommanderZone {
        /// The commander card, wherever it currently is.
        card: baylee_core::ids::ObjectId,
    },
    /// "Something is about to put your commander into your hand or library
    /// — put it into the command zone instead?" (CR 903.9b).
    ///
    /// Asked *before* the effect moves anything, because a replacement
    /// effect that asked afterwards would have already let the card land
    /// where it was not going to.
    CommanderReplace {
        /// The commander card, still where the effect found it.
        card: baylee_core::ids::ObjectId,
        /// Whether the library is the destination being replaced. The two
        /// answers are not the same question: a commander tucked into a
        /// library is gone, while one bounced to hand recasts untaxed
        /// (CR 903.8), so the prompt has to say which is happening.
        to_library: bool,
    },
    /// "You may cast that card without paying its mana cost" — cascade's
    /// offer (CR 702.85a), about the card it just exiled.
    CastWithoutPaying {
        /// The exiled card.
        card: baylee_core::ids::ObjectId,
    },
    /// "You may cast that card", paying its costs, as the ability resolves
    /// (Conduit of Worlds, CR 608.2g). A yes opens a payment window for its
    /// mana cost.
    CastPaying {
        /// The card to cast.
        card: baylee_core::ids::ObjectId,
    },
    /// "You may …" inside a resolving ability ([`baylee_cards_dsl::Effect::MayDo`]).
    MayDo,
    /// "Its owner puts it on their choice of the top or bottom of their
    /// library" ([`baylee_cards_dsl::Effect::OwnerPutsOnTopOrBottom`]),
    /// asked of the owner: yes is the top, no the bottom.
    TopOfLibrary {
        /// The card that is going.
        card: baylee_core::ids::ObjectId,
    },
    /// "You may cast that card without paying its mana cost. If you don't
    /// cast it, put that card into your hand." (discover, CR 701.57a),
    /// asked of the player who discovered it: yes casts it, no puts it
    /// into their hand. Asked only of a card that can be cast.
    Discover {
        /// The discovered card, in exile.
        card: baylee_core::ids::ObjectId,
    },
    /// "If you would begin your turn while this artifact is tapped, you may
    /// skip that turn instead" (Time Vault, CR 614.10), asked of the player
    /// whose turn would begin, before it begins: yes skips the turn and
    /// untaps `source` as the next turn that occurs begins (CR 614.10b), no
    /// takes the turn.
    SkipTurn {
        /// The tapped permanent whose replacement effect is offered.
        source: baylee_core::ids::ObjectId,
    },
    /// Generic yes/no (optional effects).
    Generic,
    /// Pay the full sacrifice required before this card may enter the battlefield.
    SacrificeToEnter {
        /// Number of eligible permanents that must be sacrificed together.
        count: u8,
    },
}

impl YesNoPrompt {
    /// Whether a stored standing answer may answer this question for the
    /// seat.
    ///
    /// The gate is the *kind of question*, never the handle it arrives
    /// under. Every yes/no a card asks carries an [`AbilityRef`] — kicker,
    /// a shockland's two life, a tax trigger, a miracle — so a rule keyed
    /// on "it names an ability" would have let the first standing answer a
    /// player ever stored silence a decision about **cost**. A player who
    /// says "always take Ondu Cleric's life" has said nothing whatsoever
    /// about paying `{2}` for a kicker.
    ///
    /// So the list holds only the questions where saying yes costs nothing
    /// but the choice itself.
    ///
    /// [`Self::CommanderZone`] is on it because it is what
    /// `AbilityRef::COMMANDER_ZONE` was reserved for — "always put Katara
    /// back" is a preference a player is meant to be able to keep — and the
    /// `{2}` it leads to is a tax on a *later* cast rather than a cost paid
    /// here. Its sibling [`Self::CommanderReplace`] is deliberately **off**
    /// it: the right answer there depends on where the commander was going
    /// (a library is a loss, a hand is strictly better than the command
    /// zone), and a stored `Yes` cannot see the `to_library` that decides
    /// it.
    #[must_use]
    pub fn automatable(self) -> bool {
        matches!(self, Self::MayDo | Self::CommanderZone { .. })
    }

    /// Whether answering "no" does nothing, which is what a seat that ran
    /// out of time is answered with ([`timeout_answer`]).
    ///
    /// Everything a card offers is declined by not doing it: the life not
    /// paid, the kicker not paid, the tax not paid, the miracle not cast,
    /// the "you may" not done, and a draw offer not accepted. The two
    /// commander questions are not: declining leaves the commander in a
    /// graveyard, in exile, or tucked into a library, which is a loss and
    /// not a pause. The house answers those. Neither is
    /// [`Self::TopOfLibrary`]: both answers move the card, and "no" is the
    /// bottom rather than nothing.
    #[must_use]
    pub const fn declining_does_nothing(self) -> bool {
        match self {
            Self::UseRegenerationShield { .. }
            | Self::SacrificeToEnter { .. }
            | Self::EnterTappedForCounters { .. }
            | Self::ProliferatePlayer { .. }
            | Self::PayEnergy { .. }
            | Self::PayLifeOrEnterTapped { .. }
            | Self::Kicker
            | Self::ManaAnyType
            | Self::PayTax { .. }
            | Self::PayMana { .. }
            | Self::PayManaAndLife { .. }
            | Self::PayLife { .. }
            | Self::Miracle { .. }
            | Self::CastWithoutPaying { .. }
            | Self::CastPaying { .. }
            | Self::DrawOffer { .. }
            | Self::MayDo
            | Self::SkipTurn { .. }
            | Self::Generic => true,
            // Declining discover moves the card into the hand: a card for
            // the player, but not nothing, and a free spell is not a
            // question to answer by the clock.
            Self::RemoveCountersInsteadOfUntapping { .. }
            | Self::UseDamageClearingReplacement { .. }
            | Self::UseShieldCounter { .. }
            | Self::ApplyAdditionalCountersNext { .. }
            | Self::CommanderZone { .. }
            | Self::CommanderReplace { .. }
            | Self::PayPact { .. }
            | Self::TopOfLibrary { .. }
            | Self::Discover { .. } => false,
        }
    }
}
