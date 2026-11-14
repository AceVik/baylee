//! `cards/creatures/mv_3/hallowed_healer.rs`, played.
//!
//! Oracle:
//! - "{T}: Prevent the next 2 damage that would be dealt to any target this
//!   turn."
//! - "Threshold — {T}: Prevent the next 4 damage that would be dealt to any
//!   target this turn. Activate only if seven or more cards are in your
//!   graveyard."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn hallowed_healer() -> CardIndex {
    card_index("b92f6adb-64c6-4692-af9d-af4241203722")
}

fn aim_player(engine: &mut Engine<RegistryLookup>, target: PlayerId) {
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![target],
            },
        )
        .unwrap();
}

#[test]
fn hallowed_healer_prevents_two_damage_this_turn_and_gates_threshold_below_seven() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), hallowed_healer()])
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[plains(), index::SHOCK, lightning_bolt()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let healer = on_battlefield(&engine, P0, hallowed_healer()).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    assert_eq!(pt(&engine, healer), (1, 1));

    // At 5 cards (< 7), shield expires at end of turn and Threshold ability 1 is barred.
    seed_graveyard(&mut engine, P0, 5);
    play_land(&mut engine, P0, plains());
    assert!(ability_offered(&engine, healer, 0));
    assert!(!ability_offered(&engine, healer, 1));

    activate(&mut engine, P0, hallowed_healer(), 0);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().shields.len(), 1);
    reach_their_main_phase(&mut engine, P1);
    assert!(engine.state().shields.is_empty());
    assert!(walk_to_own_main(&mut engine, P0));

    // 6 cards via casting Shock at P1: Threshold still off; ability 0 prevents 2 of Bolt's 3 dmg.
    cast_from_hand(&mut engine, P0, index::SHOCK);
    aim_player(&mut engine, P1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        6
    );
    assert!(!ability_offered(&engine, healer, 1));

    activate(&mut engine, P0, hallowed_healer(), 0);
    aim_player(&mut engine, P0);
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_player(&mut engine, P0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 19);
}

#[test]
fn hallowed_healer_threshold_prevents_four_damage_across_multiple_sources() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), hallowed_healer()])
        .hand(0, &[plains(), lightning_bolt(), index::SHOCK])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let healer = on_battlefield(&engine, P0, hallowed_healer()).unwrap();
    seed_graveyard(&mut engine, P0, 7);
    play_land(&mut engine, P0, plains());
    assert!(ability_offered(&engine, healer, 1));

    activate(&mut engine, P0, hallowed_healer(), 1);
    aim_player(&mut engine, P0);
    pass_until(&mut engine, stack_is_empty);

    // Lightning Bolt deals 3 -> all 3 prevented, leaving 1 point of shield.
    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_player(&mut engine, P0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 20);

    // Shock deals 2 -> 1 prevented, 1 leaks through (20 -> 19).
    cast_from_hand(&mut engine, P0, index::SHOCK);
    aim_player(&mut engine, P0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 19);
}
