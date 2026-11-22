//! Game events and the append-only journal.
//!
//! Every mutation of the game produces events; the journal is the ordered
//! record of all of them. It is the single source of truth for replays,
//! spectating, reconnect/resume, crash recovery, and golden tests.

use crate::object::CounterKind;
use crate::turn::{Phase, Step};
use crate::zone::Zone;
use baylee_core::ids::{AbilityRef, ObjectId, PlayerId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Why an event happened.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Cause {
    /// Game setup.
    Setup,
    /// Casting or resolving a spell.
    Spell,
    /// An activated/triggered ability.
    Ability,
    /// A one-shot or continuous effect.
    Effect,
    /// Paying a cost.
    Cost,
    /// A turn-based action (CR 703).
    TurnBased,
    /// A state-based action (CR 704).
    StateBased,
    /// A player left the game, and what they controlled that did not leave
    /// with them is exiled (CR 800.4a), or is exiled once the last effect
    /// giving it to a player still in the game ends (CR 800.4c). Neither is
    /// a state-based action.
    PlayerLeft,
    /// A developer-mode command (dev games only; journaled for honesty).
    DevCommand,
    /// Life lost to the inherent radiation trigger (CR 729.1a).
    Radiation,
    /// A destruction that reached its zone move, even if its destination
    /// was replaced. Regeneration, shield counters and indestructibility
    /// never produce this cause (CR 701.8).
    Destroy,
}

/// What damage was dealt to.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum DamageTarget {
    /// A player.
    Player(PlayerId),
    /// An object (creature/planeswalker/battle).
    Object(ObjectId),
}

/// Why a cast was taken back (CR 734.1), as [`GameEvent::CastCancelled`]
/// records it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum CastCancelReason {
    /// The caster took it back (`PlayerAction::CancelCast`).
    Withdrawn,
    /// Its payment window closed with the pool short of the total: partial
    /// payments are not allowed (CR 601.2h), so nothing was paid.
    Unpaid,
    /// It could not be completed for another reason: the card moved, or an
    /// object one of its costs named is gone.
    Illegal,
}

/// What a player chose objects for during an APNAP held-choice sequence
/// (CR 101.4b, CR 101.4d) before all players' choices are carried out together.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ChosenAction {
    /// Chosen to be sacrificed once all players have chosen.
    Sacrifice,
    /// Chosen to be discarded once all players have chosen (CR 101.4d: a card
    /// chosen from a hidden hand stays hidden from other seats until discarded).
    Discard,
    /// Chosen to be destroyed once all players have chosen.
    Destroy,
    /// Chosen to be returned to its owner's hand once all players have chosen.
    ReturnToHand,
}

