//! Tests for Flailing Ogre.

#[allow(clippy::wildcard_imports)]
use super::*;

// oracle_id = "e81fe764-8b26-40b3-8e17-741ab5e231b4"
fn flailing_ogre() -> CardIndex {
    card_index("e81fe764-8b26-40b3-8e17-741ab5e231b4")
}

/// {1}: This creature gets +1/+1 until end of turn. Any player may activate this ability.
/// {1}: This creature gets -1/-1 until end of turn. Any player may activate this ability.
#[test]
fn flailing_ogre_can_be_pumped_or_shrunk_by_any_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[mountain()])
        .hand(0, &[flailing_ogre()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, flailing_ogre());
    pass_until(&mut engine, stack_is_empty);
    let ogre = on_battlefield(&engine, p0, flailing_ogre()).expect("Flailing Ogre");
    assert_eq!(pt(&engine, ogre), (3, 3));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: ogre,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, ogre), (4, 4));

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: ogre,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, ogre), (3, 3));
}
