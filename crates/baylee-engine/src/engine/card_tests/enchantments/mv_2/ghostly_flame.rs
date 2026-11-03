//! `cards/enchantments/mv_2/ghostly_flame.rs`, played.
//!
//! Oracle:
//! - "Black and/or red permanents and spells are colorless sources of damage."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn ghostly_flame() -> CardIndex {
    card_index("6a461ca3-7923-4df4-9f15-4072545da117")
}

#[test]
fn ghostly_flame_casts_and_enters_battlefield() {
    let mut engine = lk::game(SEED, &[], &[], &[ghostly_flame()], &[]);
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Black, 1), (ManaColor::Red, 1)],
    );
    lk::cast_at(&mut engine, P0, ghostly_flame(), &[], &[]);
    assert!(on_battlefield(&engine, P0, ghostly_flame()).is_some());
}

#[test]
fn red_spell_damage_bypasses_protection_from_red_only_with_ghostly_flame() {
    for with_flame in [false, true] {
        let board = if with_flame {
            vec![ghostly_flame()]
        } else {
            vec![]
        };
        let mut engine = lk::game(
            SEED,
            &board,
            &[index::CRIMSON_ACOLYTE],
            &[index::PYROCLASM],
            &[],
        );
        let acolyte = lk::obj(&engine, lk::P1, index::CRIMSON_ACOLYTE);
        lk::float(
            &mut engine,
            P0,
            &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        );
        lk::cast_at(&mut engine, P0, index::PYROCLASM, &[], &[]);

        if with_flame {
            assert_eq!(lk::zone_of(&engine, acolyte), Zone::Graveyard);
            assert!(
                damage_events(&engine, 0)
                    .iter()
                    .any(|(_, target, amount, _)| {
                        *target == crate::event::DamageTarget::Object(acolyte) && *amount == 2
                    })
            );
        } else {
            assert_eq!(lk::zone_of(&engine, acolyte), Zone::Battlefield);
            assert_eq!(lk::damage_on(&engine, acolyte), 0);
        }
    }
}

fn skirk_fire_marshal() -> CardIndex {
    card_index("87105cdf-c28e-4fd9-83d0-0fef1704b174")
}

#[test]
fn red_permanent_damage_bypasses_protection_from_red_only_with_ghostly_flame() {
    for with_flame in [false, true] {
        let mut board = vec![
            skirk_fire_marshal(),
            index::RAGING_GOBLIN,
            index::RAGING_GOBLIN,
            index::RAGING_GOBLIN,
            index::RAGING_GOBLIN,
        ];
        if with_flame {
            board.push(ghostly_flame());
        }
        let mut engine = lk::game(SEED, &board, &[], &[], &[]);
        let marshal = lk::obj(&engine, P0, skirk_fire_marshal());
        let mut group = all_on_battlefield(&engine, P0, index::RAGING_GOBLIN);
        group.push(marshal);
        activate(&mut engine, P0, skirk_fire_marshal(), 1);
        engine
            .apply(P0, PlayerAction::ChooseObjects { objects: group })
            .unwrap();
        lk::settle(&mut engine);

        assert_eq!(lk::life(&engine, P0), 10);
        assert_eq!(lk::life(&engine, lk::P1), 10);
        if with_flame {
            assert_eq!(lk::zone_of(&engine, marshal), Zone::Graveyard);
            assert!(
                damage_events(&engine, 0)
                    .iter()
                    .any(|(_, target, amount, _)| {
                        *target == crate::event::DamageTarget::Object(marshal) && *amount == 10
                    })
            );
        } else {
            assert_eq!(lk::zone_of(&engine, marshal), Zone::Battlefield);
            assert_eq!(lk::damage_on(&engine, marshal), 0);
        }
    }
}

#[test]
fn ghostly_flame_does_not_change_what_a_red_spell_can_target() {
    let mut engine = lk::game(
        SEED,
        &[ghostly_flame()],
        &[index::CRIMSON_ACOLYTE, index::GRIZZLY_BEARS],
        &[index::SHOCK],
        &[],
    );
    let acolyte = lk::obj(&engine, lk::P1, index::CRIMSON_ACOLYTE);
    let bears = lk::obj(&engine, lk::P1, index::GRIZZLY_BEARS);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, index::SHOCK);

    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("Shock asks for a target, got {:?}", engine.pending());
    };
    assert!(!options.contains(&acolyte));
    assert!(options.contains(&bears));
}
