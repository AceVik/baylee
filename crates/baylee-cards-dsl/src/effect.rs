//! Effect operations — the spell/ability effect vocabulary (v1).
//!
//! Operations are data; the engine interprets them. Anything not
//! expressible here is either an M2 primitive (continuous durations, copy,
//! phases) or a candidate for a flagged `// NOT SUPPORTED:` in the card.

use crate::KeywordSet;
use crate::filter::Filter;
use baylee_core::color::ColorSet;
use baylee_core::ids::SubtypeId;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

/// Counter kinds (objects and players). Lives here so card definitions can
/// reference counters without engine dependencies.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum CounterKind {
    /// +1/+1.
    P1P1,
    /// −1/−1.
    M1M1,
    /// Loyalty.
    Loyalty,
    /// Lore (sagas).
    Lore,
    /// Time (suspend, vanishing).
    Time,
    /// Charge.
    Charge,
    /// Poison (players).
    Poison,
    /// Energy (players).
    Energy,
    /// Rad (players).
    Rad,
    /// Lifelink counter (grants lifelink, CR 122.1b).
    Lifelink,
    /// Level counters (classes, CR 716).
    Level,
    /// Card-specific counters.
    Custom(u16),
}

/// Definition of a token a card can create.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TokenDef {
    /// Token name.
    pub name: &'static str,
    /// Colors.
    pub colors: ColorSet,
    /// Types.
    pub types: TypeSet,
    /// Supertypes.
    pub supertypes: SupertypeSet,
    /// Subtypes.
    pub subtypes: &'static [SubtypeId],
    /// Power (creatures).
    pub power: Option<i16>,
    /// Toughness (creatures).
    pub toughness: Option<i16>,
    /// Keywords.
    pub keywords: KeywordSet,
}

/// A computed number (CR 107.1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Amount {
    /// A fixed value.
    Fixed(u32),
    /// The value of X chosen at cast time.
    X,
    /// The negated value of X (Toxic Deluge's `-X/-X`; evaluated as a
    /// negative at use sites).
    NegX,
    /// The value of X plus the controller's commander-cast count
    /// (Commander's Insight).
    XPlusCommanderCasts,
    /// Number of distinct colors among battlefield objects matching the
    /// filter (General Tazri).
    DistinctColorsAmong(&'static Filter),
    /// A fixed negative value (-N at use sites).
    NegXFixed(u32),
    /// The power of the first target (last known characteristics).
    TargetPower,
    /// The mana value of the first target (Reanimate's life loss).
    TargetCmc,
    /// Number of objects matching a filter in a zone.
    CountOf {
        /// What to count.
        filter: &'static Filter,
        /// Where to count.
        zone: ZoneSel,
    },
}

/// Zone selectors for amounts/searches.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ZoneSel {
    /// The battlefield.
    Battlefield,
    /// Your library.
    LibraryYou,
    /// Your graveyard.
    GraveyardYou,
    /// All graveyards.
    GraveyardAll,
    /// Your hand.
    HandYou,
}

/// Relative player references.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PlayerRel {
    /// You (the controller).
    You,
    /// An opponent (M2: choice in multiplayer; auto-resolves heads-up).
    Opponent,
    /// Each player.
    EachPlayer,
    /// Each opponent.
    EachOpponent,
    /// The controller of the first target.
    ControllerOfTarget,
    /// The player chosen via `Pending::ChoosePlayer`.
    Chosen,
}

