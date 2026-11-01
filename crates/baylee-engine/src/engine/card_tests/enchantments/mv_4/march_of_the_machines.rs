//! March of the Machines animates artifacts and stops creature Equipment equipping.

use super::*;

fn march_of_the_machines() -> CardIndex {
    card_index("51092634-308e-4779-aa51-182715dbc734")
}

fn mask_of_memory() -> CardIndex {
    card_index("d6b2c998-a226-426c-a40d-6e6007041bfe")
}

#[test]
fn march_animates_noncreature_artifacts_by_mana_value() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[march_of_the_machines(), quiet_artifact(), mask_of_memory()],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let ring = on_battlefield(&e, p0, quiet_artifact()).unwrap();
    let equipment = on_battlefield(&e, p0, mask_of_memory()).unwrap();
    assert_eq!(pt(&e, ring), (1, 1));
    assert_eq!(pt(&e, equipment), (2, 2));
    assert!(
        e.state()
            .object(ring)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
}

fn nightmare_lash() -> CardIndex {
    baylee_core::generated::index::NIGHTMARE_LASH
}

/// Ruling 2004-12-01: "You can activate the Equipment's equip ability, but
/// it won't do anything" (CR 301.5c). The equip is paid (3 life, so no mana
/// stands between the board and the offer) and resolves, and the animated
/// Lash stays unattached: the Ogre keeps its printed 2/2 although p0
/// controls three Swamps.
#[test]
fn march_lets_a_creature_equipment_activate_equip_to_no_effect() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                march_of_the_machines(),
                nightmare_lash(),
                gray_ogre(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let lash = on_battlefield(&e, p0, nightmare_lash()).unwrap();
    let ogre = on_battlefield(&e, p0, gray_ogre()).unwrap();
    assert!(
        e.state()
            .object(lash)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    activate(&mut e, p0, nightmare_lash(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![ogre],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 17, "the equip cost was paid");
    assert_eq!(e.state().object(lash).unwrap().attached_to, None);
    assert_eq!(pt(&e, ogre), (2, 2));
}

/// Ruling 2004-12-01: an Equipment that becomes a creature while attached
/// "becomes unattached (but remains on the battlefield)", by the
/// state-based action for an attached creature (CR 704.5p in the engine's
/// citation). The Ogre loses the Lash's +3/+3.
#[test]
fn march_entering_unattaches_an_equipment_already_on_a_creature() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                nightmare_lash(),
                gray_ogre(),
                swamp(),
                swamp(),
                swamp(),
                island(),
            ],
        )
        .hand(0, &[march_of_the_machines()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let lash = on_battlefield(&e, p0, nightmare_lash()).unwrap();
    let ogre = on_battlefield(&e, p0, gray_ogre()).unwrap();
    activate(&mut e, p0, nightmare_lash(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![ogre],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(lash).unwrap().attached_to, Some(ogre));
    assert_eq!(pt(&e, ogre), (5, 5), "the control: three Swamps, attached");

    cast_from_hand(&mut e, p0, march_of_the_machines());
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p0, march_of_the_machines()).is_some());
    assert_eq!(e.state().object(lash).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(lash).unwrap().attached_to, None);
    assert_eq!(pt(&e, lash), (4, 4), "a 4/4 by its mana value");
    assert_eq!(pt(&e, ogre), (2, 2), "the bonus is gone with the Lash");
}
