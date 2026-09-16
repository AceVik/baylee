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
//! - `timing: InstantSpeed` — CR 117.1b: an activated ability may be
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

use crate::ability::{
    AbilityDef, ActivationCondition, ActivationTiming, ActivationZone, SpellMode, Trigger,
};
use crate::cost::Cost;
use crate::effect::{Effect, TargetReq, TargetSpec};
use crate::filter::Filter;
use crate::static_ability::{Modifier, StaticAbility};

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
    /// A precondition the card states (metalcraft, a verge land's
    /// Plains/Swamp check, a class level).
    ///
    /// `None` is the rules default — an ability with no printed condition
    /// may be activated whenever its cost can be paid — and it is also what
    /// decides *which ability this is*: [`build`](Self::build) produces an
    /// [`AbilityDef::ActivatedConditional`] exactly when the card named a
    /// condition. The two variants are otherwise the same six fields, and
    /// having one door into both is what stops a card being written as the
    /// unconditional twin by omission.
    pub condition: Option<ActivationCondition>,
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
            condition: None,
        }
    }

    /// The same, marked as a mana ability (CR 605.1).
    #[must_use]
    pub const fn mana(cost: Cost, effects: &'static [Effect]) -> Self {
        let mut parts = Self::new(cost, effects);
        parts.mana_ability = true;
        parts
    }

    /// Turns the parts into the ability — conditional exactly when the card
    /// named a [`condition`](Self::condition).
    #[must_use]
    pub const fn build(self) -> AbilityDef {
        match self.condition {
            None => AbilityDef::Activated {
                cost: self.cost,
                effects: self.effects,
                target: self.target,
                timing: self.timing,
                mana_ability: self.mana_ability,
                zone: self.zone,
            },
            Some(condition) => AbilityDef::ActivatedConditional {
                cost: self.cost,
                effects: self.effects,
                target: self.target,
                timing: self.timing,
                mana_ability: self.mana_ability,
                zone: self.zone,
                condition,
            },
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

/// The parts of an [`AbilityDef::ModalTriggered`], with rules defaults.
#[derive(Clone, Copy, Debug)]
pub struct ModalTriggeredParts {
    /// What makes it trigger.
    pub trigger: Trigger,
    /// The modes to choose from (CR 700.2).
    pub modes: &'static [SpellMode],
    /// Whether it fires at most once each turn.
    pub once_per_turn: bool,
}

impl ModalTriggeredParts {
    /// A modal trigger that fires every time, which is CR 603.2: a triggered
    /// ability triggers whenever its event happens, and the cards that fire
    /// once a turn say so on their face.
    #[must_use]
    pub const fn new(trigger: Trigger, modes: &'static [SpellMode]) -> Self {
        Self {
            trigger,
            modes,
            once_per_turn: false,
        }
    }

    /// Turns the parts into the ability.
    #[must_use]
    pub const fn build(self) -> AbilityDef {
        AbilityDef::ModalTriggered {
            trigger: self.trigger,
            modes: self.modes,
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
    pub targets: Option<crate::effect::TargetReq>,
}

impl LoyaltyParts {
    /// A loyalty ability at this cost with these effects, untargeted.
    #[must_use]
    pub const fn new(cost: i8, effects: &'static [Effect]) -> Self {
        Self {
            cost,
            effects,
            targets: None,
        }
    }

    /// Turns the parts into the ability.
    #[must_use]
    pub const fn build(self) -> AbilityDef {
        AbilityDef::Loyalty {
            cost: self.cost,
            effects: self.effects,
            targets: self.targets,
        }
    }
}

/// The parts of an [`AbilityDef::Static`], with the layer derived.
///
/// The layer is not a field here, because it is not a decision: it follows
/// from the modifier, and [`Modifier::layer`] is the table — measured over
/// the whole pool, where 25 modifiers appeared on 25 layers with no
/// exception. A card says *what changes* and *to what*; CR 613.1 says when.
#[derive(Clone, Copy, Debug)]
pub struct StaticParts {
    /// Which objects are affected.
    pub filter: Filter,
    /// What changes.
    pub modifier: Modifier,
}

impl StaticParts {
    /// A continuous ability applying `modifier` to everything matching
    /// `filter`, on the layer the modifier belongs to.
    #[must_use]
    pub const fn new(filter: Filter, modifier: Modifier) -> Self {
        Self { filter, modifier }
    }

    /// Turns the parts into the ability.
    #[must_use]
    pub const fn build(self) -> AbilityDef {
        AbilityDef::Static(StaticAbility {
            layer: self.modifier.layer(),
            filter: self.filter,
            modifier: self.modifier,
        })
    }
}

/// The parts of an [`AbilityDef::SagaChapter`], with rules defaults.
#[derive(Clone, Copy, Debug)]
pub struct SagaChapterParts {
    /// Chapter number (1-based).
    pub chapter: u8,
    /// What the chapter does.
    pub effects: &'static [Effect],
    /// What it targets, if anything.
    pub targets: Option<TargetReq>,
}

impl SagaChapterParts {
    /// A chapter and its effects, untargeted — a saga chapter targets only
    /// when the printed chapter says "target".
    #[must_use]
    pub const fn new(chapter: u8, effects: &'static [Effect]) -> Self {
        Self {
            chapter,
            effects,
            targets: None,
        }
    }

    /// Turns the parts into the ability.
    #[must_use]
    pub const fn build(self) -> AbilityDef {
        AbilityDef::SagaChapter {
            chapter: self.chapter,
            effects: self.effects,
            targets: self.targets,
        }
    }
}

/// What an equip ability targets: "target creature you control"
/// (CR 702.6a).
///
/// The keyword names it, not the card, so it is spelled once here instead of
/// in a local `static` per Equipment — and named twice inside each of those,
/// once for the target requirement and once for what gets attached.
pub const EQUIP_TARGET: TargetSpec = TargetSpec::Object(&Filter::YOUR_CREATURE);

impl SpellMode {
    /// A mode with these effects: untargeted, and costing whatever the spell
    /// costs — so a mode states only what makes it that mode.
    #[must_use]
    pub const fn new(effects: &'static [Effect]) -> Self {
        Self {
            effects,
            targets: None,
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
/// `index` is a **path** and not a number, which is the rule rather than a
/// convenience: a card *names* its index, it does not compute one. The ledger
/// froze a constant for every card there is
/// ([`index`](baylee_core::generated::index), all of them in one namespace),
/// so `index = index::TAIGA` is the only spelling that says which card this
/// is — `index = 240` says only that somebody typed a number, and a number
/// typed one digit wrong names a different card that compiles. The fragment
/// specifier is what enforces it: `240` and `CardIndex::new(240)` are both
/// rejected by the matcher, before the type system is reached.
///
/// ```ignore
/// card!(
///     index = index::TAIGA,
///     oracle_id = "22e3cf1d-3559-4ce1-954c-8dc815342979",
///     scryfall_id = "0c2c39fc-b564-4ab5-833c-ff029760b7a7",
///     faces = &[face!(name = "Taiga", types = TypeSet::LAND, subtypes = SUBS)],
///     color_identity = ColorSet::from_slice(&[Color::Red, Color::Green]),
///     coverage = Coverage::Implemented,
///     abilities = &[mana_ability!(&[Effect::mana_choice(COLORS)])],
/// );
/// ```
#[macro_export]
macro_rules! card {
    (
        index = $index:path,
        oracle_id = $oracle:literal,
        scryfall_id = $scryfall:literal,
        $($field:ident = $value:expr),* $(,)?
    ) => {
        /// The compiled definition of this card.
        pub static CARD: $crate::CardDef = $crate::CardDef {
            index: $index,
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
    ($($field:ident = $value:expr),* $(,)?) => {
        $crate::FaceDef {
            $($field: $value,)*
            ..$crate::FaceDef::DEFAULT
        }
    };
}

/// A filter, written the way the card says it: adjectives, then the noun.
///
/// ```ignore
/// f!(CREATURE)                       // Filter::CREATURE itself
/// f!(owned CREATURE)                 // a creature you own
/// f!(another nontoken CREATURE)      // another nontoken creature
/// f!(your Filter::HasSubtype(ally::ALLY))   // an Ally you control
/// ```
///
/// # Why a macro and not a function
///
/// A `const fn` cannot do this. Combining filters means building a
/// `&'static [Filter]` from its *parameters*, and a slice built from a
/// parameter inside a `const fn` cannot be promoted to `'static` (E0716). A
/// macro expands in the caller's `static` or `const`, where the slice
/// promotes like any other literal — which is also why a card needs no local
/// `static` for a filter it mentions once.
///
/// # What it may say
///
/// The adjective list is **closed**, and every entry is one nullary
/// [`Filter`] variant (or `Not` of one): `your`, `opponents`, `owned`,
/// `another`, `token`, `nontoken`, `tapped`, `untapped`, `attacking`,
/// `colorless`. The noun is a bare identifier resolved as `Filter::$noun`
/// (`CREATURE`, `LAND`, `BASIC_LAND`, `NONLAND`, `INSTANT_OR_SORCERY`, …) or
/// any `Filter` expression.
///
/// A bare identifier is therefore always a constant **on `Filter`** and never
/// a name in the card's own file: `f!(your AIS_SPELL)` expands to
/// `Filter::AIS_SPELL` and fails to compile, whatever `AIS_SPELL` the card
/// declared above it. A card-local filter is composed by hand.
///
/// Anything that takes an argument stays a variant —
/// `Filter::HasColor(ColorSet::of(Color::Green))`, `Filter::CmcAtMost(1)` —
/// because the point is a shorter spelling of the filters we already have,
/// not a second filter language. `f!` can say nothing `Filter` cannot.
///
/// The expansion is `Filter::And(&[noun, adjectives…])` in written order, so
/// `f!(your CREATURE)` is the same **data** as [`Filter::YOUR_CREATURE`] and
/// not merely the same meaning — which is what makes replacing a
/// hand-written `static` with it provably free, and is asserted by
/// `the_filter_macro_spells_the_constants_it_replaces`.
///
/// That equality is also the one place `f!` is the **wrong** spelling. Where
/// a constant already carries the combination, the constant is what a card
/// writes: a filter with two names is the duplication the named predicates
/// were collected to end, and `Filter::YOUR_CREATURE` had nought uses in the
/// pool while twenty-six files spelled a creature out by hand. `f!` is for
/// the combination no constant carries — which is most of them, a constant
/// earning its place by being wanted twice.
#[macro_export]
macro_rules! f {
    ($($spelling:tt)+) => { $crate::__f_adjectives!([] $($spelling)+) };
}

/// The accumulator behind [`f!`](crate::f).
///
/// Separate because a muncher cannot be its own entry point: an arm that
/// re-enters `f!` would match `f!`'s own catch-all and recurse for ever on a
/// misspelled adjective, where this reports an unmatched rule.
#[doc(hidden)]
#[macro_export]
macro_rules! __f_adjectives {
    ([$($acc:expr),*] your $($rest:tt)+) => {
        $crate::__f_adjectives!([$($acc,)* $crate::Filter::ControlledByYou] $($rest)+)
    };
    ([$($acc:expr),*] opponents $($rest:tt)+) => {
        $crate::__f_adjectives!([$($acc,)* $crate::Filter::ControlledByOpponent] $($rest)+)
    };
    ([$($acc:expr),*] owned $($rest:tt)+) => {
        $crate::__f_adjectives!([$($acc,)* $crate::Filter::OwnedByYou] $($rest)+)
    };
    ([$($acc:expr),*] another $($rest:tt)+) => {
        $crate::__f_adjectives!([$($acc,)* $crate::Filter::Another] $($rest)+)
    };
    ([$($acc:expr),*] token $($rest:tt)+) => {
        $crate::__f_adjectives!([$($acc,)* $crate::Filter::IsToken] $($rest)+)
    };
    ([$($acc:expr),*] nontoken $($rest:tt)+) => {
        $crate::__f_adjectives!([$($acc,)* $crate::Filter::Not(&$crate::Filter::IsToken)] $($rest)+)
    };
    ([$($acc:expr),*] tapped $($rest:tt)+) => {
        $crate::__f_adjectives!([$($acc,)* $crate::Filter::Tapped] $($rest)+)
    };
    ([$($acc:expr),*] untapped $($rest:tt)+) => {
        $crate::__f_adjectives!([$($acc,)* $crate::Filter::Untapped] $($rest)+)
    };
    ([$($acc:expr),*] attacking $($rest:tt)+) => {
        $crate::__f_adjectives!([$($acc,)* $crate::Filter::Attacking] $($rest)+)
    };
    ([$($acc:expr),*] colorless $($rest:tt)+) => {
        $crate::__f_adjectives!([$($acc,)* $crate::Filter::IsColorless] $($rest)+)
    };
    // The noun, with nothing in front of it: the constant itself, so
    // `f!(CREATURE)` is `Filter::CREATURE` and not a one-element `And`.
    ([] $noun:ident) => { $crate::Filter::$noun };
    ([$($acc:expr),+] $noun:ident) => {
        $crate::Filter::And(&[$crate::Filter::$noun, $($acc),+])
    };
    ([] $noun:expr) => { $noun };
    ([$($acc:expr),+] $noun:expr) => {
        $crate::Filter::And(&[$noun, $($acc),+])
    };
}

/// A cost, written the way the card prints it: mana first, then the rest.
///
/// ```ignore
/// cost!("{2}")                                  // {2}: …
/// cost!(TapSelf)                                // {T}: …  (= Cost::TAP)
/// cost!("{1}{G}", TapSelf, SacrificeSelf)       // {1}{G}, {T}, Sacrifice this: …
/// cost!(TapSelf, SacrificeSelf, PayLife(1))     // a fetchland
/// cost!("{3}", DiscardSelf)                     // cycling
/// cost!(TapSelf, RemoveCounterSelf { kind: CounterKind::Charge, n: 1 })
/// ```
///
/// The mana string is the one the card prints and goes through
/// [`mana!`](baylee_core::mana), so a cost is spelled once and in the same
/// notation as a face. A part is named without its `CostPart::` prefix,
/// because the prefix is the same word three times on a fetchland and is not
/// what a reader is checking.
///
/// A part with **named fields** is written with its braces, which is the last
/// line above and the reason this macro reads three shapes rather than two.
/// The counter costs are a family — one kind, one count, and more of them
/// coming — and `RemoveCounterSelf(CounterKind::Charge, 1)` would put a bare
/// `1` in front of a reader with nothing saying what it counts. The braces
/// cost one token and say it.
///
/// [`Cost::FREE`](crate::Cost::FREE) is the empty cost; there is nothing for
/// `cost!()` to read, so it is not a form.
#[macro_export]
macro_rules! cost {
    ($mana:literal $(, $part:ident $(($($arg:expr),* $(,)?))? $({$($f:ident: $v:expr),* $(,)?})? )* $(,)?) => {
        $crate::Cost {
            mana: $crate::mana!($mana),
            parts: &[$($crate::CostPart::$part $(($($arg),*))? $({$($f: $v),*})?),*],
        }
    };
    ($($part:ident $(($($arg:expr),* $(,)?))? $({$($f:ident: $v:expr),* $(,)?})? ),+ $(,)?) => {
        $crate::Cost {
            mana: $crate::ManaCost::ZERO,
            parts: &[$($crate::CostPart::$part $(($($arg),*))? $({$($f: $v),*})?),+],
        }
    };
}

/// An activated ability: `activated!(cost, effects)` plus anything the card
/// says that the rules do not assume.
///
/// ```ignore
/// activated!(Cost::TAP, EFFECTS)
/// activated!(Cost::TAP, EFFECTS, target = Some(TargetSpec::Object(&ANY_CREATURE)))
/// activated!(EQUIP_COST, EFFECTS, timing = ActivationTiming::SorcerySpeed)
/// activated!(Cost::TAP, EFFECTS, condition = Some(ActivationCondition::ControlCount(&Filter::ARTIFACT, 3)))
/// ```
///
/// `condition =` is what makes an [`AbilityDef::ActivatedConditional`], so a
/// metalcraft ability is this macro plus one line rather than a
/// seven-field literal of its own.
#[macro_export]
macro_rules! activated {
    ($cost:expr, $effects:expr $(, $field:ident = $value:expr)* $(,)?) => {
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
    ($cost:expr, $effects:expr $(, $field:ident = $value:expr)* $(,)?) => {
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
/// triggered!(Trigger::ETB, EFFECTS)
/// triggered!(Trigger::Dies(&ALLY), EFFECTS, once_per_turn = true)
/// ```
#[macro_export]
macro_rules! triggered {
    ($trigger:expr, $effects:expr $(, $field:ident = $value:expr)* $(,)?) => {
        $crate::TriggeredParts {
            $($field: $value,)*
            ..$crate::TriggeredParts::new($trigger, $effects)
        }
        .build()
    };
}

/// A triggered ability the controller picks a mode of:
/// `modal_triggered!(trigger, modes)` plus what the card adds.
///
/// ```ignore
/// modal_triggered!(Trigger::ETB, MODES)
/// modal_triggered!(Trigger::Attacks(&Filter::This), MODES, once_per_turn = true)
/// ```
#[macro_export]
macro_rules! modal_triggered {
    ($trigger:expr, $modes:expr $(, $field:ident = $value:expr)* $(,)?) => {
        $crate::ModalTriggeredParts {
            $($field: $value,)*
            ..$crate::ModalTriggeredParts::new($trigger, $modes)
        }
        .build()
    };
}

/// An instant's or sorcery's own effect (CR 608.2).
///
/// ```ignore
/// spell!(EFFECTS)
/// spell!(EFFECTS, targets = Some(TargetReq::one(&ANY_CREATURE)))
/// ```
#[macro_export]
macro_rules! spell {
    ($effects:expr $(, $field:ident = $value:expr)* $(,)?) => {
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
/// loyalty!(-3, EFFECTS, targets = Some(TargetReq::one(TargetSpec::Object(&ANY_CREATURE))))
/// loyalty!(1, EFFECTS, targets = Some(TargetReq::up_to_one(TargetSpec::Object(&ANY_ARTIFACT))))
/// ```
#[macro_export]
macro_rules! loyalty {
    ($cost:expr, $effects:expr $(, $field:ident = $value:expr)* $(,)?) => {
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
/// mode!(BOUNCE_EFFECTS, targets = Some(TargetReq::one(TargetSpec::Object(&BOUNCE_TARGET))))
/// ```
#[macro_export]
macro_rules! mode {
    ($effects:expr $(, $field:ident = $value:expr)* $(,)?) => {
        $crate::SpellMode {
            $($field: $value,)*
            ..$crate::SpellMode::new($effects)
        }
    };
}

/// A static (continuous) ability: what changes, and to what.
///
/// ```ignore
/// static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(2, 2))
/// static_ability!(Filter::Any, Modifier::AddType(TypeSet::ARTIFACT))
/// ```
///
/// There is no layer argument: [`Modifier::layer`] derives it, which is the
/// whole reason this macro can be two words. Twenty-nine abilities in the
/// pool wrote the three-field literal by hand, and a literal is free to
/// disagree with the rules — `static_ability!` cannot.
///
/// The filter is a [`Filter`] **by value**, not a reference: a
/// `StaticAbility` owns its filter, where an `Effect::CreateContinuousEffect`
/// borrows one.
#[macro_export]
macro_rules! static_ability {
    ($filter:expr, $modifier:expr $(,)?) => {
        $crate::StaticParts::new($filter, $modifier).build()
    };
}

/// One chapter of a saga (CR 714): `chapter!(1, effects)`.
///
/// ```ignore
/// chapter!(1, &[Effect::scry(1)])
/// chapter!(3, EFFECTS, targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))
/// ```
#[macro_export]
macro_rules! chapter {
    ($chapter:expr, $effects:expr $(, $field:ident = $value:expr)* $(,)?) => {
        $crate::SagaChapterParts {
            $($field: $value,)*
            ..$crate::SagaChapterParts::new($chapter, $effects)
        }
        .build()
    };
}

/// Equip (CR 702.6): `equip!("{2}")`.
///
/// Every part of an equip ability except the cost comes from the keyword's
/// own definition — sorcery speed (CR 702.6a), "target creature you
/// control" (CR 702.6a), and attaching this permanent to it — so the cost is
/// the only thing a card prints and the only thing this takes. The four
/// Equipment in the pool each wrote it out as eight lines with the target
/// named twice, over a local `static` that was the same filter each time.
///
/// ```ignore
/// equip!("{2}")        // Equip {2}
/// equip!(Cost::FREE)   // Equip {0} — not the same data as `cost!("{0}")`
/// ```
#[macro_export]
macro_rules! equip {
    ($mana:literal) => {
        $crate::equip!($crate::cost!($mana))
    };
    ($cost:expr) => {
        $crate::ActivatedParts {
            target: Some($crate::EQUIP_TARGET),
            timing: $crate::ActivationTiming::SorcerySpeed,
            ..$crate::ActivatedParts::new(
                $cost,
                &[$crate::Effect::AttachSelf {
                    target: $crate::EQUIP_TARGET,
                }],
            )
        }
        .build()
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
    pub use crate::build::{
        ActivatedParts, EQUIP_TARGET, LoyaltyParts, ModalTriggeredParts, SagaChapterParts,
        SpellParts, StaticParts, TriggeredParts,
    };
    pub use crate::cost::{AltCondition, AlternativeCost, Cost, CostPart, CostReduction};
    /// The ids assigned to the counters that carry no rule of their own.
    ///
    /// The *module*, for [`index`]'s reason one import down: a card writes
    /// `counters::DEPLETION`, which names the printed word, where a glob
    /// would put every counter this pool has ever needed into the namespace
    /// of every card file.
    pub use crate::counters;
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
    pub use crate::{
        activated, card, chapter, cost, equip, f, face, loyalty, mana_ability, modal_triggered,
        mode, spell, static_ability, triggered,
    };
    pub use baylee_core::color::{Color, ColorSet};
    /// Every card's `CardIndex` under the name the ledger froze for it.
    ///
    /// The *module*, not its contents: a card writes `index::TAIGA`, which
    /// reads as the sentence it is, where a glob would put 33694 bare
    /// constants into the namespace every card file opens with.
    pub use baylee_core::generated::index;
    pub use baylee_core::ids::{CardIndex, SubtypeId};
    /// The one thing every card spells out that the prelude did not carry.
    ///
    /// `mana!` is `#[macro_export]`ed from `baylee-core`, so 388 files wrote
    /// `baylee_core::mana!("{1}{W}")` while importing a prelude whose whole
    /// purpose is that a card names one crate. Re-exported here, a cost is
    /// spelled the way every other piece of a card is.
    pub use baylee_core::mana;
    pub use baylee_core::mana::{ManaColor, ManaCost};
    pub use baylee_core::types::{SupertypeSet, TypeSet};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KeywordSet;
    use crate::ability::ActivationCondition;
    use crate::cost::CostPart;
    use crate::effect::{Amount, CounterKind};
    use crate::static_ability::Layer;

    /// Every macro here has to work in a `static` initializer, which is the
    /// one thing a unit test cannot assert at runtime: it either compiles or
    /// it does not.
    ///
    /// The nesting is deliberate — an inline filter behind a reference,
    /// inside a slice, inside a struct, inside a slice, inside a `static` —
    /// because that is the shape a real card writes and the shape const
    /// promotion refuses when the same expression is built inside a
    /// `const fn` from a parameter (E0716).
    static IN_A_STATIC: [AbilityDef; 4] = [
        static_ability!(f!(your CREATURE), Modifier::ModifyPT(1, 1)),
        chapter!(1, &[Effect::scry(1)]),
        equip!("{2}"),
        activated!(
            Cost::TAP,
            &[Effect::draw(1)],
            condition = Some(ActivationCondition::ControlCount(&Filter::ARTIFACT, 3)),
        ),
    ];

    /// `activated!` builds the twin the card asked for, and nothing else
    /// about the ability changes with it.
    ///
    /// The two variants carry the same six fields and differ only in the
    /// seventh, which is exactly why they are easy to confuse: six readers
    /// across the engine, the client and these lints once matched
    /// `AbilityDef::Activated` alone and silently skipped every conditional
    /// ability in the pool. One `build` reaching both is what stops a card
    /// being written as the wrong one by omission — and this is the test
    /// that it does.
    #[test]
    fn a_condition_is_the_only_thing_that_makes_the_conditional_twin() {
        const EFFECTS: &[Effect] = &[Effect::DrawCards {
            amount: Amount::Fixed(1),
        }];
        const METALCRAFT: ActivationCondition =
            ActivationCondition::ControlCount(&Filter::ARTIFACT, 3);

        assert_eq!(
            activated!(Cost::TAP, EFFECTS),
            AbilityDef::Activated {
                cost: Cost::TAP,
                effects: EFFECTS,
                target: None,
                timing: ActivationTiming::InstantSpeed,
                mana_ability: false,
                zone: ActivationZone::Battlefield,
            },
            "no condition is the plain ability, at the rules defaults"
        );
        assert_eq!(
            activated!(Cost::TAP, EFFECTS, condition = Some(METALCRAFT)),
            AbilityDef::ActivatedConditional {
                cost: Cost::TAP,
                effects: EFFECTS,
                target: None,
                timing: ActivationTiming::InstantSpeed,
                mana_ability: false,
                zone: ActivationZone::Battlefield,
                condition: METALCRAFT,
            },
            "the condition moves it to the twin and changes nothing else"
        );
    }

    /// A mana ability with a condition is still a mana ability.
    ///
    /// Mox Opal is the card: metalcraft on an ability that adds mana. It was
    /// not expressible before — `mana_ability!` and the conditional variant
    /// had no door between them — so nothing in the pool exercises this
    /// combination and the engine's handling of it is **unverified**. The
    /// test pins what the DSL builds, not what the engine does with it.
    #[test]
    fn a_conditional_ability_can_still_be_a_mana_ability() {
        const EFFECTS: &[Effect] = &[Effect::mana(baylee_core::mana::ManaColor::White, 1)];
        const METALCRAFT: ActivationCondition =
            ActivationCondition::ControlCount(&Filter::ARTIFACT, 3);

        let built = mana_ability!(Cost::TAP, EFFECTS, condition = Some(METALCRAFT));
        match built {
            AbilityDef::ActivatedConditional {
                mana_ability,
                condition,
                ..
            } => {
                assert!(mana_ability, "CR 605.1 does not stop at a precondition");
                assert_eq!(condition, METALCRAFT);
            }
            other => panic!("expected the conditional twin, got {other:?}"),
        }
    }

    /// `equip!` is the eight-line literal the four Equipment in the pool
    /// each wrote by hand.
    ///
    /// This is the equivalence the migration rests on, asserted before a
    /// single card is touched: the macro's expansion and the spelling it
    /// replaces are the same `AbilityDef`, not merely the same meaning.
    /// `CREATURE_YOU_CONTROL` below is the local `static` those cards
    /// declared, copied verbatim.
    #[test]
    fn equip_is_the_literal_the_equipment_wrote_out() {
        static CREATURE_YOU_CONTROL: Filter =
            Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]);
        static BY_HAND: AbilityDef = AbilityDef::Activated {
            cost: Cost {
                mana: baylee_core::mana!("{2}"),
                parts: &[],
            },
            effects: &[Effect::AttachSelf {
                target: TargetSpec::Object(&CREATURE_YOU_CONTROL),
            }],
            target: Some(TargetSpec::Object(&CREATURE_YOU_CONTROL)),
            timing: ActivationTiming::SorcerySpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        };

        assert_eq!(equip!("{2}"), BY_HAND);
        assert_ne!(
            equip!(Cost::FREE),
            equip!("{0}"),
            "Equip {{0}} is a cost with no mana cost at all, which is not the \
             same data as a mana cost of zero generic — Lightning Greaves has \
             to say `Cost::FREE`"
        );
    }

    /// `static_ability!` derives the layer, and derives the one the pool
    /// already used.
    ///
    /// Four modifiers, four layers, taken from cards in the pool: Mycosynth
    /// Lattice (types), Swiftfoot Boots (keywords), an anthem (7c) and a
    /// Sword's buff. `Modifier::layer` is checked pool-wide by
    /// `baylee_cards::lints`; what this pins is that the macro reads it.
    #[test]
    fn a_static_ability_derives_its_own_layer() {
        let cases = [
            (
                Modifier::AddType(baylee_core::types::TypeSet::ARTIFACT),
                Layer::Type,
            ),
            (Modifier::AddKeyword(KeywordSet::HASTE), Layer::Ability),
            (Modifier::ModifyPT(2, 2), Layer::PtModify),
            (Modifier::SetPT(0, 0), Layer::PtSet),
        ];
        for (modifier, want) in cases {
            let AbilityDef::Static(sa) = static_ability!(Filter::Any, modifier) else {
                panic!("static_ability! must build AbilityDef::Static");
            };
            assert_eq!(sa.layer, want, "{modifier:?}");
            assert_eq!(sa.modifier, modifier);
            assert_eq!(sa.filter, Filter::Any);
        }
    }

    /// Every modifier that adds or removes an *ability* derives layer 6.
    ///
    /// CR 613.1f: "Layer 6: Ability-adding effects, keyword counters,
    /// ability-removing effects, and effects that say an object can't have
    /// an ability are applied." That sentence is the whole membership rule,
    /// so the list below is read off it and not off the function under test.
    ///
    /// Two of these used to derive [`Layer::Text`], which is layer 3 and
    /// belongs to text-changing effects (CR 613.1c). `ProtectionFrom` grants
    /// a static ability (CR 702.16a opens "Protection is a static ability")
    /// and `GrantTriggered` grants a triggered one, so both add an ability
    /// and neither changes a word of text. The pool reaches every layer
    /// through `Modifier::layer`, so nothing but this assertion stands
    /// between that arm and the five abilities it decides.
    #[test]
    fn granting_an_ability_is_layer_six() {
        let adds_or_removes_an_ability = [
            Modifier::AddKeyword(KeywordSet::HASTE),
            Modifier::RemoveKeyword(KeywordSet::HASTE),
            Modifier::LoseKeywords,
            Modifier::GrantsFlashback,
            Modifier::ProtectionFrom(&Filter::ARTIFACT),
            Modifier::GrantTriggered {
                trigger: Trigger::ETB,
                effects: &[],
                target: None,
            },
        ];
        for modifier in adds_or_removes_an_ability {
            assert_eq!(
                modifier.layer(),
                Layer::Ability,
                "{modifier:?} adds or removes an ability, which CR 613.1f puts in layer 6"
            );
        }
    }

    /// `chapter!` numbers the chapter and targets nothing unless the card
    /// says "target" (CR 714).
    #[test]
    fn a_saga_chapter_targets_only_when_it_says_so() {
        const EFFECTS: &[Effect] = &[Effect::Scry {
            amount: Amount::Fixed(1),
        }];
        assert_eq!(
            chapter!(2, EFFECTS),
            AbilityDef::SagaChapter {
                chapter: 2,
                effects: EFFECTS,
                targets: None,
            }
        );
        let req = TargetReq::one(TargetSpec::Object(&Filter::CREATURE));
        assert_eq!(
            chapter!(3, EFFECTS, targets = Some(req)),
            AbilityDef::SagaChapter {
                chapter: 3,
                effects: EFFECTS,
                targets: Some(req),
            }
        );
    }

    /// The filter macro spells the composite constants it is meant to
    /// replace — the same data, not merely the same meaning.
    ///
    /// This is the assertion the filter half of the migration rests on. A
    /// card file that swaps a hand-written `static CREATURE_YOU_CONTROL` for
    /// `f!(your CREATURE)` moves no byte of the compiled pool, and that has
    /// to be provable before 159 local statics are touched — the order of
    /// the clauses inside an `And` is part of the data, so "a creature you
    /// control" written the other way round would be a different filter that
    /// happens to match the same objects.
    #[test]
    fn the_filter_macro_spells_the_constants_it_replaces() {
        assert_eq!(f!(your CREATURE), Filter::YOUR_CREATURE);
        assert_eq!(f!(opponents CREATURE), Filter::OPPONENT_CREATURE);
        assert_eq!(f!(another CREATURE), Filter::ANOTHER_CREATURE);
        assert_eq!(f!(nontoken CREATURE), Filter::NONTOKEN_CREATURE);
        assert_eq!(f!(attacking CREATURE), Filter::ATTACKING_CREATURE);
        // The two the pool had written the other way round. They are here
        // because this assertion is what decided their order: a constant
        // adjective first would have been the byte-identical spelling and
        // would have made `f!(your LAND)` a *different* filter from
        // `Filter::YOUR_LAND`, which is the duplication both exist to end.
        assert_eq!(f!(your LAND), Filter::YOUR_LAND);
        assert_eq!(f!(your BASIC_LAND), Filter::YOUR_BASIC_LAND);
        assert_eq!(f!(your ARTIFACT), Filter::YOUR_ARTIFACT);
        assert_eq!(
            f!(your another CREATURE),
            Filter::ANOTHER_CREATURE_YOU_CONTROL
        );
        assert_eq!(
            f!(CREATURE),
            Filter::CREATURE,
            "a bare noun is the constant itself, not a one-element And"
        );
    }

    /// Adjectives stack in written order, and a noun may be any filter.
    #[test]
    fn the_filter_macro_reads_left_to_right() {
        assert_eq!(
            f!(another nontoken CREATURE),
            Filter::And(&[
                Filter::CREATURE,
                Filter::Another,
                Filter::Not(&Filter::IsToken),
            ])
        );
        assert_eq!(
            f!(your Filter::CmcAtMost(1)),
            Filter::And(&[Filter::CmcAtMost(1), Filter::ControlledByYou]),
            "the noun may be any Filter expression, which is where f! stops"
        );
    }

    /// Two adjectives, one English phrase, two different filters.
    ///
    /// `f!` stacks adjectives in the order they are written, so "another
    /// creature you control" has two spellings and they are not the same
    /// data: `And(&[CREATURE, ControlledByYou, Another])` against
    /// `And(&[CREATURE, Another, ControlledByYou])`. Nothing would catch a
    /// card reaching for the second one — both compile, both match the same
    /// objects, and the only reader that can tell them apart is
    /// `state::filter_hash`. So the constant picks one and this pins the
    /// hazard: it is the reason [`Filter::ANOTHER_CREATURE_YOU_CONTROL`]
    /// exists as a name rather than as a macro call in three card files.
    ///
    /// The one-adjective constants have no such choice to make, which is why
    /// this is the first test in the file that asserts an inequality.
    #[test]
    fn the_filter_macro_cannot_be_trusted_to_spell_an_order() {
        assert_ne!(f!(your another CREATURE), f!(another your CREATURE));
        assert_eq!(
            f!(your another CREATURE),
            Filter::ANOTHER_CREATURE_YOU_CONTROL,
            "the constant is the `your`-first spelling, like ANOTHER_ALLY"
        );
    }

    /// `cost!` reads a part written with braces, in either of its two forms
    /// and beside the other two spellings.
    ///
    /// The third shape was added for the counter costs, and it is the one
    /// that could have been left out by mistake: a part with named fields is
    /// a *third* thing after "a word" and "a word with parentheses", and a
    /// macro that reads two of the three fails at the call site with
    /// "no rules expected this token", which reads as though the variant is
    /// the problem. So both arms are exercised — with mana and without — and
    /// so is the mixture, because the arms repeat the optional groups per
    /// part and a mistake there only shows when two parts disagree.
    #[test]
    fn a_cost_part_with_named_fields_is_written_with_its_braces() {
        const CHARGE: CostPart = CostPart::RemoveCounterSelf {
            kind: CounterKind::Charge,
            n: 1,
        };

        assert_eq!(
            cost!(
                TapSelf,
                RemoveCounterSelf {
                    kind: CounterKind::Charge,
                    n: 1
                }
            )
            .parts,
            &[CostPart::TapSelf, CHARGE]
        );
        assert_eq!(
            cost!(
                "{1}",
                RemoveCounterSelf {
                    kind: CounterKind::Charge,
                    n: 1
                },
                PayLife(2)
            )
            .parts,
            &[CHARGE, CostPart::PayLife(2)],
            "the braced part sits between two spellings that are not braced",
        );
        assert_eq!(
            cost!(
                "{1}",
                RemoveCounterSelf {
                    kind: CounterKind::Charge,
                    n: 1
                },
            )
            .mana,
            crate::mana!("{1}"),
            "and a trailing comma after it is still a trailing comma",
        );
    }

    /// The `static` above is the point of the whole module; this reads it so
    /// the compiler cannot decide it is dead code.
    #[test]
    fn the_macros_are_usable_in_a_static() {
        assert_eq!(IN_A_STATIC.len(), 4);
        assert!(matches!(IN_A_STATIC[0], AbilityDef::Static(_)));
        assert!(matches!(
            IN_A_STATIC[1],
            AbilityDef::SagaChapter { chapter: 1, .. }
        ));
        assert!(matches!(IN_A_STATIC[2], AbilityDef::Activated { .. }));
        assert!(matches!(
            IN_A_STATIC[3],
            AbilityDef::ActivatedConditional { .. }
        ));
    }

    /// Every `pub const` on [`Filter`] is named in `docs/card-dsl.md`.
    ///
    /// The doc's list is the only place an author is told what already
    /// exists, and a list nothing compares goes stale silently: it sat six
    /// names short of `filter.rs` — `ARTIFACT_OR_CREATURE`,
    /// `ARTIFACT_CREATURE_OR_ENCHANTMENT`, `CREATURE_OR_PLANESWALKER`,
    /// `NONBASIC_LAND`, `YOUR_ARTIFACT`, `ANOTHER_CREATURE_YOU_CONTROL` —
    /// which is six constants an author would have written out by hand,
    /// which is what the constants exist to stop.
    ///
    /// It reads the two files rather than a list retyped here, because a
    /// third copy would be the same bug once more. Direction matters: a
    /// constant must be documented, and the doc is free to say more about
    /// one than its name.
    #[test]
    fn the_authoring_contract_names_every_filter_constant() {
        let filters = include_str!("filter.rs");
        let contract = include_str!("../../../docs/card-dsl.md");

        let declared: Vec<&str> = filters
            .lines()
            .filter_map(|line| line.trim().strip_prefix("pub const "))
            .filter_map(|rest| rest.split(':').next())
            .map(str::trim)
            .collect();

        assert!(
            declared.len() > 20,
            "read {} constants out of filter.rs — the reader is broken, not the doc",
            declared.len()
        );

        let missing: Vec<&str> = declared
            .iter()
            .copied()
            .filter(|name| !contract.contains(&format!("`{name}`")))
            .collect();

        assert!(
            missing.is_empty(),
            "docs/card-dsl.md does not name {missing:?} — a constant nobody is told about \
             is a constant the next card writes out by hand"
        );
    }

    /// Every [`CostPart`] variant is named in `docs/card-dsl.md`.
    ///
    /// The same bargain as the filter contract above, and it is here because
    /// the list went stale in silence: `TapOther` was added for the convoke
    /// lands and the doc's eleven names stayed eleven, so the one document
    /// that tells a card author what a cost may say did not mention the part
    /// that had just been built. The list is short enough that nobody
    /// notices it is one short.
    ///
    /// The doc spells a part with its payload — `` `TapOther(filter)` ``,
    /// `` `RemoveCounterSelf { kind, n }` `` — so what is checked is the
    /// name at the start of a backticked span, not the bare name. Anything
    /// stricter would be a test about how the prose is punctuated.
    #[test]
    fn the_authoring_contract_names_every_cost_part() {
        let costs = include_str!("cost.rs");
        let contract = include_str!("../../../docs/card-dsl.md");

        let body = costs
            .split_once("pub enum CostPart {")
            .expect("cost.rs declares the enum")
            .1
            .split_once("\n}\n")
            .expect("and closes it")
            .0;
        let declared: Vec<&str> = body
            .lines()
            .filter_map(|line| line.strip_prefix("    "))
            .filter(|rest| rest.starts_with(|c: char| c.is_ascii_uppercase()))
            .map(|rest| {
                rest.split(|c: char| !c.is_ascii_alphanumeric())
                    .next()
                    .unwrap_or(rest)
            })
            .collect();

        assert!(
            declared.len() >= 10,
            "read {declared:?} out of cost.rs — the reader is broken, not the doc"
        );

        let missing: Vec<&str> = declared
            .iter()
            .copied()
            .filter(|name| {
                !["`", "(", " "]
                    .iter()
                    .any(|tail| contract.contains(&format!("`{name}{tail}")))
            })
            .collect();

        assert!(
            missing.is_empty(),
            "docs/card-dsl.md does not name {missing:?} — the authoring \
             contract is where a card author is told what a cost may say, \
             and a part missing from it is a part nobody writes"
        );
    }
}
