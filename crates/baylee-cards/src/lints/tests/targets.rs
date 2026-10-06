//! Sweeps, targets, the controller of a target, and layers.

use super::*;

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
        targets: Some(TargetReq::up_to_one(TargetSpec::Object(
            &NONCREATURE_ARTIFACT,
        ))),
        second_targets: None,
    };
    assert!(
        target_reuse(&broken).is_some(),
        "the lint did not see an ability sweeping its own target filter"
    );

    // And the fix — the DSL's way of saying "the target" — passes.
    let fixed = AbilityDef::Loyalty {
        cost: 1,
        effects: &ON_THE_TARGET,
        targets: Some(TargetReq::up_to_one(TargetSpec::Object(
            &NONCREATURE_ARTIFACT,
        ))),
        second_targets: None,
    };
    assert!(
        target_reuse(&fixed).is_none(),
        "`Filter::This` is the target, not a sweep"
    );

    // As does a sweep with no target at all, which is what a wrath is.
    let wrath = AbilityDef::Spell {
        effects: &WRATH,
        targets: None,
        second_targets: None,
        condition: None,
    };
    assert!(target_reuse(&wrath).is_none(), "a wrath is not a mistake");

    // And a search that looks for the same thing it targets.
    let tutor = AbilityDef::Spell {
        effects: &OTHER_SWEEP,
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        condition: None,
    };
    assert!(
        target_reuse(&tutor).is_none(),
        "two different filters are not a reuse"
    );
}