/// Target specifications (chosen at cast/activation, CR 601.2c).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TargetSpec {
    /// An object matching the filter (battlefield, or stack for spells).
    Object(&'static Filter),
    /// A spell on the stack matching the filter.
    Spell(&'static Filter),
    /// A spell on the stack OR a permanent on the battlefield (Venser).
    StackOrBattlefield(&'static Filter),
    /// A card in a graveyard matching the filter.
    CardInGraveyard(&'static Filter, PlayerRel),
    /// The source object.
    ThisObject,
    /// An activated/triggered ability on the stack (Tishana's Tidebinder).
    AbilityOnStack(&'static Filter),
    /// A spell or ability on the stack (Ertai Resurrected's counter mode).
    SpellOrAbility(&'static Filter),
    /// The object the triggering event was about (Wartime Protestors'
    /// "that creature").
    EventObject,
    /// A player relative to the controller (You/Opponent; heads-up
    /// auto-resolves for Opponent in two-player games).
    Player(PlayerRel),
    /// Any player (choice via `Pending::ChoosePlayer`).
    AnyPlayer,
}

/// How many targets an ability/spell requires.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TargetReq {
    /// What may be targeted.
    pub spec: TargetSpec,
    /// Minimum number of targets (0 = may decline).
    pub min: u8,
    /// Maximum number of targets (255 = "any number", X-driven).
    pub max: u8,
    /// Whether the count is exactly X (Curse of the Swine).
    pub count_is_x: bool,
}

impl TargetReq {
    /// Exactly one target.
    pub const fn one(spec: TargetSpec) -> Self {
        Self {
            spec,
            min: 1,
            max: 1,
            count_is_x: false,
        }
    }

    /// Up to one target.
    pub const fn up_to_one(spec: TargetSpec) -> Self {
        Self {
            spec,
            min: 0,
            max: 1,
            count_is_x: false,
        }
    }

    /// Up to `max` targets.
    pub const fn up_to(spec: TargetSpec, max: u8) -> Self {
        Self {
            spec,
            min: 0,
            max,
            count_is_x: false,
        }
    }

    /// Exactly X targets.
    pub const fn x_targets(spec: TargetSpec) -> Self {
        Self {
            spec,
            min: 0,
            max: 255,
            count_is_x: true,
        }
    }
}

/// What happens when restricted mana is spent on a matching spell.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SpendRider {
    /// Nothing extra (restriction only).
    None,
    /// The spell can't be countered (Cavern of Souls).
    Uncounterable,
    /// The caster scries N (Path of Ancestry).
    Scry(u8),
}

/// Where a searched card goes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SearchDest {
    /// Into your hand.
    Hand,
    /// Onto the battlefield (optionally tapped).
    Battlefield,
    /// On top of your library.
    TopOfLibrary,
}

/// A single effect operation.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Effect {
    /// Run operations in order.
    Sequence(&'static [Effect]),
    /// You gain life.
    GainLife {
        /// How much.
        amount: Amount,
    },
    /// A chosen relative player gains life.
    GainLifeFor {
        /// How much.
        amount: Amount,
        /// Who.
        who: PlayerRel,
    },
    /// Exile a target object.
    Exile {
        /// What.
        target: TargetSpec,
    },
    /// Exile a target and return it to the battlefield immediately
    /// (Ephemerate).
    Blink {
        /// What.
        target: TargetSpec,
    },
    /// Look at the top `count` cards of your library; put `pick` of them
    /// into your hand and the rest on the bottom in any order (Dig
    /// Through Time).
    LookAtTopPick {
        /// How many to look at.
        count: u8,
        /// How many to keep.
        pick: u8,
    },
    /// Put cards from your hand on top of your library, in the order they
    /// were chosen (Brainstorm-style).
    PutFromHandOnTop {
        /// How many.
        count: u8,
    },
    /// A player loses life.
    LoseLife {
        /// How much.
        amount: Amount,
        /// Who.
        target: PlayerRel,
    },
    /// You draw cards.
    DrawCards {
        /// How many.
        amount: Amount,
    },
    /// A relative player draws cards.
    DrawCardsFor {
        /// How many.
        amount: Amount,
        /// Who.
        who: PlayerRel,
    },
    /// Exile all targets; each exiled permanent's controller creates the
    /// token (Curse of the Swine).
    ExileTargetsCreateTokens {
        /// The token to create per exiled permanent.
        token: &'static TokenDef,
    },
    /// Deal damage to a target.
    DealDamage {
        /// How much.
        amount: Amount,
        /// To what.
        target: TargetSpec,
    },
    /// Deal damage to the first target's controller (Tuktuk Scrapper).
    DealDamageToTargetController {
        /// How much.
        amount: Amount,
    },
    /// Destroy a target permanent (can't be regenerated).
    Destroy {
        /// What.
        target: TargetSpec,
    },
    /// Put each target on the bottom of its owner's library (Banishing
    /// Stroke).
    PutTargetOnBottomOfLibrary,
    /// The first target (a card in a graveyard) gains flashback with
    /// flashback cost = its mana cost until end of turn (Snapcaster
    /// Mage).
    GrantFlashback,
    /// The controller takes an extra turn after this one (Temporal
    /// Mastery).
    TakeExtraTurn,
    /// Exile the source object (Temporal Mastery's self-exile rider).
    ExileSource,
    /// Tap each target.
    TapTarget,
    /// Untap each target.
    UntapTarget,
    /// Exile each target; return it to the battlefield under its owner's
    /// control at the beginning of the next end step (Venser +2).
    ExileAndReturnAtEndStep,
    /// Counter a spell on the stack; it goes to exile instead of the
    /// graveyard (Force of Negation).
    CounterTargetSpellToExile,
    /// Counter a spell on the stack.
    CounterTargetSpell,
    /// Counter an activated or triggered ability on the stack (Tishana's
    /// Tidebinder).
    CounterTargetAbility,
    /// Counter the first target regardless of whether it is a spell or an
    /// ability on the stack (Ertai Resurrected).
    CounterTargetSpellOrAbility,
    /// The source of the first target (an ability) loses all abilities
    /// until end of turn.
    TargetSourceLosesAbilities,
    /// Register delayed mana at the controller's next first main phase
    /// (Mana Drain): colorless mana equal to the first target's cmc.
    DelayedManaAtNextFirstMain {
        /// Color of the mana.
        color: ManaColor,
    },
    /// Change the target of the first target (a spell on the stack) to a
    /// new target matching the given filter (Misdirection).
    RedirectTarget {
        /// What the new target must match.
        new_filter: &'static Filter,
    },
    /// Exchange control of the source and the first target (Gilded
    /// Drake); if no exchange happens (no/illegal target), sacrifice the
    /// source.
    ExchangeControlOrSacrifice,
    /// For each player in `who`, that player chooses up to one matching
    /// permanent they control and it is destroyed (The True
    /// Scriptures I).
    DestroyChosenForPlayers {
        /// Who chooses.
        who: PlayerRel,
        /// What may be destroyed.
        filter: &'static Filter,
    },
    /// Each player in `who` discards `count` cards (their choice).
    DiscardForPlayers {
        /// Who discards.
        who: PlayerRel,
        /// How many cards.
        count: u8,
    },
    /// Put all creature cards from all graveyards onto the battlefield
    /// under your control (The True Scriptures III).
    AllGraveyardCreaturesToBattlefield,
    /// Exile the source, then return it to the battlefield under its
    /// owner's control as the given face (transform; Sheoldred's flip,
    /// saga final chapters).
    ExileSelfReturnAsFace {
        /// The face to return as (0 = front).
        face: u8,
    },
    /// Each player in `who` sacrifices a permanent they control matching
    /// the filter (their choice; Sheoldred's Edict).
    SacrificeFilter {
        /// Who sacrifices.
        who: PlayerRel,
        /// What may be sacrificed.
        filter: &'static Filter,
    },
    /// Remove all counters from all permanents; the source enters with
    /// that many +1/+1 counters (Thief of Blood).
    DrainAllCountersIntoSelf,
    /// Shuffle your graveyard into your library (Spirit Water Revival's
    /// waterbend outcome).
    ShuffleGraveyardIntoLibrary,
    /// Branch on whether the spell was kicked (paid its additional cost).
    IfKicked {
        /// Effects when kicked.
        then: &'static [Effect],
        /// Effects otherwise.
        otherwise: &'static [Effect],
    },
    /// Branch: you didn't lose life this turn (Luminarch Ascension).
    IfNotLostLifeThisTurn {
        /// Effects when the condition holds.
        then: &'static [Effect],
    },
    /// Branch: you control a `filter`-matching permanent with the
    /// greatest cmc among `filter`-matching permanents (or tied; Padeem).
    IfControlGreatestCmc {
        /// The comparison class.
        filter: &'static Filter,
        /// Effects when the condition holds.
        then: &'static [Effect],
    },
    /// Branch on the event object's power (Tribute to the World Tree):
    /// `then` when power >= `n`, else `otherwise`.
    IfEventPowerAtLeast {
        /// Threshold.
        n: i16,
        /// Effects when power >= n.
        then: &'static [Effect],
        /// Effects otherwise.
        otherwise: &'static [Effect],
    },
    /// Add one mana of any color that a land (yours or an opponent's)
    /// could produce (Exotic Orchard / Reflecting Pool).
    AddManaLandColor {
        /// `true` = your lands, `false` = opponents' lands.
        mine: bool,
    },
    /// Twice X (Heliod's Intervention lifegain mode) — helper amount.
    GainLifeDoubleX,
    /// Search your library for a matching card (server-side filtered).
    SearchLibrary {
        /// What to find.
        filter: &'static Filter,
        /// Where it goes.
        dest: SearchDest,
        /// Whether it enters tapped (battlefield only).
        tapped: bool,
        /// Whether to shuffle afterwards.
        shuffle: bool,
        /// Whether you may decline to find ("search … you may").
        optional: bool,
    },
    /// Scry N.
    Scry {
        /// How many.
        amount: Amount,
    },
    /// A relative player scries N (Jace's +2).
    ScryFor {
        /// Who.
        player: PlayerRel,
        /// How many.
        amount: Amount,
    },
    /// Exile all cards from a player's library, then they shuffle their
    /// hand into their library (Jace's ultimate).
    ExileLibraryAndShuffleHand {
        /// Who.
        player: PlayerRel,
    },
    /// All objects matching a filter get P/T set to computed values until
    /// a duration ends (Karn's animation).
    SetPTFilter {
        /// Which objects.
        filter: &'static Filter,
        /// New power (may be computed, e.g. `TargetCmc`).
        power: Amount,
        /// New toughness.
        toughness: Amount,
        /// How long.
        duration: crate::static_ability::Duration,
    },
    /// Mill cards.
    Mill {
        /// How many.
        amount: Amount,
        /// Who.
        target: PlayerRel,
    },
    /// Add mana to your pool.
    AddMana {
        /// Color.
        color: ManaColor,
        /// Amount.
        amount: u16,
    },
    /// Add a computed amount of one color (Harabaz Druid: X = Allies).
    AddManaDynamic {
        /// Color.
        color: ManaColor,
        /// How much.
        amount: Amount,
    },
    /// Add a subtype-granting note — placeholder for M2 (changeling etc.).
    GrantSubtype {
        /// Subtype.
        subtype: SubtypeId,
    },
    /// Put counters on the first target (or the source when no target).
    AddCounter {
        /// Counter kind.
        kind: CounterKind,
        /// How many.
        amount: Amount,
    },
    /// Put counters on every object matching a filter (Kazandu
    /// Blademaster's rally).
    AddCounterFilter {
        /// Which objects.
        filter: &'static Filter,
        /// Counter kind.
        kind: CounterKind,
        /// How many per object.
        amount: Amount,
    },
    /// Return a target object (battlefield or stack) to its owner's hand.
    ReturnToHand {
        /// What.
        target: TargetSpec,
    },
    /// Return all objects matching a filter to their owners' hands.
    ReturnAllToHand {
        /// What.
        filter: &'static Filter,
        /// Only objects controlled by opponents (Cyclonic Rift style).
        opponents_only: bool,
    },
    /// Destroy all objects matching a filter (wraths).
    DestroyAll {
        /// What.
        filter: &'static Filter,
    },
    /// Exile all cards from a player's graveyard (Bojuka Bog).
    ExileGraveyard {
        /// Whose graveyard.
        player: PlayerRel,
    },
    /// Put a graveyard card on top of its owner's library (Volrath's).
    GraveyardToTop {
        /// What (`CardInGraveyard`).
        target: TargetSpec,
    },
    /// Return a graveyard card to its owner's hand (Archaeomancer).
    GraveyardToHand {
        /// What (`CardInGraveyard`).
        target: TargetSpec,
    },
    /// Put a graveyard card onto the battlefield under your control
    /// (reanimation).
    GraveyardToBattlefield {
        /// What (`CardInGraveyard`).
        target: TargetSpec,
    },
    /// Add one mana of any color in your commander's color identity
    /// (Command Tower); the choice is made on resolution.
    AddManaCommanderIdentity,
    /// Add restricted mana in a commander-identity color (Path of
    /// Ancestry): color choice from your commander's identity; spendable
    /// only on spells matching `filter`; `rider` applies on spend.
    AddManaRestrictedCommanderIdentity {
        /// What the mana may be spent on.
        filter: &'static Filter,
        /// What happens when it's spent on a matching spell.
        rider: SpendRider,
    },
    /// Add restricted mana (Cavern of Souls, Path of Ancestry, Jasmine
    /// Dragon): it can only be spent on spells matching `filter`, and
    /// spending it applies `rider`.
    AddManaRestricted {
        /// Choosable colors (one = fixed).
        colors: &'static [ManaColor],
        /// Amount.
        amount: u16,
        /// What the mana may be spent on.
        filter: &'static Filter,
        /// What happens when it's spent on a matching spell.
        rider: SpendRider,
    },
    /// Add mana of a chosen color (choice per mana when `combination`).
    AddManaChoice {
        /// Allowed colors.
        colors: &'static [ManaColor],
        /// How much mana (dynamic amounts evaluate at resolution).
        amount: Amount,
        /// Whether each mana may be a different color (filter lands).
        combination: bool,
    },
    /// Create a token that gets +P/+T for each filter-matching permanent
    /// you control (Urza's Saga's Construct; registered as its own
    /// continuous effect).
    CreateTokenPtPerCount {
        /// The token.
        token: &'static TokenDef,
        /// What to count.
        filter: &'static Filter,
        /// Power per match.
        p: i16,
        /// Toughness per match.
        t: i16,
    },
    /// Create a token under your control.
    CreateToken {
        /// What.
        token: &'static TokenDef,
    },
    /// Create N tokens under your control (Aang and Katara).
    CreateTokenN {
        /// What.
        token: &'static TokenDef,
        /// How many.
        amount: Amount,
    },
    /// Create a token under the first target's controller (Crib Swap).
    CreateTokenForTargetController {
        /// What.
        token: &'static TokenDef,
    },
    /// Put N +1/+1 counters on an Army creature you control, or create a
    /// 0/0 Army token with N +1/+1 counters if you control none (amass).
    Amass {
        /// Army subtype.
        subtype: SubtypeId,
        /// How many counters.
        amount: u16,
    },
    /// Put the source on top of its owner's library (Sensei's Divining Top).
    PutSourceOnTopOfLibrary,
    /// Create a token that's a copy of a target permanent (Rite of
    /// Replication, Progenitor Mimic).
    CreateTokenCopyOf {
        /// What to copy (first target when set, else the source).
        target: Option<TargetSpec>,
        /// Extra copies when the spell was kicked (Rite of Replication: 4
        /// bonus tokens for a total of 5).
        kicked_bonus: u8,
    },
    /// Create a token that's a copy of the creature the source is attached
    /// to (Helm of the Host).
    CreateTokenCopyOfEquipped {
        /// Extra copies when the spell was kicked.
        kicked_bonus: u8,
        /// Copy modifications ("isn't legendary", "gains haste").
        mods: &'static [crate::ability::CopyMod],
    },
    /// Create a token that's a copy of the first creature token you
    /// control (populate; no-op if none).
    CreateTokenCopyOfFirstToken,
    /// A relative player puts a filtered card from their hand on the
    /// bottom of their library (Vendilion Clique).
    BottomCardFromHand {
        /// Whose hand.
        player: PlayerRel,
        /// Which cards may be chosen.
        filter: &'static Filter,
    },
    /// Copy a spell on the stack (Double Major, Jin-Gitaxias). The copy
    /// goes on the stack under your control; you may choose new targets
    /// (M3 protocol choice; currently same targets).
    /// Copy the first target (a spell on the stack) with modifications
    /// ("except it isn't legendary").
    CopyTargetSpell {
        /// Copy modifications.
        mods: &'static [crate::ability::CopyMod],
    },
    /// Attach the source (equipment/aura) to a target permanent.
    AttachSelf {
        /// To what.
        target: TargetSpec,
    },
    /// Look at the top N cards of your library and put them back in any
    /// order.
    ReorderTopLibrary {
        /// How many.
        count: u8,
    },
    /// Shockland entry: you may pay N life; if you don't, the source
    /// enters tapped (yes/no choice).
    PayLifeOrEnterTapped {
        /// Life to pay.
        amount: u16,
    },
    /// A player may pay {N}; if they don't, run `effect` (Rhystic Study,
    /// Esper Sentinel, Smothering Tithe).
    PlayerMayPayOr {
        /// Who decides.
        player: PlayerRel,
        /// Generic mana to pay.
        mana: u16,
        /// What happens when they don't pay.
        effect: &'static Effect,
    },
    /// Create a continuous effect (Giant Growth style): applies `modifier`
    /// on `layer` to `filter` for `duration`. `filter = This` binds to the
    /// first target.
    CreateContinuousEffect {
        /// The layer it applies in.
        layer: crate::static_ability::Layer,
        /// Which objects are affected (`This` = first target).
        filter: &'static Filter,
        /// What changes.
        modifier: crate::static_ability::Modifier,
        /// How long it lasts.
        duration: crate::static_ability::Duration,
    },
    /// Change who controls a target permanent (Gilded Drake exchange,
    /// Homeward Path restore).
    ChangeController {
        /// Who gains control.
        new_controller: PlayerRel,
    },
    /// Each player gains control of all creatures they own (Homeward
    /// Path).
    AllCreaturesToOwner,
    /// Control rotation (Aminatou −6, heads-up): each nonland permanent
    /// except the source changes controller to the other player.
    ControlRotation,
    /// Phase a target permanent out (Clever Concealment).
    PhaseOut {
        /// What phases out (first target when set, else the source).
        target: Option<TargetSpec>,
    },
    /// Exile a target with a link to the source ("until ~ leaves the
    /// battlefield", Skyclave Apparition).
    ExileLinked {
        /// What.
        target: TargetSpec,
    },
    /// Return everything exiled with a link to the source to the
    /// battlefield under its owner's control.
    ReturnLinkedToBattlefield,
    /// Create a token under the *owner* of the card exiled with a link to
    /// the source, with power/toughness set to that card's mana value
    /// (Skyclave Apparition's Illusion).
    CreateTokenFromLinked {
        /// The token to create (power/toughness are overridden by the
        /// linked card's mana value).
        token: &'static TokenDef,
    },
    /// Sacrifice the source permanent (evoke).
    SacrificeSelf,
    /// Register a delayed "pay or lose" trigger at your next upkeep
    /// (Pact of Negation).
    PayCostOrLoseLater {
        /// The mana cost to pay at your next upkeep.
        cost: ManaCost,
    },
    /// The controller gets an emblem with the given abilities
    /// (planeswalker ultimates).
    CreateEmblem {
        /// The emblem's abilities.
        abilities: &'static [crate::ability::AbilityDef],
    },
    /// You become the monarch (Palace Jailer).
    BecomeMonarch,
    /// A relative player may search their library for a basic land onto
    /// the battlefield tapped, then shuffle (Path to Exile).
    OptionalBasicLandSearchFor {
        /// Who may search.
        player: PlayerRel,
    },
    /// All objects matching a filter get computed P/T modifiers until a
    /// duration ends (Toxic Deluge: `-X/-X` on all creatures).
    PumpFilter {
        /// Which objects are pumped.
        filter: &'static Filter,
        /// Power modifier (may be negative/X-driven).
        power: Amount,
        /// Toughness modifier (may be negative/X-driven).
        toughness: Amount,
        /// How long.
        duration: crate::static_ability::Duration,
    },
}
