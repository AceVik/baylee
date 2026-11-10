//! Oracle scenarios for Shelter.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn shelter() -> CardIndex {
    card_index("c1909d70-d8f4-4c54-8587-0a910f123238")
}

#[test]
fn shelter_grants_protection_from_chosen_color_and_draws_card() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, index::PLAINS)
        .battlefield(
            0,
            &[
                index::PLAINS,
                index::PLAINS,
                index::MOUNTAIN,
                index::GRIZZLY_BEARS,
                index::HILL_GIANT,
            ],
        )
        .hand(0, &[shelter(), index::SHOCK])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&e, p0, index::HILL_GIANT).unwrap();
    let hand_before = e.state().zones.list(ZoneLocation::Hand(p0)).len();

    let plains_ids = all_on_battlefield(&e, p0, index::PLAINS);
    tap_mana_where(&mut e, p0, |id| plains_ids.contains(&id));
    cast_with_floating(&mut e, p0, shelter());
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![bear],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );

    let mountain_id = on_battlefield(&e, p0, index::MOUNTAIN).unwrap();
    tap_mana_where(&mut e, p0, |id| id == mountain_id);
    cast_with_floating(&mut e, p0, index::SHOCK);
    let menu = aim_at(&mut e, p0, giant);
    assert!(!menu.contains(&bear));
}
