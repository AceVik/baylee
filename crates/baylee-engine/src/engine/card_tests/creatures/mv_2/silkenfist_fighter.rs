//! `cards/creatures/mv_2/silkenfist_fighter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn silkenfist_fighter_untaps_when_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[card_index("bf5ceba2-9def-4ff4-9291-740ca90c1c4d")])
        .battlefield(0, &[plains(); 2])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(
        &mut engine,
        p0,
        card_index("bf5ceba2-9def-4ff4-9291-740ca90c1c4d"),
    );
    pass_until(&mut engine, stack_is_empty);

    let fighter = on_battlefield(
        &engine,
        p0,
        card_index("bf5ceba2-9def-4ff4-9291-740ca90c1c4d"),
    )
    .unwrap();
    let blocker = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(fighter, Defender::Player(p1))],
            },
        )
        .unwrap();
    assert!(is_tapped(&engine, fighter));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, fighter)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, fighter));
}