/// A single game event.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum GameEvent {
    /// All choices and costs of an activated ability have been completed.
    AbilityActivated {
        /// Exact resulting stack ability; mana abilities never use the stack.
        #[serde(default)]
        ability: Option<baylee_core::ids::DamageSourceRef>,
        /// Exact source incarnation, including a source sacrificed as a cost.
        source: baylee_core::ids::DamageSourceRef,
        /// The activating player.
        player: PlayerId,
        /// The activation cost contains the tap symbol.
        tap_cost: bool,
        /// Whether the ability resolves without the stack as a mana ability.
        mana_ability: bool,
    },
    /// A copy or retargeted stack object acquired an object target.
    /// Casting/activation already announce their targets in their own events.
    BecameTarget {
        /// The spell or ability on the stack.
        object: ObjectId,
        /// The newly targeted object, once even if chosen in multiple slots.
        target: ObjectId,
        /// Controller of the spell or ability, not the retargeting effect.
        controller: PlayerId,
    },
    /// The same for a player a copy or a retargeting effect newly aimed a
    /// spell or ability at: "whenever you … become the target" (Leovold)
    /// reads it the way ward reads [`GameEvent::BecameTarget`].
    PlayerBecameTarget {
        /// The spell or ability on the stack.
        object: ObjectId,
        /// The newly targeted player.
        player: PlayerId,
        /// Controller of the spell or ability, not the retargeting effect.
        controller: PlayerId,
    },
    /// The game was set up from a preset.
    GameStarted {
        /// RNG seed.
        seed: u64,
        /// Number of seats.
        seats: u8,
    },
    /// Cards were shown to every player.
    ///
    /// A search that names anything narrower than "a card" reveals what it
    /// found on the way to a hidden zone — that reveal is the only thing
    /// that holds the searcher to the filter, which is exactly why the
    /// printed cards say it (Mystical Tutor, Cultivate) and why the ones
    /// that fetch to the battlefield do not.
    Revealed {
        /// Who revealed them.
        player: PlayerId,
        /// The cards, in the order they were found.
        cards: Vec<ObjectId>,
    },
    /// Public choices of permanents to keep, before a simultaneous sacrifice.
    /// Never used for private hand-card selections, nor a face-down reveal.
    CardsKept {
        /// The choosing player.
        player: PlayerId,
        /// The kept permanents.
        cards: Vec<ObjectId>,
    },
    /// Objects chosen by one player in an APNAP sequence whose action happens
    /// simultaneously once every player has chosen (CR 101.4b, CR 101.4d).
    CardsChosen {
        /// The choosing player.
        player: PlayerId,
        /// The chosen objects; hidden-hand identities stay withheld from other
        /// seats until the simultaneous discard occurs (CR 101.4d).
        cards: Vec<ObjectId>,
        /// What will happen to the chosen objects.
        action: ChosenAction,
    },
    /// A player cast a ballot in a vote (CR 701.38a), announced as it is
    /// cast (CR 608.2d), so every later voter knows it.
    Voted {
        /// The voter.
        player: PlayerId,
        /// What they voted for.
        object: ObjectId,
    },
    /// A player flipped a coin and won or lost the flip (CR 705).
    CoinFlipped {
        /// Who flipped.
        player: PlayerId,
        /// Whether they won.
        won: bool,
    },
    /// A zone was shuffled.
    Shuffled {
        /// Whose zone.
        player: PlayerId,
        /// Which zone.
        zone: Zone,
    },
    /// An object changed zone (its `version` bumped, CR 400.7).
    ZoneChanged {
        /// The object.
        object: ObjectId,
        /// Origin zone.
        from: Zone,
        /// Destination zone.
        to: Zone,
        /// Why.
        cause: Cause,
        /// Where in the library it went, when `to` is a library; `None`
        /// for every other zone (#300). The position is public, even when
        /// the card is not.
        #[serde(default)]
        place: Option<LibraryPlace>,
    },
    /// Counters on an object changed.
    CounterChanged {
        /// The object.
        object: ObjectId,
        /// Exact incarnation receiving or losing counters.
        version: u32,
        /// Player placing counters; removal has no placing player.
        by: Option<PlayerId>,
        /// First positive placement of this kind on this incarnation this turn.
        first_this_turn: bool,
        /// Counter kind.
        kind: CounterKind,
        /// Previous amount.
        old: u16,
        /// New amount.
        new: u16,
    },
    /// A Siege's intrinsic last-defense-counter ability triggered.
    BattleDefeated {
        /// Exact battlefield incarnation whose last defense was removed.
        source: baylee_core::ids::DamageSourceRef,
        /// Controller at the occurrence.
        controller: PlayerId,
        /// Source timestamp used for ordinary trigger ordering.
        timestamp: u64,
        /// Number of occurrences after event-time trigger modifications.
        times: u32,
    },
    /// A player gained or spent counters.
    PlayerCountersChanged {
        /// Recipient.
        player: PlayerId,
        /// Counter kind.
        kind: CounterKind,
        /// Previous amount.
        old: u16,
        /// New amount.
        new: u16,
    },
    /// A player's life total changed.
    LifeChanged {
        /// The player.
        player: PlayerId,
        /// Previous life.
        old: i32,
        /// New life.
        new: i32,
        /// Why.
        cause: Cause,
    },
    /// The inherent speed ability triggered; it has no permanent source.
    SpeedTriggered {
        /// The player whose speed can increase.
        player: PlayerId,
    },
    /// A turn began.
    TurnStarted {
        /// Turn number (1-based).
        number: u32,
        /// Active player.
        active: PlayerId,
    },
    /// Phase/step changed.
    StepChanged {
        /// Active player when this step began.
        active: PlayerId,
        /// New phase.
        phase: Phase,
        /// New step.
        step: Step,
    },
    /// Damage was dealt.
    DamageDealt {
        /// Source object, if any.
        source: Option<ObjectId>,
        /// What was damaged.
        target: DamageTarget,
        /// Amount (after prevention).
        amount: u32,
        /// Whether it was combat damage.
        is_combat: bool,
    },
    /// A die was rolled (custom modes).
    DiceRolled {
        /// Rolling player.
        player: PlayerId,
        /// Die size.
        sides: u32,
        /// Result.
        result: u32,
    },
    /// Mana was produced.
    ManaProduced {
        /// The player whose pool received it.
        player: PlayerId,
        /// Color of the mana.
        color: baylee_core::mana::ManaColor,
        /// Amount.
        amount: u16,
        /// Producing object, if any.
        source: Option<ObjectId>,
    },
    /// An object became tapped.
    ObjectTapped {
        /// The object.
        object: ObjectId,
        /// Why.
        cause: Cause,
    },
    /// An object became untapped.
    ObjectUntapped {
        /// The object.
        object: ObjectId,
        /// Why.
        cause: Cause,
    },
    /// A land was played.
    LandPlayed {
        /// The land object.
        object: ObjectId,
        /// The playing player.
        player: PlayerId,
    },
    /// A spell was cast (moved to the stack, costs paid).
    SpellCast {
        /// The spell object.
        object: ObjectId,
        /// The casting player.
        player: PlayerId,
        /// Instant ordinal frozen at casting; zero for noninstant spells.
        #[serde(default)]
        instant_number: u32,
        /// This player's spell number this turn, frozen at casting.
        number: u32,
    },
    /// A copy of a spell was put on the stack, without being cast (CR 707.10).
    SpellCopied {
        /// The newly created copy.
        object: ObjectId,
        /// The player who copied the spell.
        player: PlayerId,
    },
    /// A spell or ability resolved and left the stack.
    StackObjectResolved {
        /// The object.
        object: ObjectId,
    },
    /// A spell or ability left the stack **without resolving**.
    ///
    /// Two rules, through one door. A triggered ability whose
    /// intervening-`if` clause has stopped being true "is removed from the
    /// stack and does nothing" (CR 603.4); and a spell or ability all of
    /// whose targets have become illegal "doesn't resolve" and is removed
    /// the same way (CR 608.2b). The second one arrived after this comment
    /// said it had not, which is why it is worth saying that the event was
    /// already the right shape for it: the difference between the two is
    /// what left the stack and where it went, not how it is recorded.
    ///
    /// It is deliberately not [`GameEvent::SpellCountered`]: nothing
    /// countered this, and a card that cares about being countered would
    /// read one as the other.
    StackObjectDidNotResolve {
        /// The object.
        object: ObjectId,
    },
    /// A triggered/activated ability was put on the stack.
    AbilityTriggered {
        /// The ability object on the stack.
        object: ObjectId,
        /// Its source permanent/spell.
        source: ObjectId,
        /// Index into the source card's abilities.
        ability_index: u32,
        /// Controlling player.
        controller: PlayerId,
    },
    /// A spell was countered.
    SpellCountered {
        /// The countered spell object.
        object: ObjectId,
    },
    /// A card was discarded.
    Discarded {
        /// The card.
        object: ObjectId,
        /// Its exact incarnation in the hand.
        version: u32,
        /// The discarding player.
        player: PlayerId,
        /// Controller of the spell or ability causing the discard; no cost or cleanup.
        by: Option<PlayerId>,
        /// Whether its characteristics are defined (CR 701.9c).
        known: bool,
    },
    /// A permanent completed conniving (CR 701.50f); this is not a cast or draw.
    Connived {
        /// The permanent's exact incarnation, possibly departed.
        object: baylee_core::ids::DamageSourceRef,
        /// Its controller when instructed to connive.
        player: PlayerId,
    },
    /// A card was cycled: discarded to pay the cost of its own cycling
    /// ability (CR 702.29c). Recorded after that discard's own
    /// [`Self::Discarded`], which is the event every other reader of a
    /// discard hears; this one exists for "when you cycle this card".
    Cycled {
        /// The card, wherever the discard put it.
        object: ObjectId,
        /// The player who cycled it.
        player: PlayerId,
    },
    /// Cards were drawn (drives "whenever you draw" triggers).
    CardsDrawn {
        /// The drawing player.
        player: PlayerId,
        /// How many.
        count: u16,
        /// Whether the first of these cards is the first card `player`
        /// drew in their own draw step (CR 504.1). At most one card of an
        /// entry is. Read at the draw, not later: by the time triggers are
        /// collected, other draws may have followed it in the same batch.
        #[serde(default)]
        first_in_draw_step: bool,
        /// Total cards `player` has drawn this turn after this draw batch.
        #[serde(default)]
        drawn_this_turn_after: u32,
    },
    /// A player searched a library (CR 701.23a).
    LibrarySearched {
        /// The searching player.
        player: PlayerId,
    },
    /// A creature was declared as attacker.
    BecameAttacker {
        /// The attacking creature.
        object: ObjectId,
        /// What it was declared against.
        defending: baylee_core::ids::Defender,
    },
    /// An attacking creature was put in a band with one that has banding
    /// (CR 508.1e, 702.22c), one entry per member besides the one asked.
    Banded {
        /// The creature that joined.
        object: ObjectId,
        /// The creature with banding whose band it joined.
        with: ObjectId,
    },
    /// A creature was declared as blocker.
    BecameBlocker {
        /// The blocking creature.
        object: ObjectId,
        /// The attacker it blocks.
        attacker: ObjectId,
    },
    /// An attacking creature is unblocked once every defending player has
    /// declared blockers (CR 509.1h): no creature blocks it and none was
    /// declared to.
    AttackerUnblocked {
        /// The attacking creature.
        object: ObjectId,
    },
    /// A player lost the game.
    PlayerLost {
        /// The player.
        player: PlayerId,
        /// Why.
        reason: LossReason,
    },
    /// Control of a permanent changed.
    ControllerChanged {
        /// The object.
        object: ObjectId,
        /// Previous controller.
        old: PlayerId,
        /// New controller.
        new: PlayerId,
    },
    /// A permanent phased out or in.
    PhaseChanged {
        /// The object.
        object: ObjectId,
        /// Whether it is now phased out.
        phased_out: bool,
    },
    /// The game became day or night (CR 732.1).
    ///
    /// One event for both directions, because CR 732.1a defines "night
    /// becomes day" as a single change — losing one designation and
    /// gaining the other — and the game's first designation is the same
    /// change from nothing.
    DayNightChanged {
        /// The designation the game now has.
        now: crate::turn::DayNight,
    },
    /// A cast was taken back before it was complete (`PlayerAction::CancelCast`,
    /// or a payment window passed short), and reversed (CR 734.1). The card
    /// never left its zone, so this says nothing an opponent did not see.
    CastCancelled {
        /// The player who was casting.
        player: PlayerId,
        /// The card, where it still is.
        card: ObjectId,
        /// Why it was taken back.
        reason: CastCancelReason,
    },
    /// A player became the monarch (CR 726.3), and whoever was the monarch
    /// ceased to be.
    ///
    /// Recorded only when the designation moves: a monarch told to become
    /// the monarch again does not become it (CR 603.2e reads "becomes" as
    /// the change, not the state). The game going on with no monarch after
    /// the last heir left (CR 726.4) records nothing.
    BecameMonarch {
        /// The new monarch.
        player: PlayerId,
    },
    /// A player took the initiative (CR 725.1–725.3), including when they
    /// already held it.
    TookInitiative {
        /// The player who took the initiative.
        player: PlayerId,
    },
    /// A permanent turned over (CR 701.27).
    ///
    /// Recorded by both doors that flip a face: the one a resolving effect
    /// queues and the one daybound/nightbound take immediately. Without it
    /// a client's `face_index` would be right while nothing that reads the
    /// journal — triggers, replays — knew a transform had happened.
    /// A face-down permanent was turned face up without changing zones.
    TurnedFaceUp {
        /// The permanent.
        object: ObjectId,
    },
    /// A permanent transformed to its other printed face.
    Transformed {
        /// The permanent.
        object: ObjectId,
        /// The face it now shows.
        face: u8,
    },
    /// A Room was given a half's unlocked designation (CR 709.5c), as it
    /// entered cast as that half or by the unlock special action.
    DoorUnlocked {
        /// The permanent.
        object: ObjectId,
        /// Which half: 0 the left, 1 the right.
        half: u8,
    },
    /// A decision-free segment was found to repeat itself: a real endless
    /// loop rather than a large-but-finite pile of work (house rule, see
    /// [`crate::loops`]).
    LoopDetected {
        /// Length of the repeat, in sampled machine iterations.
        period: u64,
        /// `true` when the engine broke the loop and play continued;
        /// `false` when the game ended in a draw instead (CR 104.4b).
        broken: bool,
    },
    /// The game ended with a winner.
    GameWon {
        /// Who won (`None` = draw) — a seat, or a whole team.
        winner: Option<crate::win::Victor>,
    },
    /// A developer command was applied (dev games only).
    DevCommandApplied {
        /// Issuing seat.
        seat: PlayerId,
        /// Command text.
        command: String,
    },
    /// A seat's standing policy for one ability answered a question for it
    /// (#234): passed priority over that ability on top of the stack, or
    /// gave the seat's standing yes or no to that ability's optional
    /// question.
    ///
    /// Recorded only where the per-ability policy made the difference. A
    /// pass the seat's own priority hold would have made anyway is the
    /// hold's, which the seat already sees as held. The policy is the
    /// seat's own setting, so a host tells this to that seat alone; the game
    /// log leaves it out.
    AutoAnswered {
        /// The seat answered for.
        player: PlayerId,
        /// The ability the policy is filed under.
        ability: AbilityRef,
        /// The stack object the question was about: the ability passed
        /// over, or the spell or ability resolving when it asked. `None`
        /// when nothing on the stack asked.
        object: Option<ObjectId>,
        /// What the policy answered.
        answer: PolicyAnswer,
    },
    /// An attacker became blocked, independently of the individual block pairs.
    AttackerBlocked {
        /// The attacking creature.
        object: ObjectId,
    },
    /// A mutating creature spell merged with a creature on the battlefield (CR 702.140c).
    CreatureMutated {
        /// The resulting mutated creature.
        object: ObjectId,
    },
    /// Madness replaced a discard's destination with exile (CR 702.35a).
    /// This records the exact exile incarnation whose triggered ability may
    /// offer the cast; an arbitrary hand-to-exile move is not madness.
    MadnessExiled {
        /// The discarded card, now in exile.
        object: ObjectId,
        /// Its identity after entering exile (CR 400.7).
        version: u32,
        /// Its owner, who controls the madness trigger.
        owner: PlayerId,
    },
    /// A permanent on the battlefield became exerted (CR 701.43a–d).
    PermanentExerted {
        /// The exerted permanent.
        object: ObjectId,
        /// The player who exerted it.
        player: PlayerId,
        /// Whether exerted to pay a cost or by an effect.
        cause: Cause,
    },
    /// An attacking creature enlisted another creature by tapping it as an
    /// optional attack cost (CR 702.154a–d).
    CreatureEnlisted {
        /// The attacking creature whose enlist cost was paid.
        attacker: ObjectId,
        /// The creature tapped to pay that enlist cost.
        enlisted: ObjectId,
        /// The attacking player who paid the enlist cost.
        player: PlayerId,
    },
    /// A card became plotted in exile (CR 702.170a, CR 702.170c).
    CardPlotted {
        /// The plotted card, now face up in exile.
        object: ObjectId,
        /// The player who plotted it.
        player: PlayerId,
    },
    /// A player gave a promised gift to an opponent (CR 702.174c).
    GiftGiven {
        /// The controller of the resolving gift spell or permanent gift trigger.
        giver: PlayerId,
        /// The chosen opponent who received the gift.
        recipient: PlayerId,
    },
    /// A creature exploited a creature by sacrificing it as its exploit ability
    /// resolved (CR 702.110a–b).
    CreatureExploited {
        /// Exact incarnation of the exploiting creature.
        exploiter: baylee_core::ids::DamageSourceRef,
        /// The sacrificed exploited creature.
        exploited: ObjectId,
        /// The player who sacrificed the creature.
        player: PlayerId,
    },
    /// A spell was cast for its warp cost (CR 702.185a–c).
    SpellWarped {
        /// The warped spell on the stack.
        object: ObjectId,
        /// The player who cast it for its warp cost.
        player: PlayerId,
    },
    /// A player scried one or more cards (CR 701.22a–d; scrying 0 emits no
    /// event per CR 701.22b).
    PlayerScried {
        /// The player who scried.
        player: PlayerId,
        /// Number of cards scried (`>= 1`).
        count: u8,
    },
    /// A permanent became monstrous via monstrosity N (CR 701.37a–c).
    BecameMonstrous {
        /// The permanent that became monstrous.
        object: ObjectId,
        /// The monstrosity value `N` / `X` (`CR 701.37c`).
        count: u32,
    },
    /// The Ring tempted a player (`CR 701.54a–e`).
    RingTempted {
        /// The player whom The Ring tempted.
        player: PlayerId,
        /// The player's Ring-bearer after choosing, if any (`CR 701.54a`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ring_bearer: Option<ObjectId>,
        /// Total times The Ring has tempted `player` this game (`1..=255`).
        count: u8,
    },
    /// A player firebent via Firebending N (`CR 702.189a–b`).
    PlayerFirebent {
        /// The player who firebent.
        player: PlayerId,
        /// Source of the firebending ability.
        source: ObjectId,
        /// Amount of `{R}` produced (`>= 1`).
        amount: u16,
    },
    /// A player committed a crime by casting a spell, activating an ability, or
    /// putting a triggered ability on the stack targeting an opponent, a
    /// permanent/spell/ability an opponent controls, or a card in an opponent's
    /// graveyard (CR 700.13).
    CommittedCrime {
        /// The player who committed the crime.
        player: PlayerId,
    },
    /// A player spent mana to cast a spell, advancing their cumulative spell
    /// mana expended this turn from `previous` to `total` (expend N, CR 700.14).
    ManaExpended {
        /// The casting player.
        player: PlayerId,
        /// Cumulative mana spent on spells this turn before this spell.
        previous: u32,
        /// Cumulative mana spent on spells this turn after this spell.
        total: u32,
    },
    /// A player participated in a clash (CR 701.30a–d).
    Clashed {
        /// The clashing player.
        player: PlayerId,
        /// Whether this player won the clash (CR 701.30d).
        won: bool,
    },
    /// A class level bar set the level of this battlefield incarnation (CR 717.2a).
    ClassLevelChanged {
        /// The permanent whose level changed.
        object: ObjectId,
        /// Its battlefield incarnation when the level changed.
        version: u32,
        /// Its new level.
        level: u8,
        /// Whether the identity of the permanent was concealed at this event.
        face_down: bool,
    },
    /// A player cast a named-option ballot in an [`baylee_cards_dsl::Effect::Vote`]
    /// (`CR 701.38`).
    VotedOption {
        /// The voter.
        player: PlayerId,
        /// Source spell/permanent whose vote option was chosen.
        source: ObjectId,
        /// Zero-based index into `options`.
        option: u8,
    },
}

