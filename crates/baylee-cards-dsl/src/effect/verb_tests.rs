use super::*;

#[test]
fn a_nonmana_paid_reward_is_visited() {
    static EFFECTS: &[Effect] = &[Effect::PlayerMayPayCostThen {
        player: PlayerRel::You,
        cost: &crate::CostPart::SacrificeSelf,
        effect: &Effect::DrawCards {
            amount: Amount::Fixed(2),
        },
    }];
    let mut seen = 0;
    let mut reward_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        reward_seen |= matches!(effect, Effect::DrawCards { .. });
    });
    assert_eq!(seen, 2);
    assert!(reward_seen);
}

#[test]
fn destruction_followup_instructions_are_visited() {
    static EFFECTS: &[Effect] = &[Effect::DestroyAllThenForEachDestroyed {
        filter: &Filter::LAND,
        no_regen: false,
        effects: &[Effect::SacrificeSelf],
    }];
    let mut seen = 0;
    let mut consequence_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        consequence_seen |= matches!(effect, Effect::SacrificeSelf);
    });
    assert_eq!(seen, 2);
    assert!(consequence_seen);
}

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

/// A coin flip's two branches (the Arabian Nights coin flips) and "when
/// that creature dies this turn" (Sandals of Abdallah) carry effects, and
/// the walk goes into each.
#[test]
fn flip_coin_and_dies_this_turn_bodies_are_visited() {
    static EFFECTS: &[Effect] = &[
        Effect::FlipCoin {
            won: &[Effect::draw(1)],
            lost: &[Effect::SacrificeSelf],
        },
        Effect::WhenTargetDiesThisTurn {
            effects: &[Effect::destroy(TargetSpec::ThisObject)],
        },
    ];
    let mut seen = 0;
    let (mut drew, mut sacrificed, mut destroyed) = (false, false, false);
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        drew |= matches!(effect, Effect::DrawCards { .. });
        sacrificed |= matches!(effect, Effect::SacrificeSelf);
        destroyed |= matches!(effect, Effect::Destroy { .. });
    });
    assert_eq!(seen, 5);
    assert!(drew && sacrificed && destroyed);
}

/// "If you sacrifice an Island this way, this creature deals 3 damage to
/// you" (Serendib Djinn) is a body the walk goes into.
#[test]
fn sacrifice_one_then_body_is_visited() {
    static EFFECTS: &[Effect] = &[Effect::SacrificeOneThen {
        filter: &crate::Filter::LAND,
        if_it_was: &crate::Filter::LAND,
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

#[test]
fn poison_or_combat_damage_attacks_and_exile_graveyard_bodies_are_visited() {
    static EFFECTS: &[Effect] = &[
        Effect::PlayerMayGetPoisonOr {
            player: crate::PlayerRel::EachOpponent,
            poison: crate::Amount::Fixed(1),
            effect: &Effect::draw(1),
        },
        Effect::WheneverOneOrMoreDealCombatDamageToPlayerThisTurn {
            filter: &crate::Filter::CREATURE,
            effects: &[Effect::LoseHalfLife {
                player: crate::PlayerRel::DamagedPlayer,
            }],
        },
        Effect::WheneverCreatureAttacksYouOrYourPlaneswalkerUntilYourNextTurn {
            effects: &[Effect::TapEventObject],
        },
        Effect::MayExileFromYourGraveyardThen {
            count: 3,
            filter: &crate::Filter::Any,
            then: &[Effect::BecomePrepared],
        },
    ];
    let mut seen = 0;
    let (mut drew, mut halved, mut tapped, mut prepared) = (false, false, false, false);
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        drew |= matches!(effect, Effect::DrawCards { .. });
        halved |= matches!(effect, Effect::LoseHalfLife { .. });
        tapped |= matches!(effect, Effect::TapEventObject);
        prepared |= matches!(effect, Effect::BecomePrepared);
    });
    assert_eq!(seen, 8);
    assert!(drew && halved && tapped && prepared);
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

/// A coin flip's two outcomes are both branches a pool walk has to find,
/// the lost one as much as the won one (Mijae Djinn does nothing on a win).
#[test]
fn both_flip_branches_are_visited() {
    static EFFECTS: &[Effect] = &[Effect::FlipCoin {
        won: &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        lost: &[Effect::CounterTargetSpellOrAbility],
    }];
    let mut seen = 0;
    let (mut won_seen, mut lost_seen) = (false, false);
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        won_seen |= matches!(effect, Effect::GainLife { .. });
        lost_seen |= matches!(effect, Effect::CounterTargetSpellOrAbility);
    });
    assert_eq!(seen, 3);
    assert!(won_seen && lost_seen);
}

