use super::*;

#[test]
fn life_payment_fallback_is_visited() {
    static EFFECTS: &[Effect] = &[Effect::PlayerMayPayLifeOr {
        player: PlayerRel::ControllerOfTarget,
        life: Amount::SourcePower,
        effect: &Effect::CounterTargetSpellOrAbility,
    }];
    let mut seen = 0;
    let mut counter_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        counter_seen |= matches!(effect, Effect::CounterTargetSpellOrAbility);
    });
    assert_eq!(seen, 2);
    assert!(counter_seen);
}

/// Crystal Rod's life gain sits behind the payment, and a pool walk has
/// to find it there.
#[test]
fn a_price_paid_body_is_visited() {
    static EFFECTS: &[Effect] = &[Effect::PlayerMayPayThen {
        player: PlayerRel::You,
        mana: Amount::Fixed(1),
        effects: &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
    }];
    let mut seen = 0;
    let mut gain_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        gain_seen |= matches!(effect, Effect::GainLife { .. });
    });
    assert_eq!(seen, 2);
    assert!(gain_seen);
}

/// Phantasmal Forces' sacrifice and Farmstead's life gain sit behind a
/// coloured price, one on each answer, and a pool walk has to find both.
#[test]
fn a_coloured_price_body_is_visited() {
    static EFFECTS: &[Effect] = &[
        Effect::PlayerMayPayManaOr {
            player: PlayerRel::You,
            cost: baylee_core::mana!("{U}"),
            effect: &Effect::SacrificeSelf,
        },
        Effect::PlayerMayPayManaThen {
            player: PlayerRel::You,
            cost: baylee_core::mana!("{W}{W}"),
            effects: &[Effect::GainLife {
                amount: Amount::Fixed(1),
            }],
        },
    ];
    let mut seen = 0;
    let (mut sacrifice_seen, mut gain_seen) = (false, false);
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        sacrifice_seen |= matches!(effect, Effect::SacrificeSelf);
        gain_seen |= matches!(effect, Effect::GainLife { .. });
    });
    assert_eq!(seen, 4);
    assert!(sacrifice_seen && gain_seen);
}

/// Nissa, Resurgent Animist's reveal sits inside
/// `IfResolvedTimesThisTurn`, and a pool walk has to find it there.
#[test]
fn the_nth_resolution_branch_is_visited() {
    static EFFECTS: &[Effect] = &[Effect::IfResolvedTimesThisTurn {
        times: 2,
        then: &[Effect::RevealUntil {
            filter: &crate::Filter::Any,
            found: SearchDest::Hand,
        }],
    }];
    let mut seen = 0;
    let mut reveal_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        reveal_seen |= matches!(effect, Effect::RevealUntil { .. });
    });
    assert_eq!(seen, 2);
    assert!(reveal_seen);
}

/// Each of the nth-resolution effects is walked.
#[test]
fn nth_resolution_effects_are_visited() {
    static EFFECTS: &[Effect] = &[Effect::NthResolutionThisTurn {
        effects: &[Effect::gain_life(4), Effect::draw(1)],
    }];
    let mut seen = 0;
    let mut draw_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        draw_seen |= matches!(effect, Effect::DrawCards { .. });
    });
    assert_eq!(seen, 3);
    assert!(draw_seen);
}

/// "Destroy that creature at the beginning of the next end step" (Stone
/// Giant) carries the delayed trigger's effects, and the walk goes into
/// them.
#[test]
fn at_next_end_step_body_is_visited() {
    static EFFECTS: &[Effect] = &[Effect::AtNextEndStep {
        effects: &[Effect::destroy(TargetSpec::EventObject)],
    }];
    let mut seen = 0;
    let mut body_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        body_seen |= matches!(effect, Effect::Destroy { .. });
    });
    assert_eq!(seen, 2);
    assert!(body_seen);
}

#[test]
fn linked_counter_cleanup_body_is_visited() {
    let effects = &[Effect::ScheduleLinkedCounterCleanup {
        kind: crate::counters::MIRE,
        effects: &[Effect::CleanLinkedCounters {
            kind: crate::counters::MIRE,
        }],
    }];
    let mut seen = 0;
    let mut found = false;
    Effect::walk(effects, &mut seen, &mut |effect| {
        found |= matches!(effect, Effect::CleanLinkedCounters { .. });
    });
    assert_eq!(seen, 2);
    assert!(found);
}

/// "Destroy that creature at end of combat" (Cockatrice) carries the
/// delayed trigger's effects, and the walk goes into them.
#[test]
fn at_end_of_combat_body_is_visited() {
    static EFFECTS: &[Effect] = &[Effect::AtEndOfCombat {
        about: TargetSpec::EventObject,
        effects: &[Effect::destroy(TargetSpec::EventObject)],
    }];
    let mut seen = 0;
    let mut body_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        body_seen |= matches!(effect, Effect::Destroy { .. });
    });
    assert_eq!(seen, 2);
    assert!(body_seen);
}

/// Dragon Whelp's "if this ability has been activated four or more
/// times this turn" carries the delayed sacrifice, and the walk reaches
/// it through both carriers.
#[test]
fn activated_at_least_body_is_visited() {
    static EFFECTS: &[Effect] = &[Effect::IfActivatedThisTurnAtLeast {
        n: 4,
        then: &[Effect::AtNextEndStep {
            effects: &[Effect::SacrificeObject {
                target: TargetSpec::EventObject,
            }],
        }],
    }];
    let mut seen = 0;
    let mut body_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        body_seen |= matches!(effect, Effect::SacrificeObject { .. });
    });
    assert_eq!(seen, 3);
    assert!(body_seen);
}

