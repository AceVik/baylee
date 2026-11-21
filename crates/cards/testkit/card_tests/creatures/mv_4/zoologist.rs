//! `cards/creatures/mv_4/zoologist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn zoologist() -> CardIndex {
    card_index("878e4e3b-c546-4174-906e-6fe8e64d9d41")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Zoologist (878e4e3b-c546-4174-906e-6fe8e64d9d41) — `{3}{G}` 1/2
/// Creature — Human Druid:
/// "`{3}{G}`, `{T}`: Reveal the top card of your library. If it's a creature card,
/// put it onto the battlefield. Otherwise, put it into your graveyard."
#[test]
fn zoologist_puts_revealed_creature_onto_battlefield_and_noncreature_into_graveyard() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[zoologist(), hill_giant(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, zoologist());
    pass_until(&mut engine, stack_is_empty);
    let zoo = on_battlefield(&engine, P0, zoologist()).expect("Zoologist entered");
    assert_eq!(pt(&engine, zoo), (1, 2));

    // Turn 3: top card is Hill Giant (creature) -> enters the battlefield under P0's control.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let _ = hand_to_library_top(&mut engine, P0, hill_giant());
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, zoologist(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, hill_giant()).is_some());
    assert!(in_graveyard(&engine, P0, hill_giant()).is_none());

    // Turn 5: top card is Giant Growth (noncreature) -> goes into P0's graveyard.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let _ = hand_to_library_top(&mut engine, P0, giant_growth());
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, zoologist(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, giant_growth()).is_some());
}
