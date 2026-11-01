//! `cards/creatures/mv_4/snorting_gahr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "3e7f67d6-db12-4034-b8e3-9c9d401c8f57"
fn snorting_gahr() -> CardIndex {
    card_index("3e7f67d6-db12-4034-b8e3-9c9d401c8f57")
}

/// `Snorting Gahr` (`Coverage::Generated`):
/// "Whenever this creature becomes blocked, it gets +2/+2 until end of turn."
///
/// Verifies that when `Snorting Gahr` (3/3) becomes blocked, its triggered
/// ability gives it +2/+2 until end of turn, making it a 5/5.
#[test]
fn snorting_gahr_gets_plus_two_plus_two_when_it_becomes_blocked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9617, forest())
        .battlefield(0, &[snorting_gahr()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let gahr = on_battlefield(&engine, p0, snorting_gahr()).expect("P0 Snorting Gahr");
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("P1 Grizzly Bears");
    assert_eq!(pt(&engine, gahr), (3, 3));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(gahr, Defender::Player(p1))],
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
                blockers: vec![(bears, gahr)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, gahr),
        (5, 5),
        "Snorting Gahr gets +2/+2 until end of turn when blocked"
    );
}
