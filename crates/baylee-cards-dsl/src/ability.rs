//! Ability definitions on cards.

use crate::cost::Cost;
use crate::effect::{Effect, TargetSpec};
use crate::filter::Filter;

/// When an activated ability may be played.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ActivationTiming {
    /// Any time you have priority.
    InstantSpeed,
    /// Requires priority even when the ability produces mana.
    PriorityOnly,
    /// Only in your main phase, empty stack ("as a sorcery").
    SorcerySpeed,
}

/// A printed constraint on an activated ability's X (CR 107.3a).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ActivationX {
    /// The controller may announce any nonnegative payable value.
    Any,
    /// The controller must announce at least this value.
    AtLeast(u32),
    /// The card defines X; determine it when announcing the activation.
    Defined(crate::Amount),
}

/// How often an activated ability may be used.
///
/// "Activate only once each turn" (Wall of Roots, Quirion Ranger, Scryb
/// Ranger) — a restriction on *activating*, so it is checked and recorded
/// where an activation is announced and paid for (CR 602.2), not where the
/// ability resolves. The tally is per object, so a permanent that leaves the
/// battlefield and comes back may be used again: CR 400.7 makes it a new
/// object with no memory of the old one.
///
/// **Each turn, not each of your turns.** A mana ability is activatable
/// whenever its controller has priority, so a Wall of Roots used on its
/// own turn is available again on the opponent's.
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ActivationLimit {
    /// No printed limit — as often as its cost can be paid (CR 602.2).
    Unlimited,
    /// Exhaust: once for this ability on this incarnation (CR 702.177a).
    Exhaust,
    /// At most this many activations per turn, per object.
    PerTurn(u8),
    /// At most the currently evaluated number of activations each turn.
    PerTurnAmount(crate::Amount),
    /// Boast: this exact creature must have attacked this turn; normally once.
    /// Rules effects may allow a second activation during its controller's turn.
    Boast,
}

/// Where an activated ability may be activated from.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ActivationZone {
    /// On the battlefield (default).
    Battlefield,
    /// On the battlefield, activated only by its owner, regardless of controller.
    BattlefieldOwner,
    /// Any player may activate this battlefield ability.
    BattlefieldAnyPlayer,
    /// Only opponents of this permanent's controller may activate it.
    BattlefieldOpponent,
    /// On the battlefield, activated only by the controller of the permanent
    /// this Aura/Equipment is attached to (Merseine: "Enchanted creature's
    /// controller may pay").
    BattlefieldAttachedController,
    /// From the owner's command zone.
    Command,
    /// From your hand (cycling).
    Hand,
    /// From your graveyard (eternalize, embalm): the card is in its owner's
    /// graveyard and the owner activates it.
    Graveyard,
    /// On the battlefield under your control, or from your command zone.
    BattlefieldOrCommand,
    /// From your hand or from your command zone (commander ninjutsu, CR 702.49d).
    /// Appended to preserve replay identities.
    HandOrCommand,
}

/// Where a printed triggered ability functions (CR 113.6).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TriggerZone {
    /// The normal zone; leaves-the-battlefield triggers also look back here.
    Battlefield,
    /// In the card owner's graveyard (CR 113.6b, 113.6m).
    Graveyard,
    /// In exile (CR 113.6b, All Hallow's Eve).
    Exile,
    /// In the card owner's command zone (CR 113.6b: Oloro, Ageless Ascetic).
    Command,
    /// On the battlefield or in the card owner's command zone (CR 113.6b:
    /// eminence — Edgar Markov, Inalla, Arahbo, Sidar Jabari).
    BattlefieldOrCommand,
}

impl TriggerZone {
    /// Whether this trigger zone includes the battlefield.
    #[must_use]
    pub const fn functions_on_battlefield(self) -> bool {
        matches!(self, Self::Battlefield | Self::BattlefieldOrCommand)
    }

    /// Whether this trigger zone includes the owner's command zone.
    #[must_use]
    pub const fn functions_in_command(self) -> bool {
        matches!(self, Self::Command | Self::BattlefieldOrCommand)
    }
}

/// Steps a trigger can listen to, and a step a timing restriction names.
///
/// Listed in turn order, which is the order [`Condition::BeforeStep`]
/// compares in.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum StepKind {
    /// Upkeep step.
    Upkeep,
    /// Draw step.
    Draw,
    /// Beginning of combat.
    CombatBegin,
    /// Declare attackers step (CR 508): attackers are declared as it
    /// begins.
    DeclareAttackers,
    /// Declare blockers step (CR 509): blockers are declared as it begins.
    DeclareBlockers,
    /// A combat damage step (CR 510), the first-strike one included: a
    /// combat with first strike has two, and "the combat damage step" is
    /// reached at the first of them.
    CombatDamage,
    /// End of combat step.
    CombatEnd,
    /// End step.
    End,
    /// First actual main phase reached this turn (not necessarily precombat).
    FirstMain,
}

/// Relational comparison operator between two [`crate::Amount`] expressions.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AmountCmp {
    /// `left >= right`.
    AtLeast,
    /// `left > right`.
    GreaterThan,
    /// `left <= right`.
    AtMost,
    /// `left < right`.
    LessThan,
    /// `left == right`.
    Equal,
    /// `left != right`.
    NotEqual,
}

impl AmountCmp {
    /// Evaluates `left <op> right`.
    #[must_use]
    pub const fn holds(self, left: u32, right: u32) -> bool {
        match self {
            Self::AtLeast => left >= right,
            Self::GreaterThan => left > right,
            Self::AtMost => left <= right,
            Self::LessThan => left < right,
            Self::Equal => left == right,
            Self::NotEqual => left != right,
        }
    }
}

