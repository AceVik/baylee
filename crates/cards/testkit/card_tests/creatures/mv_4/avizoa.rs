//! `cards/creatures/mv_4/avizoa.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn avizoa() -> CardIndex {
    card_index("91520db8-2d50-4c17-9f0e-2f92f63285f7")
}

/// Partial Avizoa retains its flying body. The optional pump is unavailable
/// until each activation can create an independent future untap obligation.
#[test]
fn avizoa_flies_over_a_ground_blocker_without_the_unsupported_pump() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[avizoa()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, avizoa());
    pass_until(&mut engine, stack_is_empty);
    let jellyfish = on_battlefield(&engine, p0, avizoa()).unwrap();
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(source, _)| *source == jellyfish),
        "the unsupported pump is not offered"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: jellyfish,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the pump cannot be activated by naming the missing ability"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        all_on_battlefield(&engine, p0, island())
            .into_iter()
            .all(|id| !is_tapped(&engine, id)),
        "the omitted activation creates no skipped untap"
    );
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(jellyfish, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bear, jellyfish)],
                },
            )
            .is_err(),
        "a ground creature cannot block the flying Avizoa"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "the unpumped body hits for 2"
    );
    assert_eq!(engine.state().players[0].life, 20);
}