/// "You may …. Do this only once each turn." carries its body the way
/// `MayDo` does, and the walk goes into it.
#[test]
fn once_each_turn_body_is_visited() {
    static EFFECTS: &[Effect] = &[Effect::MayDoOnceEachTurn {
        effects: &[Effect::GraveyardToBattlefield {
            target: TargetSpec::EventObject,
            owner_control: false,
            counters: None,
        }],
    }];
    let mut seen = 0;
    let mut body_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        body_seen |= matches!(effect, Effect::GraveyardToBattlefield { .. });
    });
    assert_eq!(seen, 2);
    assert!(body_seen);
}

/// "Choose a creature you control. It …" carries what happens to the
/// chosen one the way a one-branch conditional does, and the walk goes
/// into it.
#[test]
fn chosen_permanent_body_is_visited() {
    static EFFECTS: &[Effect] = &[Effect::ChooseYoursThen {
        filter: &crate::Filter::CREATURE,
        then: &[Effect::draw(1)],
    }];
    let mut seen = 0;
    let mut body_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        body_seen |= matches!(effect, Effect::DrawCards { .. });
    });
    assert_eq!(seen, 2);
    assert!(body_seen);
}

/// "… if it's [filter]" carries its effects the way the other one-branch
/// conditionals do, and the walk goes into them.
#[test]
fn if_event_object_matches_body_is_visited() {
    static EFFECTS: &[Effect] = &[Effect::AtNextEndStep {
        effects: &[Effect::IfEventObjectMatches {
            filter: &crate::Filter::AttackedThisTurn,
            then: &[Effect::destroy(TargetSpec::EventObject)],
        }],
    }];
    let mut seen = 0;
    let mut body_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        body_seen |= matches!(effect, Effect::Destroy { .. });
    });
    assert_eq!(seen, 3);
    assert!(body_seen);
}

#[test]
fn if_target_matches_body_is_visited() {
    static EFFECTS: &[Effect] = &[Effect::IfTargetMatches {
        filter: &crate::Filter::CmcAtMostColorsSpent,
        then: &[Effect::Exile {
            target: TargetSpec::Object(&crate::Filter::NONLAND),
        }],
    }];
    let mut seen = 0;
    let mut body_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        body_seen |= matches!(effect, Effect::Exile { .. });
    });
    assert_eq!(seen, 2);
    assert!(body_seen);
}

use crate::ability::Trigger;
use crate::static_ability::{Duration, Layer, Modifier};

/// Every verb is the literal it replaces.
///
/// A verb is only worth having if adopting it is free, and "free" here
/// means the same `Effect` value and not a near-enough one. There was no
/// test of this shape for `Effect::mana` either, which has 219 uses.
#[test]
fn a_verb_is_the_literal_it_replaces() {
    assert_eq!(
        Effect::draw(3),
        Effect::DrawCards {
            amount: Amount::Fixed(3)
        }
    );
    assert_eq!(
        Effect::scry(2),
        Effect::Scry {
            amount: Amount::Fixed(2)
        }
    );
    assert_eq!(
        Effect::surveil(1),
        Effect::Surveil {
            amount: Amount::Fixed(1)
        }
    );
    assert_eq!(
        Effect::gain_life(4),
        Effect::GainLife {
            amount: Amount::Fixed(4)
        }
    );

    let target = TargetSpec::Object(&Filter::CREATURE);
    assert_eq!(
        Effect::destroy(target),
        Effect::Destroy {
            target,
            no_regen: false
        }
    );
    assert_eq!(Effect::exile(target), Effect::Exile { target });
    assert_eq!(
        Effect::blink_to_owner(target),
        Effect::Blink {
            target,
            owner_control: true
        }
    );
    assert_eq!(
        Effect::blink_to_you(target),
        Effect::Blink {
            target,
            owner_control: false
        }
    );
    assert_eq!(Effect::bounce(target), Effect::ReturnToHand { target });
}

/// `Effect::continuous` derives the layer, and derives the one the
/// seventy-nine effects in the pool already state.
#[test]
fn a_continuous_effect_derives_its_own_layer() {
    assert_eq!(
        Effect::continuous(
            &Filter::This,
            Modifier::ModifyPT(1, 1),
            Duration::UntilEndOfTurn,
        ),
        Effect::CreateContinuousEffect {
            layer: Layer::PtModify,
            filter: &Filter::This,
            modifier: Modifier::ModifyPT(1, 1),
            duration: Duration::UntilEndOfTurn,
        }
    );
    let Effect::CreateContinuousEffect { layer, .. } = Effect::continuous(
        &Filter::Any,
        Modifier::AddType(baylee_core::types::TypeSet::ARTIFACT),
        Duration::WhileSourceOnBattlefield,
    ) else {
        panic!("continuous must build CreateContinuousEffect");
    };
    assert_eq!(layer, Layer::Type, "a type change is layer 4 (CR 613.1)");
}

/// `Trigger::ETB` is the enter-trigger 99 of the pool's 110 spell out.
#[test]
fn etb_is_the_trigger_the_pool_writes_a_hundred_times() {
    assert_eq!(Trigger::ETB, Trigger::EntersBattlefield(&Filter::This));
}

/// Vendilion Clique's "if you do, … then draws a card" rides on
/// `BottomCardFromHand::then`, and the walk reaches it.
#[test]
fn bottom_card_from_hand_body_is_visited() {
    static EFFECTS: &[Effect] = &[Effect::BottomCardFromHand {
        player: PlayerRel::Chosen,
        filter: &Filter::NONLAND,
        then: &[Effect::DrawCardsFor {
            amount: Amount::Fixed(1),
            who: PlayerRel::Chosen,
        }],
    }];
    let mut seen = 0;
    let mut body_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        body_seen |= matches!(effect, Effect::DrawCardsFor { .. });
    });
    assert_eq!(seen, 2);
    assert!(body_seen);
}
