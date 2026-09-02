//! Authoring surface: the macros a card file is written with.
//!
//! # Why any of this exists
//!
//! [`CardDef`](crate::CardDef) and [`FaceDef`](crate::FaceDef) have always
//! had a `DEFAULT`, and the authoring rule has always been "state only what
//! distinguishes this card, then `..DEFAULT`". Abilities had no such thing —
//! an enum variant cannot take a struct-update tail — so every activated
//! ability in the pool spelled out `target`, `timing`, `mana_ability` and
//! `zone` whether or not the card said anything about them. Four lines of
//! noise per ability, and four more chances to write the wrong one.
//!
//! The `*Parts` structs here close that gap: each is a plain struct whose
//! `new` takes the fields an ability cannot be written without, leaves the
//! rest at their rules defaults, and has a `const fn build` that produces the
//! enum variant. The macros are the sugar over that, so a card file reads
//!
//! ```ignore
//! abilities: &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
//! ```
//!
//! instead of a seven-line literal.
//!
//! # The defaults are rules defaults
//!
//! Every default below is what the *comprehensive rules* say when a card is
//! silent, not merely what most cards happen to do — which is the only thing
//! that makes omitting a field safe:
//!
//! - `timing: InstantSpeed` — CR 602.2: an activated ability may be
//!   activated whenever its controller has priority, unless the card
//!   restricts it.
//! - `mana_ability: false` — CR 605.1 makes a mana ability the *exception*
//!   (it must add mana, have no target, and not be a loyalty ability). The
//!   pessimistic default is the load-bearing one: an ability wrongly marked
//!   `true` would silently skip the stack.
//! - `zone: Battlefield` — CR 113.6: an ability functions on the battlefield
//!   unless it says otherwise.
//! - `target` / `targets: None` — an ability targets only when it says
//!   "target".
//! - `once_per_turn: false` — a trigger fires every time its event happens.
//!
//! Anything with no rules default is a parameter of `new` instead, so it
//! cannot be forgotten: a trigger has no neutral value, and an ability with
//! no effects is not an ability.

use crate::ability::{AbilityDef, ActivationTiming, ActivationZone, SpellMode, Trigger};
use crate::cost::Cost;
use crate::effect::{Effect, TargetReq, TargetSpec};

/// The parts of an [`AbilityDef::Activated`], with rules defaults.
///
/// Written through [`activated!`](crate::activated) or
/// [`mana_ability!`](crate::mana_ability) rather than by hand.
#[derive(Clone, Copy, Debug)]
pub struct ActivatedParts {
    /// What it costs to activate.
    pub cost: Cost,
    /// What it does.
    pub effects: &'static [Effect],
    /// What it targets, if anything.
    pub target: Option<TargetSpec>,
    /// When it may be activated.
    pub timing: ActivationTiming,
    /// Whether it is a mana ability (CR 605.1 — does not use the stack).
    pub mana_ability: bool,
    /// Where it functions.
    pub zone: ActivationZone,
}

impl ActivatedParts {
    /// An ability with this cost and these effects, everything else at the
    /// rules default: untargeted, instant speed, on the battlefield, and not
    /// a mana ability.
    #[must_use]
    pub const fn new(cost: Cost, effects: &'static [Effect]) -> Self {
        Self {
            cost,
            effects,
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        }
    }

    /// The same, marked as a mana ability (CR 605.1).
    #[must_use]
    pub const fn mana(cost: Cost, effects: &'static [Effect]) -> Self {
        let mut parts = Self::new(cost, effects);
        parts.mana_ability = true;
        parts
    }

    /// Turns the parts into the ability.
    #[must_use]
    pub const fn build(self) -> AbilityDef {
        AbilityDef::Activated {
            cost: self.cost,
            effects: self.effects,
            target: self.target,
            timing: self.timing,
            mana_ability: self.mana_ability,
            zone: self.zone,
        }
    }
}

/// The parts of an [`AbilityDef::Triggered`], with rules defaults.
#[derive(Clone, Copy, Debug)]
pub struct TriggeredParts {
    /// What makes it trigger.
    pub trigger: Trigger,
    /// What it does.
    pub effects: &'static [Effect],
    /// What it targets, if anything.
    pub targets: Option<TargetReq>,
    /// Whether it fires at most once each turn.
    pub once_per_turn: bool,
}

impl TriggeredParts {
    /// A trigger and its effects, untargeted and firing every time.
    #[must_use]
    pub const fn new(trigger: Trigger, effects: &'static [Effect]) -> Self {
        Self {
            trigger,
            effects,
            targets: None,
            once_per_turn: false,
        }
    }

    /// Turns the parts into the ability.
    #[must_use]
    pub const fn build(self) -> AbilityDef {
        AbilityDef::Triggered {
            trigger: self.trigger,
            effects: self.effects,
            targets: self.targets,
            once_per_turn: self.once_per_turn,
        }
    }
}

