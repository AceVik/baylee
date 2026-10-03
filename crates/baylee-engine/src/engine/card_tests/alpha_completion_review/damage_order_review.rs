//! Independent real-card decisions for CR 616.1 and CR 615.7.

#[allow(clippy::wildcard_imports)] // Shared behavioral card-test vocabulary.
use super::*;
use crate::choice::{DamageChoiceId, DamageEffectKind};
use crate::event::DamageTarget;

fn leak() -> CardIndex {
    card_index("dc2f0000-870b-487f-9623-618fc8eb9765")
}

fn host() -> CardIndex {
    card_index("a2310312-6e1e-4e34-a351-9aef499a810f")
}

fn salve() -> CardIndex {
    card_index("8da8644c-75a1-4fe9-8e94-900d948d631c")
}

fn reverse() -> CardIndex {
    card_index("eaaf7c30-f463-4115-a40e-7dc717063413")
}

fn salve_target(engine: &mut Engine<RegistryLookup>, player: PlayerId, target: DamageTarget) {
    cast_from_hand(engine, player, salve());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("Healing Salve's prevention mode must be chosen");
    };
    let slot = options
        .iter()
        .position(|o| o.kind == CastModeKind::Mode(1))
        .unwrap();
    engine
        .apply(player, PlayerAction::ChooseMode(slot))
        .unwrap();
    match target {
        DamageTarget::Player(player) => aim(engine, vec![], vec![player]),
        DamageTarget::Object(object) => aim(engine, vec![object], vec![]),
    }
}

fn reverse_source(engine: &mut Engine<RegistryLookup>, player: PlayerId, source: ObjectId) {
    cast_from_hand(engine, player, reverse());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![source],
            },
        )
        .unwrap();
}

fn choose_effect(
    engine: &mut Engine<RegistryLookup>,
    player: PlayerId,
    kind: impl Fn(DamageEffectKind) -> bool,
) -> PlayerAction {
    let Pending::ChooseDamageEffect {
        player: chooser,
        choice,
        options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the affected player must choose the damage effect: {:?}",
            engine.pending()
        );
    };
    assert_eq!(chooser, player);
    let effect = options.iter().find(|option| kind(option.kind)).unwrap().id;
    let action = PlayerAction::ChooseDamageEffect { choice, effect };
    engine.apply(player, action.clone()).unwrap();
    action
}

fn leak_with_shield(shield: CardIndex) -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), mountain()])
        .hand(0, &[leak(), lightning_bolt()])
        .battlefield(1, &[host(), plains(), plains(), plains(), plains()])
        .hand(1, &[shield])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let attached = on_battlefield(&engine, p1, host()).unwrap();
    for source in all_on_battlefield(&engine, p0, island()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    cast_with_floating(&mut engine, p0, leak());
    aim(&mut engine, vec![attached], vec![]);
    pass_until(&mut engine, stack_is_empty);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && !e.state().zones.stack_is_empty()
    });
    engine
}

fn pay_one_for_leak(engine: &mut Engine<RegistryLookup>) {
    let p1 = PlayerId::new(1);
    pass_until(engine, |e| e.payment_window().is_some());
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    engine.apply(p1, PlayerAction::ChooseNumber(1)).unwrap();
}

#[test]
fn power_leak_reverse_damage_both_legal_orders_gain_one_or_two_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for leak_first in [true, false] {
        let mut engine = leak_with_shield(reverse());
        let aura = on_battlefield(&engine, p0, leak()).unwrap();
        reverse_source(&mut engine, p1, aura);
        pay_one_for_leak(&mut engine);
        assert_eq!(
            engine.state().players[1].life,
            20,
            "no order has been chosen yet"
        );
        let Pending::ChooseDamageEffect {
            damage, options, ..
        } = engine.pending()
        else {
            panic!("overlapping prevention is a decision");
        };
        assert!(
            damage
                .iter()
                .any(|part| part.source == aura && part.amount == 2)
        );
        assert!(options.iter().any(|option| matches!(
            option.kind,
            DamageEffectKind::PreventThisEvent { remaining: 1 }
        )));
        assert!(options.iter().any(|option| matches!(option.kind, DamageEffectKind::PreventFromSource { source, gain_life: true, .. } if source == aura)));
        choose_effect(&mut engine, p1, |kind| {
            if leak_first {
                matches!(kind, DamageEffectKind::PreventThisEvent { .. })
            } else {
                matches!(
                    kind,
                    DamageEffectKind::PreventFromSource {
                        gain_life: true,
                        ..
                    }
                )
            }
        });
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().players[1].life,
            if leak_first { 21 } else { 22 }
        );
        assert_eq!(
            engine.state().players[1].mana_pool.total(),
            0,
            "the same one mana was paid in both orders"
        );
    }
}

