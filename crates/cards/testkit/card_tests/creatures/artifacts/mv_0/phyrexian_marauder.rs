//! `cards/creatures/artifacts/mv_0/phyrexian_marauder.rs`, played.
//!
//! Oracle:
//! - "{X}: This creature enters with X +1/+1 counters on it." (mana cost `{X}`)
//! - "This creature can't block."
//! - "This creature can't attack unless you pay {1} for each +1/+1 counter on it."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn phyrexian_marauder() -> CardIndex {
    card_index("405674c5-607a-43d3-8b46-8741d2339f80")
}

#[test]
fn phyrexian_marauder_enters_with_x_counters_and_cant_block() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[index::SAVANNAH_LIONS])
        .hand(0, &[phyrexian_marauder(), phyrexian_marauder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // X = 0 puts 0 +1/+1 counters on a 0/0, which SBA moves to the graveyard.
    cast_with_floating(&mut engine, P0, phyrexian_marauder());
    assert!(matches!(engine.pending(), Pending::ChooseNumber { .. }));
    engine.apply(P0, PlayerAction::ChooseNumber(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, P0, phyrexian_marauder()).is_some(),
        "0/0 Marauder with X = 0 dies to state-based actions"
    );

    // X = 2 enters with two +1/+1 counters as a 2/2 artifact creature.
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, phyrexian_marauder());
    assert!(matches!(engine.pending(), Pending::ChooseNumber { .. }));
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let marauder = on_battlefield(&engine, P0, phyrexian_marauder()).expect("Marauder resolved");
    assert_eq!(counters_on(&engine, marauder, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, marauder), (2, 2));
    let t = types(&engine, marauder);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));
    assert!(keywords(&engine, marauder).contains(KeywordSet::CANT_BLOCK));

    // Even as a 2/2, Marauder cannot block when P1 attacks.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1),
    );
    let lions = on_battlefield(&engine, P1, index::SAVANNAH_LIONS).unwrap();
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(lions, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("stopped at ChooseBlockers");
    };
    assert!(
        blockers.iter().all(|o| o.blocker != marauder),
        "Phyrexian Marauder can't block: {blockers:?}"
    );
}

#[test]
fn phyrexian_marauder_attack_tax_equals_plus_one_counters() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[phyrexian_marauder()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, phyrexian_marauder());
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let marauder = on_battlefield(&engine, P0, phyrexian_marauder()).unwrap();

    // Next own turn: tap one Forest in FirstMain so its mana drains at step end,
    // leaving only 1 untapped Forest for combat (short of the {2} attack tax).
    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));
    let forests = all_on_battlefield(&engine, P0, forest());
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: forests[0] })
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("stopped at ChooseAttackers");
    };
    assert!(
        !attackers.contains(&marauder),
        "1 mana available cannot pay the {{2}} attack tax for 2 counters"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(marauder, Defender::Player(P1))],
                },
            )
            .is_err(),
        "engine refuses declaring Marauder when tax cannot be paid"
    );
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();

    // Next own turn: leave both Forests untapped, enter CR 508.1i tax window on attack, tap {2}, pass to pay, and hit for 2.
    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("stopped at ChooseAttackers");
    };
    assert!(attackers.contains(&marauder));
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(marauder, Defender::Player(P1))],
            },
        )
        .unwrap();
    tap_all_mana(&mut engine, P0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "declaring Marauder with 2 counters spends {{2}}"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 18);
}