/// The parts of an [`AbilityDef::Spell`], with rules defaults.
#[derive(Clone, Copy, Debug)]
pub struct SpellParts {
    /// What the spell does on resolution.
    pub effects: &'static [Effect],
    /// What it targets, if anything.
    pub targets: Option<TargetReq>,
}

impl SpellParts {
    /// A spell with these effects and no targets.
    #[must_use]
    pub const fn new(effects: &'static [Effect]) -> Self {
        Self {
            effects,
            targets: None,
        }
    }

    /// Turns the parts into the ability.
    #[must_use]
    pub const fn build(self) -> AbilityDef {
        AbilityDef::Spell {
            effects: self.effects,
            targets: self.targets,
        }
    }
}

/// The parts of an [`AbilityDef::Loyalty`], with rules defaults.
#[derive(Clone, Copy, Debug)]
pub struct LoyaltyParts {
    /// Loyalty delta: positive adds counters, negative removes them.
    pub cost: i8,
    /// What it does.
    pub effects: &'static [Effect],
    /// What it targets, if anything.
    pub target: Option<TargetSpec>,
}

impl LoyaltyParts {
    /// A loyalty ability at this cost with these effects, untargeted.
    #[must_use]
    pub const fn new(cost: i8, effects: &'static [Effect]) -> Self {
        Self {
            cost,
            effects,
            target: None,
        }
    }

    /// Turns the parts into the ability.
    #[must_use]
    pub const fn build(self) -> AbilityDef {
        AbilityDef::Loyalty {
            cost: self.cost,
            effects: self.effects,
            target: self.target,
        }
    }
}

impl SpellMode {
    /// A mode with these effects: untargeted, and costing whatever the spell
    /// costs — so a mode states only what makes it that mode.
    #[must_use]
    pub const fn new(effects: &'static [Effect]) -> Self {
        Self {
            effects,
            target: None,
            cost_override: None,
        }
    }
}

// ------------------------------------------------------------------ macros
//
// Every macro below expands to a plain struct literal with a struct-update
// tail, which is why it works in a `static` initializer: the `*Parts` types
// above are `Copy` and their constructors are `const fn`, so the whole card
// is still a compile-time constant.

/// Defines the card in this file as `pub static CARD`.
///
/// The three identity fields are mandatory and come first, in the order
/// `cargo xtask codegen` writes them — they are the card's identity and the
/// one thing a card file may never invent. Everything else is optional and
/// falls back to [`CardDef::DEFAULT`](crate::CardDef::DEFAULT).
///
/// ```ignore
/// card! {
///     index: 165,
///     oracle_id: "22e3cf1d-3559-4ce1-954c-8dc815342979",
///     scryfall_id: "0c2c39fc-b564-4ab5-833c-ff029760b7a7",
///     faces: &[face! { name: "Taiga", types: TypeSet::LAND, subtypes: SUBS }],
///     color_identity: ColorSet::from_slice(&[Color::Red, Color::Green]),
///     coverage: Coverage::Implemented,
///     abilities: &[mana_ability!(&[Effect::mana_choice(COLORS)])],
/// }
/// ```
#[macro_export]
macro_rules! card {
    (
        index: $index:literal,
        oracle_id: $oracle:literal,
        scryfall_id: $scryfall:literal,
        $($field:ident : $value:expr),* $(,)?
    ) => {
        /// The compiled definition of this card.
        pub static CARD: $crate::CardDef = $crate::CardDef {
            index: $crate::CardIndex::new($index),
            oracle_id: $oracle,
            scryfall_id: $scryfall,
            $($field: $value,)*
            ..$crate::CardDef::DEFAULT
        };
    };
}

/// One printed face, stating only what is printed on it.
///
/// Everything else comes from [`FaceDef::DEFAULT`](crate::FaceDef::DEFAULT),
/// so adding a field to `FaceDef` costs one line there instead of one line in
/// every card file.
#[macro_export]
macro_rules! face {
    ($($field:ident : $value:expr),* $(,)?) => {
        $crate::FaceDef {
            $($field: $value,)*
            ..$crate::FaceDef::DEFAULT
        }
    };
}

/// An activated ability: `activated!(cost, effects)` plus anything the card
/// says that the rules do not assume.
///
/// ```ignore
/// activated!(Cost::TAP, EFFECTS)
/// activated!(Cost::TAP, EFFECTS, target: Some(TargetSpec::Object(&ANY_CREATURE)))
/// activated!(EQUIP_COST, EFFECTS, timing: ActivationTiming::SorcerySpeed)
/// ```
#[macro_export]
macro_rules! activated {
    ($cost:expr, $effects:expr $(, $field:ident : $value:expr)* $(,)?) => {
        $crate::ActivatedParts {
            $($field: $value,)*
            ..$crate::ActivatedParts::new($cost, $effects)
        }
        .build()
    };
}