/// Where in a library a card was put.
///
/// What the move asked for, not where the card happens to sit: a card put
/// on the bottom of an empty library is [`Self::Bottom`]. A card shuffled
/// in is put on top and then shuffled, and says so as a `Shuffled` of that
/// library right after it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum LibraryPlace {
    /// On top.
    Top,
    /// On the bottom.
    Bottom,
    /// Nth from the top, counting the top card as 1. Never 1 and never the
    /// bottom card: those are [`Self::Top`] and [`Self::Bottom`].
    FromTop(u32),
}

/// What a seat's standing policy for one ability answered for it (#234).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum PolicyAnswer {
    /// Passed priority over the ability on top of the stack.
    Passed,
    /// Said yes to the ability's optional question.
    Yes,
    /// Said no to it.
    No,
}

/// Why a player lost the game.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum LossReason {
    /// Life total reached 0 or less (CR 104.3b).
    Life,
    /// Drew from an empty library (CR 104.3c).
    EmptyDraw,
    /// Ten or more poison counters (CR 104.3d).
    Poison,
    /// Twenty-one combat damage from one commander (CR 903.10a).
    ///
    /// Its own reason and not [`Self::Life`] because it is not a life
    /// total: a player on forty life loses to it, and the log saying "life"
    /// there would describe a game nobody played.
    CommanderDamage,
    /// Concession (CR 104.3a).
    Conceded,
    /// An effect said so (CR 104.3e): a pact's "if you don't, you lose the
    /// game" and its kind.
    ///
    /// Its own reason for the same cause [`Self::CommanderDamage`] is: the
    /// seat that did not pay for its pact may be on twenty life, and "life"
    /// would name a loss that did not happen.
    Effect,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

