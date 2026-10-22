//! `cards/creatures/mv_2/goblin_recruiter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn goblin_recruiter() -> CardIndex {
    card_index("792e1072-00bf-4f63-b956-5f20cdf09f11")
}

fn goblin_swine_rider() -> CardIndex {
    card_index("9ebe32fc-3b6e-4cb9-9af0-1a6c3c6ad885")
}

/// `Goblin Recruiter` ({1}{R}, 1/1 Goblin, `Coverage::Implemented`):
/// `When this creature enters, search your library for any number of Goblin
/// cards, reveal them, then shuffle and put those cards on top in any order.`
#[test]
fn goblin_recruiter_etb_searches_goblins_to_top_of_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[goblin_recruiter(), goblin_swine_rider()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rider_in_hand = in_hand(&engine, p0, goblin_swine_rider()).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            rider_in_hand,
            ZoneLocation::Library(p0),
            crate::zone::ZonePosition::Bottom,
            crate::event::Cause::Effect,
        )
        .unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, goblin_recruiter());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(options, vec![rider_in_hand]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rider_in_hand],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(top).unwrap().card.map(|c| c.index),
        Some(goblin_swine_rider())
    );
}
