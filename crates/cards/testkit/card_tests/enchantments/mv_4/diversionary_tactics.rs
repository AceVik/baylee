//! `cards/enchantments/mv_4/diversionary_tactics.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn diversionary_tactics() -> CardIndex {
    card_index("880367e4-50b9-49d0-b596-d462d9b42855")
}

/// "Tap two untapped creatures you control: Tap target creature."
#[test]
fn diversionary_tactics_taps_two_own_creatures_to_tap_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[
            diversionary_tactics(),
            llanowar_elves(),
            llanowar_elves(),
            grizzly_bears(),
        ],
        &[grizzly_bears()],
        &[],
        &[],
    );
    let my_elves = all_on_battlefield(&e, p0, llanowar_elves());
    let opp_bear = lk::obj(&e, p1, grizzly_bears());

    activate(&mut e, p0, diversionary_tactics(), 0);
    lk::aim(&mut e, &[opp_bear], &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: my_elves.clone(),
        },
    )
    .unwrap();
    lk::settle(&mut e);

    assert!(lk::tapped(&e, my_elves[0]));
    assert!(lk::tapped(&e, my_elves[1]));
    assert!(lk::tapped(&e, opp_bear));
    // With only 1 untapped creature left (Grizzly Bears), activation is unavailable.
    assert!(!lk::offered(&e, diversionary_tactics(), p0, 0));

    // Opponent's tapped Bear cannot attack on P1's turn.
    reach_their_main_phase(&mut e, p1);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p0), 20);
}
