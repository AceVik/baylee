//! `cards/creatures/mv_2/bloodcurdler.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_engine::turn::Step;

fn bloodcurdler() -> CardIndex {
    card_index("3f53fd3e-9116-4599-81df-a9390461b549")
}

/// Bloodcurdler ({1}{B} 1/1 Horror):
/// "Flying
/// At the beginning of your upkeep, mill a card.
/// Threshold — As long as there are seven or more cards in your graveyard, this
/// creature gets +1/+1 and has 'At the beginning of your end step, exile two
/// cards from your graveyard.'"
#[test]
fn bloodcurdler_mills_on_upkeep_and_with_threshold_gets_pump_and_end_step_exile() {
    let mut e = lk::game(
        SEED,
        &[
            bloodcurdler(),
            plains(),
            plains(),
            plains(),
            plains(),
            plains(),
        ],
        &[],
        &[],
        &[],
    );
    let horror = lk::obj(&e, lk::P0, bloodcurdler());
    assert!(keywords(&e, horror).contains(KeywordSet::FLYING));
    assert_eq!(
        cards_in(&e, ZoneLocation::Graveyard(lk::P0)),
        1,
        "Turn 1 upkeep milled 1 card"
    );
    assert_eq!(pt(&e, horror), (1, 1));

    // Seed 5 Plains into graveyard -> 6 total (< 7): still 1/1 and no end-step exile.
    for p in all_on_battlefield(&e, lk::P0, plains()) {
        e.dev_state_mut(lk::P0)
            .unwrap()
            .move_object(
                p,
                ZoneLocation::Graveyard(lk::P0),
                baylee_engine::zone::ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .unwrap();
    }
    e.apply(lk::P0, PlayerAction::PassPriority).unwrap();
    assert_eq!(cards_in(&e, ZoneLocation::Graveyard(lk::P0)), 6);
    assert_eq!(pt(&e, horror), (1, 1));

    // Pass through P1's turn to P0's next FirstMain: upkeep mills card #7 -> Threshold!
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    assert_eq!(cards_in(&e, ZoneLocation::Graveyard(lk::P0)), 7);
    assert_eq!(pt(&e, horror), (2, 2));

    // At P0's end step, mandatory exile of 2 graveyard cards drops count to 5 -> back to 1/1.
    pass_until(&mut e, |e| {
        e.state().turn.step == Step::End && matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!((min, max), (2, 2));
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![options[0], options[1]],
        },
    )
    .unwrap();
    lk::settle(&mut e);

    assert_eq!(cards_in(&e, ZoneLocation::Graveyard(lk::P0)), 5);
    assert_eq!(cards_in(&e, ZoneLocation::Exile(lk::P0)), 2);
    assert_eq!(pt(&e, horror), (1, 1));
}
