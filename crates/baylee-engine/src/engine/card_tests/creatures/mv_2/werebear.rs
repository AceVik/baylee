//! Oracle scenarios for Werebear.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn werebear_taps_for_green_and_gets_plus_3_plus_3_with_threshold() {
    let p0 = PlayerId::new(0);
    let card = card_index("96cf3320-c730-4618-8768-28a179f4d9c0");
    let mut board = vec![card];
    board.extend_from_slice(&[index::PLAINS; 7]);
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &board)
        .hand(0, &[index::LLANOWAR_ELVES])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let plains = all_on_battlefield(&e, p0, index::PLAINS);
    for &p in &plains[..6] {
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                p,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    e.refresh_offer();

    let bear = on_battlefield(&e, p0, card).unwrap();
    assert_eq!(pt(&e, bear), (1, 1));

    tap_mana_where(&mut e, p0, |id| id == bear);
    cast_with_floating(&mut e, p0, index::LLANOWAR_ELVES);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p0, index::LLANOWAR_ELVES).is_some());

    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            plains[6],
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.apply(p0, PlayerAction::PassPriority).unwrap();

    assert_eq!(pt(&e, bear), (4, 4));
}
