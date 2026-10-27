//! Oracle scenarios for Bloodfire Infusion.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn bloodfire_infusion() -> CardIndex {
    card_index("d5bd7c0b-c412-47ed-afb8-1a9ba04be77c")
}

#[test]
fn bloodfire_infusion_sacrifices_enchanted_creature_and_deals_its_power_to_each_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, index::MOUNTAIN)
        .battlefield(
            0,
            &[
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::HILL_GIANT,
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::GRAY_OGRE, index::EARTH_ELEMENTAL])
        .hand(0, &[bloodfire_infusion()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let giant = on_battlefield(&e, p0, index::HILL_GIANT).unwrap();
    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let ogre = on_battlefield(&e, p1, index::GRAY_OGRE).unwrap();
    let elemental = on_battlefield(&e, p1, index::EARTH_ELEMENTAL).unwrap();

    cast_from_hand(&mut e, p0, bloodfire_infusion());
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected Aura target prompt")
    };
    assert!(
        options.contains(&giant) && options.contains(&bears),
        "your creatures are legal Enchant creature you control targets"
    );
    assert!(
        !options.contains(&ogre) && !options.contains(&elemental),
        "opponent's creatures cannot be targeted by Enchant creature you control"
    );
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![giant],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    let aura = on_battlefield(&e, p0, bloodfire_infusion()).unwrap();
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: aura,
            ability_index: 1,
        },
    )
    .unwrap();
    // Only the enchanted creature (Hill Giant) is a legal sacrifice choice.
    if let Pending::ChooseCards { options, .. } = e.pending().clone() {
        assert_eq!(options, vec![giant]);
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    }
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, p0, index::HILL_GIANT).is_some(),
        "enchanted Hill Giant was sacrificed as the activation cost"
    );
    assert!(
        in_graveyard(&e, p0, index::GRIZZLY_BEARS).is_some(),
        "Grizzly Bears (2/2) took 3 damage from Hill Giant's power and died"
    );
    assert!(
        in_graveyard(&e, p1, index::GRAY_OGRE).is_some(),
        "Gray Ogre (2/2) took 3 damage from Hill Giant's power and died"
    );
    assert!(
        on_battlefield(&e, p1, index::EARTH_ELEMENTAL).is_some(),
        "Earth Elemental (4/5) survived 3 damage"
    );
    assert_eq!(
        e.state().object(elemental).unwrap().damage,
        3,
        "Earth Elemental has 3 damage marked on it"
    );
}