/// Immutable battlefield information for a zone-change event. Kept on the
/// journal entry because an object may move again before triggers are collected.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Departure {
    /// The operation was a sacrifice, even if a replacement changed its destination.
    #[serde(default)]
    pub sacrificed: bool,
    /// Independent undying and persist instances immediately before departure.
    #[serde(default)]
    pub return_keywords: [u32; 2],
    /// Independent modular instances immediately before departure (CR 702.43b).
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub modular_instances: u32,
    /// Identity immediately before leaving the battlefield.
    #[serde(default)]
    pub version: u32,
    /// Power immediately before departure (zero for a noncreature).
    #[serde(default)]
    pub power: i16,
    /// Controller immediately before departure.
    pub controller: PlayerId,
    /// Toughness immediately before departure (zero for a noncreature).
    pub toughness: i16,
    /// Auras/equipment attached immediately before departure.
    pub attachments: Vec<ObjectId>,
    /// Player whose Ring-bearer this permanent was immediately before departure (`CR 603.10a`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub was_ring_bearer_of: Option<PlayerId>,
    /// Whether this incarnation was attacking immediately before departure.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub was_attacking: bool,
}

impl std::hash::Hash for Departure {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.sacrificed.hash(state);
        self.return_keywords.hash(state);
        if self.modular_instances > 0 {
            self.modular_instances.hash(state);
        }
        self.version.hash(state);
        self.power.hash(state);
        self.controller.hash(state);
        self.toughness.hash(state);
        self.attachments.hash(state);
        if let Some(who) = self.was_ring_bearer_of {
            who.hash(state);
        }
        if self.was_attacking {
            Departure::ATTACKING_TAG.hash(state);
        }
    }
}