#[test]
fn power_leak_salve_order_preserves_two_or_one_prevention_for_later_bolt() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for leak_first in [true, false] {
        let mut engine = leak_with_shield(salve());
        salve_target(&mut engine, p1, DamageTarget::Player(p1));
        pay_one_for_leak(&mut engine);
        choose_effect(&mut engine, p1, |kind| {
            if leak_first {
                matches!(kind, DamageEffectKind::PreventThisEvent { .. })
            } else {
                matches!(kind, DamageEffectKind::PreventNext { .. })
            }
        });
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(engine.state().players[1].life, 20);
        pass_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
        );
        cast_from_hand(&mut engine, p0, lightning_bolt());
        aim(&mut engine, vec![], vec![p1]);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().players[1].life,
            if leak_first { 19 } else { 18 }
        );
        assert_eq!(engine.state().players[0].life, 20);
    }
}

fn rejected_without_change(
    engine: &mut Engine<RegistryLookup>,
    player: PlayerId,
    action: PlayerAction,
) {
    let fingerprint = engine.fingerprint();
    let pending = serde_json::to_value(engine.pending()).unwrap();
    let journal = engine.state().journal.len();
    assert!(engine.apply(player, action).is_err());
    assert_eq!(engine.fingerprint(), fingerprint);
    assert_eq!(serde_json::to_value(engine.pending()).unwrap(), pending);
    assert_eq!(engine.state().journal.len(), journal);
}

#[test]
fn power_leak_order_rejects_wrong_player_unknown_effect_and_stale_choice_atomically() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = leak_with_shield(reverse());
    let aura = on_battlefield(&engine, p0, leak()).unwrap();
    reverse_source(&mut engine, p1, aura);
    pay_one_for_leak(&mut engine);
    let Pending::ChooseDamageEffect {
        choice, options, ..
    } = engine.pending().clone()
    else {
        panic!("overlapping shields must offer an effect");
    };
    let valid = PlayerAction::ChooseDamageEffect {
        choice,
        effect: options[0].id,
    };
    rejected_without_change(&mut engine, p0, valid);
    rejected_without_change(
        &mut engine,
        p1,
        PlayerAction::ChooseDamageEffect {
            choice,
            effect: u32::MAX,
        },
    );
    rejected_without_change(
        &mut engine,
        p1,
        PlayerAction::ChooseDamageEffect {
            choice: DamageChoiceId {
                batch: choice.batch.wrapping_add(1),
                step: choice.step,
            },
            effect: options[0].id,
        },
    );
    let old = choose_effect(&mut engine, p1, |kind| {
        matches!(kind, DamageEffectKind::PreventThisEvent { .. })
    });
    pass_until(&mut engine, stack_is_empty);
    rejected_without_change(&mut engine, p1, old);
    assert_eq!(engine.state().players[1].life, 21);
}

