//! `cards/creatures/mv_2/rock_hydra.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rock Hydra: it enters with X +1/+1 counters, and "{R}: Prevent the next
/// 1 damage that would be dealt to this creature this turn." X = 3, the
/// shield bought, then a Lightning Bolt at it: the shield prevents 1 of the
/// 3, and each of the other 2 takes a +1/+1 counter instead of being dealt
/// (its controller chooses the shield first, CR 616.1).
#[test]
fn rock_hydra_enters_with_x_counters_and_buys_a_shield() {
    let p0 = PlayerId::new(0);
    let hydra_card = card_index("aff84707-f5f8-4f53-869e-feec78da8d8d");
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 7])
        .hand(0, &[hydra_card, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, hydra_card);
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let hydra = on_battlefield(&engine, p0, hydra_card).expect("the Hydra entered");
    assert_eq!(pt(&engine, hydra), (3, 3), "three +1/+1 counters on a 0/0");

    activate(&mut engine, p0, hydra_card, 0);
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![hydra],
                players: vec![],
            },
        )
        .expect("the Hydra is a legal target");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageEffect { .. })
    });
    let Pending::ChooseDamageEffect {
        player,
        choice,
        options,
        ..
    } = engine.pending().clone()
    else {
        panic!("the Hydra's controller chooses prevention order");
    };
    assert_eq!(player, p0);
    let effect = options
        .iter()
        .find(|option| {
            matches!(
                option.kind,
                crate::choice::DamageEffectKind::PreventNext { .. }
            )
        })
        .unwrap()
        .id;
    engine
        .apply(player, PlayerAction::ChooseDamageEffect { choice, effect })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(hydra).map(|o| (o.zone, o.damage)),
        Some((Zone::Battlefield, 0)),
        "1 of the Bolt's 3 prevented by the shield, 2 by counters"
    );
    assert_eq!(pt(&engine, hydra), (1, 1), "one counter left");
}

// ---------------------------------------------------------------------------
// Rock Hydra.
// ---------------------------------------------------------------------------

/// Rock Hydra's counter ability, without any shield in play: "For each 1
/// damage that would be dealt to this creature, if it has a +1/+1 counter
/// on it, remove a +1/+1 counter from it and prevent that 1 damage." A
/// three-counter Hydra hit by a Lightning Bolt has all three damage points
/// prevented — no `DamageDealt` event reaches it at all — but a 0/0 with
/// no counters left still dies (CR 704.5f), not from the Bolt's damage but
/// from its own toughness. With only one counter to spend, the same Bolt
/// still marks it for the other 2 (a journaled `DamageDealt` of exactly
/// that amount) before it dies the same way.
#[allow(clippy::too_many_lines)] // two counter counts, each read from the journal
#[test]
fn rock_hydra_s_counters_prevent_one_point_of_damage_each() {
    let p0 = PlayerId::new(0);
    let hydra_card = card_index("aff84707-f5f8-4f53-869e-feec78da8d8d");

    // Three counters absorb all three of the Bolt's damage points. The
    // counters are seeded before the first mulligan question, not after:
    // the printed body is a 0/0, and a state-based action would otherwise
    // bury it (CR 704.5f) the moment the engine next checks, before this
    // test ever gets to add any.
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[hydra_card, mountain(), mountain(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    let hydra = on_battlefield(&engine, p0, hydra_card).expect("the Hydra is seated");
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        state
            .object_mut(hydra)
            .expect("seated")
            .counters
            .add(CounterKind::P1P1, 3);
    }
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(pt(&engine, hydra), (3, 3), "three +1/+1 counters on a 0/0");

    let before = engine.journal().entries().len();
    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![hydra],
                players: vec![],
            },
        )
        .expect("the Hydra is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine.journal().entries()[before..]
            .iter()
            .any(|e| matches!(
                e.event,
                crate::event::GameEvent::CounterChanged {
                    object,
                    kind: CounterKind::P1P1,
                    old: 3,
                    new: 0,
                } if object == hydra
            )),
        "all three counters were removed absorbing the Bolt's damage \
         (so the negative below cannot pass on a Bolt that never \
         touched the Hydra)"
    );
    assert!(
        !engine.journal().entries()[before..]
            .iter()
            .any(|e| matches!(
                e.event,
                crate::event::GameEvent::DamageDealt {
                    target: crate::event::DamageTarget::Object(hit),
                    ..
                } if hit == hydra
            )),
        "all 3 of the Bolt's damage was prevented: no `DamageDealt` event \
         ever names the Hydra"
    );
    assert!(
        in_graveyard(&engine, p0, hydra_card).is_some(),
        "0 counters left is a printed 0/0 (CR 704.5f): it dies either way, \
         so this alone doesn't distinguish working prevention from a \
         mechanic that never fired at all — the CounterChanged and \
         DamageDealt checks above are what prove it"
    );

    // One counter absorbs 1 of the Bolt's 3; the other 2 are marked.
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[hydra_card, mountain(), mountain(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    let hydra = on_battlefield(&engine, p0, hydra_card).expect("the Hydra is seated");
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        state
            .object_mut(hydra)
            .expect("seated")
            .counters
            .add(CounterKind::P1P1, 1);
    }
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(pt(&engine, hydra), (1, 1), "one +1/+1 counter on a 0/0");

    let before = engine.journal().entries().len();
    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![hydra],
                players: vec![],
            },
        )
        .expect("the Hydra is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine.journal().entries()[before..]
            .iter()
            .any(|e| matches!(
                e.event,
                crate::event::GameEvent::DamageDealt {
                    target: crate::event::DamageTarget::Object(hit),
                    amount: 2,
                    ..
                } if hit == hydra
            )),
        "the one counter stopped only 1 of the Bolt's 3: the Hydra is \
         marked for the other 2"
    );
    assert!(
        in_graveyard(&engine, p0, hydra_card).is_some(),
        "0 counters left is a printed 0/0 (CR 704.5f): it dies anyway, \
         just like the three-counter case above — this doesn't prove the \
         one-counter absorption worked, only the DamageDealt check above \
         does that"
    );
}