#[test]
fn inline_choice_walk_visits_every_branch_and_nested_sequence() {
    static EFFECTS: &[Effect] = &[Effect::ChooseOne {
        choices: &[Effect::Sequence(&[Effect::draw(1)]), Effect::draw(2)],
    }];
    let mut draws = Vec::new();
    let mut seen = 0;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        if let Effect::DrawCards { amount } = effect {
            draws.push(*amount);
        }
    });
    assert_eq!(draws, vec![Amount::Fixed(1), Amount::Fixed(2)]);
    assert_eq!(seen, 4);
}

#[test]
fn retained_target_type_branch_is_visited() {
    static EFFECTS: &[Effect] = &[Effect::IfTargetHadType {
        types: baylee_core::types::TypeSet::CREATURE,
        then: &[Effect::draw(1)],
    }];
    let mut draws = 0;
    Effect::walk(EFFECTS, &mut 0, &mut |effect| {
        if matches!(effect, Effect::DrawCards { .. }) {
            draws += 1;
        }
    });
    assert_eq!(draws, 1);
}

#[test]
fn both_target_match_branches_are_visited() {
    static EFFECTS: &[Effect] = &[Effect::IfTargetMatchesElse {
        filter: &crate::Filter::CREATURE,
        then: &[Effect::draw(1)],
        otherwise: &[Effect::GainLife {
            amount: crate::Amount::Fixed(1),
        }],
    }];
    let mut seen = 0;
    let mut draw = false;
    let mut life = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        draw |= matches!(effect, Effect::DrawCards { .. });
        life |= matches!(effect, Effect::GainLife { .. });
    });
    assert_eq!(seen, 3);
    assert!(draw && life);
}

#[test]
fn at_next_upkeep_body_is_visited() {
    static EFFECTS: &[Effect] = &[
        Effect::AtNextUpkeep {
            effects: &[Effect::destroy(TargetSpec::EventObject)],
        },
        Effect::AtNextTurnsUpkeep {
            effects: &[Effect::SacrificeSelf],
        },
    ];
    let mut seen = 0;
    let mut body_seen = false;
    let mut next_turn_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        body_seen |= matches!(effect, Effect::Destroy { .. });
        next_turn_seen |= matches!(effect, Effect::SacrificeSelf);
    });
    assert_eq!(seen, 4);
    assert!(body_seen && next_turn_seen);
}

/// Wave 1's three carriers: a repeated list, a repeated pass and a watch on
/// the source's death each hold a body every walker must read.
#[test]
fn repeated_and_source_watching_bodies_are_visited() {
    static REPEAT: &[Effect] = &[Effect::Repeat {
        times: Amount::Fixed(2),
        effects: &[Effect::destroy(TargetSpec::EventObject)],
    }];
    static WHILE: &[Effect] = &[Effect::RepeatWhile {
        effects: &[Effect::destroy(TargetSpec::EventObject)],
        condition: crate::Condition::YourTurn,
    }];
    static WATCH: &[Effect] = &[Effect::WhenSourceDiesThisTurn {
        effects: &[Effect::destroy(TargetSpec::EventObject)],
    }];
    static DAMAGE_WATCH: &[Effect] = &[Effect::WheneverTargetDealtDamageThisTurn {
        by: &Filter::CREATURE,
        effects: &[Effect::destroy(TargetSpec::EventObject)],
    }];
    static THIS_TURNS_COMBAT: &[Effect] = &[Effect::AtEndOfCombatThisTurn {
        about: TargetSpec::EventObject,
        effects: &[Effect::destroy(TargetSpec::EventObject)],
    }];
    static ATTACK_WATCH: &[Effect] = &[Effect::WheneverYouAttackThisTurn {
        effects: &[Effect::destroy(TargetSpec::EventObject)],
    }];
    static SUBJECT_END_STEP: &[Effect] = &[Effect::AtNextEndStepAboutSubject {
        effects: &[Effect::destroy(TargetSpec::EventObject)],
    }];
    static TOKEN_END_STEP: &[Effect] = &[Effect::AtNextEndStepForCreatedTokens {
        effects: &[Effect::destroy(TargetSpec::EventObject)],
    }];
    for list in [
        REPEAT,
        WHILE,
        WATCH,
        DAMAGE_WATCH,
        THIS_TURNS_COMBAT,
        ATTACK_WATCH,
        SUBJECT_END_STEP,
        TOKEN_END_STEP,
    ] {
        let mut seen = 0;
        let mut body_seen = false;
        Effect::walk(list, &mut seen, &mut |effect| {
            body_seen |= matches!(effect, Effect::Destroy { .. });
        });
        assert_eq!(seen, 2, "{list:?}");
        assert!(body_seen, "{list:?}");
    }
}

