//! Lat-Nam's Legacy — `{1}{U}` Instant (Alliances)
//!
//! (`Coverage::Partial`: uses `BottomCardFromHand` followed by `ShuffleLibrary`;
//! if you choose a card from your hand, shuffles it into your library and draws
//! two cards at the beginning of the next turn's upkeep.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn lat_nam_s_legacy() -> CardIndex {
    card_index("f885ee54-3b26-4d61-9783-3930b0e625aa")
}

#[test]
fn lat_nam_s_legacy_shuffles_card_from_hand_into_library_and_draws_two_next_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[lat_nam_s_legacy(), grizzly_bears()])
        .battlefield(0, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let legacy = in_hand(&engine, p0, lat_nam_s_legacy()).unwrap();
    let bears = in_hand(&engine, p0, grizzly_bears()).unwrap();
    let lib_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: legacy })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, grizzly_bears()).is_none());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        lib_before + 1
    );

    // Advance to next turn's upkeep (p1's turn) -> p0 draws 2 cards.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2
    );
}
