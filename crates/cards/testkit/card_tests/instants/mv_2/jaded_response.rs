//! `cards/instants/mv_2/jaded_response.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn jaded_response() -> CardIndex {
    card_index("23917517-7561-45b3-b156-77cd2f340466")
}

/// Jaded Response counters a spell that shares a color with a creature you control
/// and leaves a spell of an unshared color uncountered.
#[test]
fn jaded_response_counters_only_when_sharing_color_with_your_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                index::LLANOWAR_ELVES,
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[jaded_response(), jaded_response()])
        .battlefield(1, &[forest(), forest(), plains()])
        .hand(1, &[index::GRIZZLY_BEARS, index::SWORDS_TO_PLOWSHARES])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let p0_elf = on_battlefield(&engine, p0, index::LLANOWAR_ELVES).unwrap();

    let p1_forests = all_on_battlefield(&engine, p1, forest());
    tap_mana_where(&mut engine, p1, |id| p1_forests.contains(&id));
    cast_with_floating(&mut engine, p1, index::GRIZZLY_BEARS);
    let bears_spell = engine.state().zones.list(ZoneLocation::Stack)[0];
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    let p0_islands = all_on_battlefield(&engine, p0, island());
    tap_mana_where(&mut engine, p0, |id| p0_islands[..2].contains(&id));
    cast_with_floating(&mut engine, p0, jaded_response());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears_spell],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());

    let p1_plains = on_battlefield(&engine, p1, plains()).unwrap();
    tap_mana_where(&mut engine, p1, |id| id == p1_plains);
    cast_with_floating(&mut engine, p1, index::SWORDS_TO_PLOWSHARES);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![p0_elf],
                players: vec![],
            },
        )
        .unwrap();
    let swords_spell = engine.state().zones.list(ZoneLocation::Stack)[0];
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    tap_mana_where(&mut engine, p0, |id| p0_islands[2..4].contains(&id));
    cast_with_floating(&mut engine, p0, jaded_response());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![swords_spell],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, index::LLANOWAR_ELVES).is_none(),
        "white Swords to Plowshares resolved and exiled Llanowar Elves"
    );
}