#[test]
fn mana_retention_preserves_restrictions_and_spend_riders() {
    assert_eq!(
        Effect::mana(ManaColor::Blue, 2)
            .mana_until_end_of_turn()
            .restricted(&Filter::CREATURE, SpendRider::Uncounterable),
        Effect::mana(ManaColor::Blue, 2)
            .restricted(&Filter::CREATURE, SpendRider::Uncounterable)
            .mana_until_end_of_turn(),
    );
    assert_eq!(
        Effect::mana(ManaColor::Blue, 2)
            .mana_until_end_of_turn()
            .when_spent(&Filter::CREATURE, SpendRider::Scry(1)),
        Effect::mana(ManaColor::Blue, 2)
            .when_spent(&Filter::CREATURE, SpendRider::Scry(1))
            .mana_until_end_of_turn(),
    );
    let effect =
        Effect::mana(ManaColor::Blue, 2).restricted(&Filter::CREATURE, SpendRider::Uncounterable);
    let Effect::AddMana {
        source,
        amount,
        distribution,
        restriction: Some(mut restriction),
    } = effect
    else {
        panic!("mana")
    };
    restriction.until_end_of_turn = true;
    assert_eq!(
        effect.mana_until_end_of_turn(),
        Effect::AddMana {
            source,
            amount,
            distribution,
            restriction: Some(restriction)
        }
    );
    let Effect::AddMana {
        restriction: Some(ordinary),
        ..
    } = Effect::mana(ManaColor::Red, 1).mana_until_end_of_turn()
    else {
        panic!("mana")
    };
    assert!(!ordinary.restricts);
    assert!(ordinary.until_end_of_turn);
    assert_eq!(ordinary.rider, SpendRider::None);
}

#[test]
fn energy_payment_fallback_is_visited() {
    static EFFECTS: &[Effect] = &[Effect::MayPayEnergyOr {
        amount: 2,
        effect: &Effect::DiscardRandom {
            who: PlayerRel::You,
            count: Amount::Fixed(1),
        },
    }];
    let mut seen = 0;
    let mut discard_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        discard_seen |= matches!(
            effect,
            Effect::DiscardRandom {
                who: PlayerRel::You,
                count: Amount::Fixed(1)
            }
        );
    });
    assert_eq!(seen, 2);
    assert!(discard_seen);
}

#[test]
fn sacrifice_outcomes_are_visited() {
    static EFFECTS: &[Effect] = &[
        Effect::PlayerMaySacrificeNOr {
            player: PlayerRel::You,
            filter: &Filter::LAND,
            count: 2,
            effect: &Effect::SacrificeSelf,
        },
        Effect::SacrificeAndIf {
            filter: &Filter::LAND,
            check: &Filter::LAND,
            effect: &Effect::GainLife {
                amount: Amount::Fixed(3),
            },
        },
    ];
    let mut seen = 0;
    let mut branches = 0;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        if matches!(effect, Effect::SacrificeSelf | Effect::GainLife { .. }) {
            branches += 1;
        }
    });
    assert_eq!(seen, 4);
    assert_eq!(branches, 2);
}

#[test]
fn behold_body_is_visited() {
    static EFFECTS: &[Effect] = &[Effect::MayBehold {
        filter: &Filter::CREATURE,
        then: &[Effect::UntapFound],
    }];
    let mut seen = 0;
    let mut untap_seen = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        untap_seen |= matches!(effect, Effect::UntapFound);
    });
    assert_eq!(seen, 2);
    assert!(untap_seen);
}

#[test]
fn taking_the_top_card_visits_its_successful_move_continuation() {
    static EFFECTS: &[Effect] = &[Effect::LookAtTopMayTake {
        filter: &Filter::CREATURE,
        rest: LookRest::Keep,
        then: &[Effect::TransformSource],
    }];
    let mut seen = 0;
    let mut transformed = false;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        transformed |= matches!(effect, Effect::TransformSource);
    });
    assert_eq!(seen, 2);
    assert!(transformed);
}