impl Departure {
    // Above the entire encoded Ring-bearer seat range.
    const ATTACKING_TAG: u16 = 257;

    /// Stable continuation fingerprint while a departure awaits trigger collection.
    pub(crate) fn fingerprint(&self) -> u64 {
        let mut hash =
            1 + u64::from(self.controller.get()) + (u64::from(self.toughness as u16) << 8);
        hash = hash
            .wrapping_mul(31)
            .wrapping_add(u64::from(self.sacrificed));
        hash = hash.wrapping_mul(31).wrapping_add(u64::from(self.version));
        hash = hash
            .wrapping_mul(31)
            .wrapping_add(u64::from(self.power as u16));
        for count in self.return_keywords {
            hash = hash.wrapping_mul(31).wrapping_add(u64::from(count));
        }
        if self.modular_instances > 0 {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(u64::from(self.modular_instances));
        }
        for id in &self.attachments {
            hash = hash.wrapping_mul(31).wrapping_add(u64::from(id.slot()));
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(u64::from(id.generation()));
        }
        if let Some(who) = self.was_ring_bearer_of {
            hash = hash.wrapping_mul(31).wrapping_add(u64::from(who.get()) + 1);
        }
        if self.was_attacking {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(u64::from(Self::ATTACKING_TAG));
        }
        hash
    }
}

