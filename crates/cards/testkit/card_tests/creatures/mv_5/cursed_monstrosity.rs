//! Oracle scenarios for Cursed Monstrosity.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn cursed_monstrosity() -> CardIndex {
    card_index("16e03598-30bf-4b6c-9b88-36c3c5c010dd")
}

#[test]
fn cursed_monstrosity_stays_when_land_discarded_and_is_sacrificed_when_declined() {
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
                index::FOREST,
                index::FOREST,
            ],
        )
        .hand(
            0,
            &[
                cursed_monstrosity(),
                index::GIANT_GROWTH,
                index::GIANT_GROWTH,
                index::SWAMP,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, cursed_monstrosity());
    pass_until(&mut e, stack_is_empty);

    let monstrosity = on_battlefield(&e, p0, cursed_monstrosity()).unwrap();
    assert!(keywords(&e, monstrosity).contains(KeywordSet::FLYING));
    let land_in_hand = in_hand(&e, p0, index::SWAMP).unwrap();

    cast_with_floating(&mut e, p0, index::GIANT_GROWTH);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![monstrosity],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![land_in_hand],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, p0, cursed_monstrosity()).is_some());
    assert!(in_graveyard(&e, p0, index::SWAMP).is_some());
    assert_eq!(pt(&e, monstrosity), (7, 6));

    cast_with_floating(&mut e, p0, index::GIANT_GROWTH);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![monstrosity],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, p0, cursed_monstrosity()).is_none());
    assert!(in_graveyard(&e, p0, cursed_monstrosity()).is_some());
}
