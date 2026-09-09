//! Pool-wide lints: shapes in the card data that are almost always a
//! mistake, checked against the DSL alone.
//!
//! # Why these are not tests about the engine
//!
//! Everything here reads `CardDef` and nothing else — no `Engine`, no
//! Scryfall payload, no oracle text. That makes them the cheapest tier there
//! is (the whole module runs in milliseconds over 1365 cards) and it fixes
//! what they can prove: they cannot say a card does the *right* thing, only
//! that it is built in a shape that cannot be right.
//!
//! That is a narrower claim than it sounds, and it is worth having because
//! the pool's most expensive bug so far was exactly such a shape. Karn, the
//! Great Creator's `+1` reads "until end of turn, target noncreature
//! artifact becomes a 0/0 artifact creature". It was written with the
//! ability's *target filter* handed to the effect as well, so it animated
//! every noncreature artifact on the table — and then state-based actions
//! put all of them in a graveyard. Nothing in the engine was wrong, the
//! card's header matched its `CardDef`, and no gate had anything to say.
//! [`target_reuse`] is that shape, and it is decidable without playing a
//! single turn.
//!
//! # The shape of every lint here
//!
//! One `#[test]`, sweeping [`crate::all`], collecting *every* offender and
//! failing with the whole list — the shape
//! `no_card_claims_a_keyword_the_engine_ignores` already has, because a
//! sweep that stops at the first card makes a pool-wide problem look like
//! one card's problem.
//!
//! And each lint is a **pure function** over the data, with a unit test that
//! hands it the broken shape and watches it fire. A sweep over 1365 cards
//! that finds nothing is otherwise indistinguishable from one that checks
//! nothing.

use crate::dsl::ability::{AbilityDef, SpellMode};
use crate::dsl::effect::{Effect, TargetSpec};
use crate::dsl::filter::Filter;

/// One `(what it targets, what it does)` pair out of an ability.
///
/// An ability is not always one of these: a modal spell is one per mode,
/// because each mode targets for itself.
#[derive(Clone, Copy, Debug)]
struct Branch {
    /// What the branch targets, if anything.
    target: Option<TargetSpec>,
    /// What it does when it resolves.
    effects: &'static [Effect],
}

/// Every `(target, effects)` pair an ability resolves through.
///
/// The `match` is deliberately **exhaustive with no wildcard arm**: a new
/// [`AbilityDef`] variant must be classified here before the crate compiles,
/// because a variant silently falling into a `_ => vec![]` would leave every
/// lint in this module quietly passing over it.
fn branches(ability: &AbilityDef) -> Vec<Branch> {
    let modal = |modes: &'static [SpellMode]| {
        modes
            .iter()
            .map(|m| Branch {
                target: m.target,
                effects: m.effects,
            })
            .collect()
    };
    match ability {
        AbilityDef::Spell { effects, targets } => vec![Branch {
            target: targets.map(|t| t.spec),
            effects,
        }],
        AbilityDef::Triggered {
            effects, targets, ..
        } => vec![Branch {
            target: targets.map(|t| t.spec),
            effects,
        }],
        AbilityDef::Activated {
            effects, target, ..
        }
        | AbilityDef::ActivatedConditional {
            effects, target, ..
        }
        | AbilityDef::SagaChapter {
            effects, target, ..
        }
        | AbilityDef::Loyalty {
            effects, target, ..
        } => vec![Branch {
            target: *target,
            effects,
        }],
        AbilityDef::ModalSpell { modes } | AbilityDef::ModalTriggered { modes, .. } => modal(modes),
        // Nothing that resolves through an effect list of its own: a
        // keyword the engine synthesises, a cost, a continuous rule, or a
        // choice made as the permanent enters.
        AbilityDef::Unimplemented
        | AbilityDef::Ward { .. }
        | AbilityDef::Prepared { .. }
        | AbilityDef::Echo { .. }
        | AbilityDef::Static(_)
        | AbilityDef::Replacement(_)
        | AbilityDef::Suspend { .. }
        | AbilityDef::CopyOnEnterUntilEot { .. }
        | AbilityDef::CopyOnEnter { .. } => Vec::new(),
    }
}

