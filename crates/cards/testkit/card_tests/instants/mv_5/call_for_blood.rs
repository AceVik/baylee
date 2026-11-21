//! `cards/instants/mv_5/call_for_blood.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "f2782c21-ed99-4821-8938-cc539e783ec7"
fn call_for_blood() -> CardIndex {
    card_index("f2782c21-ed99-4821-8938-cc539e783ec7")
}

/// Call for Blood ({4}{B}): requires sacrificing a creature you control as an
/// additional cost, and gives target creature -X/-X until EOT where X is the
/// sacrificed creature's power (including pumps via LKI).
#[test]
fn call_for_blood_sacrifices_your_creature_and_shrinks_by_its_power() {
    let mut no_sac = lk::game(SEED, &[], &[index::SERRA_ANGEL], &[call_for_blood()], &[]);
    lk::float(&mut no_sac, P0, &[(ManaColor::Black, 5)]);
    assert!(
        !lk::castable(&no_sac, P0, call_for_blood()),
        "cannot cast without a creature you control to sacrifice"
    );

    let mut engine = lk::game(
        SEED,
        &[index::HILL_GIANT],
        &[index::SERRA_ANGEL],
        &[call_for_blood()],
        &[],
    );
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let angel = lk::obj(&engine, P1, index::SERRA_ANGEL);

    lk::float(&mut engine, P0, &[(ManaColor::Black, 5)]);
    cast_with_floating(&mut engine, P0, call_for_blood());
    lk::aim(&mut engine, &[angel], &[]);
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![giant],
                },
            )
            .unwrap();
    }
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P0, index::HILL_GIANT).is_some());
    assert_eq!(
        pt(&engine, angel),
        (1, 1),
        "4/4 Angel got -3/-3 from 3-power Giant"
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, angel), (4, 4), "-3/-3 wore off at EOT");
}
