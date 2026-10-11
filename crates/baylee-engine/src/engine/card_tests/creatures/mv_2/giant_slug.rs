//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn giant_slug_delays_each_choice_and_all_five_landwalks_expire_that_turn() {
    use baylee_core::generated::subtypes::land;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let choices = [
        (land::PLAINS, KeywordSet::PLAINSWALK),
        (land::ISLAND, KeywordSet::ISLANDWALK),
        (land::SWAMP, KeywordSet::SWAMPWALK),
        (land::MOUNTAIN, KeywordSet::MOUNTAINWALK),
        (land::FOREST, KeywordSet::FORESTWALK),
    ];
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::GIANT_SLUG])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let slug = on_battlefield(&e, p0, index::GIANT_SLUG).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 25);
    e.refresh_offer();
    for _ in 0..5 {
        activate(&mut e, p0, index::GIANT_SLUG, 0);
        pass_until(&mut e, stack_is_empty);
    }
    assert_eq!(keywords(&e, slug), KeywordSet::EMPTY);
    reach_their_main_phase(&mut e, p1);
    assert_eq!(keywords(&e, slug), KeywordSet::EMPTY);
    for (subtype, keyword) in choices {
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseSubtype { .. })
        });
        let Pending::ChooseSubtype { player, options } = e.pending().clone() else {
            unreachable!()
        };
        assert_eq!(player, p0);
        assert_eq!(options, choices.map(|(subtype, _)| subtype));
        assert!(
            e.apply(
                p0,
                PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::creature::SLUG)
            )
            .is_err()
        );
        e.apply(p0, PlayerAction::ChooseSubtype(subtype)).unwrap();
        assert!(keywords(&e, slug).contains(keyword));
    }
    reach_main_phase(&mut e, p0);
    assert_eq!(
        e.state().object(slug).unwrap().chosen_subtype,
        None,
        "independent resolution choices, not an entry field"
    );
    reach_their_main_phase(&mut e, p1);
    assert_eq!(keywords(&e, slug), KeywordSet::EMPTY);
}

#[test]
fn giant_slug_choice_does_not_affect_a_new_incarnation() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::GIANT_SLUG])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let slug = on_battlefield(&e, p0, index::GIANT_SLUG).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 5);
    e.refresh_offer();
    activate(&mut e, p0, index::GIANT_SLUG, 0);
    pass_until(&mut e, stack_is_empty);
    let state = e.dev_state_mut(p0).unwrap();
    state
        .move_object(
            slug,
            ZoneLocation::Exile(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            slug,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    reach_their_main_phase(&mut e, p1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::land::ISLAND),
    )
    .unwrap();
    assert!(!keywords(&e, slug).contains(KeywordSet::ISLANDWALK));
}
