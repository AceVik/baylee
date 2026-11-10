//! `cards/creatures/mv_5/fire_elemental.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Twelve vanilla reprints whose whole printed text is a body: cast each and
/// check the P/T, the creature type and the creature subtype the card
/// prints, with nothing else on the battlefield afterward.
#[test]
#[allow(clippy::too_many_lines)] // one loop table, twelve vanilla bodies checked the same way
fn alpha_vanilla_creatures_are_their_printed_bodies() {
    let p0 = PlayerId::new(0);
    for (name, card, land, lands, body, subtype) in [
        (
            "Pearled Unicorn",
            pearled_unicorn(),
            plains(),
            3,
            (2, 2),
            baylee_core::generated::subtypes::creature::UNICORN,
        ),
        (
            "Savannah Lions",
            savannah_lions(),
            plains(),
            1,
            (2, 1),
            baylee_core::generated::subtypes::creature::CAT,
        ),
        (
            "Merfolk of the Pearl Trident",
            merfolk_of_the_pearl_trident(),
            island(),
            1,
            (1, 1),
            baylee_core::generated::subtypes::creature::MERFOLK,
        ),
        (
            "Water Elemental",
            water_elemental(),
            island(),
            5,
            (5, 4),
            baylee_core::generated::subtypes::creature::ELEMENTAL,
        ),
        (
            "Scathe Zombies",
            scathe_zombies(),
            swamp(),
            3,
            (2, 2),
            baylee_core::generated::subtypes::creature::ZOMBIE,
        ),
        (
            "Earth Elemental",
            earth_elemental(),
            mountain(),
            5,
            (4, 5),
            baylee_core::generated::subtypes::creature::ELEMENTAL,
        ),
        (
            "Fire Elemental",
            fire_elemental(),
            mountain(),
            5,
            (5, 4),
            baylee_core::generated::subtypes::creature::ELEMENTAL,
        ),
        (
            "Gray Ogre",
            gray_ogre(),
            mountain(),
            3,
            (2, 2),
            baylee_core::generated::subtypes::creature::OGRE,
        ),
        (
            "Hurloon Minotaur",
            hurloon_minotaur(),
            mountain(),
            3,
            (2, 3),
            baylee_core::generated::subtypes::creature::MINOTAUR,
        ),
        (
            "Mons's Goblin Raiders",
            mons_s_goblin_raiders(),
            mountain(),
            1,
            (1, 1),
            baylee_core::generated::subtypes::creature::GOBLIN,
        ),
        (
            "Craw Wurm",
            craw_wurm(),
            forest(),
            6,
            (6, 4),
            baylee_core::generated::subtypes::creature::WURM,
        ),
        (
            "Ironroot Treefolk",
            ironroot_treefolk(),
            forest(),
            5,
            (3, 5),
            baylee_core::generated::subtypes::creature::TREEFOLK,
        ),
    ] {
        assert_eq!(
            cast_saying_nothing(card, land, lands),
            Zone::Battlefield,
            "{name}"
        );
        let mut engine = Duel::new(SEED, land).battlefield(0, &[card]).start();
        keep_mulligans(&mut engine);
        let id = on_battlefield(&engine, p0, card).expect("seated");
        assert_eq!(pt(&engine, id), body, "{name}");
        assert!(
            types(&engine, id).contains(TypeSet::CREATURE),
            "{name} is a creature"
        );
        assert!(
            engine
                .state()
                .object(id)
                .expect("seated")
                .characteristics()
                .subtypes
                .contains(subtype),
            "{name} carries its printed creature subtype"
        );
    }
}

/// Fire Elemental — vanilla `{3}{R}{R}` 5/4 Elemental.
#[test]
fn fire_elemental_is_a_five_four_elemental_for_3rr() {
    let p0 = PlayerId::new(0);
    assert_eq!(
        cast_saying_nothing(fire_elemental(), mountain(), 5),
        Zone::Battlefield
    );
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[fire_elemental()])
        .start();
    keep_mulligans(&mut engine);
    let id = on_battlefield(&engine, p0, fire_elemental()).expect("seated");
    assert_eq!(pt(&engine, id), (5, 4));
}