#[test]
fn sacrifice_self_success_body_is_visited() {
    let mut seen = 0;
    let mut nested = false;
    Effect::walk(
        &[Effect::SacrificeSelfThen {
            then: &[Effect::ReturnLinkedExile {
                to: crate::ZoneRef::Hand,
            }],
        }],
        &mut seen,
        &mut |effect| {
            nested |= matches!(effect, Effect::ReturnLinkedExile { .. });
        },
    );
    assert_eq!(seen, 2);
    assert!(nested);
}

#[test]
fn both_mana_payment_outcomes_are_visited() {
    static EFFECTS: &[Effect] = &[Effect::PlayerMayPayThenElse {
        player: PlayerRel::You,
        mana: Amount::Fixed(1),
        paid: &[Effect::CounterEventAbility],
        unpaid: &[Effect::RemoveEventObjectFromCombat { unblock: true }],
    }];
    let mut seen = 0;
    let (mut paid, mut unpaid) = (false, false);
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        paid |= matches!(effect, Effect::CounterEventAbility);
        unpaid |= matches!(effect, Effect::RemoveEventObjectFromCombat { .. });
    });
    assert_eq!(seen, 3);
    assert!(paid && unpaid);
}

#[test]
fn both_repeated_mana_payment_outcomes_are_visited() {
    static EFFECTS: &[Effect] = &[Effect::PlayerMayPayManaEachThenElse {
        player: PlayerRel::You,
        each: baylee_core::mana::ManaCost::ZERO,
        times: Amount::Fixed(2),
        paid: &[Effect::CounterEventAbility],
        unpaid: &[Effect::RemoveEventObjectFromCombat { unblock: true }],
    }];
    let mut seen = 0;
    let (mut paid, mut unpaid) = (false, false);
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        paid |= matches!(effect, Effect::CounterEventAbility);
        unpaid |= matches!(effect, Effect::RemoveEventObjectFromCombat { .. });
    });
    assert_eq!(seen, 3);
    assert!(paid && unpaid);
}

#[test]
fn transform_history_operations_are_leaf_effects() {
    static EFFECTS: &[Effect] = &[
        Effect::DiscardAnyThenDraw,
        Effect::DiscoverAmount {
            mana_value: Amount::EventManaValue,
        },
        Effect::GraveyardToBattlefieldLinked {
            target: TargetSpec::ThisObject,
        },
    ];
    let mut seen = 0;
    Effect::walk(EFFECTS, &mut seen, &mut |_| {});
    assert_eq!(seen, 3);
    for effect in EFFECTS {
        assert_eq!(effect.branches(), (&[][..], &[][..]));
    }
}

#[test]
fn ltb_and_sacrifice_then_bodies_are_visited() {
    static INNER: Effect = Effect::SacrificeSelf;
    static EFFECTS: &[Effect] = &[
        Effect::PlayerMaySacrificeNThen {
            player: PlayerRel::You,
            filter: &Filter::LAND,
            count: 2,
            effect: &INNER,
        },
        Effect::WhenSourceLeavesBattlefieldForCreatedTokens {
            effects: &[Effect::SacrificeSelf],
        },
        Effect::WhenCreatedTokenLeavesBattlefield {
            effects: &[Effect::SacrificeSelf],
        },
        Effect::WhenTargetLeavesBattlefieldThisTurn {
            effects: &[Effect::SacrificeSelf],
        },
        Effect::WhenSourceLeavesBattlefieldThisTurn {
            effects: &[Effect::SacrificeSelf],
        },
        Effect::AnyPlayerMaySacrificeOrTakeDamageThen {
            filter: &Filter::LAND,
            count: 2,
            damage: 5,
            effects: &[Effect::SacrificeSelf],
        },
        Effect::WheneverTappedForManaThisTurn {
            by: PlayerRel::EachPlayer,
            filter: &Filter::LAND,
            effects: &[Effect::SacrificeSelf],
        },
        Effect::WhenTargetAttacksAndIsntBlockedThisTurn {
            effects: &[Effect::SacrificeSelf],
        },
        Effect::PlayerMayMillOr {
            player: PlayerRel::You,
            count: 2,
            effect: &INNER,
        },
        Effect::ForEachMatching {
            filter: &Filter::CREATURE,
            effects: &[Effect::SacrificeSelf],
        },
    ];
    let mut seen = 0;
    let mut inner_seen = 0;
    Effect::walk(EFFECTS, &mut seen, &mut |effect| {
        if matches!(effect, Effect::SacrificeSelf) {
            inner_seen += 1;
        }
    });
    assert_eq!(inner_seen, 10);
    assert_eq!(seen, 20);
}
