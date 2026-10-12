#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch46_avengers_tower_checks_hero_type_for_spells_and_activation_sources() {
    for nexus in [false, true] {
        let mut battlefield = vec![index::AVENGERS_TOWER, index::SHIVAN_DRAGON];
        if nexus {
            battlefield.push(index::MASKWOOD_NEXUS);
        }
        let mut e = prepared(&[index::BENALISH_HERO], &battlefield, &[]);
        let tower = source(&e, index::AVENGERS_TOWER);
        let dragon = source(&e, index::SHIVAN_DRAGON);
        let hero = in_hand(&e, P0, index::BENALISH_HERO).unwrap();
        pool(&mut e, &[]);
        ability(&mut e, tower, 1);
        color(&mut e, ManaColor::White);
        if nexus {
            e.apply(P0, PlayerAction::CastSpell { card: hero }).unwrap();
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state.object(hero).unwrap().zone, Zone::Battlefield);
        } else {
            // A name containing "Hero" is no creature-type permission.
            no_mana_to_cast(&e, hero);
        }
        pool(&mut e, &[]);
        e.state.set_tapped(tower, false);
        ability(&mut e, tower, 1);
        color(&mut e, ManaColor::Red);
        let activated = e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: dragon,
                ability_index: 0,
            },
        );
        assert_eq!(activated.is_ok(), nexus);
        if nexus {
            assert_eq!(e.state.players[0].mana_pool.total(), 0);
            pass_until(&mut e, stack_is_empty);
            assert_eq!(pt(&e, dragon), (6, 5));
        }
    }
}

#[test]
fn batch46_avengers_tower_reveals_a_hero_and_orders_only_the_rest() {
    let mut e = prepared(
        &[index::RAGING_GOBLIN, index::FOREST, index::GIANT_GROWTH],
        &[index::AVENGERS_TOWER, index::MASKWOOD_NEXUS],
        &[],
    );
    let goblin = hand_to_library_top(&mut e, P0, index::RAGING_GOBLIN);
    let forest = hand_to_library_top(&mut e, P0, index::FOREST);
    let growth = hand_to_library_top(&mut e, P0, index::GIANT_GROWTH);
    let tower = source(&e, index::AVENGERS_TOWER);
    ability(&mut e, tower, 2);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending()
    else {
        unreachable!()
    };
    assert_eq!(options, &[goblin]);
    assert_eq!((*min, *max), (0, 1));
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![forest]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    choose(&mut e, vec![goblin]);
    let Pending::Arrange { cards, piles, .. } = e.pending() else {
        panic!("{:?}", e.pending())
    };
    assert_eq!(cards.len(), 2);
    assert!(cards.contains(&forest) && cards.contains(&growth));
    assert_eq!(piles.len(), 1);
    e.apply(
        P0,
        PlayerAction::Arrange {
            piles: vec![vec![forest, growth]],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(goblin).unwrap().zone, Zone::Hand);
    assert!(
        e.journal().entries().iter().any(|entry| matches!(
            &entry.event,
            crate::event::GameEvent::Revealed { player, cards }
                if *player == P0 && cards == &[goblin]
        )),
        "the chosen Hero is publicly revealed before entering the hand"
    );
    let library = e.state.zones.list(ZoneLocation::Library(P0));
    assert_eq!(
        &library[..2],
        &[growth, forest],
        "chosen top-to-bottom order survives bottom insertion"
    );
}

#[test]
fn batch46_avengers_tower_handles_short_library_no_hero_and_declined_hero() {
    for (nexus, take) in [(false, false), (true, false), (true, true)] {
        let mut battlefield = vec![index::AVENGERS_TOWER];
        if nexus {
            battlefield.push(index::MASKWOOD_NEXUS);
        }
        let mut e = prepared(&[index::RAGING_GOBLIN], &battlefield, &[]);
        let old = e.state.zones.list(ZoneLocation::Library(P0)).clone();
        for card in old {
            e.state
                .move_object(
                    card,
                    ZoneLocation::Exile(P0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        let goblin = hand_to_library_top(&mut e, P0, index::RAGING_GOBLIN);
        let tower = source(&e, index::AVENGERS_TOWER);
        ability(&mut e, tower, 2);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards { options, max, .. } = e.pending() else {
            unreachable!()
        };
        assert_eq!(options.contains(&goblin), nexus);
        assert_eq!(*max, u8::from(nexus));
        choose(&mut e, if take { vec![goblin] } else { vec![] });
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state.object(goblin).unwrap().zone,
            if take { Zone::Hand } else { Zone::Library }
        );
    }
}
