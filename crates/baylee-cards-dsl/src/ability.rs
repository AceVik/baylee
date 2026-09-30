//! Ability definitions on cards.

use crate::cost::Cost;
use crate::effect::{Effect, TargetSpec};
use crate::filter::Filter;

/// When an activated ability may be played.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ActivationTiming {
    /// Any time you have priority.
    InstantSpeed,
    /// Only in your main phase, empty stack ("as a sorcery").
    SorcerySpeed,
}

/// How often an activated ability may be used in one turn.
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
/// There is no per-*game* variant, and that is a measurement rather than an
/// omission: the only cards that want one are Urza's Fun House, which also
/// needs a condition the DSL cannot say, and the exhaust keyword — and
/// exhaust is a keyword other cards look for ("whenever you activate an
/// exhaust ability"), so it is a keyword bit and not a number.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ActivationLimit {
    /// No printed limit — as often as its cost can be paid (CR 602.2).
    Unlimited,
    /// At most this many activations per turn, per object.
    PerTurn(u8),
}

/// Where an activated ability may be activated from.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ActivationZone {
    /// On the battlefield (default).
    Battlefield,
    /// From your hand (cycling).
    Hand,
    /// From your graveyard (eternalize, embalm): the card is in its owner's
    /// graveyard and the owner activates it.
    Graveyard,
}

/// Steps/phases triggers can listen to.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum StepKind {
    /// Upkeep step.
    Upkeep,
    /// Draw step.
    Draw,
    /// Beginning of combat.
    CombatBegin,
    /// End step.
    End,
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
    /// It is the current controller's turn (not necessarily the owner's).
    YourTurn,
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
    /// on it (CR 721.2a, "As long as this permanent has N or more charge
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
    /// There is no `All`. A conjunction is printed, but inside an effect:
    /// the Urza lands' "if you control an Urza's Mine and an Urza's
    /// Power-Plant, add {C}{C}{C} instead" is two nested
    /// `Effect::IfCondition`s. It is added the day one stands where a single
    /// condition is all there is room for (an activation restriction, an
    /// intervening `if`), and not before.
    Any(&'static [Condition]),
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
}