/// Damage to each creature beside a creature target is the same
/// mistake as the wrath above: "deals 2 damage to target creature"
/// written as a sweep over the target's filter.
#[test]
fn the_lint_reads_a_damage_sweep_as_a_sweep() {
    static BURN_EVERY_ONE: [Effect; 1] = [Effect::damage_each(2, &Filter::CREATURE)];
    let broken = AbilityDef::Spell {
        effects: &BURN_EVERY_ONE,
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        condition: None,
    };
    assert!(
        target_reuse(&broken).is_some(),
        "a damage sweep over the target's own filter went unseen"
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

/// The player lint fires on a player target read as the controller of
/// an object, and stays quiet on the targeted player and on an object
/// whose controller is the point.
///
/// The broken half is Ashiok, Dream Render's −1 as it was written,
/// reduced to the one field that was wrong: "target player mills four
/// cards" with the player read as `ControllerOfTarget`.
#[test]
fn the_player_target_lint_catches_ashioks_minus_one() {
    use crate::dsl::effect::{Amount, PlayerRel};

    static MILL_THE_CONTROLLER: [Effect; 1] = [Effect::Mill {
        amount: Amount::Fixed(4),
        target: PlayerRel::ControllerOfTarget,
    }];
    static MILL_THE_CHOSEN: [Effect; 1] = [Effect::Mill {
        amount: Amount::Fixed(4),
        target: PlayerRel::Chosen,
    }];

    let broken = AbilityDef::Loyalty {
        cost: -1,
        effects: &MILL_THE_CONTROLLER,
        targets: Some(TargetReq::one(TargetSpec::AnyPlayer)),
        second_targets: None,
    };
    assert_eq!(
        controller_of_a_player_target(&broken),
        Some(TargetSpec::AnyPlayer),
        "the lint did not see a targeted player read as an object's controller"
    );

    // The fix: `Chosen` is the player the ability targeted.
    let fixed = AbilityDef::Loyalty {
        cost: -1,
        effects: &MILL_THE_CHOSEN,
        targets: Some(TargetReq::one(TargetSpec::AnyPlayer)),
        second_targets: None,
    };
    assert!(
        controller_of_a_player_target(&fixed).is_none(),
        "`PlayerRel::Chosen` is the targeted player"
    );

    // And Path to Exile's shape, where the controller of an object
    // target is exactly who the sentence is about.
    let object = AbilityDef::Loyalty {
        cost: -1,
        effects: &MILL_THE_CONTROLLER,
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
    };
    assert!(
        controller_of_a_player_target(&object).is_none(),
        "an object target has a controller to read"
    );
}

/// No ability in the pool targets only a player and then reads the
/// controller of its target.
#[test]
fn no_ability_reads_the_controller_of_a_player_target() {
    let mut wrong = Vec::new();
    let mut seen = 0_usize;
    for def in crate::all() {
        for ability in def.abilities.iter().chain(
            def.faces
                .iter()
                .flat_map(|f| f.abilities.iter())
                .collect::<Vec<_>>(),
        ) {
            if let Some(spec) = controller_of_a_player_target(ability) {
                wrong.push(format!("{} targets {spec:?}", def.name()));
            }
            for branch in branches(ability) {
                if branch.target.is_some_and(can_target_an_object)
                    && reads_controller_of_target(branch.effects)
                {
                    seen += 1;
                }
            }
        }
    }
    // The floor: seventeen branches over sixteen cards read the
    // controller of an object target on 23.09.2026 (Path to Exile, Mana
    // Leak, Ghost Quarter, …; Ertai Resurrected counts twice), and a walk
    // that reached none of them would pass exactly as loudly.
    assert!(
        seen >= 12,
        "only {seen} branch(es) read the controller of an object target; \
         the sweep has gone blind"
    );
    assert!(
        wrong.is_empty(),
        "{} card(s) target a player and then read `PlayerRel::ControllerOfTarget`, \
         which is the controller of an *object* target and names nobody here. \
         \"Target player …\" is `PlayerRel::Chosen`.\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// The layer lint fires on a modifier put on the wrong layer, and stays
/// quiet on the same modifier put on the right one.
///
/// Both halves matter. A sweep that finds nothing over 1365 cards is
/// indistinguishable from one that looks at nothing, and a lint that
/// fires on everything would be just as useless.
#[test]
fn the_layer_lint_catches_a_modifier_on_the_wrong_layer() {
    let wrong = AbilityDef::Static(crate::dsl::StaticAbility {
        layer: Layer::Color,
        filter: Filter::Any,
        modifier: Modifier::AddType(TypeSet::ARTIFACT),
        condition: None,
    });
    let right = AbilityDef::Static(crate::dsl::StaticAbility {
        layer: Layer::Type,
        filter: Filter::Any,
        modifier: Modifier::AddType(TypeSet::ARTIFACT),
        condition: None,
    });
    assert_eq!(
        layer_fault(&wrong),
        Some((
            Layer::Color,
            Layer::Type,
            Modifier::AddType(TypeSet::ARTIFACT)
        )),
        "a type-changing modifier on layer 5 is the mistake this exists for"
    );
    assert_eq!(layer_fault(&right), None, "layer 4 is where it belongs");

    // And the same through an effect, which is the other half of the
    // pool: 78 of the 108 pairings are `CreateContinuousEffect`.
    let via_effect = AbilityDef::Spell {
        effects: &[Effect::CreateContinuousEffect {
            layer: Layer::Text,
            filter: &Filter::This,
            modifier: Modifier::ModifyPT(1, 1),
            duration: Duration::UntilEndOfTurn,
        }],
        targets: None,
        second_targets: None,
        condition: None,
    };
    assert_eq!(
        layer_fault(&via_effect),
        Some((Layer::Text, Layer::PtModify, Modifier::ModifyPT(1, 1))),
        "a pump is layer 7c wherever it is written"
    );
}

/// The recursion reaches a continuous effect inside a conditional.
///
/// The pool has exactly one — Jin-Gitaxias's kicked half — so a walker
/// that stopped at the top level would have passed this sweep while
/// being blind to the one card that needed it.
#[test]
fn the_layer_lint_looks_inside_a_conditional() {
    let nested = AbilityDef::Spell {
        effects: &[Effect::IfKicked {
            then: &[Effect::CreateContinuousEffect {
                layer: Layer::Copy,
                filter: &Filter::This,
                modifier: Modifier::AddKeyword(crate::dsl::KeywordSet::FLYING),
                duration: Duration::UntilEndOfTurn,
            }],
            otherwise: &[],
        }],
        targets: None,
        second_targets: None,
        condition: None,
    };
    assert_eq!(
        layer_fault(&nested).map(|(declared, derived, _)| (declared, derived)),
        Some((Layer::Copy, Layer::Ability)),
        "a granted keyword inside `IfKicked` is still layer 6"
    );
}

/// Every continuous effect in the pool sits on the layer its modifier
/// derives (CR 613.1).
///
/// This is what makes the two-argument
/// [`crate::dsl::static_ability!`] safe: a card states what changes and
/// to what, and the layer follows. A raw literal may still be written,
/// and this is what stops one disagreeing with the macro beside it.
#[test]
fn every_layer_in_the_pool_is_the_one_its_modifier_derives() {
    let mut wrong = Vec::new();
    let mut seen = 0_usize;
    for def in crate::all() {
        let faces = def.faces.iter().flat_map(|f| f.abilities.iter());
        for ability in def.abilities.iter().chain(faces) {
            seen += match ability {
                AbilityDef::Static(_) => 1,
                _ => 0,
            } + branches(ability)
                .into_iter()
                .flat_map(|b| b.effects.iter().flat_map(declared_layers))
                .count();
            if let Some((declared, derived, modifier)) = layer_fault(ability) {
                wrong.push(format!(
                    "{}: {modifier:?} is declared on {declared:?}, derives {derived:?}",
                    def.name()
                ));
            }
        }
    }
    assert!(
        seen > 100,
        "only {seen} layer/modifier pairings found — the walker has gone \
         blind, and an empty sweep proves nothing"
    );
    assert!(
        wrong.is_empty(),
        "{} continuous effect(s) name a layer their modifier does not \
         belong to. The layer is not a card's decision: write \
         `static_ability!(filter, modifier)` and let `Modifier::layer` \
         answer.\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
