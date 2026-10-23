//! `cards/creatures/mv_3/shimmering_efreet.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn shimmering_efreet() -> CardIndex {
    card_index("34f52d00-301d-4065-a86b-8393b4c6e99d")
}

/// `Shimmering Efreet` ({2}{U}, 2/2 Creature — Efreet, `Coverage::Partial`)
/// enters with flying and phasing and phases out on its controller's next untap
/// step.
#[test]
fn shimmering_efreet_enters_with_flying_and_phasing_and_phases_out_on_untap() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[shimmering_efreet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, shimmering_efreet());
    pass_until(&mut engine, stack_is_empty);
    let efreet = on_battlefield(&engine, p0, shimmering_efreet()).unwrap();
    assert!(keywords(&engine, efreet).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, efreet).contains(KeywordSet::PHASING));
    assert!(
        !engine
            .state()
            .object(efreet)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        engine
            .state()
            .object(efreet)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );

    // Phasing (CR 702.26): it phases in again at the next of its controller's
    // untap steps, and nothing of the opponent's turn moves it.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        engine
            .state()
            .object(efreet)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT),
        "the opponent's untap step does not phase it in"
    );
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !engine
            .state()
            .object(efreet)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT),
        "its controller's next untap step phases it in"
    );
}
