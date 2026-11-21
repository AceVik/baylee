//! `cards/creatures/mv_4/yavimaya_kavu.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn yavimaya_kavu() -> CardIndex {
    card_index("285c9f49-4aeb-468c-bb41-f5ccb5257e2b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Yavimaya Kavu (285c9f49-4aeb-468c-bb41-f5ccb5257e2b) — `{2}{R}{G}` `*/*`
/// Creature — Kavu:
/// - Yavimaya Kavu's power is equal to the number of red creatures on the battlefield.
/// - Yavimaya Kavu's toughness is equal to the number of green creatures on the battlefield.
#[test]
fn yavimaya_kavu_counts_red_creatures_for_power_and_green_creatures_for_toughness() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), forest(), forest(), grizzly_bears()],
        )
        .battlefield(1, &[hill_giant(), hill_giant()])
        .hand(0, &[yavimaya_kavu(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Off-battlefield CDA in hand: 2 red creatures (P1's two Hill Giants), 1 green (P0's Bears).
    let in_hand_id = in_hand(&engine, P0, yavimaya_kavu()).unwrap();
    assert_eq!(pt(&engine, in_hand_id), (2, 1));

    cast_from_hand(&mut engine, P0, yavimaya_kavu());
    pass_until(&mut engine, stack_is_empty);

    // Yavimaya Kavu itself is both red and green, so counts become 3 red / 2 green -> 3/2.
    let kavu = on_battlefield(&engine, P0, yavimaya_kavu()).expect("Yavimaya Kavu entered");
    assert_eq!(pt(&engine, kavu), (3, 2));

    // Next turn cycle: killing one Hill Giant drops red creature count to 2 -> 2/2.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, P0);
    let giant = on_battlefield(&engine, PlayerId::new(1), hill_giant()).unwrap();
    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, kavu), (2, 2));
}
