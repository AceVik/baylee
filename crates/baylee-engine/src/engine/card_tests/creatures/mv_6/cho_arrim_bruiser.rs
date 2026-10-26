//! `cards/creatures/mv_6/cho_arrim_bruiser.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "3db407ec-da4b-4b25-b613-e45c164ffe86"
fn cho_arrim_bruiser() -> CardIndex {
    card_index("3db407ec-da4b-4b25-b613-e45c164ffe86")
}

/// Cho-Arrim Bruiser ({5}{W} 3/4 Creature — Ogre Rebel):
/// "Whenever this creature attacks, you may tap up to two target creatures."
#[test]
fn cho_arrim_bruiser_taps_up_to_two_target_creatures_when_attacking() {
    let mut engine = lk::game(
        SEED,
        &[cho_arrim_bruiser()],
        &[index::GRIZZLY_BEARS, index::HILL_GIANT],
        &[],
        &[],
    );
    let bruiser = lk::obj(&engine, P0, cho_arrim_bruiser());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    let able = lk::attackers_offered(&mut engine);
    assert!(able.contains(&bruiser));
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bruiser, Defender::Player(P1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut engine, &[bears, giant], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, true);

    lk::through_combat(&mut engine);

    assert!(
        lk::tapped(&engine, bears),
        "first target creature was tapped"
    );
    assert!(
        lk::tapped(&engine, giant),
        "second target creature was tapped"
    );
    assert_eq!(
        lk::life(&engine, P1),
        17,
        "with both potential blockers tapped before declare-blockers, Cho-Arrim Bruiser dealt 3 combat damage"
    );
}