/// A sentence a card states about the game, for an ability that only does
/// something while it is true.
///
/// It was `ActivationCondition` and named for its one reader — metalcraft
/// and the verge lands, gating [`AbilityDef::ActivatedConditional`]. The
/// name was the accident: a condition is about the *game*, not about how
/// the ability that states it gets used, and Magic asks the same sentences
/// of a triggered ability's intervening-`if` clause (CR 603.4). One
/// vocabulary, and one reader in `eval::condition_holds`, which is what
/// keeps the two from drifting into different answers to the same words.
///
/// What differs between the two is not the sentence but **when it is
/// asked**: an activation condition is checked once, when the ability
/// would be activated (CR 602.5 — "a player can't begin to activate an
/// ability that's prohibited from being activated"), and an intervening
/// `if` is checked twice — once when the ability would trigger and again
/// as it resolves.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Condition {
    /// This source incarnation was cast by the ability controller.
    SourceWasCastByYou,
    /// A card discarded for this ability's cost matched before leaving the hand.
    DiscardedForCost(&'static Filter),
    /// It is one of your first N turns to actually begin.
    DuringYourFirstTurns(u32),
    /// A permanent left the battlefield under your control this turn.
    Revolt,
    /// You cast an instant or sorcery spell this turn, even if countered.
    CastInstantOrSorceryThisTurn,
    /// You cast a creature spell this turn, even if countered.
    CastCreatureThisTurn,
    /// You declared at least this many distinct creature incarnations as attackers this turn.
    AttackedWithAtLeast(u32),
    /// This exact incarnation attacked during its controller's last completed turn.
    SourceAttackedDuringYourLastTurn,
    /// A permanent with any listed subtype entered under your control this turn.
    EnteredSubtypeUnderYourControlThisTurn(&'static [baylee_core::ids::SubtypeId]),
    /// One opponent was dealt at least this much damage this turn.
    OpponentWasDealtAtLeast(u32),
    /// Every player still in the game has an empty hand.
    EveryHandEmpty,
    /// At least one remaining player's library has at most this many cards.
    AnyLibraryAtMost(u32),
    /// The signed sum of power of your creatures meets this bound.
    ControlledCreaturePowerAtLeast(i32),
    /// This incarnation dealt damage to an opponent of its current controller this turn.
    SourceDamagedOpponentThisTurn,
    /// The source was present and untapped at the beginning of this turn.
    SourceStartedTurnUntapped,
    /// At least one opponent has this many poison counters.
    OpponentPoisonAtLeast(u16),
    /// An evaluated magnitude meets the printed lower bound.
    AmountAtLeast(crate::Amount, u32),
    /// The evaluated number is even, including zero.
    AmountIsEven(crate::Amount),
    /// This incarnation blocked or became blocked since your previous upkeep.
    SourceBlockedOrWasBlockedSinceYourLastUpkeep,
    /// A creature damaged by this source incarnation died this turn.
    CreatureDamagedBySourceDiedThisTurn,
    /// At least one eligible defending player controls no matching permanent.
    SomeDefendingPlayerControlsNone(&'static Filter),
    /// At least one opponent controls at most this many matching permanents.
    OpponentControlCountAtMost(&'static Filter, u32),
    /// An opponent controls more matching permanents than you do.
    OpponentControlsMore(&'static Filter),
    /// An opponent controls at least N more matching permanents than you.
    OpponentControlsMoreBy(&'static Filter, u32),
    /// This player did not start the game, independent of the current turn.
    NotStartingPlayer,
    /// At least this many of the named card types occur in your graveyard.
    GraveyardTypesAmongAtLeast {
        /// Card types that contribute to the count.
        types: baylee_core::types::TypeSet,
        /// Minimum number of distinct types.
        minimum: u8,
    },
    /// This incarnation was declared as an attacker or blocker this combat,
    /// even if it has since been removed from combat.
    AttackedOrBlockedThisCombat,
    /// It is the current controller's turn (not necessarily the owner's).
    YourTurn,
    /// The controller has at most this many life.
    LifeAtMost(i32),
    /// "If your life total is less than your starting life total" (Resolute
    /// Archangel): the controller's life is below the starting life total of
    /// this game (CR 103.4), whatever the format made it.
    LifeBelowStarting,
    /// "If no spells were cast last turn" — by any player (the Innistrad
    /// werewolves). There was no last turn at the first upkeep of the game,
    /// so the sentence is false there, as daybound's own check is skipped.
    NoSpellsCastLastTurn,
    /// "If you haven't cast a spell this turn" (Conduit of Worlds): the
    /// controller has cast no spell this turn, counting spells that were
    /// countered since and spells cast before the source was theirs.
    YouCastNoSpellThisTurn,
    /// "If a player cast N or more spells last turn" — one player, any
    /// player: the werewolves' way back to their front face.
    APlayerCastLastTurnAtLeast(u8),
    /// You control at least N permanents matching the filter.
    ControlCount(&'static Filter, u8),
    /// At least this many controlled matches, excluding the exact event object.
    ControlCountExceptEvent(&'static Filter, u8),
    /// About the triggering event's creature: at least one creature other
    /// than this source blocking it matches `some`, and no creature blocking
    /// it matches `none` (Wall of Caltrops: "if at least one other Wall
    /// creature is blocking that creature and no non-Wall creatures are
    /// blocking that creature").
    EventBlockedByOthers {
        /// What at least one other blocker is.
        some: &'static Filter,
        /// What no blocker may be.
        none: &'static Filter,
    },
    /// "Cast this spell only after combat" (Glyph of Reincarnation): the
    /// turn is in its postcombat main phase or its end phase.
    AfterCombat,
    /// The opponent the source chose as it entered
    /// (`EnterModifier::ChooseOpponent`) controls a permanent the filter
    /// matches, read with the ability's controller as "you" and the source
    /// as "this" (Jihad: "as long as the chosen player controls a nontoken
    /// permanent of the chosen color"). False with no chosen opponent, or
    /// once that player has left the game.
    ChosenOpponentControls(&'static crate::Filter),
    /// The chosen color occurs strictly more often than every other color among
    /// the chosen opponent's matching permanents. Multicolored objects count for each color.
    ChosenColorIsUniqueMostCommon(&'static crate::Filter),
    /// The resolution's first target is a spell that would destroy a
    /// permanent the filter matches if it resolved now (Equinox: "if it
    /// would destroy a land you control"): one of its own top-level
    /// `Destroy` instructions names such a permanent among its targets, or
    /// one of its `DestroyAll` sweeps (read with the spell's controller as
    /// "you") reaches one. What the spell would destroy only after a choice
    /// (a "may", a choice made as it resolves, a random pick) or only
    /// indirectly (damage, a sacrifice) does not count (the 2004 and 2008
    /// rulings), and neither does an indestructible permanent. Meaningful
    /// only while a resolution asks it; anywhere else it is false.
    TargetSpellWouldDestroy(&'static crate::Filter),
    /// You control **at most** N permanents matching the filter —
    /// Glimmervoid's "if you control no artifacts" and Thran Quarry's "if
    /// you control no creatures", both at nought.
    ///
    /// The downward twin of the line above, and not a parameter on it: the
    /// two sentences are opposite in what a board does to them, and a card
    /// that counts downwards is sacrificing itself while one that counts up
    /// is being rewarded. Its absence was not two cards sitting unwritten —
    /// both lands shipped with the end-step clause **off**, which is a land
    /// that never sacrifices itself and so is strictly stronger than the one
    /// printed.
    ControlCountAtMost(&'static Filter, u8),
    /// At least N permanents on the battlefield match the filter, whoever
    /// controls them — "if there are three or more creatures on the
    /// battlefield". [`Self::ControlCount`] counts one player's side of the
    /// table and this one all of it.
    BattlefieldCount(&'static Filter, u8),
    /// At most N permanents on the battlefield match the filter, whoever
    /// controls them — Pestilence's "if no creatures are on the
    /// battlefield" is at most none.
    BattlefieldCountAtMost(&'static Filter, u8),
    /// You control permanents matching the filter with at least N
    /// **different names** among them — Field of the Dead's "if you
    /// control seven or more lands with different names".
    ///
    /// A count of names, not of permanents: two Forests are one. A
    /// permanent with no name (a face-down one, CR 708.2a) has no name to
    /// differ by and adds nothing.
    ControlDistinctNames(&'static Filter, u8),
    /// **An opponent** controls at least N permanents matching the filter
    /// (Tectonic Edge — "activate only if an opponent controls four or more
    /// lands").
    ///
    /// `any` and not a sum: the sentence is true at a table where one of
    /// three opponents has four lands, which is the same reading
    /// [`Self::OpponentGraveyardCountAtLeast`] already makes of its own
    /// seat.
    ///
    /// The seat is narrowed by the walk and the filter is evaluated from
    /// the **asker's** side, so `Filter::ControlledByOpponent` inside one
    /// is true and `Filter::ControlledByYou` is the contradiction. That is
    /// not a free choice: `xtask validate` holds a card whose text says
    /// "an opponent controls" to a filter that says so, so the scope is
    /// written on the card as well as walked here.
    OpponentControlCount(&'static Filter, u8),
    /// You have at most N cards in hand — hellbent at nought (Keldon
    /// Megaliths, Sea Gate Wreckage).
    ///
    /// Hellbent is an ability word with no rules meaning, exactly like the
    /// threshold below, so the count is a parameter rather than a fixed
    /// nought and nothing here may read the word.
    HandSizeAtMost(u8),
    /// The active player had at most N cards in hand at the beginning of
    /// this turn, before any turn-based action or trigger.
    HandSizeAtTurnStartAtMost(u8),
    /// At least one player still in the game has at most N cards in hand.
    APlayerHandSizeAtMost(u8),
    /// The controller has played at least N lands this turn (CR 305.2).
    /// Fastbond's "if it wasn't the first land you played this turn" is 2:
    /// a trigger on the play is collected after the land it is about was
    /// counted, and the count only grows within a turn, so the second ask
    /// at resolution (CR 603.4) answers as the first did. The count is
    /// reset as its player's own turn begins, which is the only turn this
    /// engine lets a player play lands on.
    LandsPlayedThisTurnAtLeast(u8),
    /// You have **exactly** N cards in hand (Library of Alexandria).
    ///
    /// The sibling of the line above for the same reason
    /// [`Self::CountersOnSelfExactly`] is the sibling of
    /// [`Self::CountersOnSelf`]: a card that prints "exactly seven" is a
    /// card that stops working when you draw the eighth, and an "at most"
    /// reading of it would hand the draw to every hand size below seven.
    HandSizeExactly(u8),
    /// An opponent has at least N cards in their graveyard (Sheoldred's
    /// flip condition).
    OpponentGraveyardCountAtLeast(u8),
    /// **Your** graveyard holds at least N cards — threshold, always seven.
    ///
    /// Threshold has **no CR rule** and deliberately no keyword bit: the
    /// glossary says it "used to be a keyword ability. It is now an ability
    /// word and has no rules meaning", and every card printed with it was
    /// errata'd into the words it stands for. So the condition is the whole
    /// of it, the count is a parameter rather than a fixed 7, and nothing
    /// here may read the word.
    ///
    /// The sibling of the line above and not a parameter on it, because the
    /// two count different players and no card asks the question with the
    /// seat left open. Its absence was not a card sitting unwritten: Cabal
    /// Pit and Centaur Garden shipped the gated ability **ungated**, which
    /// is a land strictly stronger than the one printed, while Barbarian
    /// Ring left the same ability off entirely. One missing variant, two
    /// opposite wrong answers — which is the argument for the variant rather
    /// than for a house style.
    GraveyardCountAtLeast(u8),
    /// At least this many distinct card types among cards in your graveyard.
    GraveyardTypesAtLeast(u8),
    /// This card is in your graveyard with at least N matching **cards**
    /// above it. The graveyard's order matters (CR 404.1–3); cards below
    /// the source and tokens do not count.
    GraveyardCardsAbove(&'static Filter, u8),
    /// The source has at least N counters of a kind (Luminarch
    /// Ascension's quest counters).
    CountersOnSelf(crate::effect::CounterKind, u8),
    /// The source has EXACTLY N counters of a kind (class level gating).
    CountersOnSelfExactly(crate::effect::CounterKind, u8),
    /// The source has at least the first number and no more than the second
    /// of a kind of counter: a leveler's `{LEVEL N1-N2}` band (CR 711.2a,
    /// "As long as this creature has at least N1 level counters on it, but
    /// no more than N2 level counters on it"). The open `{LEVEL N3+}` band
    /// is [`Self::CountersOnSelf`] (CR 711.2b).
    CountersOnSelfBetween(crate::effect::CounterKind, u8, u8),
    /// A station symbol, `{N+}`: the source has N or more charge counters
    /// on it (CR 722.2a, "As long as this permanent has N or more charge
    /// counters on it, it has [abilities]").
    ///
    /// The same count as `CountersOnSelf(CounterKind::Charge, N)`, and a
    /// variant of its own because it is asked differently. It says whether
    /// the permanent **has** the ability, not an intervening `if` (CR
    /// 603.4): a static ability carrying it applies only while it holds, an
    /// activated one may be activated only while it holds, and a triggered
    /// one triggers only while it holds and is then not asked again, because
    /// a triggered ability on the stack exists independently of its source
    /// (CR 113.7a). An Inspirit, Flagship Vessel destroyed in response to
    /// its own combat trigger still puts the counters on its target.
    Station(u8),
    /// The controller has earned an enduring story (CR 702.195).
    EnduringStory,
    /// The controller has the city's blessing (CR 702.131c).
    CitysBlessing,
    /// The controller has speed 4.
    MaxSpeed,
    /// The source itself matches the filter — "if this land is tapped".
    ///
    /// The other four sentences here count something the source is not;
    /// this one is a filter pointed back at the object that states it,
    /// which is what the storage lands' upkeep trigger asks and what most
    /// of the reference corpus's `PresentDefined$ Self` writes.
    ///
    /// A source that is no longer there does **not** match: the ability is
    /// a separate object from the moment it goes on the stack (CR 113.7a),
    /// so a land that left the battlefield between the trigger and its
    /// resolution leaves "this land is tapped" with nothing to be true of.
    SourceMatches(&'static Filter),
    /// The permanent enchanted or equipped by the source matches this
    /// filter. An intervening condition reads the attachment again on
    /// resolution, using the source's last attachment if it has left.
    AttachedMatches(&'static Filter),
    /// "If you can't" of "sacrifice a [filter]": you control a permanent
    /// the filter matches, asked with the ability's source as the filter's
    /// `This` — the permanents `Effect::SacrificeFilter` would offer you.
    /// Lord of the Pit's "sacrifice a creature other than this creature. If
    /// you can't, …" is `IfCondition { condition: CanSacrifice(&f), then:
    /// &[SacrificeFilter { who: You, filter: &f }], otherwise: … }`.
    ///
    /// Not `ControlCount(&f, 1)`: that one asks each permanent with itself
    /// as `This`, so `Filter::Another` never matches there.
    CanSacrifice(&'static Filter),
    /// "Activate only during combat" (Jade Statue): the combat phase of any
    /// player's turn, from the beginning of combat step to the end of combat
    /// step (CR 506.1).
    DuringCombat,
    /// At least one of these holds ("activate only if this land entered
    /// this turn **or** if you control a basic land" — the Gathering Place
    /// cycle).
    ///
    /// A combinator rather than a flag on each variant, because the two
    /// halves of that sentence are different kinds of question — one reads
    /// the source, the other counts the board — and a variant that carried
    /// "or you control a basic land" would have to be added to every
    /// `Condition` the cycle could ever pair it with.
    ///
    /// A conjunction printed inside an effect is nested
    /// `Effect::IfCondition`s (the Urza lands' "if you control an Urza's
    /// Mine and an Urza's Power-Plant"); [`Condition::All`] is for one that
    /// stands where a single condition is all there is room for.
    Any(&'static [Condition]),
    /// Every one of these holds. "Activate only during your upkeep" is
    /// `All(&[YourTurn, DuringStep(StepKind::Upkeep)])`: a restriction on
    /// activating (CR 602.5) is one condition, and the sentence is two.
    All(&'static [Condition]),
    /// "If a player has more life than each other player" (Ghazbán Ogre):
    /// one player still in the game has strictly the most life.
    APlayerHasMostLife,
    /// It is an opponent's turn: the active player is an opponent of
    /// "you". Not `Not(&YourTurn)`, which a teammate's turn also satisfies.
    OpponentsTurn,
    /// The current step is this one ("activate only during the declare
    /// blockers step"). A turn's two main phases have no step and are never
    /// it.
    DuringStep(StepKind),
    /// This turn has not yet reached this step (CR 506.8): "cast this spell
    /// only before blockers are declared" is `BeforeStep(DeclareBlockers)`,
    /// since blockers are declared as that step begins (CR 506.8b), and
    /// "before the combat damage step" is `BeforeStep(CombatDamage)`. Past
    /// the step's place in the turn it is false, whether or not the step
    /// happened: with no attackers there is no combat damage step (CR
    /// 508.8), and the end of combat step is after it all the same (CR
    /// 506.7e).
    BeforeStep(StepKind),
    /// "If X is N or more" — the X announced for the spell that is the
    /// source (Finale of Devastation). Read off the source's announced X,
    /// as `Filter::CmcAtMostX` reads it; a source that is gone or announced
    /// none has X = 0.
    XAtLeast(u32),
    /// Holds while the condition it names does not — the printed "unless":
    /// Wayward Swordtooth "can't attack or block unless you have the city's
    /// blessing" is a static that holds while `Not(&CitysBlessing)` does.
    Not(&'static Condition),
    /// "If this spell's dash cost was paid" (CR 702.109a), asked of the
    /// source: it is on the battlefield as the permanent a spell cast for
    /// its dash cost became, and has not left it since. The engine writes
    /// dash's return itself; no card prints this.
    DashCostPaid,
    /// "Unless it escaped" (Uro, Titan of Nature's Wrath): the source is the
    /// spell cast from a graveyard with escape, or the permanent that spell
    /// became, and has not left the battlefield since (CR 702.138b).
    Escaped,
    /// At least this many creatures died during the current turn.
    CreaturesDiedThisTurnAtLeast(u32),
    /// This player sacrificed at least this many permanents with the subtype this turn.
    /// Reads actual sacrifice events and captured subtype/controller facts.
    SacrificedSubtypeThisTurn {
        /// Subtype captured when the permanent was sacrificed.
        subtype: baylee_core::ids::SubtypeId,
        /// Minimum actual sacrifices by this player this turn.
        count: u32,
    },
    /// A permanent card owned by this player reached their graveyard this turn.
    /// Tokens and destination-replaced moves do not qualify.
    DescendedThisTurn,
    /// Total counters of all kinds on the source is even (0 is even; Sab-Sunen, Luxa Embodied).
    EvenCountersOnSelf,
    /// The permanent sacrificed or exiled as a cost to cast this spell or
    /// activate this ability matched the filter as it last existed on the
    /// battlefield (Ebon Praetor, Soul Exchange: "if the sacrificed/exiled
    /// creature was a Thrull").
    SacrificedForCost(&'static Filter),
    /// Echo's intervening `if` (CR 702.30a, CR 603.4): this permanent came
    /// under your control since the beginning of your last upkeep, and it is
    /// still yours to sacrifice.
    SourceCameUnderYourControlSinceYourLastUpkeep,
    /// The ability's source was blocked earlier this turn, as that exact
    /// battlefield incarnation (CR 400.7, CR 603.4).
    SourceWasBlockedThisTurn,
    /// You control strictly more matching permanents than each other
    /// still-in-game player (Peer Pressure). Appended to preserve replay
    /// identities.
    ControlMoreThanEachOtherPlayer(&'static Filter),
    /// The source spell (or the permanent that spell became as it resolved)
    /// was kicked with `face.additional_costs[index]` (CR 702.33f, CR 400.7d).
    /// Appended to preserve replay identities.
    KickedWith(u8),
    /// "If this spell's blitz cost was paid" (CR 702.152a), asked of the
    /// source on the battlefield as the permanent a spell cast for its blitz
    /// cost became, and not having left it since. Appended to preserve replay
    /// identities.
    BlitzCostPaid,
    /// The source spell (or the permanent that spell became on resolving from
    /// the stack) was cast for an `AlternativeCost` (`CastModeKind::Alternative`,
    /// e.g. spectacle, surge, prowl, evoke; CR 601.2b, CR 400.7d). Appended to
    /// preserve replay identities.
    AltCostPaid,
    /// An opponent lost life this turn (Spectacle, CR 702.137a; Bloodthirst).
    /// Appended to preserve replay identities.
    OpponentLostLifeThisTurn,
    /// You or one of your teammates has cast another spell this turn (Surge,
    /// CR 702.117a). Appended to preserve replay identities.
    Surge,
    /// A player was dealt combat damage this turn by a source that, at the
    /// time it dealt that damage, was under your control and had any of this
    /// spell's creature types (Prowl, CR 702.76a). Appended to preserve replay
    /// identities.
    Prowl,
    /// You control one of your own designated commanders on the battlefield
    /// (CR 903.3c–d, 702.124d: Bastion Protector, Loyal Apprentice). Appended
    /// to preserve replay identities.
    ControlYourCommander,
    /// This exact source object is currently in the stated zone (CR 113.6b,
    /// CR 603.4, CR 400.7: Oloro's and eminence's intervening `if`). Appended
    /// to preserve replay identities.
    SourceInZone(crate::ZoneRef),
    /// This spell (or the permanent it became) was bargained by sacrificing an
    /// artifact, enchantment, or token as an additional cost (Bargain,
    /// CR 702.166b–c). Appended to preserve replay identities.
    Bargained,
    /// This spell (or the permanent it became, or a copy of either) is
    /// prototyped (Prototype, CR 702.160, CR 719.3b–d). Appended to preserve
    /// replay identities.
    Prototyped,
    /// The game is in the combat phase in or after the declare-blockers step
    /// (`DeclareBlockers`, `CombatDamageFirst`, `CombatDamage`, or `CombatEnd`),
    /// so blockers have already been declared (ninjutsu, CR 702.49a). Appended
    /// to preserve replay identities.
    InCombatAfterBlockers,
    /// Whether the object named by `what` (`&TargetSpec::Object(&Filter::Any)`
    /// for first target, `&TargetSpec::EventObject` for the event object, or
    /// `&TargetSpec::ThisObject` for source) matches `filter`, reading
    /// last-known characteristics if it departed earlier in resolution
    /// (CR 608.2h, CR 603.4). Appended to preserve replay identities.
    ObjectMatches {
        /// Which object to test.
        what: &'static TargetSpec,
        /// Predicate tested against that object.
        filter: &'static Filter,
    },
    /// Compares two evaluated [`crate::Amount`] expressions (`left <op> right`).
    /// Appended to preserve replay identities.
    AmountCompare {
        /// Left-hand evaluated amount.
        left: &'static crate::Amount,
        /// Relational operator.
        op: AmountCmp,
        /// Right-hand evaluated amount.
        right: &'static crate::Amount,
    },
}

/// Trigger conditions for triggered abilities.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Trigger {
    /// A player discards a matching card with defined characteristics.
    Discarded {
        /// The discarding player, relative to this ability's controller.
        who: crate::PlayerRel,
        /// The discarded card's characteristics in hand (CR 701.9c).
        filter: &'static Filter,
    },
    /// An opponent's spell or ability causes this hand card to be discarded.
    /// Functions from the hand under CR 113.6k; `EventPlayer` is that opponent.
    DiscardedThisByOpponent,
    /// This permanent is turned face up (not transformed or entering).
    TurnedFaceUp,
    /// "Whenever this creature transforms into [this face]" (CR 701.27f):
    /// printed on the face the permanent turns *to*, and read off the face
    /// it shows right after the transform — which is how Huntmaster of the
    /// Fells' front and Ravager of the Fells' back each hear only their own
    /// half of the flip.
    TransformsIntoThis,
    /// "When you unlock this door" (CR 709.5h), printed on half `n` of a
    /// Room: 0 the left, 1 the right. It hears the permanent being given
    /// that half's unlocked designation, however it was given: as the Room
    /// enters cast as that half (CR 709.5d), or later, when its controller
    /// pays the half's mana cost (CR 709.5e).
    UnlockThisDoor(u8),
    /// An object matching the filter enters the battlefield.
    EntersBattlefield(&'static Filter),
    /// This permanent enters from the specified absolute zone.
    EnteredThisFrom(crate::ZoneRef),
    /// An object matching the filter leaves the battlefield.
    LeavesBattlefield(&'static Filter),
    /// An object matching the filter dies (battlefield → graveyard).
    Dies(&'static Filter),
    /// A matching permanent died through an operation other than sacrifice.
    DiesNotSacrificed(&'static Filter),
    /// An object matching the filter dies after this source dealt damage to
    /// it this turn. Both objects retain their zone-change identities; the
    /// filter and the source's ability are read immediately before death.
    DiesAfterDamageByThis(&'static Filter),
    /// A spell matching the filter is cast.
    SpellCast(&'static Filter),
    /// A matching spell was cast using mana produced by this exact source.
    /// This is an ordinary permanent trigger, not a delayed mana rider.
    SpellCastUsingSourceMana(&'static Filter),
    /// Magecraft: a spell is cast or a spell copy is created (CR 707.10).
    SpellCastOrCopied(&'static Filter),
    /// This spell was cast using at least one actual mana of this type.
    /// The historical payment cannot change after the cast (CR 603.4).
    CastThisWithMana(crate::ManaColor),
    /// The source becomes the target of a spell or ability (ward,
    /// Phantasmal Image).
    BecomesTarget,
    /// Enchanted permanent becomes the target of a spell or ability; the
    /// controller of that spell or ability is recorded as `EventPlayer`
    /// (Fractured Loyalty).
    EnchantedBecomesTarget,
    /// This permanent becomes a target of an opponent's spell or ability.
    /// Its implicit resolution subject is that stack object (ward).
    Ward,
    /// "Whenever you or a permanent you control becomes the target of a
    /// spell or ability an opponent controls" (Leovold, Emissary of Trest).
    ///
    /// It fires once **per target** that fits, not once per spell: a spell
    /// that targets you and one of your permanents triggers it twice, and so
    /// does one that targets two of your permanents (Leovold's Scryfall
    /// rulings). `filter` names the permanents that count, read against
    /// the controller of this ability; `you` is whether the controller
    /// counts as well.
    TargetedByOpponent {
        /// The permanents whose targeting fires this.
        filter: &'static Filter,
        /// Whether "you" — this ability's controller — counts too.
        you: bool,
    },
    /// A creature matching the filter is exiled from the battlefield
    /// (Soulherder).
    ExiledFromBattlefield(&'static Filter),
    /// A source matching the filter deals combat damage to a player
    /// (Sword of Hearth and Home: the equipped creature).
    DealsCombatDamageToPlayer(&'static Filter),
    /// A source matching the filter deals combat damage to **you**, the
    /// ability's controller — once for each such source (Contested War
    /// Zone: "whenever a creature deals combat damage to you"). The source
    /// is the event object.
    DealsCombatDamageToYou(&'static Filter),
    /// Once per damaged player and simultaneous combat-damage event.
    OneOrMoreDealCombatDamageToPlayer(&'static Filter),
    /// A source matching the filter deals combat damage to an **opponent**
    /// of the ability's controller (Questing Beast). The player dealt to
    /// and the amount ride on the trigger, for "that player" and "that
    /// much".
    DealsCombatDamageToOpponent(&'static Filter),
    /// A source matching the filter deals damage, combat or not, to an
    /// **opponent** of the ability's controller (Hypnotic Specter: "whenever
    /// this creature deals damage to an opponent, that player discards a
    /// card at random"). Once per damage event (CR 603.2c). The player dealt
    /// to and the amount ride on the trigger, as on
    /// [`Self::DealsCombatDamageToOpponent`].
    DealsDamageToOpponent(&'static Filter),
    /// A matching source deals positive damage to a player named by the relation.
    DealsDamageToPlayer {
        /// Source filter, evaluated with the triggering ability's controller.
        source: &'static Filter,
        /// Recipient relative to the triggering ability's controller.
        player: crate::PlayerRel,
    },
    /// A source matching the filter deals damage, combat or not, to any
    /// player or permanent (El-Hajjâj: "whenever this creature deals damage,
    /// you gain that much life"). Once per damage event, and once for a
    /// step's combat damage, whatever it was dealt to (CR 510.2, 603.2c);
    /// "that much" (`Amount::EventAmount`) is all of it.
    DealsDamage(&'static Filter),
    /// A permanent matching the filter is dealt damage (Fungusaur:
    /// "whenever this creature is dealt damage, put a +1/+1 counter on
    /// it"). All combat damage is dealt at once (CR 510.2), so a creature
    /// blocked by three is dealt damage in one event and this triggers once
    /// for it (CR 603.2c); every other damage event triggers it once.
    /// Damage that was prevented was not dealt, and triggers nothing
    /// (CR 603.2g).
    DealtDamage(&'static Filter),
    /// A permanent matching the filter is dealt combat damage (Pious Warrior:
    /// "whenever this creature is dealt combat damage, you gain that much
    /// life"). Fires once for a step's combat damage to that permanent
    /// (CR 510.2, 603.2c), with `Amount::EventAmount` carrying the step's
    /// total combat damage dealt to it.
    DealtCombatDamage(&'static Filter),
    /// A player the relation names is dealt damage (Living Artifact and
    /// Lich: "whenever you're dealt damage"). The player's sibling of
    /// [`Self::DealtDamage`], one event the same way: all combat damage in
    /// a step is dealt at once (CR 510.2), so two attackers unblocked are
    /// one event and trigger it once (CR 603.2c), and "that many" is the
    /// step's total to that player, not the first creature's share. Every
    /// other damage event triggers it once, for its own amount. Damage that
    /// was prevented was not dealt (CR 603.2g). The relation is asked of the
    /// state alone, as [`Self::TappedForMana`]'s is.
    PlayerDealtDamage(crate::effect::PlayerRel),
    /// A permanent matching the filter becomes tapped, for any reason:
    /// City of Brass's own (`Filter::This`), Lifetap's "a Forest an
    /// opponent controls", Psychic Venom's enchanted land. The permanent is
    /// the event's object, so `PlayerRel::ControllerOfEvent` is "that
    /// land's controller".
    BecomesTapped(&'static Filter),
    /// A matching permanent becomes untapped. The permanent is the event's
    /// object, so `PlayerRel::ControllerOfEvent` is "that permanent's controller".
    BecomesUntapped(&'static Filter),
    /// Positive combat damage from a matching source to a creature.
    /// The damaged creature is the event object.
    DealsCombatDamageToCreature(&'static Filter),
    /// A successfully completed activation; copies of abilities do not activate.
    AbilityActivated {
        /// Ability source, evaluated independently of who activates it.
        filter: &'static Filter,
        /// Activating player, not necessarily the source's controller.
        who: crate::PlayerRel,
        /// Whether the paid activation cost contains the tap symbol.
        tap_cost: Option<bool>,
        /// Restrict to mana or nonmana abilities, or accept either.
        mana_ability: Option<bool>,
    },
    /// One positive placement, optionally restricted to its first occurrence this turn.
    CountersPlaced {
        /// Receiving permanent.
        filter: &'static Filter,
        /// Kind placed.
        kind: crate::CounterKind,
        /// Player doing the placing, not necessarily the recipient's controller.
        by: crate::PlayerRel,
        /// Match only the first placement of this kind on this incarnation.
        first_this_turn: bool,
    },
    /// The count of a kind of counter on the source rises from below `n`
    /// to `n` or more — the window CR 715.2b writes out for a chapter
    /// symbol, asked of any kind.
    ///
    /// Druid Class's "When this Class becomes level 3": this pool keeps a
    /// Class's level as level counters over level 1 (Wizard Class), so
    /// level 3 is `n: 2`. It is a trigger of its own and not a rider on
    /// the level-up activation, because the ability it starts targets: a
    /// target removed in response would otherwise take the level with it
    /// (CR 608.2b).
    CountersReach {
        /// The kind counted.
        kind: crate::effect::CounterKind,
        /// The count that must be reached.
        n: u8,
    },
    /// "When [you remove / the] last [kind] counter [is removed] from this
    /// permanent" (Divine Intervention: `by: Some(PlayerRel::You)`; Vanishing,
    /// CR 702.63a: `by: None`, matching any player, effect, cost, or SBA).
    LastCounterRemoved {
        /// The kind removed.
        kind: crate::effect::CounterKind,
        /// Optional player relation restricting whose removal counts (`None`
        /// triggers regardless of who or what removed the last counter).
        by: Option<crate::effect::PlayerRel>,
    },
    /// The controller casts their Nth spell this turn (Storm of
    /// Saruman's second-spell trigger).
    NthSpellCast {
        /// Which spell number.
        n: u8,
        /// The spell filter.
        filter: &'static Filter,
    },
    /// An instant cast after that player's first instant in the current turn.
    InstantCastAfterFirst {
        /// Which casting players are observed.
        who: crate::effect::PlayerRel,
    },
    /// A player draws a card.
    Draws(crate::effect::PlayerRel),
    /// "Whenever [a player] plays a land" (Fastbond): the special action of
    /// playing a land (CR 116.2a, 305.1), from whatever zone a permission
    /// allows, and never a land an effect puts onto the battlefield, which
    /// is not played. The relation names whose play: `You` for "you".
    PlaysLand(crate::effect::PlayerRel),
    /// The controller plays a land other than this permanent (not a land put into play).
    PlaysAnotherLand,
    /// "Whenever you tap [a permanent matching the filter] for mana"
    /// (Badgermole Cub): its controller activates a mana ability of it with
    /// {T} in the cost (CR 106.12), and that ability resolves and produces
    /// mana (CR 106.12a). Once per activation, however many colours it made.
    ///
    /// `by` is who tapped it: `You` for "whenever you tap", `EachPlayer` for
    /// "whenever a player taps a land" (Manabarbs) and for "whenever a
    /// Mountain is tapped for mana" (Gauntlet of Might), which names nobody,
    /// `EachOpponent` for "an opponent". The permanent is the event's
    /// object, so `PlayerRel::ControllerOfEvent` is "that player" and "its
    /// controller": only its controller can activate its abilities
    /// (CR 602.2).
    ///
    /// Without a target and with effects that add mana, the ability is
    /// itself a mana ability (CR 605.1b) and resolves at once, off the
    /// stack (CR 605.4a): "add an additional {G}" is in the pool before the
    /// player acts again.
    TappedForMana {
        /// Who tapped it.
        by: crate::effect::PlayerRel,
        /// What was tapped.
        filter: &'static Filter,
    },
    /// A player draws a card except the first one they draw in each of
    /// their draw steps (Orcish Bowmasters). A card drawn in their upkeep
    /// or on another player's turn is in none of their draw steps.
    /// Fires once per card drawn (CR 121.2).
    DrawsExceptFirst(crate::effect::PlayerRel),
    /// An object matching the filter attacks (Sun Titan).
    Attacks(&'static Filter),
    /// "Whenever one or more [creatures matching the filter] attack"
    /// (Raging River): once for a declaration of attackers that holds at
    /// least one, however many (CR 508.3a, 603.2c).
    OneOrMoreAttack(&'static Filter),
    /// Once when this controller declares at least this many creature attackers.
    AttackWithAtLeast(u8),
    /// "Whenever you attack with at least `n` creatures that [filter]" (Path
    /// of Mettle): once for a declaration of attackers in which at least `n`
    /// of this controller's attacking creatures match the filter as they are
    /// declared (CR 508.3a, 603.2c).
    AttackWithAtLeastMatching {
        /// How many.
        n: u8,
        /// What each of them must be.
        filter: &'static Filter,
    },
    /// This creature blocks a creature matching the filter (CR 509.3b) or
    /// becomes blocked by one (CR 509.3d): once for each such pair, each
    /// time a blocker is declared, so a creature blocked by two of them
    /// triggers twice. The filter describes the *other* creature, as it is
    /// when it blocks or is blocked (CR 509.3f), and that creature is the
    /// trigger's event object, [`TargetSpec::EventObject`](crate::TargetSpec)
    /// — Cockatrice's "whenever this creature blocks or becomes blocked by a
    /// non-Wall creature, destroy that creature at end of combat".
    BlocksOrBecomesBlockedBy(&'static Filter),
    /// This creature blocks or becomes blocked by one or more creatures
    /// matching the filter: triggers at most once per declare-blockers step
    /// even when multiple matching blockers are declared (Dwarven Soldier:
    /// "Whenever this creature blocks or becomes blocked by one or more Orcs").
    BlocksOrBecomesBlockedByOneOrMore(&'static Filter),
    /// The creature this source is attached to (enchanted or equipped
    /// creature) blocks a creature matching the filter or becomes blocked by
    /// one: the Aura's own trigger, not one granted to the creature, so its
    /// controller is the Aura's (CR 603.3a) and a creature that lost its
    /// abilities still has it fire. Once per pair, as
    /// [`Self::BlocksOrBecomesBlockedBy`]. The other creature is the event
    /// object; the attached creature, as it was then, is the trigger's
    /// subject, which `Effect::AtNextEndStepAboutSubject` names later
    /// (Infinite Authority's "the first creature").
    AttachedBlocksOrBecomesBlockedBy(&'static Filter),
    /// Rampage's "whenever this creature becomes blocked" (CR 702.23).
    /// Its effects carry N, so every separate instance triggers independently.
    Rampage,
    /// This creature becomes blocked by a creature matching the filter;
    /// once per matching blocker (CR 509.3d). The event object is the blocker.
    BecomesBlockedBy(&'static Filter),
    /// "Whenever this creature becomes blocked" (CR 509.3c): once, however
    /// many creatures block it. The event object is the first of them.
    BecomesBlocked,
    /// Whenever an attacking creature matching `filter` becomes blocked:
    /// once per attacker, however many creatures block it. The attacker,
    /// rather than a blocker, is the event object.
    ControlledBecomesBlocked(&'static Filter),
    /// "Whenever [a creature matching the filter] blocks" (Ydwen Efreet):
    /// once when it is declared as a blocker, however many attackers it
    /// blocks (CR 509.3a). The blocker is the trigger's event object.
    Blocks(&'static Filter),
    /// This creature blocks a matching creature, once for each blocked creature.
    /// The event object is that attacker; becoming blocked is not this event.
    BlocksCreature(&'static Filter),
    /// "Whenever [a creature matching the filter] attacks and isn't
    /// blocked" (Merchant Ship): once per attacker that is unblocked when
    /// the last defending player has declared blockers (CR 509.1h). The
    /// attacker is the trigger's event object.
    AttacksAndIsntBlocked(&'static Filter),
    /// A matching creature is the sole declared attacker (exalted).
    /// This is checked when attacking, not again when the trigger resolves.
    AttacksAlone(&'static Filter),
    /// The first noncreature spell cast by a player each turn (Esper
    /// Sentinel).
    FirstNoncreatureSpellCast(crate::effect::PlayerRel),
    /// The source entered the battlefield AND was evoked (cast for its
    /// evoke cost, CR 702.74).
    EntersBattlefieldEvoked,
    /// "When you cycle this card" — "when you discard this card to pay an
    /// activation cost of a cycling ability" (CR 702.29c). It triggers from
    /// the zone the card winds up in, which is a graveyard unless something
    /// replaced the discard. A cycling ability is the one
    /// [`AbilityDef::is_cycling`] reads; a card's other discard-this-card
    /// abilities (Trumpeting Carnosaur's damage, a channel ability) and a
    /// discard for any other reason are not cycling it.
    CycledThis,
    /// A step begins (whose turn: you/opponent/any).
    StepBegin {
        /// Which step.
        step: StepKind,
        /// Whose turn.
        whose: crate::effect::PlayerRel,
    },
    /// Beginning of a step of the opponent chosen as this permanent entered.
    StepBeginChosenOpponent {
        /// The step to observe.
        step: StepKind,
    },
    /// A state trigger (CR 603.8): "When you control no Islands, sacrifice
    /// this creature." It triggers whenever the condition holds, asked with
    /// the source as `this` and its controller as "you", and not again
    /// while the ability is waiting to go on the stack or is on it; once it
    /// has left the stack, a source still on the battlefield with the
    /// condition still true triggers again. No event matches it.
    State(&'static Condition),
    /// A creature linked by this exact rules acquisition leaves the battlefield.
    LinkedCreatureLeaves,
    /// A player searches a library (Wan Shi Tong, Librarian).
    SearchesLibrary(crate::effect::PlayerRel),
    /// A player draws their Nth card in a turn (Faerie Mastermind, Tamiyo, Inquisitive Student).
    NthCardDrawnInTurn {
        /// Which player's draws are observed.
        who: crate::effect::PlayerRel,
        /// Which draw number in the turn triggers it.
        n: u8,
    },
    /// A source matching the filter deals combat damage to a player or planeswalker (Psychic Frog).
    DealsCombatDamageToPlayerOrPlaneswalker(&'static Filter),
    /// A source matching the filter deals combat damage (Umezawa's Jitte).
    DealsCombatDamage(&'static Filter),
    /// A player gains life (Professor Dellian Fel).
    GainsLife(crate::effect::PlayerRel),
    /// The indicated player actually loses life, including paying life costs.
    LosesLife(crate::effect::PlayerRel),
    /// "Whenever this creature mutates" (Gemrazer, CR 702.140d).
    Mutates,
    /// The attached creature blocks or becomes blocked by one or more matching
    /// creatures: the source's own trigger, once when it blocks and once when
    /// it becomes blocked, not once for every opposing creature (CR 509.3a,
    /// 509.3c). Otherwise this follows [`Self::AttachedBlocksOrBecomesBlockedBy`].
    AttachedBlocksOrBecomesBlockedByOneOrMore(&'static Filter),
    /// Positive damage from a matching source to a matching permanent, combat
    /// or otherwise. The damaged permanent is the event object. Simultaneous
    /// combat damage triggers once per matching damaged permanent.
    DealsDamageToObject {
        /// The damage source, evaluated for the triggering ability's controller.
        source: &'static Filter,
        /// The damaged permanent, evaluated for the same controller.
        recipient: &'static Filter,
    },
    /// A matching permanent enters directly from a graveyard, or was cast from
    /// a graveyard before entering from the stack. The provenance belongs to
    /// the entering object; another object's graveyard cast does not count.
    EntersBattlefieldFromGraveyardOrCast(&'static Filter),
    /// A player cycles a matching card (CR 702.29a, 702.29f).
    Cycled {
        /// The player who cycled the card, relative to this ability's controller.
        by: crate::effect::PlayerRel,
        /// Filter on the cycled card.
        filter: &'static Filter,
    },
    /// One or more chapter numbers of a Saga (CR 715.2, CR 715.2c): triggers
    /// when the number of lore counters on this Saga moves from below a listed
    /// chapter number to at or above it (or equals it on the Saga's entry turn
    /// while this Saga has read ahead, CR 702.155a).
    Chapter(&'static [u8]),
    /// A permanent matching the filter phases in (CR 702.26c, CR 502.1 —
    /// Shimmering Efreet, Teferi's Imp, Warping Wurm).
    PhasesIn(&'static Filter),
    /// A permanent matching the filter phases out (CR 702.26b, CR 603.10b —
    /// Teferi's Imp; looks back in time immediately prior to phasing out).
    PhasesOut(&'static Filter),
    /// A permanent matching the filter phases out or leaves the battlefield
    /// (CR 702.26d, CR 603.10a, CR 603.10b — Ertai's Familiar).
    PhasesOutOrLeavesBattlefield(&'static Filter),
    /// A player flips a coin (CR 705.2): `outcome: None` triggers on any coin
    /// flip by `who`, `Some(true)` on a won flip, and `Some(false)` on a lost
    /// flip. Sets `EventPlayer` to the flipping player.
    CoinFlipped {
        /// Which player(s)' coin flips count.
        who: crate::effect::PlayerRel,
        /// Optional win (`true`) or loss (`false`) filter.
        outcome: Option<bool>,
    },
    /// A player rolls a die (CR 706.2): `min_result: None` triggers on any
    /// non-ignored die roll by `who`; `Some(n)` triggers when the result is at
    /// least `n`. Sets `EventPlayer` to the rolling player and `EventAmount`
    /// to the die result.
    DieRolled {
        /// Which player(s)' die rolls count.
        who: crate::effect::PlayerRel,
        /// Optional inclusive minimum natural die result.
        min_result: Option<u8>,
    },
    /// A permanent matching the filter becomes exerted (CR 701.43a–d), whether
    /// by [`crate::CostPart::ExertSelf`] or [`Effect::ExertSource`]. Sets
    /// `EventObject` to the exerted permanent and `EventPlayer` to the player
    /// who exerted it. Appended to preserve replay identities.
    Exerted(&'static Filter),
    /// One or more objects matching `filter` change zones simultaneously or in
    /// one action batch (`"Whenever one or more [filter] [enter / leave your
    /// graveyard / die / are put into a library]..."`, CR 603.2c, CR 603.10a):
    /// fires once per batch in which at least one matching object moves from
    /// `origin` (any zone when `None`) to `destination` (any zone when `None`),
    /// excluding this ability's source when `exclude_self` is `true`. Binds
    /// `Amount::EventAmount` to the matched batch count and `EventObject` to the
    /// first matching object. Appended to preserve replay identities.
    ChangesZoneAll {
        /// Optional required origin zone.
        origin: Option<crate::ZoneRef>,
        /// Optional required destination zone.
        destination: Option<crate::ZoneRef>,
        /// Filter each counted moving object must match.
        filter: &'static Filter,
        /// Whether this ability's own source is excluded ("one or more other").
        exclude_self: bool,
    },
    /// A player matching `who` sacrifices a permanent matching `filter` via a
    /// cost, effect, or Saga completion SBA (CR 701.21a, CR 715.4, CR 603.10a;
    /// distinct from destruction, lethal damage, zero toughness, and the legend
    /// rule). Looks back to pre-sacrifice battlefield characteristics and binds
    /// `EventPlayer` to the sacrificing player and `EventObject` to the
    /// sacrificed object. Appended to preserve replay identities.
    Sacrificed {
        /// Which player(s)' sacrifices trigger this ability.
        who: crate::effect::PlayerRel,
        /// Pre-sacrifice permanent filter.
        filter: &'static Filter,
    },
    /// One or more creatures are declared as attackers attacking a player
    /// matching `PlayerRel` (`"Whenever [you / enchanted player / a player] is
    /// attacked"`, CR 508.3b): fires once per attacked matching player per
    /// declare-attackers step, binding `EventPlayer` to the attacking player and
    /// `EventAmount` to the number of creatures declared attacking that player.
    /// Appended to preserve replay identities.
    PlayerAttacked(crate::effect::PlayerRel),
    /// A player matching `who` casts a spell matching `spell` that targets at
    /// least one permanent matching `target` and/or `you` (`targets_you`),
    /// firing at most once per cast (CR 207.2c heroic, CR 601.2h, CR 603.2c).
    /// When `only` is `true`, the spell must have exactly one distinct target in
    /// total across object and player targets (Zada, Hedron Grinder; Beamsplitter
    /// Mage). Sets `EventObject` to the cast spell and `EventPlayer` to the
    /// caster. Appended to preserve replay identities.
    SpellCastTargeting {
        /// Which casting players count.
        who: crate::effect::PlayerRel,
        /// Filter the cast spell must match (`&Filter::Any` for any spell).
        spell: &'static Filter,
        /// Optional filter a targeted permanent on the battlefield must match
        /// (`Some(&Filter::This)` for heroic, CR 109.2).
        target: Option<&'static Filter>,
        /// Whether targeting the ability's controller satisfies the trigger.
        targets_you: bool,
        /// Whether the spell must have only that single distinct target in total.
        only: bool,
    },
    /// An attacking creature matching the filter enlists a creature by paying an
    /// enlist tap cost as it attacks (CR 702.154a–d). Sets `EventObject` to the
    /// enlisted creature and `EventPlayer` to the attacking player. Appended to
    /// preserve replay identities.
    Enlisted(&'static Filter),
    /// A card becomes plotted by a player matching `who` (`"Whenever [you / a
    /// player] plot(s) a card"` or `"Whenever a card becomes plotted"`,
    /// CR 702.170a–e), binding `EventPlayer` to that player and `EventObject`
    /// to the plotted card in exile. Appended to preserve replay identities.
    Plotted {
        /// Which player(s) plotting the card trigger this ability.
        who: crate::effect::PlayerRel,
        /// Filter matched against the exiled plotted card.
        filter: &'static Filter,
    },
}

impl Trigger {
    /// "When this enters the battlefield…" — the trigger 99 of the pool's
    /// 110 enter-triggers are.
    ///
    /// `etb` is the word this is called at a table, and it is not a third
    /// name for anything: it abbreviates the older printed wording ("enters
    /// the battlefield") *and* [`Trigger::EntersBattlefield`]. The other ten
    /// triggers point at something other than the source and keep the
    /// variant with its filter, which is the whole reason this is a
    /// constant and not a macro — there is nothing to parameterise.
    pub const ETB: Self = Self::EntersBattlefield(&Filter::This);

    /// "Whenever this permanent phases in…" (CR 702.26c).
    pub const PHASES_IN: Self = Self::PhasesIn(&Filter::This);

    /// "Whenever this permanent phases out…" (CR 702.26b, CR 603.10b).
    pub const PHASES_OUT: Self = Self::PhasesOut(&Filter::This);

    /// "Whenever you exert this permanent…" (CR 701.43a).
    pub const EXERTED_THIS: Self = Self::Exerted(&Filter::This);

    /// Heroic — "Whenever you cast a spell that targets this creature…"
    /// (CR 207.2c, CR 601.2h, CR 603.2c).
    pub const HEROIC: Self = Self::SpellCastTargeting {
        who: crate::effect::PlayerRel::You,
        spell: &Filter::Any,
        target: Some(&Filter::This),
        targets_you: false,
        only: false,
    };

    /// "Whenever this creature enlists a creature…" (CR 702.154c).
    pub const ENLISTED_THIS: Self = Self::Enlisted(&Filter::This);
}

/// An ability definition on a [`crate::CardDef`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AbilityDef {
    /// Placeholder for unimplemented stubs.
    Unimplemented,
    /// Starts the controller's speed as a state-based action.
    StartYourEngines,
    /// The spell's own effect (instants/sorceries; permanent spells with
    /// cast/ETB-relevant spell text use triggered/static abilities).
    Spell {
        /// Effect operations, in order.
        effects: &'static [Effect],
        /// Target requirement, if any.
        targets: Option<crate::effect::TargetReq>,
        /// The requirement for a **second** instance of the word "target"
        /// (CR 115.3), chosen after the first and kept apart from it.
        ///
        /// Apart because every reader of the first list takes it to be one
        /// instance: a fight's two creatures appended to `targets` would be
        /// exiled together by an `Exile`, pumped together by a `PumpTarget`,
        /// and narrowed together by CR 608.2b's re-check, which drops an
        /// illegal target *and the position it held*. An effect reaches this
        /// list only by naming [`crate::effect::TargetSlot::Second`].
        second_targets: Option<crate::effect::TargetReq>,
        /// "Cast this spell only [when]" (CR 506.8, 601.3): asked before the
        /// spell may be cast, as an activated ability's condition is asked
        /// before it may be activated. `None` for the spell every other
        /// card is.
        condition: Option<Condition>,
    },
    /// Activated ability (`cost: effect`).
    Activated {
        /// Activation cost.
        cost: Cost,
        /// Effect operations.
        effects: &'static [Effect],
        /// Target requirement: a count as well as a spec, as `Spell`,
        /// `Triggered`, `Loyalty` and `SagaChapter` carry. "Tap two target
        /// lands" is min 2 max 2, and "up to one target" is min 0
        /// (CR 601.2c, CR 115.6). A bare spec could only say exactly one,
        /// and four lands in the pool print something else.
        targets: Option<crate::effect::TargetReq>,
        /// A second instance of the word "target", as on
        /// [`AbilityDef::Spell::second_targets`] (Contested Cliffs).
        second_targets: Option<crate::effect::TargetReq>,
        /// Timing restriction.
        timing: ActivationTiming,
        /// Mana abilities don't use the stack (CR 605.1).
        mana_ability: bool,
        /// Where the ability may be activated from (cycling = from hand).
        zone: ActivationZone,
        /// "Activate only once each turn", if the card prints one.
        limit: ActivationLimit,
        /// Printed announcement constraint for X.
        x_value: ActivationX,
        /// "This ability costs {1} less to activate for each …", if the
        /// ability prints one.
        cost_reduction: Option<crate::cost::CostReduction>,
    },
    /// Triggered ability (`when/whenever/at …, effect`).
    Triggered {
        /// Trigger condition.
        trigger: Trigger,
        /// The zone in which this ability functions.
        zone: TriggerZone,
        /// Effect operations.
        effects: &'static [Effect],
        /// Target requirement.
        targets: Option<crate::effect::TargetReq>,
        /// A second instance of the word "target", as on
        /// [`AbilityDef::Spell::second_targets`], chosen after the first as
        /// the ability goes on the stack (CR 603.3d, 601.2c): Ravager of the
        /// Fells' "and 2 damage to up to one target creature that player or
        /// that planeswalker's controller controls".
        second_targets: Option<crate::effect::TargetReq>,
        /// Fires at most once each turn (Jin-Gitaxias).
        once_per_turn: bool,
        /// The intervening-`if` clause, if the card prints one (CR 603.4).
        ///
        /// `if` between the trigger event and the effect — "at the
        /// beginning of your upkeep, **if this land is tapped**, put a
        /// storage counter on it" — and it is not the same word as the
        /// `if` inside an effect. This one is asked **twice**: the ability
        /// does not trigger at all while it is false, and an ability that
        /// did trigger is removed from the stack and does nothing if it
        /// has stopped being true by the time it would resolve.
        condition: Option<Condition>,
        /// "This ability still resolves if its target becomes illegal"
        /// (Gilded Drake): the card's own exception to CR 608.2b, which
        /// otherwise removes an ability whose every target has become
        /// illegal from the stack without resolving it. With this set it
        /// resolves with no target, and its effects do what they do with
        /// none — the Drake's "if you don't or can't make an exchange,
        /// sacrifice this creature".
        resolves_if_targets_illegal: bool,
    },
    /// Ward {N}: "whenever this becomes the target of a spell or ability
    /// an opponent controls, counter it unless that player pays {N}".
    /// Engine-level keyword trigger (synthetic effects, like prowess).
    Ward {
        /// Generic mana to pay.
        mana: u16,
    },
    /// Toxic N (CR 702.164a): a static ability. Combat damage this
    /// creature deals to a player also gives that player poison counters
    /// equal to its total toxic value, the sum over every toxic ability it
    /// has (CR 702.164b–c). Read by the engine where combat damage is
    /// dealt; like ward, a keyword with a number is data and not a bit.
    Toxic {
        /// N.
        poison: u8,
    },
    /// Static/continuous ability (layers, CR 613).
    /// An activated ability with a precondition (Mox Opal's metalcraft,
    /// Bleachbone Verge's Plains/Swamp check).
    ActivatedConditional {
        /// The cost.
        cost: crate::cost::Cost,
        /// Effect operations.
        effects: &'static [crate::effect::Effect],
        /// Target requirement, with its count, as on the unconditional twin.
        targets: Option<crate::effect::TargetReq>,
        /// A second instance of the word "target", as on the unconditional
        /// twin.
        second_targets: Option<crate::effect::TargetReq>,
        /// Instant/sorcery timing.
        timing: ActivationTiming,
        /// Whether this is a mana ability.
        mana_ability: bool,
        /// Where it may be activated.
        zone: ActivationZone,
        /// The precondition.
        condition: Condition,
        /// "Activate only once each turn", if the card prints one.
        limit: ActivationLimit,
        /// Printed announcement constraint for X.
        x_value: ActivationX,
        /// As on the unconditional twin.
        cost_reduction: Option<crate::cost::CostReduction>,
    },
    /// One chapter of a saga (CR 715): triggers when the corresponding
    /// lore counter is added.
    SagaChapter {
        /// Chapter number (1-based).
        chapter: u8,
        /// Effect operations.
        effects: &'static [crate::effect::Effect],
        /// Target requirement — a count as well as a filter, for the reason
        /// [`AbilityDef::Loyalty::targets`] gives.
        targets: Option<crate::effect::TargetReq>,
    },
    /// Prepared: while this permanent has the prepared marker, you may
    /// cast a copy of the linked spell card; doing so removes the marker
    /// (Emeritus of Woe & co.).
    Prepared {
        /// The definition containing the prepare spell.
        card: baylee_core::ids::CardIndex,
        /// The prepare spell face within that definition.
        face: u8,
    },
    /// Echo (CR 702.30a), a triggered ability the keyword writes out: at the
    /// beginning of your upkeep, if this came under your control since the
    /// beginning of your last upkeep, sacrifice it unless you pay the cost.
    Echo {
        /// The echo cost.
        cost: baylee_core::mana::ManaCost,
    },
    /// Static/continuous ability (layers, CR 613).
    Static(crate::static_ability::StaticAbility),
    /// A replacement or trigger-modification rule (CR 614; Doubling
    /// Season, Panharmonicon, Elesh Norn).
    Replacement(crate::static_ability::ReplacementRule),
    /// A spell with modes (CR 700.2): the caster chooses as many as
    /// `choose` says as the spell is cast (CR 700.2a, 601.2b). Each mode may
    /// override the cost (overload) or add one of its own (spree).
    ModalSpell {
        /// The modes to choose from.
        modes: &'static [SpellMode],
        /// How many of them are chosen: "Choose one —", "Choose two —",
        /// "Choose one or more —".
        choose: ModeCount,
    },
    /// Suspend: exile with N time counters from your hand (sorcery speed);
    /// remove one at your upkeep, cast for free when the last is removed.
    Suspend {
        /// Time counters.
        counters: u8,
        /// The cost to suspend the card (`Suspend N—{C}`).
        cost: baylee_core::mana::ManaCost,
    },
    /// "As ~ enters, you may have it become a copy of … until end of
    /// turn" (Cursed Mirror). Choice is made as it enters; the copy is a
    /// layer-1 continuous effect with `UntilEndOfTurn` duration.
    CopyOnEnterUntilEot {
        /// What may be copied.
        target: crate::effect::TargetSpec,
        /// Copy modifications applied as their own layer effects.
        mods: &'static [CopyMod],
    },
    /// "You may have ~ enter the battlefield as a copy of …" (clone
    /// family). Choice is made as it enters.
    CopyOnEnter {
        /// What may be copied.
        target: TargetSpec,
        /// Modifications applied after copying (artifact, not legendary…).
        mods: &'static [CopyMod],
    },
    /// A planeswalker loyalty ability (cost in loyalty counters; positive
    /// = add, negative = remove).
    Loyalty {
        /// Loyalty delta.
        cost: i8,
        /// Effect operations.
        effects: &'static [Effect],
        /// Target requirement.
        ///
        /// A [`crate::effect::TargetReq`] rather than a bare spec, because
        /// two of the pool's seven walkers print "up to one target": Karn,
        /// the Great Creator's `+1` and Teferi, Time Raveler's `−3`. Read as
        /// *exactly* one, both were abilities a player could not activate at
        /// all with nothing on the board to point at — which for Teferi is a
        /// card that cannot be drawn, and for Karn a loyalty tick that cannot
        /// be taken.
        targets: Option<crate::effect::TargetReq>,
        /// A second instance of the word "target", as on
        /// [`AbilityDef::Spell::second_targets`] (Oko, Thief of Crowns' −5:
        /// one of yours, one of theirs).
        second_targets: Option<crate::effect::TargetReq>,
    },
    /// An activated ability whose controller chooses exactly one mode during
    /// announcement, before selecting targets and paying the activation cost.
    ModalActivated {
        /// One activation cost shared by all modes.
        cost: Cost,
        /// Mode effects and target requirements; mode-specific costs are unsupported.
        modes: &'static [SpellMode],
        /// Timing restriction.
        timing: ActivationTiming,
        /// Whether the ability meets the mana-ability criteria.
        mana_ability: bool,
        /// Zone from which the ability can be activated.
        zone: ActivationZone,
        /// Activation precondition, evaluated before announcement.
        condition: Option<Condition>,
        /// Per-turn or per-game activation limit.
        limit: ActivationLimit,
        /// Printed announcement constraint for X.
        x_value: ActivationX,
        /// Generic activation-cost reduction.
        cost_reduction: Option<crate::cost::CostReduction>,
    },
    /// A triggered ability with modes: the controller chooses one when it
    /// triggers (Charming Prince, Aether Channeler).
    ModalTriggered {
        /// Trigger condition.
        trigger: Trigger,
        /// The zone in which this ability functions.
        zone: TriggerZone,
        /// The modes to choose from.
        modes: &'static [SpellMode],
        /// Fires at most once each turn.
        once_per_turn: bool,
        /// The intervening-`if` clause, as on [`AbilityDef::Triggered`].
        ///
        /// No card in the pool prints a modal trigger with one. The field
        /// is here because this variant is the *forgotten twin* — six
        /// readers across the engine, the client and the lints once
        /// matched only the unmodal one — and a second door into
        /// `trigger.rs` that could not carry a condition is exactly how
        /// the omission would be found again, one card at a time.
        condition: Option<Condition>,
    },
    /// Unearth (CR 702.84a): `[cost]: Return this card from your graveyard to
    /// the battlefield. It gains haste. Exile it at the beginning of the next
    /// end step. If it would leave the battlefield, exile it instead of putting
    /// it anywhere else. Activate only as a sorcery.`
    Unearth {
        /// The unearth activation cost.
        cost: Cost,
    },
    /// Encore (CR 702.141a): `[cost], Exile this card from your graveyard: For
    /// each opponent, create a token that's a copy of this card that attacks
    /// that opponent this turn if able. The tokens gain haste. Sacrifice them
    /// at the beginning of the next end step. Activate only as a sorcery.`
    Encore {
        /// The encore cost before the implicit `CostPart::ExileSelf`.
        cost: Cost,
    },
}

impl AbilityDef {
    /// Whether this is a cycling ability: "Cycling [cost]" means "[Cost],
    /// Discard this card: Draw a card" and "[Type]cycling [cost]" means
    /// "[Cost], Discard this card: Search your library for a [type] card,
    /// reveal it, put it into your hand, then shuffle" at instant speed from
    /// the hand (CR 702.29a, 702.29e-f). A card that discards itself for
    /// anything else — Trumpeting Carnosaur's damage, Boseiju's channel, or
    /// sorcery-speed transmute (CR 702.53a) — is not cycled.
    #[must_use]
    pub fn is_cycling(&self) -> bool {
        match self {
            Self::Activated {
                cost,
                effects,
                timing: ActivationTiming::InstantSpeed,
                zone: ActivationZone::Hand,
                ..
            }
            | Self::ActivatedConditional {
                cost,
                effects,
                timing: ActivationTiming::InstantSpeed,
                zone: ActivationZone::Hand,
                ..
            } => {
                cost.parts.contains(&crate::cost::CostPart::DiscardSelf)
                    && (*effects == [crate::effect::Effect::draw(1)]
                        || matches!(
                            *effects,
                            [crate::effect::Effect::SearchLibrary { finds, optional: false, .. }]
                                if *finds == [crate::effect::Find::HAND]
                        ))
            }
            _ => false,
        }
    }

    /// Whether this entry represents the intrinsic CR 305.6 mana ability.
    /// Printed mana symbols deliberately do not qualify, even if identical.
    #[must_use]
    pub fn is_intrinsic_mana_ability(&self) -> bool {
        matches!(self, Self::Activated { effects, .. }
        if matches!(*effects, [crate::Effect::AddMana {
            source: crate::ManaSource::IntrinsicBasicLandTypes, ..
        }]))
    }

    /// Whether this is a mana ability, which the stack never sees (CR 605.1).
    ///
    /// One reading for every caller, because two would disagree: an ability
    /// wrongly read as a mana ability skips the stack, and one wrongly read
    /// as an ordinary ability cannot be activated while a cost is being
    /// paid. Both arms are matched deliberately —
    /// `ActivatedConditional` is the same ability with a condition on it,
    /// and six readers across the engine, the client and the lints once
    /// matched only the unconditional one.
    #[must_use]
    pub fn is_mana_ability(&self) -> bool {
        match self {
            Self::ModalActivated {
                cost,
                modes,
                mana_ability,
                ..
            } => *mana_ability && crate::mana_rule::modal_activated_mana_ability(cost, modes),
            Self::Activated {
                cost,
                effects,
                targets,
                second_targets,
                mana_ability,
                ..
            }
            | Self::ActivatedConditional {
                cost,
                effects,
                targets,
                second_targets,
                mana_ability,
                ..
            } => {
                *mana_ability
                    && crate::mana_rule::activated_mana_ability(
                        cost,
                        effects,
                        targets.is_some() || second_targets.is_some(),
                    )
            }
            _ => false,
        }
    }

    /// Whether this is a **triggered** mana ability (CR 605.1b): it has no
    /// target, it triggers from a mana ability — [`Trigger::TappedForMana`]
    /// is the one trigger in the vocabulary that does — and it could add
    /// mana. Such an ability resolves the moment it triggers, off the stack
    /// (CR 605.4a).
    ///
    /// Derived, as [`Self::is_mana_ability`]'s flag is checked against the
    /// activated criteria by `lints::mana_ability_fault`: a "whenever you
    /// tap this land for mana" that targets (Forbidden Orchard's) is an
    /// ordinary trigger and goes on the stack.
    #[must_use]
    pub fn is_triggered_mana_ability(&self) -> bool {
        matches!(
            self,
            Self::Triggered {
                trigger: Trigger::TappedForMana { .. },
                targets: None,
                second_targets: None,
                effects,
                ..
            } if crate::mana_rule::could_add_mana(effects)
        )
    }

    /// The Saga chapter numbers (1-based) this ability represents, if it is a
    /// chapter ability on the battlefield (CR 715.2, CR 715.2c).
    #[must_use]
    pub const fn saga_chapters(&self) -> &[u8] {
        match self {
            Self::SagaChapter { chapter, .. } => core::slice::from_ref(chapter),
            Self::Triggered {
                trigger: Trigger::Chapter(chapters),
                zone: TriggerZone::Battlefield,
                ..
            }
            | Self::ModalTriggered {
                trigger: Trigger::Chapter(chapters),
                zone: TriggerZone::Battlefield,
                ..
            } => chapters,
            _ => &[],
        }
    }

    /// Whether this ability is a Saga chapter ability (CR 715.2).
    #[must_use]
    pub const fn is_saga_chapter(&self) -> bool {
        !self.saga_chapters().is_empty()
    }

    /// The greatest chapter number named by this ability, or `0` if it is not
    /// a Saga chapter ability (CR 715.2d).
    #[must_use]
    pub const fn max_saga_chapter(&self) -> u8 {
        let chapters = self.saga_chapters();
        let mut max = 0;
        let mut i = 0;
        while i < chapters.len() {
            if chapters[i] > max {
                max = chapters[i];
            }
            i += 1;
        }
        max
    }
}

/// A modification applied after a clone copies its target.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CopyMod {
    /// Adds types ("except it's an artifact").
    AddType(baylee_core::types::TypeSet),
    /// Removes types ("except it's not a creature").
    RemoveType(baylee_core::types::TypeSet),
    /// Adds supertypes to the copy's copiable values ("except it's legendary").
    AddSupertype(baylee_core::types::SupertypeSet),
    /// Removes supertypes ("except it's not legendary").
    RemoveSupertype(baylee_core::types::SupertypeSet),
    /// Adds a subtype ("except it's a Shapeshifter").
    AddSubtype(baylee_core::ids::SubtypeId),
    /// Replace creature types, retaining subtypes of other card types.
    SetCreatureTypes(&'static [baylee_core::ids::SubtypeId]),
    /// Grants a keyword ("with haste").
    AddKeyword(crate::KeywordSet),
    /// Enters tapped only if this copy replacement is chosen. Entry state is
    /// not a characteristic and does not become part of the copiable values.
    EnterTapped,
    /// Enters with counters of a kind.
    AddCounter(crate::CounterKind, u16),
    /// Sets power and toughness ("except it's a 4/4", eternalize,
    /// CR 702.129a). A copiable value of the copy (CR 707.9b).
    SetPT(i16, i16),
    /// Sets the colors ("except it's black"), replacing the copied ones
    /// (CR 707.9b).
    SetColor(baylee_core::color::ColorSet),
    /// Keep the copier's color rather than taking the copied object's color.
    KeepColor,
    /// Add this ability to the copy's copiable values, including duplicates.
    GrantAbility(&'static AbilityDef),
    /// Preserve the ability currently resolving as an additional copiable
    /// ability. This represents "it has this ability" without a cyclic AST.
    KeepResolvingAbility,
    /// "…with no mana cost" (embalm, eternalize): the copy has no mana
    /// cost, so its mana value is 0 (CR 202.3a).
    NoManaCost,
    /// Enters with **X** counters of a kind: Altered Ego's "except it enters
    /// with X additional +1/+1 counters on it".
    ///
    /// X is the value announced for the spell that became this permanent
    /// (CR 107.3m), which is what the entering object's X holds by the time
    /// the copy is made; a copier put onto the battlefield from anywhere but
    /// the stack has an X of 0 (CR 107.3g) and gets none. The counters come
    /// with the copy and only with it, which is why this is not
    /// `EnterModifier::WithCounters`: an Ego that declines to copy is the
    /// 0/0 it prints.
    AddCounterX(crate::CounterKind),
    /// Enters with counters of a kind **if** what it became has one of these
    /// card types: Spark Double's "…except it enters with an additional
    /// +1/+1 counter on it if it's a creature, it enters with an additional
    /// loyalty counter on it if it's a planeswalker".
    ///
    /// The types asked are the copy's, read after the copying: the copied
    /// permanent's copiable values (CR 707.2) with this list's own type
    /// changes applied (CR 707.9b), never the copier's printed types, which
    /// the copy replaced. Each clause asks for itself, so a copy of a
    /// permanent that is both a creature and a planeswalker takes both
    /// counters, and a copy of one that is neither takes none.
    AddCounterIf(baylee_core::types::TypeSet, crate::CounterKind, u16),
    /// Keeps the copier's own printed **static** abilities beside the
    /// copied ones ("except it has Sakashima's other abilities").
    ///
    /// CR 707.9a is the rule, and only half of it is reachable here. It
    /// says a copy effect may cause the copy to gain an ability as part of
    /// the copying process, **and** that the ability joins the copy's
    /// *copiable* values — so a second clone copying this one would copy it
    /// too. What the engine does is register the kept statics as the copy's
    /// own continuous effects, which delivers the first half and not the
    /// second: a Spark Double copying a Sakashima-that-became-a-Padeem gets
    /// Padeem's list.
    ///
    /// That limit is a type and not an oversight.
    /// `GameObject::own_abilities` is a `&'static [AbilityDef]`, so the
    /// concatenation of "what I copied" and "what I keep" is a list no
    /// object can hold; the copiable half waits on that field growing an
    /// owned form, and nothing in the pool can see the difference today.
    ///
    /// Only statics survive the trip, because a continuous effect is what
    /// the engine has to put them in.
    /// `combo_tests::every_copy_that_keeps_its_own_abilities_keeps_only_statics`
    /// is the bound, so a card whose other abilities are triggered or
    /// activated stops the build rather than losing them in silence.
    ///
    /// "Other" is the rest of the arithmetic: what is kept is every
    /// printed ability of the copier **except the copy ability itself**,
    /// or a Sakashima would arrive holding a second offer to copy
    /// something, having already taken one.
    KeepOtherAbilities,
    /// "…except it has '{T}: Add {U}.'" — the copy gains an ability as part
    /// of the copying process (CR 707.9a), carried as the continuous effect
    /// that grants it to the copy: a [`Modifier::GrantActivated`] or a
    /// [`Modifier::GrantTriggered`].
    ///
    /// Every ability printed *beside* a copy ability is overwritten by the
    /// copy (CR 707.2), so an ability the clause names has to travel inside
    /// the clause: written as a sibling it is gone the moment the copy is
    /// made. Three cards shipped that way and claimed
    /// `Coverage::Implemented` — Machine God's Effigy copied an Elf and
    /// tapped for {G} and never for {U}, Progenitor Mimic made no token, and
    /// Phantasmal Image survived being targeted. A card that prints the
    /// ability on its own *as well* (the Effigy, uncopied) keeps its sibling
    /// for that case and writes the grant for this one.
    ///
    /// The grant's filter is the copy itself, so the modifier has to be an
    /// ability the copy *has* — a static that reaches other objects ("other
    /// creatures you control get +1/+1") would land on the copy alone.
    ///
    /// Paid the way [`CopyMod::KeepOtherAbilities`] is paid, and with its
    /// limit: the ability applies to the copy and does not join its
    /// *copiable* values, so a second clone copying this one does not get
    /// it. The token and spell-copy doors reach no effect table and ignore
    /// it; nothing in the pool is copied "except it has …" through either.
    ///
    /// A reference rather than the modifier itself because a `Modifier` is
    /// the size of the `Cost` inside a grant, and every other variant here
    /// is a word or two.
    ///
    /// [`Modifier::GrantActivated`]: crate::static_ability::Modifier::GrantActivated
    /// [`Modifier::GrantTriggered`]: crate::static_ability::Modifier::GrantTriggered
    Grant(&'static crate::static_ability::Modifier),
    /// Sets the copy's copiable name ("except its name is …", CR 707.9b).
    SetName(&'static str),
    /// Retains the copier's prior copiable name ("except it keeps its name", CR 707.9b).
    KeepName,
    /// Sets the copy's copiable mana cost ("except its mana cost is …", CR 707.9b).
    SetManaCost(baylee_core::mana::ManaCost),
    /// Retains the copier's prior copiable mana cost (CR 707.9b).
    KeepManaCost,
    /// Replaces the copy's card types outright (CR 707.9b).
    SetTypes(baylee_core::types::TypeSet),
    /// Adds color(s) to the copy's copiable colors ("in addition to its other colors", CR 707.9b).
    AddColor(baylee_core::color::ColorSet),
}

/// How many modes a modal spell's caster chooses (CR 700.2): the count
/// its instruction prints before the bulleted list.
///
/// A choice of more than one is announced as one set (CR 700.2a), no mode
/// twice (CR 700.2d), and the chosen modes are carried out in the order
/// they are printed, whatever order they were picked in (CR 608.2c).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ModeCount {
    /// The fewest modes that may be chosen.
    pub min: u8,
    /// The most, never more than the modes printed.
    pub max: u8,
}

impl ModeCount {
    /// "Choose one —".
    pub const ONE: Self = Self { min: 1, max: 1 };
    /// "Choose two —" (Cryptic Command).
    pub const TWO: Self = Self { min: 2, max: 2 };
    /// "Choose one or more —" (Farewell), and what spree means
    /// (CR 702.172a).
    pub const ONE_OR_MORE: Self = Self {
        min: 1,
        max: u8::MAX,
    };

    /// Whether only one mode is ever chosen: the spell carries a mode index
    /// rather than a set of them.
    #[must_use]
    pub const fn is_one(self) -> bool {
        self.max == 1
    }
}

/// One mode of a [`crate::AbilityDef::ModalSpell`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SpellMode {
    /// Effect operations of this mode.
    pub effects: &'static [crate::effect::Effect],
    /// Target requirement of this mode.
    ///
    /// A mode targets for itself, and it states a count as well as a filter:
    /// Inspirit, Flagship Vessel puts a counter "on **up to one** other
    /// target artifact", and read as exactly one that trigger vanishes off
    /// the stack on a board with no other artifact on it.
    pub targets: Option<crate::effect::TargetReq>,
    /// A second instance of the word "target" in this mode, as on
    /// [`crate::AbilityDef::Spell::second_targets`]: Archdruid's Charm's
    /// "Put a +1/+1 counter on target creature you control. It deals damage
    /// equal to its power to target creature you don't control."
    ///
    /// A modal **spell**'s only: the cast wizard asks it once the mode is
    /// chosen. A modal trigger's mode never carries one, which
    /// `no_modal_trigger_mode_prints_a_second_target` holds.
    pub second_targets: Option<crate::effect::TargetReq>,
    /// Cost override for this mode (overload); `None` = the printed cost.
    pub cost_override: Option<baylee_core::mana::ManaCost>,
    /// The cost printed before this mode's effect, paid on top of the
    /// spell's when the mode is chosen (CR 700.2h): spree's "+ {1} —"
    /// (CR 702.172a). Several chosen modes pay every one of theirs. Not an
    /// alternative cost: a spell cast for one — without paying its mana
    /// cost, say — still has these added to it (CR 118.9d, 601.2f).
    pub additional_cost: Option<baylee_core::mana::ManaCost>,
}

/// Which event a trigger-modifying rule cares about.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TriggerEventKind {
    /// A permanent entering the battlefield.
    EntersBattlefield,
    /// Any event.
    Any,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::static_ability::{Layer, Modifier, ReplacementRule, StaticAbility};

    const NOTHING: &[Effect] = &[];
    const TEST_MANA: &[Effect] = &[Effect::mana(crate::ManaColor::Green, 1)];

    fn activated(mana_ability: bool) -> AbilityDef {
        AbilityDef::Activated {
            cost: crate::cost::Cost::TAP,
            effects: TEST_MANA,
            targets: None,
            second_targets: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability,
            zone: ActivationZone::Battlefield,
            limit: ActivationLimit::Unlimited,
            x_value: crate::ActivationX::Any,
            cost_reduction: None,
        }
    }

    fn conditional(mana_ability: bool) -> AbilityDef {
        AbilityDef::ActivatedConditional {
            cost: crate::cost::Cost::TAP,
            effects: TEST_MANA,
            targets: None,
            second_targets: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability,
            zone: ActivationZone::Battlefield,
            condition: Condition::ControlCount(&crate::Filter::ARTIFACT, 3),
            limit: ActivationLimit::Unlimited,
            x_value: crate::ActivationX::Any,
            cost_reduction: None,
        }
    }

    /// CR 702.29a is the definition, read as a shape: "[Cost], Discard this
    /// card: Draw a card", from the hand. Each neighbour that shares all but
    /// one half is not cycling: another effect (Trumpeting Carnosaur's
    /// damage), another zone, a cost that keeps the card.
    #[test]
    fn only_discard_this_card_draw_a_card_from_the_hand_is_cycling() {
        use crate::cost::{Cost, CostPart};
        use crate::effect::{Amount, TargetSpec};
        const DISCARD: Cost = Cost {
            mana: baylee_core::mana::ManaCost::ZERO,
            parts: &[CostPart::DiscardSelf],
        };
        const DRAW: &[Effect] = &[Effect::draw(1)];
        const DAMAGE: &[Effect] = &[Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::Object(&crate::Filter::CREATURE),
        }];
        let ability = |cost, effects, zone| AbilityDef::Activated {
            cost,
            effects,
            targets: None,
            second_targets: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone,
            limit: ActivationLimit::Unlimited,
            x_value: crate::ActivationX::Any,
            cost_reduction: None,
        };
        assert!(ability(DISCARD, DRAW, ActivationZone::Hand).is_cycling());
        assert!(!ability(DISCARD, DAMAGE, ActivationZone::Hand).is_cycling());
        assert!(!ability(DISCARD, DRAW, ActivationZone::Battlefield).is_cycling());
        assert!(!ability(crate::cost::Cost::FREE, DRAW, ActivationZone::Hand).is_cycling());
        assert!(!activated(false).is_cycling());
    }

    /// Typecycling (CR 702.29e-f) searches the library and puts the card into
    /// hand at instant speed, which is cycling; sorcery-speed transmute (CR
    /// 702.53a) is not cycling even though it has the same search effect.
    #[test]
    fn typecycling_to_hand_is_cycling_while_sorcery_transmute_is_not() {
        use crate::cost::{Cost, CostPart};
        use crate::effect::Find;
        const DISCARD: Cost = Cost {
            mana: baylee_core::mana::ManaCost::ZERO,
            parts: &[CostPart::DiscardSelf],
        };
        const SEARCH_HAND: &[Effect] = &[Effect::SearchLibrary {
            filter: &crate::Filter::BASIC_LAND,
            finds: &[Find::HAND],
            optional: false,
        }];
        const SEARCH_BATTLEFIELD: &[Effect] = &[Effect::SearchLibrary {
            filter: &crate::Filter::BASIC_LAND,
            finds: &[Find::BATTLEFIELD],
            optional: false,
        }];
        const SEARCH_TWO: &[Effect] = &[Effect::SearchLibrary {
            filter: &crate::Filter::BASIC_LAND,
            finds: &[Find::HAND, Find::HAND],
            optional: false,
        }];
        const OPTIONAL_SEARCH: &[Effect] = &[Effect::SearchLibrary {
            filter: &crate::Filter::BASIC_LAND,
            finds: &[Find::HAND],
            optional: true,
        }];
        let hand_ability = |effects, timing| AbilityDef::Activated {
            cost: DISCARD,
            effects,
            targets: None,
            second_targets: None,
            timing,
            mana_ability: false,
            zone: ActivationZone::Hand,
            limit: ActivationLimit::Unlimited,
            x_value: crate::ActivationX::Any,
            cost_reduction: None,
        };
        assert!(hand_ability(SEARCH_HAND, ActivationTiming::InstantSpeed).is_cycling());
        assert!(!hand_ability(SEARCH_HAND, ActivationTiming::SorcerySpeed).is_cycling());
        assert!(!hand_ability(SEARCH_BATTLEFIELD, ActivationTiming::InstantSpeed).is_cycling());
        assert!(!hand_ability(SEARCH_TWO, ActivationTiming::InstantSpeed).is_cycling());
        assert!(!hand_ability(OPTIONAL_SEARCH, ActivationTiming::InstantSpeed).is_cycling());
    }

    /// CR 605.1 makes a mana ability the exception, and the flag is the
    /// whole difference: an ability read as one skips the stack, where an
    /// opponent can no longer respond to it, and an ability wrongly read as
    /// an ordinary one cannot be activated while a cost is being paid.
    ///
    /// Both arms are the point. `ActivatedConditional` is the same ability
    /// with a precondition on it — Mox Opal's three artifacts — and six readers
    /// across the engine, the client and the lints once matched only the
    /// unconditional twin, so a conditional mana ability answered "no" to
    /// every one of them.
    #[test]
    fn a_mana_ability_is_read_the_same_through_both_of_its_doors() {
        assert!(activated(true).is_mana_ability());
        assert!(
            conditional(true).is_mana_ability(),
            "the conditional twin is the one that gets forgotten"
        );
        assert!(!activated(false).is_mana_ability());
        assert!(!conditional(false).is_mana_ability());
    }

    /// And nothing else is one, whatever it does. A mana ability is an
    /// *activated* ability by CR 605.1a — a triggered ability that produces
    /// mana is a triggered mana ability only under 605.1b, which this pool
    /// does not have a shape for, and a spell that adds mana uses the stack
    /// like any other spell.
    #[test]
    fn no_other_kind_of_ability_is_a_mana_ability() {
        let others = [
            AbilityDef::Unimplemented,
            AbilityDef::Spell {
                effects: NOTHING,
                targets: None,
                second_targets: None,
                condition: None,
            },
            AbilityDef::Triggered {
                trigger: Trigger::ETB,
                effects: NOTHING,
                targets: None,
                second_targets: None,
                zone: TriggerZone::Battlefield,
                once_per_turn: false,
                condition: None,
                resolves_if_targets_illegal: false,
            },
            AbilityDef::Ward { mana: 2 },
            AbilityDef::SagaChapter {
                chapter: 1,
                effects: NOTHING,
                targets: None,
            },
            AbilityDef::Prepared {
                card: baylee_core::ids::CardIndex::new(1),
                face: 0,
            },
            AbilityDef::Echo {
                cost: baylee_core::mana::ManaCost::ZERO,
            },
            AbilityDef::Static(StaticAbility {
                source_zone: crate::ZoneRef::Battlefield,
                layer: Layer::Ability,
                filter: crate::Filter::This,
                modifier: Modifier::GrantActivated {
                    cost: crate::cost::Cost::TAP,
                    effects: NOTHING,
                    // A *granted* mana ability, and the ability granting it
                    // is still not one itself.
                    mana_ability: true,
                },
                condition: None,
            }),
            AbilityDef::Replacement(ReplacementRule::DoubleTokenCreation {
                controller_filter: &crate::Filter::Any,
            }),
            AbilityDef::ModalSpell {
                modes: &[],
                choose: ModeCount::ONE,
            },
            AbilityDef::Suspend {
                counters: 3,
                cost: baylee_core::mana::ManaCost::ZERO,
            },
            AbilityDef::CopyOnEnterUntilEot {
                target: crate::effect::TargetSpec::Object(&crate::Filter::CREATURE),
                mods: &[],
            },
            AbilityDef::CopyOnEnter {
                target: crate::effect::TargetSpec::Object(&crate::Filter::CREATURE),
                mods: &[],
            },
            AbilityDef::Loyalty {
                cost: 1,
                effects: NOTHING,
                targets: None,
                second_targets: None,
            },
            AbilityDef::ModalTriggered {
                trigger: Trigger::ETB,
                modes: &[],
                zone: TriggerZone::Battlefield,
                once_per_turn: false,
                condition: None,
            },
            AbilityDef::Unearth {
                cost: crate::cost::Cost::FREE,
            },
            AbilityDef::Encore {
                cost: crate::cost::Cost::FREE,
            },
        ];
        for ability in others {
            assert!(
                !ability.is_mana_ability(),
                "{ability:?} answered that it is a mana ability"
            );
        }
    }

    /// `Trigger::ETB` is the spelling most cards use, and it is an alias
    /// rather than a kind of its own: "when **this** enters". A card
    /// reaching for it must get the self-filter, because the same variant
    /// with a wider filter is "whenever *another* creature enters" — a
    /// different card.
    #[test]
    fn the_enters_alias_is_about_the_permanent_itself() {
        assert_eq!(
            Trigger::ETB,
            Trigger::EntersBattlefield(&crate::Filter::This)
        );
        assert_ne!(
            Trigger::ETB,
            Trigger::EntersBattlefield(&crate::Filter::CREATURE),
            "a filter that is not the source is another trigger entirely"
        );
    }
}