/// Trigger conditions for triggered abilities.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Trigger {
    /// This permanent is turned face up (not transformed or entering).
    TurnedFaceUp,
    /// "Whenever this creature transforms into [this face]" (CR 701.27e):
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
    /// An object matching the filter leaves the battlefield.
    LeavesBattlefield(&'static Filter),
    /// An object matching the filter dies (battlefield → graveyard).
    Dies(&'static Filter),
    /// A spell matching the filter is cast.
    SpellCast(&'static Filter),
    /// The source becomes the target of a spell or ability (ward,
    /// Phantasmal Image).
    BecomesTarget,
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
    /// A permanent matching the filter is dealt damage (Fungusaur:
    /// "whenever this creature is dealt damage, put a +1/+1 counter on
    /// it"). All combat damage is dealt at once (CR 510.2), so a creature
    /// blocked by three is dealt damage in one event and this triggers once
    /// for it (CR 603.2c); every other damage event triggers it once.
    /// Damage that was prevented was not dealt, and triggers nothing
    /// (CR 603.2g).
    DealtDamage(&'static Filter),
    /// A permanent matching the filter becomes tapped, for any reason:
    /// City of Brass's own (`Filter::This`), Lifetap's "a Forest an
    /// opponent controls", Psychic Venom's enchanted land. The permanent is
    /// the event's object, so `PlayerRel::ControllerOfEvent` is "that
    /// land's controller".
    BecomesTapped(&'static Filter),
    /// The count of a kind of counter on the source rises from below `n`
    /// to `n` or more — the window CR 714.2b writes out for a chapter
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
    /// The controller casts their Nth spell this turn (Storm of
    /// Saruman's second-spell trigger).
    NthSpellCast {
        /// Which spell number.
        n: u8,
        /// The spell filter.
        filter: &'static Filter,
    },
    /// A player draws a card.
    Draws(crate::effect::PlayerRel),
    /// "Whenever [a player] plays a land" (Fastbond): the special action of
    /// playing a land (CR 116.2a, 305.1), from whatever zone a permission
    /// allows, and never a land an effect puts onto the battlefield, which
    /// is not played. The relation names whose play: `You` for "you".
    PlaysLand(crate::effect::PlayerRel),
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
    /// This creature blocks a creature matching the filter (CR 509.3b) or
    /// becomes blocked by one (CR 509.3d): once for each such pair, each
    /// time a blocker is declared, so a creature blocked by two of them
    /// triggers twice. The filter describes the *other* creature, as it is
    /// when it blocks or is blocked (CR 509.3f), and that creature is the
    /// trigger's event object, [`TargetSpec::EventObject`](crate::TargetSpec)
    /// — Cockatrice's "whenever this creature blocks or becomes blocked by a
    /// non-Wall creature, destroy that creature at end of combat".
    BlocksOrBecomesBlockedBy(&'static Filter),
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
    /// A state trigger (CR 603.8): "When you control no Islands, sacrifice
    /// this creature." It triggers whenever the condition holds, asked with
    /// the source as `this` and its controller as "you", and not again
    /// while the ability is waiting to go on the stack or is on it; once it
    /// has left the stack, a source still on the battlefield with the
    /// condition still true triggers again. No event matches it.
    State(&'static Condition),
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
}

/// An ability definition on a [`crate::CardDef`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AbilityDef {
    /// Placeholder for unimplemented stubs.
    Unimplemented,
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
        /// "This ability costs {1} less to activate for each …", if the
        /// ability prints one.
        cost_reduction: Option<crate::cost::CostReduction>,
    },
    /// Triggered ability (`when/whenever/at …, effect`).
    Triggered {
        /// Trigger condition.
        trigger: Trigger,
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
        /// As on the unconditional twin.
        cost_reduction: Option<crate::cost::CostReduction>,
    },
    /// One chapter of a saga (CR 714): triggers when the corresponding
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
        /// The linked spell card.
        card: baylee_core::ids::CardIndex,
    },
    /// Echo (CR 702.30): at your next upkeep after this enters, pay the
    /// cost or sacrifice it.
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
    /// A triggered ability with modes: the controller chooses one when it
    /// triggers (Charming Prince, Aether Channeler).
    ModalTriggered {
        /// Trigger condition.
        trigger: Trigger,
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
}

impl AbilityDef {
    /// Whether this is a cycling ability: "Cycling [cost]" means "[Cost],
    /// Discard this card: Draw a card" and works only from the hand
    /// (CR 702.29a). That shape is the whole definition, so it is what is
    /// read: an ability activated from the hand whose cost discards the card
    /// itself and whose effect is to draw one card. A card that discards
    /// itself for anything else — Trumpeting Carnosaur's damage, Boseiju's
    /// channel — is not cycled. Typecycling (702.29e), which searches
    /// instead, is not read here; no card in the pool that cares about being
    /// cycled prints it.
    #[must_use]
    pub fn is_cycling(&self) -> bool {
        match self {
            Self::Activated {
                cost,
                effects,
                zone: ActivationZone::Hand,
                ..
            }
            | Self::ActivatedConditional {
                cost,
                effects,
                zone: ActivationZone::Hand,
                ..
            } => {
                cost.parts.contains(&crate::cost::CostPart::DiscardSelf)
                    && *effects == [crate::effect::Effect::draw(1)]
            }
            _ => false,
        }
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
    pub const fn is_mana_ability(&self) -> bool {
        matches!(
            self,
            Self::Activated {
                mana_ability: true,
                ..
            } | Self::ActivatedConditional {
                mana_ability: true,
                ..
            }
        )
    }

    /// Whether this is a **triggered** mana ability (CR 605.1b): it has no
    /// target, it triggers from a mana ability — [`Trigger::TappedForMana`]
    /// is the one trigger in the vocabulary that does — and it could add
    /// mana. Such an ability resolves the moment it triggers, off the stack
    /// (CR 605.4a).
    ///
    /// Derived, as [`Self::is_mana_ability`]'s flag is checked against the
    /// same three questions by `lints::mana_ability_fault`: a "whenever you
    /// tap this land for mana" that targets (Forbidden Orchard's) is an
    /// ordinary trigger and goes on the stack.
    #[must_use]
    pub fn is_triggered_mana_ability(&self) -> bool {
        matches!(
            self,
            Self::Triggered {
                trigger: Trigger::TappedForMana { .. },
                targets: None,
                effects,
                ..
            } if effects.iter().any(|effect| matches!(
                effect,
                crate::effect::Effect::AddMana { .. } | crate::effect::Effect::AddManaFor { .. }
            ))
        )
    }
}

/// A modification applied after a clone copies its target.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CopyMod {
    /// Adds types ("except it's an artifact").
    AddType(baylee_core::types::TypeSet),
    /// Removes types ("except it's not a creature").
    RemoveType(baylee_core::types::TypeSet),
    /// Removes supertypes ("except it's not legendary").
    RemoveSupertype(baylee_core::types::SupertypeSet),
    /// Adds a subtype ("except it's a Shapeshifter").
    AddSubtype(baylee_core::ids::SubtypeId),
    /// Grants a keyword ("with haste").
    AddKeyword(crate::KeywordSet),
    /// Enters with counters of a kind.
    AddCounter(crate::CounterKind, u16),
    /// Sets power and toughness ("except it's a 4/4", eternalize,
    /// CR 702.129a). A copiable value of the copy (CR 707.9b).
    SetPT(i16, i16),
    /// Sets the colors ("except it's black"), replacing the copied ones
    /// (CR 707.9b).
    SetColor(baylee_core::color::ColorSet),
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

    fn activated(mana_ability: bool) -> AbilityDef {
        AbilityDef::Activated {
            cost: crate::cost::Cost::TAP,
            effects: NOTHING,
            targets: None,
            second_targets: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability,
            zone: ActivationZone::Battlefield,
            limit: ActivationLimit::Unlimited,
            cost_reduction: None,
        }
    }

    fn conditional(mana_ability: bool) -> AbilityDef {
        AbilityDef::ActivatedConditional {
            cost: crate::cost::Cost::TAP,
            effects: NOTHING,
            targets: None,
            second_targets: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability,
            zone: ActivationZone::Battlefield,
            condition: Condition::ControlCount(&crate::Filter::ARTIFACT, 3),
            limit: ActivationLimit::Unlimited,
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
            cost_reduction: None,
        };
        assert!(ability(DISCARD, DRAW, ActivationZone::Hand).is_cycling());
        assert!(!ability(DISCARD, DAMAGE, ActivationZone::Hand).is_cycling());
        assert!(!ability(DISCARD, DRAW, ActivationZone::Battlefield).is_cycling());
        assert!(!ability(crate::cost::Cost::FREE, DRAW, ActivationZone::Hand).is_cycling());
        assert!(!activated(false).is_cycling());
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
            },
            AbilityDef::Triggered {
                trigger: Trigger::ETB,
                effects: NOTHING,
                targets: None,
                second_targets: None,
                once_per_turn: false,
                condition: None,
            },
            AbilityDef::Ward { mana: 2 },
            AbilityDef::SagaChapter {
                chapter: 1,
                effects: NOTHING,
                targets: None,
            },
            AbilityDef::Prepared {
                card: baylee_core::ids::CardIndex::new(1),
            },
            AbilityDef::Echo {
                cost: baylee_core::mana::ManaCost::ZERO,
            },
            AbilityDef::Static(StaticAbility {
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
                once_per_turn: false,
                condition: None,
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
