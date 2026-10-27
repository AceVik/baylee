//! Oracle scenarios for Terravore.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn terravore_power_and_toughness_equal_land_cards_in_all_graveyards() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let card = card_index("f44fb61a-9aa5-4178-b958-5115d62a7f50");
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::FOREST,
                index::FOREST,
                index::FOREST,
                index::PLAINS,
                index::MOUNTAIN,
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::ISLAND, index::SWAMP, index::HILL_GIANT])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    for &c in &[index::PLAINS, index::MOUNTAIN, index::GRIZZLY_BEARS] {
        let id = on_battlefield(&e, p0, c).unwrap();
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    for &c in &[index::ISLAND, index::SWAMP, index::HILL_GIANT] {
        let id = on_battlefield(&e, p1, c).unwrap();
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Graveyard(p1),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    e.refresh_offer();

    cast_from_hand(&mut e, p0, card);
    pass_until(&mut e, stack_is_empty);

    let terravore = on_battlefield(&e, p0, card).unwrap();
    assert_eq!(pt(&e, terravore), (4, 4));
    assert!(keywords(&e, terravore).contains(KeywordSet::TRAMPLE));
}