#[test]
fn jade_monolith_redirection_transfers_the_effect_choice_to_the_new_recipient() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let jade = card_index("1e105ab7-fb10-4cfd-ac2f-5e11488cf1b0");
    let mut engine = Duel::table(SEED, mountain(), 3)
        .battlefield(0, &[mountain()])
        .hand(0, &[lightning_bolt()])
        .battlefield(1, &[quiet_creature(), plains()])
        .hand(1, &[salve()])
        .battlefield(2, &[jade, plains(), plains(), plains(), plains(), plains()])
        .hand(2, &[salve(), reverse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let body = on_battlefield(&engine, p1, quiet_creature()).unwrap();
    for (player, target) in [
        (p1, DamageTarget::Object(body)),
        (p2, DamageTarget::Player(p2)),
    ] {
        pass_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::Priority { player: p, .. } if *p == player),
        );
        salve_target(&mut engine, player, target);
        pass_until(&mut engine, stack_is_empty);
    }
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim(&mut engine, vec![body], vec![]);
    let bolt = top(&engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p2),
    );
    activate(&mut engine, p2, jade, 0);
    aim(&mut engine, vec![body], vec![]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p2,
            PlayerAction::ChooseObjects {
                objects: vec![bolt],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p2),
    );
    reverse_source(&mut engine, p2, bolt);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageEffect { .. })
    });
    choose_effect(
        &mut engine,
        p1,
        |kind| matches!(kind, DamageEffectKind::Redirect { to: DamageTarget::Player(player) } if player == p2),
    );
    assert!(
        matches!(engine.pending(), Pending::ChooseDamageEffect { player, damage, .. }
        if *player == p2 && damage.iter().any(|part| part.recipient == DamageTarget::Player(p2)))
    );
    choose_effect(&mut engine, p2, |kind| {
        matches!(
            kind,
            DamageEffectKind::PreventFromSource {
                gain_life: true,
                ..
            }
        )
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[2].life, 23);
    assert_eq!(engine.state().object(body).unwrap().damage, 0);
    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn fireball_simultaneous_damage_effect_choices_follow_apnap_from_active_seat_one() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let fireball = card_index("aa7714b0-2bfb-458a-8ebf-37ec2c53383e");
    let mut lands = vec![plains(); 11];
    lands.extend([mountain(); 5]);
    let mut engine = Duel::table(SEED, mountain(), 3)
        .battlefield(0, &[plains(); 4])
        .hand(0, &[salve(), reverse()])
        .battlefield(1, &lands)
        .hand(1, &[fireball, salve(), reverse()])
        .battlefield(2, &[plains(); 4])
        .hand(2, &[salve(), reverse()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    for player in [p1, p2, p0] {
        pass_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::Priority { player: p, .. } if *p == player),
        );
        salve_target(&mut engine, player, DamageTarget::Player(player));
        pass_until(&mut engine, stack_is_empty);
    }
    assert_eq!(engine.state().turn.phase, Phase::FirstMain);
    cast_from_hand(&mut engine, p1, fireball);
    engine.apply(p1, PlayerAction::ChooseNumber(6)).unwrap();
    aim(&mut engine, vec![], vec![p2, p0, p1]);
    let source = top(&engine);
    for player in [p1, p2, p0] {
        pass_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::Priority { player: p, .. } if *p == player),
        );
        reverse_source(&mut engine, player, source);
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageEffect { .. })
    });
    let mut previous = None;
    for player in [p1, p2, p0] {
        assert!(
            engine.state().players.iter().all(|seat| seat.life == 20),
            "the simultaneous event is not committed while its choices remain"
        );
        if let Some(old) = previous.take() {
            rejected_without_change(&mut engine, player, old);
        }
        previous = Some(choose_effect(&mut engine, player, |kind| {
            if player == p2 {
                matches!(kind, DamageEffectKind::PreventNext { .. })
            } else {
                matches!(
                    kind,
                    DamageEffectKind::PreventFromSource {
                        gain_life: true,
                        ..
                    }
                )
            }
        }));
    }
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 22);
    assert_eq!(engine.state().players[1].life, 22);
    assert_eq!(engine.state().players[2].life, 20);
}