/// The filters an effect applies to **every match of**, rather than to a
/// target.
///
/// This is the list that makes [`target_reuse`] mean something, so it is
/// worth being exact about what belongs on it. A filter can appear in an
/// effect for three different jobs, and only the first is a sweep:
///
/// * *which objects this happens to* — `DestroyAll`, `PumpFilter`. On the
///   list.
/// * *what may be found* — `SearchLibrary`, `WishToHand`. A filter over
///   cards nobody has chosen yet, and reusing the target's filter there is
///   ordinary ("search for a creature card" beside "target creature").
/// * *a condition* — `IfControlGreatestCmc`. Reads the board, changes
///   nothing.
///
/// `Filter::This` is the DSL's way of writing "the target" inside a
/// continuous effect, so it is never a sweep however it is spelled.
fn swept_filters(effect: &Effect) -> Vec<&'static Filter> {
    match effect {
        Effect::DestroyAll { filter }
        | Effect::ReturnAllToHand { filter, .. }
        | Effect::SetPTFilter { filter, .. }
        | Effect::AddCounterFilter { filter, .. }
        | Effect::PumpFilter { filter, .. }
        | Effect::SacrificeFilter { filter, .. }
        | Effect::CreateContinuousEffect { filter, .. } => {
            if matches!(filter, Filter::This) {
                Vec::new()
            } else {
                vec![filter]
            }
        }
        // A conditional runs its branches, so the sweep inside one is still
        // a sweep. Everything else either carries no filter or carries one
        // for a job that is not "every object matching this".
        Effect::IfKicked { then, otherwise }
        | Effect::IfEventPowerAtLeast {
            then, otherwise, ..
        } => then
            .iter()
            .chain(*otherwise)
            .flat_map(swept_filters)
            .collect(),
        Effect::IfCreaturesDiedAtLeast { then, .. }
        | Effect::IfNotLostLifeThisTurn { then, .. }
        | Effect::IfControlGreatestCmc { then, .. } => {
            then.iter().flat_map(swept_filters).collect()
        }
        _ => Vec::new(),
    }
}

/// The filter a target spec picks its target with, when it picks one by
/// characteristics at all.
fn target_filter(spec: TargetSpec) -> Option<&'static Filter> {
    match spec {
        TargetSpec::Object(f)
        | TargetSpec::Spell(f)
        | TargetSpec::StackOrBattlefield(f)
        | TargetSpec::CardInGraveyard(f, _)
        | TargetSpec::AbilityOnStack(f)
        | TargetSpec::SpellOrAbility(f) => Some(f),
        // A player, the source, or the event's own object: no filter, and
        // nothing an effect could reuse by accident.
        TargetSpec::ThisObject
        | TargetSpec::EventObject
        | TargetSpec::Player(_)
        | TargetSpec::AnyPlayer
        | TargetSpec::AnyOpponent
        | TargetSpec::AnyTarget => None,
    }
}

/// An ability that targets and then does something to **everything its
/// target filter matches**, rather than to the target.
///
/// Returns the reused filter, which is what a failure has to print: the
/// mistake is invisible in the card's text and obvious in the filter's name.
fn target_reuse(ability: &AbilityDef) -> Option<&'static Filter> {
    for branch in branches(ability) {
        let Some(wanted) = branch.target.and_then(target_filter) else {
            continue;
        };
        for effect in branch.effects {
            for swept in swept_filters(effect) {
                if swept == wanted {
                    return Some(swept);
                }
            }
        }
    }
    None
}

/// Whether an effect adds mana.
fn makes_mana(effect: &Effect) -> bool {
    matches!(effect, Effect::AddMana { .. })
}