/// One journaled entry.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct JournalEntry {
    /// 1-based sequence number.
    pub seq: u64,
    /// The event.
    pub event: GameEvent,
    /// Last battlefield information, present only on a departure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub departure: Option<Box<Departure>>,
    /// Identity and controller of the permanent when it tapped or untapped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tap_observation: Option<TapObservation>,
    /// Recipient-wide excess damage for this batch (`CR 120.10`), stored once on
    /// its first `GameEvent::DamageDealt` to this recipient. Zero otherwise.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub excess_damage: u32,
}

/// Information captured at a tap or untap, before later instructions resolve.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct TapObservation {
    /// Incarnation of the permanent at the event.
    pub version: u32,
    /// Controller of that incarnation.
    pub controller: PlayerId,
    /// Projected power of the permanent at the event.
    pub power: i16,
    /// Projected toughness of the permanent at the event.
    pub toughness: i16,
}

/// The append-only event journal.
///
/// Checkpoints share immutable entries. Recording or undoing an event copies
/// shared storage, so neither operation can change an earlier snapshot.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Journal {
    entries: Arc<Vec<JournalEntry>>,
}

impl Journal {
    /// Combat membership of this exact permanent immediately before it left.
    pub(crate) fn was_attacking_on_departure(&self, id: ObjectId, version: u32) -> bool {
        self.entries()
            .iter()
            .rev()
            .find_map(|entry| {
                let departure = entry.departure.as_ref()?;
                (departure.version == version
                    && matches!(entry.event, GameEvent::ZoneChanged { object, .. } if object == id))
                .then_some(departure.was_attacking)
            })
            .unwrap_or(false)
    }

