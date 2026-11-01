//! Oracle scenarios for Aboshan's Desire.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn aboshan_s_desire() -> CardIndex {
    card_index("95db05eb-2c25-43f2-8e18-dfc64f490634")
}

#[test]
fn aboshan_s_desire_grants_flying_and_shroud_only_with_threshold() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut graveyard_fodder = vec![index::FOREST; 7];
    graveyard_fodder.extend_from_slice(&[index::ISLAND, index::GRIZZLY_BEARS, index::GRAY_OGRE]);
    let mut e = Duel::new(SEED, index::ISLAND)
        .battlefield(0, &graveyard_fodder)
        .battlefield(1, &[index::PLAINS])
        .hand(0, &[aboshan_s_desire()])
        .hand(1, &[index::SWORDS_TO_PLOWSHARES])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let forests = all_on_battlefield(&e, p0, index::FOREST);
    assert_eq!(forests.len(), 7);
    for &f in &forests[..6] {
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                f,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    e.refresh_offer();

    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let ogre = on_battlefield(&e, p0, index::GRAY_OGRE).unwrap();

    cast_from_hand(&mut e, p0, aboshan_s_desire());
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bears],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(keywords(&e, bears).contains(KeywordSet::FLYING));
    assert!(
        !keywords(&e, bears).contains(KeywordSet::SHROUD),
        "six cards in graveyard is below threshold"
    );

    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            forests[6],
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    reach_their_main_phase(&mut e, p1);

    assert!(keywords(&e, bears).contains(KeywordSet::FLYING));
    assert!(
        keywords(&e, bears).contains(KeywordSet::SHROUD),
        "seven cards in graveyard activates threshold shroud"
    );

    cast_from_hand(&mut e, p1, index::SWORDS_TO_PLOWSHARES);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected target choice")
    };
    assert!(!options.contains(&bears));
    assert!(options.contains(&ogre));
}
