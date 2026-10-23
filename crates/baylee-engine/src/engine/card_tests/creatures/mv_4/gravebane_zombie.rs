//! `cards/creatures/mv_4/gravebane_zombie.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::LIGHTNING_BOLT;

const P0: PlayerId = PlayerId::new(0);

fn gravebane_zombie() -> CardIndex {
    card_index("d04a836b-7e42-424d-9600-42a40d0dfccd")
}

/// Gravebane Zombie (d04a836b-7e42-424d-9600-42a40d0dfccd) — {3}{B} 3/2:
/// "If this creature would die, put it on top of its owner's library instead."
#[test]
fn gravebane_zombie_puts_itself_on_top_of_library_when_it_dies() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[gravebane_zombie(), mountain()])
        .hand(0, &[LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let zombie = on_battlefield(&engine, P0, gravebane_zombie()).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, LIGHTNING_BOLT);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![zombie],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, gravebane_zombie()).is_none());
    assert!(
        in_graveyard(&engine, P0, gravebane_zombie()).is_none(),
        "it left the graveyard for the library"
    );
    let top_id = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(P0))
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(top_id).unwrap().card.map(|c| c.index),
        Some(gravebane_zombie())
    );
}