/// Rock Hydra's third ability, "{R}{R}{R}: Put a +1/+1 counter on this
/// creature. Activate only during your upkeep" (ability index 2): offered
/// and working in its controller's own upkeep; not offered once its
/// controller's main phase begins, and not offered during the opponent's
/// upkeep either (the "your" half of the restriction, not only the
/// "upkeep" half) — the engine refuses the activation in both windows,
/// even by address.
#[allow(clippy::too_many_lines)] // three windows, each read from a fresh floating pool
#[test]
fn rock_hydra_s_growth_ability_activates_only_in_its_controllers_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let hydra_card = card_index("aff84707-f5f8-4f53-869e-feec78da8d8d");
    // Nine Mountains, not three: three pay the ability's cost in the
    // upkeep half below, three more in the main-phase half, and the last
    // three are held back, untapped, for the opponent's-upkeep half —
    // so none of the three "offered"/"not offered" readings is a reading
    // of mana affordability instead of the timing restriction (this
    // engine's activation offer reads the floating pool, not untapped
    // sources — `Engine::can_pay_mana` — so an empty pool alone would
    // also refuse the ability, timing restriction or not).
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                hydra_card,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    // The counter is seeded before the first mulligan question, not after:
    // the printed body is a 0/0, and a state-based action would otherwise
    // bury it (CR 704.5f) before this test ever gets to add one.
    let hydra = on_battlefield(&engine, p0, hydra_card).expect("the Hydra is seated");
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        state
            .object_mut(hydra)
            .expect("seated")
            .counters
            .add(CounterKind::P1P1, 1);
    }
    keep_mulligans(&mut engine);
    let mountains = all_on_battlefield(&engine, p0, mountain());
    assert_eq!(mountains.len(), 9, "nine Mountains are seated");

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    for &source in &mountains[..3] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    assert!(
        priority_offer(&engine).abilities.contains(&(hydra, 2)),
        "\"Activate only during your upkeep\": offered right here, with \
         its {{R}}{{R}}{{R}} floating"
    );

    activate(&mut engine, p0, hydra_card, 2);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, hydra),
        (2, 2),
        "the ability worked: a second +1/+1 counter"
    );

    reach_main_phase(&mut engine, p0);
    for &source in &mountains[3..6] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        3,
        "three more Mountains are floating {{R}}{{R}}{{R}}: the refusal \
         below is not for want of mana"
    );
    assert!(
        !priority_offer(&engine).abilities.contains(&(hydra, 2)),
        "not offered once its controller's main phase begins, mana \
         floating or not"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: hydra,
                    ability_index: 2,
                },
            )
            .is_err(),
        "the window is the engine's rule, not only the offer's"
    );

    // "Your" upkeep, not any upkeep: the opponent's upkeep (turn 2) is
    // still a window this controller could float mana in, and still not
    // the right one.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    for &source in &mountains[6..9] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        3,
        "the last three Mountains are floating {{R}}{{R}}{{R}} during the \
         opponent's upkeep: the refusal below is not for want of mana"
    );
    assert!(
        !priority_offer(&engine).abilities.contains(&(hydra, 2)),
        "\"your upkeep\": not offered in the opponent's upkeep, mana \
         floating or not"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: hydra,
                    ability_index: 2,
                },
            )
            .is_err(),
        "the window is the engine's rule, not only the offer's"
    );
}