/// Whether an activated ability's `mana_ability` flag disagrees with what
/// the ability actually does (CR 605.1), and how.
///
/// `None` for an ability that is honest, and for every ability the rule has
/// nothing to say about — a spell, a trigger, a loyalty ability. CR 605.1a
/// is why the target is read here: an ability with a target is *never* a
/// mana ability, however much mana it makes.
fn mana_ability_fault(ability: &AbilityDef) -> Option<&'static str> {
    let (claimed, effects, target) = match ability {
        AbilityDef::Activated {
            mana_ability,
            effects,
            target,
            ..
        }
        | AbilityDef::ActivatedConditional {
            mana_ability,
            effects,
            target,
            ..
        } => (*mana_ability, *effects, *target),
        _ => return None,
    };
    let only_mana = !effects.is_empty() && effects.iter().all(makes_mana);
    if claimed && !effects.iter().any(makes_mana) {
        return Some("claims a mana ability that makes no mana");
    }
    if claimed && target.is_some() {
        return Some("claims a mana ability that targets (CR 605.1a)");
    }
    if !claimed && only_mana && target.is_none() {
        return Some("only adds mana and is not marked a mana ability");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::effect::TargetReq;
    use crate::dsl::static_ability::{Duration, Layer, Modifier};

    /// The filter Karn's `+1` targeted with, and then swept the board with.
    static NONCREATURE_ARTIFACT: Filter = Filter::And(&[
        Filter::ARTIFACT,
        Filter::LacksType(baylee_core::types::TypeSet::CREATURE),
    ]);

    /// Animating every artifact on the table: the bug, as written.
    static SWEEP: [Effect; 1] = [Effect::CreateContinuousEffect {
        layer: Layer::Type,
        filter: &NONCREATURE_ARTIFACT,
        modifier: Modifier::AddType(baylee_core::types::TypeSet::CREATURE),
        duration: Duration::UntilEndOfTurn,
    }];

    /// The same effect pointed at the target, which is the fix.
    static ON_THE_TARGET: [Effect; 1] = [Effect::CreateContinuousEffect {
        layer: Layer::Type,
        filter: &Filter::This,
        modifier: Modifier::AddType(baylee_core::types::TypeSet::CREATURE),
        duration: Duration::UntilEndOfTurn,
    }];

    /// A wrath: a sweep with nothing targeted.
    static WRATH: [Effect; 1] = [Effect::DestroyAll {
        filter: &Filter::CREATURE,
    }];

    /// A sweep beside a target, of two different kinds.
    static OTHER_SWEEP: [Effect; 1] = [Effect::DestroyAll {
        filter: &Filter::LAND,
    }];

    /// An ability that targets one artifact and animates every artifact.
    ///
    /// This is Karn, the Great Creator's `+1` as it was actually written,
    /// reduced to the two fields that were wrong. It is the counter-test for
    /// [`no_targeted_ability_sweeps_the_board_instead`]: a sweep over 1365
    /// cards that finds nothing proves nothing unless the thing it is
    /// looking for would have been found.
    #[test]
    fn the_lint_catches_the_bug_it_was_written_for() {
        let broken = AbilityDef::Loyalty {
            cost: 1,
            effects: &SWEEP,
            target: Some(TargetSpec::Object(&NONCREATURE_ARTIFACT)),
        };
        assert!(
            target_reuse(&broken).is_some(),
            "the lint did not see an ability sweeping its own target filter"
        );

        // And the fix — the DSL's way of saying "the target" — passes.
        let fixed = AbilityDef::Loyalty {
            cost: 1,
            effects: &ON_THE_TARGET,
            target: Some(TargetSpec::Object(&NONCREATURE_ARTIFACT)),
        };
        assert!(
            target_reuse(&fixed).is_none(),
            "`Filter::This` is the target, not a sweep"
        );

        // As does a sweep with no target at all, which is what a wrath is.
        let wrath = AbilityDef::Spell {
            effects: &WRATH,
            targets: None,
        };
        assert!(target_reuse(&wrath).is_none(), "a wrath is not a mistake");

        // And a search that looks for the same thing it targets.
        let tutor = AbilityDef::Spell {
            effects: &OTHER_SWEEP,
            targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        };
        assert!(
            target_reuse(&tutor).is_none(),
            "two different filters are not a reuse"
        );
    }

    /// No card in the pool targets one thing and then does it to every
    /// thing of that kind.
    #[test]
    fn no_targeted_ability_sweeps_the_board_instead() {
        let mut wrong = Vec::new();
        for def in crate::all() {
            for ability in def.abilities.iter().chain(
                def.faces
                    .iter()
                    .flat_map(|f| f.abilities.iter())
                    .collect::<Vec<_>>(),
            ) {
                if let Some(filter) = target_reuse(ability) {
                    wrong.push(format!("{} sweeps with {filter:?}", def.name()));
                }
            }
        }
        assert!(
            wrong.is_empty(),
            "{} card(s) target one object and then affect every object the \
             same filter matches. Inside a continuous effect the DSL spells \
             \"the target\" as `Filter::This`; elsewhere the effect wants a \
             filter of its own.\n{}",
            wrong.len(),
            wrong.join("\n")
        );
    }

    /// A mana ability makes mana, and an ability that only makes mana is a
    /// mana ability (CR 605.1).
    ///
    /// The flag is not decoration: an ability marked `mana_ability` does not
    /// use the stack and cannot be responded to, and one that is not marked
    /// does. Both directions of the mistake are silent — a mana ability that
    /// forgot the flag merely feels slow, and a non-mana ability that claims
    /// it skips a window the rules guarantee — and nothing else in the suite
    /// reads either as a bug.
    ///
    /// CR 605.1a is why the target is checked at all: an ability with a
    /// target is *never* a mana ability, however much mana it makes.
    #[test]
    fn a_mana_ability_is_exactly_an_ability_that_only_makes_mana() {
        let mut wrong = Vec::new();
        let mut seen = 0_usize;
        for def in crate::all() {
            let faces = def.faces.iter().flat_map(|f| f.abilities.iter());
            for ability in def.abilities.iter().chain(faces) {
                if matches!(
                    ability,
                    AbilityDef::Activated { .. } | AbilityDef::ActivatedConditional { .. }
                ) {
                    seen += 1;
                }
                if let Some(fault) = mana_ability_fault(ability) {
                    wrong.push(format!("{} {fault}", def.name()));
                }
            }
        }
        assert!(
            wrong.is_empty(),
            "{} ability/abilities disagree with CR 605.1:\n{}",
            wrong.len(),
            wrong.join("\n")
        );
        // The floor: a sweep that inspected nothing is indistinguishable
        // from one that found nothing, and the pool has four hundred lands
        // in it.
        assert!(
            seen > 300,
            "only {seen} activated abilities were looked at; the sweep is not reaching the pool"
        );
    }

    /// And the mana lint bites in both directions.
    #[test]
    fn the_mana_lint_catches_both_halves_of_cr_605_1() {
        use crate::dsl::ability::{ActivationTiming, ActivationZone};
        use crate::dsl::effect::Amount;
        static MANA: [Effect; 1] = [Effect::mana(baylee_core::mana::ManaColor::Green, 1)];
        static DRAW: [Effect; 1] = [Effect::DrawCards {
            amount: Amount::Fixed(1),
        }];

        let unmarked = AbilityDef::Activated {
            cost: crate::dsl::cost::Cost::TAP,
            effects: &MANA,
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        };
        assert!(
            mana_ability_fault(&unmarked).is_some(),
            "an ability that only adds mana and is not marked one slipped through"
        );

        let lying = AbilityDef::Activated {
            cost: crate::dsl::cost::Cost::TAP,
            effects: &DRAW,
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        };
        assert!(
            mana_ability_fault(&lying).is_some(),
            "a mana ability that makes no mana slipped through"
        );
    }
}
