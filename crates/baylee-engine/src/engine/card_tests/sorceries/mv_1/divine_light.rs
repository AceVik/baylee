//! Oracle scenarios for Divine Light.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn divine_light() -> CardIndex {
    card_index("924bf3bb-9f06-4a2c-a17f-3fe11551bcf2")
}

#[test]
fn divine_light_prevents_all_damage_to_creatures_you_control_this_turn_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::GRAY_OGRE])
        .hand(0, &[divine_light(), index::SHOCK, index::SHOCK])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let ogre = on_battlefield(&e, p1, index::GRAY_OGRE).unwrap();

    cast_from_hand(&mut e, p0, divine_light());
    pass_until(&mut e, stack_is_empty);

    // Shock your own Grizzly Bears — all 2 damage is prevented.
    cast_with_floating(&mut e, p0, index::SHOCK);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bears],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        on_battlefield(&e, p0, index::GRIZZLY_BEARS).is_some(),
        "Grizzly Bears survived Shock because Divine Light prevents all damage to creatures you control"
    );
    assert_eq!(e.state().object(bears).unwrap().damage, 0);

    // Shock opponent's Gray Ogre — opponent's creatures are not protected.
    cast_with_floating(&mut e, p0, index::SHOCK);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![ogre],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, p1, index::GRAY_OGRE).is_some(),
        "opponent's Gray Ogre is not protected by Divine Light and dies to Shock"
    );
}
