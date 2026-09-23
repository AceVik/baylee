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
    /// There is no `All`: an activation already takes one condition, and two
    /// conditions that must both hold are a sentence no card in this pool
    /// prints. It is added the day one does, and not before.
    Any(&'static [Condition]),
}

/// Trigger conditions for triggered abilities.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Trigger {
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
    /// A creature matching the filter is exiled from the battlefield
    /// (Soulherder).
    ExiledFromBattlefield(&'static Filter),
    /// A source matching the filter deals combat damage to a player
    /// (Sword of Hearth and Home: the equipped creature).
    DealsCombatDamageToPlayer(&'static Filter),
    /// The source becomes tapped (City of Brass).
    BecomesTapped(&'static Filter),
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
    /// A player draws a card except the first one they draw each turn
    /// (Orcish Bowmasters).
    DrawsExceptFirst(crate::effect::PlayerRel),
    /// An object matching the filter attacks (Sun Titan).
    Attacks(&'static Filter),
    /// The first noncreature spell cast by a player each turn (Esper
    /// Sentinel).
    FirstNoncreatureSpellCast(crate::effect::PlayerRel),
    /// The source entered the battlefield AND was evoked (cast for its
    /// evoke cost, CR 702.74).
    EntersBattlefieldEvoked,
    /// A step begins (whose turn: you/opponent/any).
    StepBegin {
        /// Which step.
        step: StepKind,
        /// Whose turn.
        whose: crate::effect::PlayerRel,
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
    },
    /// Activated ability (`cost: effect`).
    Activated {
        /// Activation cost.
        cost: Cost,
        /// Effect operations.
        effects: &'static [Effect],
        /// Target requirement.
        target: Option<TargetSpec>,
        /// Timing restriction.
        timing: ActivationTiming,
        /// Mana abilities don't use the stack (CR 605.1).
        mana_ability: bool,
        /// Where the ability may be activated from (cycling = from hand).
        zone: ActivationZone,
        /// "Activate only once each turn", if the card prints one.
        limit: ActivationLimit,
    },
    /// Triggered ability (`when/whenever/at …, effect`).
    Triggered {
        /// Trigger condition.
        trigger: Trigger,
        /// Effect operations.
        effects: &'static [Effect],
        /// Target requirement.
        targets: Option<crate::effect::TargetReq>,
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
    /// Static/continuous ability (layers, CR 613).
    /// An activated ability with a precondition (Mox Opal's metalcraft,
    /// Bleachbone Verge's Plains/Swamp check).
    ActivatedConditional {
        /// The cost.
        cost: crate::cost::Cost,
        /// Effect operations.
        effects: &'static [crate::effect::Effect],
        /// Target requirement.
        target: Option<crate::effect::TargetSpec>,
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
    /// A spell with modes: the caster chooses one (overload, choose-one
    /// charms). Each mode may override the cost.
    ModalSpell {
        /// The modes to choose from.
        modes: &'static [SpellMode],
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
    /// Cost override for this mode (overload); `None` = the printed cost.
    pub cost_override: Option<baylee_core::mana::ManaCost>,
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
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability,
            zone: ActivationZone::Battlefield,
            limit: ActivationLimit::Unlimited,
        }
    }

    fn conditional(mana_ability: bool) -> AbilityDef {
        AbilityDef::ActivatedConditional {
            cost: crate::cost::Cost::TAP,
            effects: NOTHING,
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability,
            zone: ActivationZone::Battlefield,
            condition: Condition::ControlCount(&crate::Filter::ARTIFACT, 3),
            limit: ActivationLimit::Unlimited,
        }
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
            },
            AbilityDef::Triggered {
                trigger: Trigger::ETB,
                effects: NOTHING,
                targets: None,
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
            }),
            AbilityDef::Replacement(ReplacementRule::DoubleTokenCreation {
                controller_filter: &crate::Filter::Any,
            }),
            AbilityDef::ModalSpell { modes: &[] },
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