    /// Appends an event, returning its sequence number.
    pub fn record(&mut self, event: GameEvent) -> u64 {
        self.record_with_tap_observation(event, None)
    }

    pub(crate) fn record_with_tap_observation(
        &mut self,
        event: GameEvent,
        tap_observation: Option<TapObservation>,
    ) -> u64 {
        let seq = self.entries.len() as u64 + 1;
        Arc::make_mut(&mut self.entries).push(JournalEntry {
            seq,
            event,
            departure: None,
            tap_observation,
            excess_damage: 0,
        });
        seq
    }

    /// Stores the batch's excess damage to `target` on its first `DamageDealt` entry.
    pub(crate) fn set_last_excess_damage(&mut self, target: ObjectId, excess: u32) {
        if excess == 0 {
            return;
        }
        if let Some(last) = Arc::make_mut(&mut self.entries).last_mut()
            && matches!(
                last.event,
                GameEvent::DamageDealt {
                    target: DamageTarget::Object(id),
                    ..
                } if id == target
            )
        {
            last.excess_damage = excess;
        }
    }

    /// Record a zone change with its immutable battlefield information.
    pub(crate) fn record_departure(
        &mut self,
        event: GameEvent,
        departure: Option<Departure>,
    ) -> u64 {
        let seq = self.record(event);
        Arc::make_mut(&mut self.entries)
            .last_mut()
            .expect("just recorded")
            .departure = departure.map(Box::new);
        seq
    }

