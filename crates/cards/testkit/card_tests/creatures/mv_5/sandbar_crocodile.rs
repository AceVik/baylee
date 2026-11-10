//! `cards/creatures/mv_5/sandbar_crocodile.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sandbar_crocodile() -> CardIndex {
    card_index("e11926bf-ca14-4e1e-aca5-9ffedb91868c")
}

/// Sandbar Crocodile (`{4}{U}`, `Coverage::Implemented`): 6/5 Creature —
/// Crocodile with phasing.
#[test]
fn sandbar_crocodile_has_6_5_body_and_phases_out_and_in() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(); 5])
        .hand(0, &[sandbar_crocodile()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, sandbar_crocodile());
    pass_until(&mut engine, stack_is_empty);
    let croc = on_battlefield(&engine, p0, sandbar_crocodile()).unwrap();
    assert_eq!(pt(&engine, croc), (6, 5));
    assert!(keywords(&engine, croc).contains(KeywordSet::PHASING));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        engine
            .state()
            .object(croc)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT),
        "phased out on controller's next turn"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !engine
            .state()
            .object(croc)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT),
        "phases back in on the turn after"
    );
}
