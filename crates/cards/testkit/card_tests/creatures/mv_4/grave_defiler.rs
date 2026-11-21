//! `cards/creatures/mv_4/grave_defiler.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn grave_defiler() -> CardIndex {
    card_index("42c5c1aa-10a4-4cd5-b1f8-a393ed553f99")
}

fn phyrexian_scuta() -> CardIndex {
    card_index("af2845cf-8770-4efa-8a83-bf9eb4122b78")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Grave Defiler (42c5c1aa-10a4-4cd5-b1f8-a393ed553f99) — `{3}{B}` 2/1 Creature — Zombie:
/// - When this creature enters, reveal the top four cards of your library. Put all
///   Zombie cards revealed this way into your hand and the rest on the bottom of
///   your library in any order.
/// - `{1}{B}`: Regenerate this creature.
#[test]
fn grave_defiler_draws_revealed_zombies_from_top_four_and_regenerates() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                mountain(),
            ],
        )
        .hand(
            0,
            &[
                grave_defiler(),
                festering_goblin(),
                phyrexian_scuta(),
                grizzly_bears(),
                island(),
                lightning_bolt(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let miss_bear = hand_to_library_top(&mut engine, P0, grizzly_bears());
    let _hit_a = hand_to_library_top(&mut engine, P0, festering_goblin());
    let miss_island = hand_to_library_top(&mut engine, P0, island());
    let _hit_b = hand_to_library_top(&mut engine, P0, phyrexian_scuta());

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, grave_defiler());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![vec![miss_bear, miss_island]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let defiler = on_battlefield(&engine, P0, grave_defiler()).expect("Grave Defiler entered");
    assert_eq!(pt(&engine, defiler), (2, 1));
    assert!(in_hand(&engine, P0, festering_goblin()).is_some());
    assert!(in_hand(&engine, P0, phyrexian_scuta()).is_some());
    assert!(in_hand(&engine, P0, grizzly_bears()).is_none());

    // Pay {1}{B} to shield Grave Defiler, then cast Lightning Bolt at it -> regenerates.
    activate(&mut engine, P0, grave_defiler(), 1);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, defiler);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, grave_defiler()).is_some());
    assert!(
        engine
            .state()
            .object(defiler)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "regeneration taps Grave Defiler and clears lethal damage"
    );
    assert_eq!(engine.state().object(defiler).unwrap().damage, 0);
}
