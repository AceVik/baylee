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
    /// A player draws a card.
    Draws(crate::effect::PlayerRel),
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
        target: Option<TargetSpec>,
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
    },
    /// Triggered ability (`when/whenever/at …, effect`).
    Triggered {
        /// Trigger condition.
        trigger: Trigger,
        /// Effect operations.
        effects: &'static [Effect],
        /// Target requirement.
        target: Option<TargetSpec>,
        /// "Up to one target" — the choice may be declined.
        up_to_one: bool,
    },
    /// Static/continuous ability (layers, CR 613).
    Static(crate::static_ability::StaticAbility),
    /// A replacement or trigger-modification rule (CR 614; Doubling
    /// Season, Panharmonicon, Elesh Norn).
    Replacement(crate::static_ability::ReplacementRule),
}

/// Which event a trigger-modifying rule cares about.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TriggerEventKind {
    /// A permanent entering the battlefield.
    EntersBattlefield,
    /// Any event.
    Any,
}
