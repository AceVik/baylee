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
        target: Option<TargetSpec>,
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
    },
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
}

/// One mode of a [`crate::AbilityDef::ModalSpell`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SpellMode {
    /// Effect operations of this mode.
    pub effects: &'static [crate::effect::Effect],
    /// Target requirement of this mode.
    pub target: Option<crate::effect::TargetSpec>,
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
