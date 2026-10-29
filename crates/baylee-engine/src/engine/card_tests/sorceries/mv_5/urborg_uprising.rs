//! Oracle scenarios for Urborg Uprising.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn urborg_uprising() -> CardIndex {
    card_index("882d3be7-1c0f-4d2b-8f2e-32369488ef82")
}

#[test]
fn urborg_uprising_returns_two_creatures_from_graveyard_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(
            0,
            &[
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::FOREST,
                index::GRIZZLY_BEARS,
                index::HILL_GIANT,
            ],
        )
        .battlefield(1, &[index::GRAY_OGRE])
        .hand(0, &[urborg_uprising()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&e, p0, index::HILL_GIANT).unwrap();
    let forest = on_battlefield(&e, p0, index::FOREST).unwrap();
    let ogre = on_battlefield(&e, p1, index::GRAY_OGRE).unwrap();
    for id in [bears, giant, forest] {
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
    e.dev_state_mut(p1)
        .unwrap()
        .move_object(
            ogre,
            ZoneLocation::Graveyard(p1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    let lib_before = library_size(&e, p0);

    cast_from_hand(&mut e, p0, urborg_uprising());
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected target choice")
    };
    assert!(options.contains(&bears) && options.contains(&giant));
    assert!(!options.contains(&forest) && !options.contains(&ogre));

    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bears, giant],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_hand(&e, p0, index::GRIZZLY_BEARS).is_some());
    assert!(in_hand(&e, p0, index::HILL_GIANT).is_some());
    assert_eq!(library_size(&e, p0), lib_before - 1);
}

#[test]
fn urborg_uprising_can_choose_zero_targets_and_still_draw_a_card() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(
            0,
            &[
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[urborg_uprising()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            bears,
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();

    let lib_before = library_size(&e, p0);
    cast_from_hand(&mut e, p0, urborg_uprising());
    e.apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p0, index::GRIZZLY_BEARS).is_some());
    assert_eq!(library_size(&e, p0), lib_before - 1);
}