/// A finite shield must be assigned by the defending controller, because
/// which source gets through changes deathtouch and lifelink (CR 615.7).
#[allow(clippy::too_many_lines)] // Two complete allocations and their refusal controls share one combat setup.
#[test]
fn healing_salve_simultaneous_sources_allow_deathtouch_or_lifelink_to_be_prevented() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let giant_card = card_index("38aa31bd-7145-43b9-9409-463d9ad6cd69");
    let specialist_card = card_index("4164034a-5e59-4e40-a150-2c1000b0bd0d");
    let jump_card = card_index("f7518456-45ed-41d4-bd3c-5aacea28eb35");
    for prevent_deathtouch in [true, false] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[baleful_strix(), specialist_card])
            .battlefield(1, &[giant_card, island(), plains()])
            .hand(1, &[jump_card, salve()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let strix = on_battlefield(&engine, p0, baleful_strix()).unwrap();
        let specialist = on_battlefield(&engine, p0, specialist_card).unwrap();
        let giant = on_battlefield(&engine, p1, giant_card).unwrap();
        pass_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
        );
        cast_from_hand(&mut engine, p1, jump_card);
        aim(&mut engine, vec![giant], vec![]);
        pass_until(&mut engine, stack_is_empty);
        pass_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
        );
        salve_target(&mut engine, p1, DamageTarget::Object(giant));
        pass_until(&mut engine, stack_is_empty);
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![
                        (strix, Defender::Player(p1)),
                        (specialist, Defender::Player(p1)),
                    ],
                },
            )
            .unwrap();
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseBlockers { .. })
        });
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(giant, strix), (giant, specialist)],
                },
            )
            .unwrap();
        for _ in 0..20 {
            match engine.pending().clone() {
                Pending::AllocatePrevention { .. } => break,
                Pending::Priority { player, .. } => {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                Pending::ChooseNumber {
                    player,
                    reason: crate::choice::NumberPrompt::CombatDamage { recipient, .. },
                    ..
                } => {
                    engine
                        .apply(
                            player,
                            PlayerAction::ChooseNumber(if recipient == strix { 1 } else { 3 }),
                        )
                        .unwrap();
                }
                pending => panic!("unexpected before simultaneous prevention: {pending:?}"),
            }
        }
        let Pending::AllocatePrevention {
            player,
            choice,
            damage,
            total,
            ..
        } = engine.pending().clone()
        else {
            panic!("Salve must allow a source allocation");
        };
        assert_eq!(player, p1);
        assert_eq!(total, 3);
        let dt = damage.iter().find(|part| part.source == strix).unwrap();
        let ll = damage
            .iter()
            .find(|part| part.source == specialist)
            .unwrap();
        assert_eq!((dt.amount, ll.amount), (1, 3));
        assert_eq!(engine.state().object(giant).unwrap().damage, 0);
        let valid = if prevent_deathtouch {
            vec![(dt.id, 1), (ll.id, 2)]
        } else {
            vec![(ll.id, 3)]
        };
        rejected_without_change(
            &mut engine,
            p0,
            PlayerAction::AllocatePrevention {
                choice,
                allocation: valid.clone(),
            },
        );
        for allocation in [
            vec![(u32::MAX, 3)],
            vec![(dt.id, 1), (dt.id, 2)],
            vec![(dt.id, 2), (ll.id, 1)],
            vec![(ll.id, 2)],
            vec![(dt.id, 1), (ll.id, 3)],
        ] {
            rejected_without_change(
                &mut engine,
                p1,
                PlayerAction::AllocatePrevention { choice, allocation },
            );
        }
        rejected_without_change(
            &mut engine,
            p1,
            PlayerAction::AllocatePrevention {
                choice: DamageChoiceId {
                    batch: choice.batch,
                    step: choice.step.wrapping_add(1),
                },
                allocation: valid.clone(),
            },
        );
        let old = PlayerAction::AllocatePrevention {
            choice,
            allocation: valid,
        };
        engine.apply(p1, old.clone()).unwrap();
        rejected_without_change(&mut engine, p1, old);
        assert_eq!(
            engine.state().players[0].life,
            if prevent_deathtouch { 21 } else { 20 }
        );
        if prevent_deathtouch {
            assert_eq!(
                engine.state().object(giant).unwrap().zone,
                Zone::Battlefield
            );
            assert_eq!(engine.state().object(giant).unwrap().damage, 1);
        } else {
            assert!(
                in_graveyard(&engine, p1, giant_card).is_some(),
                "the surviving one point was deathtouch"
            );
        }
    }
}