    /// All entries in order.
    #[must_use]
    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the journal is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The latest sequence number (0 = empty).
    #[must_use]
    pub fn last_seq(&self) -> u64 {
        self.entries.len() as u64
    }

    /// Drops every entry after the first `len`: the one exception to "only
    /// grows", for a payment that was canceled (CR 734.1) and so never
    /// happened ([`crate::state::GameState::roll_back`]).
    pub(crate) fn cancel_from(&mut self, len: usize) {
        if len < self.entries.len() {
            Arc::make_mut(&mut self.entries).truncate(len);
        }
    }

    /// Edits every entry in place, keeping its sequence number: the other
    /// exception, for a forked simulation withholding what its seat was not
    /// shown (`Engine::fork_for`). A journal shared with a snapshot is
    /// copied first, so the snapshot's entries never change.
    #[cfg(any(test, feature = "fork"))]
    pub(crate) fn rewrite(&mut self, mut edit: impl FnMut(&mut JournalEntry)) {
        for entry in Arc::make_mut(&mut self.entries) {
            edit(entry);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn started(seed: u64) -> GameEvent {
        GameEvent::GameStarted { seed, seats: 2 }
    }

    #[test]
    fn a_checkpoint_preserves_entries_across_append_undo_and_serialization() {
        let mut journal = Journal::default();
        journal.record(started(1));
        journal.record(started(2));
        let saved = journal.clone();
        assert!(Arc::ptr_eq(&journal.entries, &saved.entries));
        let before = serde_json::to_value(&saved).unwrap();
        journal.cancel_from(1);
        journal.record(started(3));
        assert_eq!(serde_json::to_value(&saved).unwrap(), before);
        assert_eq!(journal.last_seq(), 2);
        assert!(matches!(
            journal.entries()[1].event,
            GameEvent::GameStarted { seed: 3, .. }
        ));
        let restored: Journal = serde_json::from_value(before.clone()).unwrap();
        assert_eq!(serde_json::to_value(restored).unwrap(), before);
        journal = saved.clone();
        journal.record(started(4));
        assert_eq!(saved.len(), 2);
        assert_eq!(journal.len(), 3);
    }

    /// A sequence number is how everything else addresses an entry — a
    /// client's "what happened since" and the engine's own replay both take
    /// one and ask for what came after it. So it is 1-based and dense, and
    /// the number [`Journal::record`] hands back is the one the entry it
    /// just appended carries.
    #[test]
    fn a_journal_numbers_its_entries_from_one_and_densely() {
        let mut journal = Journal::default();
        assert!(journal.is_empty());
        assert_eq!(journal.len(), 0);
        assert_eq!(journal.last_seq(), 0, "0 is the empty journal's answer");

        let seqs: Vec<u64> = (0..3).map(|i| journal.record(started(i))).collect();
        assert_eq!(seqs, vec![1, 2, 3]);
        assert!(!journal.is_empty());
        assert_eq!(journal.len(), 3);

        for (i, entry) in journal.entries().iter().enumerate() {
            assert_eq!(entry.seq, seqs[i], "record answered with its own entry");
            assert!(
                matches!(entry.event, GameEvent::GameStarted { seed, .. } if seed == i as u64),
                "entries are in the order they were recorded"
            );
        }
    }

    /// The same number derived two ways, which is the only thing about this
    /// wrapper that can go wrong: [`Journal::last_seq`] counts the entries
    /// and the entry counts itself. They agree for as long as the journal
    /// is append-only, and a removal — a compaction, a rollback, a rewind
    /// that dropped what it undid — would part them silently, because every
    /// reader takes the cheap one.
    #[test]
    fn the_last_sequence_number_agrees_with_the_last_entry() {
        let mut journal = Journal::default();
        for i in 0..5 {
            let seq = journal.record(started(i));
            assert_eq!(journal.last_seq(), seq);
            assert_eq!(
                journal.last_seq(),
                journal.entries().last().expect("just recorded").seq
            );
        }
    }
}
