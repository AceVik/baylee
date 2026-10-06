//! Author regression: every offered mana door uses the changed basic land type.
//! Kept separate from independent card acceptance.

#[allow(clippy::wildcard_imports)] // Shared real-card test vocabulary.
use super::*;
use baylee_core::generated::index;

const PLAYER: PlayerId = PlayerId::new(0);

fn hack_land(card: CardIndex) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(305_612, forest())
        .battlefield(0, &[island(), card])
        .hand(0, &[index::MAGICAL_HACK])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PLAYER);
    let land = on_battlefield(&engine, PLAYER, card).unwrap();
    let blue = on_battlefield(&engine, PLAYER, island()).unwrap();
    engine
        .apply(PLAYER, PlayerAction::ActivateManaAbility { source: blue })
        .unwrap();
    cast_with_floating(&mut engine, PLAYER, index::MAGICAL_HACK);
    engine
        .apply(
            PLAYER,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    // Basic-land word order WUBRG: Forest (4) -> Island (1).
    engine
        .apply(PLAYER, PlayerAction::ChooseNumber(17))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == PLAYER),
    );
    (engine, land)
}

fn independent_copy(engine: &Engine<RegistryLookup>) -> Engine<RegistryLookup> {
    let mut copy = Duel::new(305_612, forest()).start();
    crate::engine::checkpoint::Checkpoint::capture(engine).restore(&mut copy);
    // A checkpoint keeps the journal's length, not the journal, for the
    // engine it was taken of; a copy into another engine takes it whole.
    copy.state.journal = engine.state.journal.clone();
    assert_eq!(copy.snapshot_hash(), engine.snapshot_hash());
    copy
}

#[test]
fn hacked_forest_every_offered_mana_route_produces_only_blue() {
    let (engine, land) = hack_land(forest());
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("mana is offered with priority");
    };
    let mut routes: Vec<_> = legal
        .mana_abilities
        .iter()
        .filter(|&&source| source == land)
        .map(|&source| PlayerAction::ActivateManaAbility { source })
        .collect();
    routes.extend(
        legal
            .abilities
            .iter()
            .filter(|&&(source, _)| source == land)
            .map(|&(source, ability_index)| PlayerAction::ActivateAbility {
                source,
                ability_index,
            }),
    );
    assert!(
        !routes.is_empty(),
        "the Island type supplies a mana ability"
    );
    for route in routes {
        let mut copy = independent_copy(&engine);
        copy.apply(PLAYER, route.clone()).unwrap();
        let pool = &copy.state().players[0].mana_pool;
        assert_eq!(pool.available(ManaColor::Blue), 1, "route {route:?}");
        assert_eq!(pool.available(ManaColor::Green), 0, "route {route:?}");
        assert_eq!(pool.total(), 1, "route {route:?}");
    }
}

#[test]
fn hack_preserves_a_real_printed_land_mana_ability_and_its_symbols() {
    let (engine, land) = hack_land(index::YAVIMAYA_COAST);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("printed mana is offered with priority");
    };
    assert!(legal.abilities.contains(&(land, 0)));
    assert!(legal.abilities.contains(&(land, 1)));
    let mut colorless = independent_copy(&engine);
    colorless
        .apply(
            PLAYER,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .unwrap();
    assert_eq!(
        colorless.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    let mut colored = independent_copy(&engine);
    colored
        .apply(
            PLAYER,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .unwrap();
    let Pending::ChooseColor { options, .. } = colored.pending() else {
        panic!("the printed choice of mana symbols survives Hack");
    };
    assert!(options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue));
    colored
        .apply(PLAYER, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    assert_eq!(
        colored.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
    assert_eq!(
        colored.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        0
    );
    assert_eq!(colored.state().players[0].life, 19);
}
