//! `cards/creatures/mv_4/kavu_runner.rs`, played.
//!
//! Oracle:
//! - "Kavu Runner has haste as long as no opponent controls a white or blue
//!   creature."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn kavu_runner() -> CardIndex {
    card_index("2c90b7fa-d5bf-4c46-92d5-3014ea1bec50")
}

#[test]
fn kavu_runner_haste_gates_on_opponent_white_creature_and_ignores_noncreature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[plains(), plains(), plains(), index::GRIZZLY_BEARS])
        .hand(0, &[kavu_runner(), kavu_runner(), index::SHOCK])
        .hand(1, &[index::SAVANNAH_LIONS, index::PACIFISM])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // No opponent white/blue creature (only green Bears): cast Kavu Runner has Haste and attacks immediately.
    let mountains = all_on_battlefield(&engine, P0, mountain());
    for &m in &mountains[..4] {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: m })
            .unwrap();
    }
    cast_with_floating(&mut engine, P0, kavu_runner());
    pass_until(&mut engine, stack_is_empty);
    let r1 = on_battlefield(&engine, P0, kavu_runner()).unwrap();
    assert_eq!(pt(&engine, r1), (3, 3));
    assert!(keywords(&engine, r1).contains(KeywordSet::HASTE));
    attack_and_collect_blocks(&mut engine, r1, P1);
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 17);

    // On P1's turn: casting a white noncreature (Pacifism on Bears) keeps Haste on r1,
    // then casting a white creature (Savannah Lions) immediately removes Haste.
    reach_their_main_phase(&mut engine, P1);
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, index::PACIFISM);
    aim_at(&mut engine, P1, bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, r1).contains(KeywordSet::HASTE));

    cast_with_floating(&mut engine, P1, index::SAVANNAH_LIONS);
    pass_until(&mut engine, stack_is_empty);
    assert!(!keywords(&engine, r1).contains(KeywordSet::HASTE));

    // P0 Turn 3: second Kavu Runner enters without Haste while Lions lives; Shocking Lions restores Haste!
    assert!(walk_to_own_main(&mut engine, P0));
    let mountains = all_on_battlefield(&engine, P0, mountain());
    for &m in &mountains[..4] {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: m })
            .unwrap();
    }
    cast_with_floating(&mut engine, P0, kavu_runner());
    pass_until(&mut engine, stack_is_empty);
    let r2 = all_on_battlefield(&engine, P0, kavu_runner())
        .into_iter()
        .find(|id| *id != r1)
        .unwrap();
    assert!(!keywords(&engine, r2).contains(KeywordSet::HASTE));

    let lions = on_battlefield(&engine, P1, index::SAVANNAH_LIONS).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ActivateManaAbility {
                source: mountains[4],
            },
        )
        .unwrap();
    cast_with_floating(&mut engine, P0, index::SHOCK);
    aim_at(&mut engine, P0, lions);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, r2).contains(KeywordSet::HASTE));

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("stopped on ChooseAttackers");
    };
    assert!(
        attackers.contains(&r2),
        "regained Haste lets new Runner attack"
    );
}

#[test]
fn kavu_runner_loses_haste_when_opponent_controls_blue_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[index::PHANTOM_MONSTER])
        .hand(0, &[kavu_runner()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, kavu_runner());
    pass_until(&mut engine, stack_is_empty);
    let runner = on_battlefield(&engine, P0, kavu_runner()).unwrap();
    assert!(!keywords(&engine, runner).contains(KeywordSet::HASTE));

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("stopped on ChooseAttackers");
    };
    assert!(!attackers.contains(&runner));
}
