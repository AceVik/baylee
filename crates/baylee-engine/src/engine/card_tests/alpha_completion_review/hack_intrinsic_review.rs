//! Independent CR 305.6/612.2–3 land mana provenance controls.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::{index, subtypes};
const USER: PlayerId = PlayerId::new(0);
const OTHER: PlayerId = PlayerId::new(1);

fn hacked_land(card: CardIndex, lantern: bool) -> (Engine<RegistryLookup>, ObjectId) {
    let mut board = vec![island(), card];
    if lantern {
        board.push(index::CHROMATIC_LANTERN);
    }
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[index::MAGICAL_HACK])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    let land = on_battlefield(&engine, USER, card).unwrap();
    let payment = on_battlefield(&engine, USER, island()).unwrap();
    engine
        .apply(USER, PlayerAction::ActivateManaAbility { source: payment })
        .unwrap();
    cast_with_floating(&mut engine, USER, index::MAGICAL_HACK);
    engine
        .apply(
            USER,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(USER, PlayerAction::ChooseNumber(17)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, OTHER);
    reach_their_main_phase(&mut engine, USER);
    assert!(
        engine
            .state()
            .object(land)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::land::ISLAND)
    );
    assert!(
        !engine
            .state()
            .object(land)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::land::FOREST)
    );
    (engine, land)
}

#[test]
fn hack_intrinsic_review_dual_land_collapsing_to_island_has_no_green_route() {
    let (engine, land) = hacked_land(index::TROPICAL_ISLAND, false);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority");
    };
    let mut routes: Vec<Option<u32>> = legal
        .mana_abilities
        .iter()
        .filter(|&&source| source == land)
        .map(|_| None)
        .collect();
    routes.extend(
        legal
            .abilities
            .iter()
            .filter(|&&(source, _)| source == land)
            .map(|&(_, i)| Some(i)),
    );
    assert!(!routes.is_empty());
    for route in routes {
        let (mut engine, land) = hacked_land(index::TROPICAL_ISLAND, false);
        let action = route.map_or(
            PlayerAction::ActivateManaAbility { source: land },
            |ability_index| PlayerAction::ActivateAbility {
                source: land,
                ability_index,
            },
        );
        engine.apply(USER, action).unwrap();
        if let Pending::ChooseColor { options, .. } = engine.pending() {
            assert_eq!(options.as_slice(), &[ManaColor::Blue]);
            engine
                .apply(USER, PlayerAction::ChooseColor(ManaColor::Blue))
                .unwrap();
        }
        assert_eq!(
            engine.state().players[0]
                .mana_pool
                .available(ManaColor::Blue),
            1
        );
        assert_eq!(
            engine.state().players[0]
                .mana_pool
                .available(ManaColor::Green),
            0
        );
        assert_eq!(engine.state().players[0].mana_pool.total(), 1);
        assert_eq!(engine.state().players[0].life, 20);
    }
}

#[test]
fn hack_intrinsic_review_bosk_changes_intrinsic_mana_but_preserves_printed_pain_mana() {
    for color in [ManaColor::Blue, ManaColor::White, ManaColor::Black] {
        let (mut engine, land) = hacked_land(index::MURMURING_BOSK, false);
        let ability_index = u32::from(color != ManaColor::Blue);
        let Pending::Priority { legal, .. } = engine.pending() else {
            panic!("priority");
        };
        assert!(legal.abilities.contains(&(land, ability_index)));
        engine
            .apply(
                USER,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index,
                },
            )
            .unwrap();
        if color != ManaColor::Blue {
            let Pending::ChooseColor { options, .. } = engine.pending() else {
                panic!("printed W/B choice");
            };
            assert_eq!(options.as_slice(), &[ManaColor::White, ManaColor::Black]);
            engine
                .apply(USER, PlayerAction::ChooseColor(color))
                .unwrap();
        }
        assert_eq!(engine.state().players[0].mana_pool.available(color), 1);
        assert_eq!(
            engine.state().players[0]
                .mana_pool
                .available(ManaColor::Green),
            0
        );
        assert_eq!(engine.state().players[0].mana_pool.total(), 1);
        assert_eq!(
            engine.state().players[0].life,
            if color == ManaColor::Blue { 20 } else { 19 }
        );
        assert!(is_tapped(&engine, land));
        assert!(stack_is_empty(&engine));
    }
}

#[test]
fn hack_intrinsic_review_lantern_grant_still_produces_green_on_hacked_island() {
    let (mut engine, land) = hacked_land(forest(), true);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority");
    };
    assert!(
        legal
            .abilities
            .contains(&(land, crate::choice::GRANTED_ABILITY))
    );
    engine
        .apply(
            USER,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: crate::choice::GRANTED_ABILITY,
            },
        )
        .unwrap();
    let Pending::ChooseColor { options, .. } = engine.pending() else {
        panic!("Lantern's grant");
    };
    assert!(options.contains(&ManaColor::Green));
    engine
        .apply(USER, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert_eq!(engine.state().players[0].life, 20);
    assert!(is_tapped(&engine, land));
    assert!(stack_is_empty(&engine));
}
