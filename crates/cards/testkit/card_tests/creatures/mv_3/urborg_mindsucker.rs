//! `cards/creatures/mv_3/urborg_mindsucker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_engine::turn::Step;

fn urborg_mindsucker() -> CardIndex {
    card_index("19d11ca8-a95b-4e6e-a71f-8f9d06c580fd")
}

/// Urborg Mindsucker ({2}{B} 2/2 Horror):
/// "{B}, Sacrifice this creature: Target opponent discards a card at random.
/// Activate only as a sorcery."
#[test]
fn urborg_mindsucker_sacrifices_at_sorcery_speed_to_make_target_opponent_discard_at_random() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[urborg_mindsucker(), index::SWAMP])
        .hand(0, &[index::SHOCK])
        .hand(1, &[plains(), mountain()])
        .start();
    keep_mulligans(&mut e);

    // Not legal in Upkeep (sorcery speed).
    lk::to_step(&mut e, lk::P0, Step::Upkeep);
    lk::float(&mut e, lk::P0, &[(ManaColor::Black, 1)]);
    assert!(!lk::offered(&e, urborg_mindsucker(), lk::P0, 0));

    reach_main_phase(&mut e, lk::P0);
    lk::float(
        &mut e,
        lk::P0,
        &[(ManaColor::Black, 1), (ManaColor::Red, 1)],
    );
    assert!(lk::offered(&e, urborg_mindsucker(), lk::P0, 0));

    // Not legal while a spell is on the stack (sorcery speed).
    lk::announce(&mut e, lk::P0, index::SHOCK, 0, 0, &[], &[lk::P1]);
    assert!(!lk::offered(&e, urborg_mindsucker(), lk::P0, 0));
    lk::settle(&mut e);

    // Legal again on an empty stack in FirstMain: random discard asks P1 no choice.
    assert!(lk::offered(&e, urborg_mindsucker(), lk::P0, 0));
    lk::use_ability(&mut e, lk::P0, urborg_mindsucker(), 0, &[], &[lk::P1]);
    assert!(on_battlefield(&e, lk::P0, urborg_mindsucker()).is_none());
    assert!(in_graveyard(&e, lk::P0, urborg_mindsucker()).is_some());
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(lk::P1)).len(), 1);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Graveyard(lk::P1)).len(),
        1
    );
}