/// A mana ability (CR 605.1): does not use the stack, has no target.
///
/// The one-argument form is `{T}: Add …`, which is what almost every mana
/// ability in the pool is; pass a cost first for anything else.
///
/// ```ignore
/// mana_ability!(&[Effect::mana(ManaColor::Green, 1)])
/// mana_ability!(SACRIFICE_COST, &[Effect::mana_of_any_color()])
/// ```
#[macro_export]
macro_rules! mana_ability {
    ($effects:expr) => {
        $crate::mana_ability!($crate::Cost::TAP, $effects)
    };
    ($cost:expr, $effects:expr $(, $field:ident : $value:expr)* $(,)?) => {
        $crate::ActivatedParts {
            $($field: $value,)*
            ..$crate::ActivatedParts::mana($cost, $effects)
        }
        .build()
    };
}

/// A triggered ability: `triggered!(trigger, effects)` plus what the card
/// adds.
///
/// ```ignore
/// triggered!(Trigger::EntersBattlefield(&Filter::This), EFFECTS)
/// triggered!(Trigger::Dies(&ALLY), EFFECTS, once_per_turn: true)
/// ```
#[macro_export]
macro_rules! triggered {
    ($trigger:expr, $effects:expr $(, $field:ident : $value:expr)* $(,)?) => {
        $crate::TriggeredParts {
            $($field: $value,)*
            ..$crate::TriggeredParts::new($trigger, $effects)
        }
        .build()
    };
}

/// An instant's or sorcery's own effect (CR 608.2).
///
/// ```ignore
/// spell!(EFFECTS)
/// spell!(EFFECTS, targets: Some(TargetReq::one(&ANY_CREATURE)))
/// ```
#[macro_export]
macro_rules! spell {
    ($effects:expr $(, $field:ident : $value:expr)* $(,)?) => {
        $crate::SpellParts {
            $($field: $value,)*
            ..$crate::SpellParts::new($effects)
        }
        .build()
    };
}

/// A planeswalker's loyalty ability: `loyalty!(+1, effects)`.
///
/// ```ignore
/// loyalty!(1, EFFECTS)
/// loyalty!(-3, EFFECTS, target: Some(TargetSpec::Object(&ANY_CREATURE)))
/// ```
#[macro_export]
macro_rules! loyalty {
    ($cost:expr, $effects:expr $(, $field:ident : $value:expr)* $(,)?) => {
        $crate::LoyaltyParts {
            $($field: $value,)*
            ..$crate::LoyaltyParts::new($cost, $effects)
        }
        .build()
    };
}

/// One mode of a modal spell or trigger, stating only what makes it a mode.
///
/// ```ignore
/// mode!(DRAW_EFFECTS)
/// mode!(BOUNCE_EFFECTS, target: Some(TargetSpec::Object(&BOUNCE_TARGET)))
/// ```
#[macro_export]
macro_rules! mode {
    ($effects:expr $(, $field:ident : $value:expr)* $(,)?) => {
        $crate::SpellMode {
            $($field: $value,)*
            ..$crate::SpellMode::new($effects)
        }
    };
}

/// Everything a card file needs, in one import.
///
/// A card file used to open with eight `use` lines and
/// `#![allow(unused_imports, missing_docs)]` — the allow being necessary
/// because the generated import list was the same for every card whether or
/// not the card used all of it, and because `pub static CARD` carried no doc
/// comment. Both are gone: this is one glob, and [`card!`](crate::card)
/// documents the static it defines.
pub mod prelude {
    pub use crate::ability::{
        AbilityDef, ActivationCondition, ActivationTiming, ActivationZone, CopyMod, SpellMode,
        StepKind, Trigger, TriggerEventKind,
    };
    pub use crate::build::{ActivatedParts, LoyaltyParts, SpellParts, TriggeredParts};
    pub use crate::cost::{AltCondition, AlternativeCost, Cost, CostPart, CostReduction};
    pub use crate::effect::{
        Amount, CounterKind, Effect, Find, ManaRestriction, ManaSource, PlayerRel, SearchDest,
        SpendRider, TargetReq, TargetSpec, TokenDef, ZoneSel,
    };
    pub use crate::filter::{Filter, ZoneRef};
    pub use crate::static_ability::{
        Duration, LAYERS, Layer, Modifier, ReplacementRule, StaticAbility,
    };
    pub use crate::{
        ALL_MANA_COLORS, ANY_COLOR_MANA, CardDef, CommanderRule, Coverage, EnterModifier, FaceDef,
        KeywordSet, PartnerKind,
    };
    pub use crate::{activated, card, face, loyalty, mana_ability, mode, spell, triggered};
    pub use baylee_core::color::{Color, ColorSet};
    pub use baylee_core::ids::{CardIndex, SubtypeId};
    pub use baylee_core::mana::{ManaColor, ManaCost};
    pub use baylee_core::types::{SupertypeSet, TypeSet};
}
