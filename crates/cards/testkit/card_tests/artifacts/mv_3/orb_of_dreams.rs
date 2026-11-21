//! `cards/artifacts/mv_3/orb_of_dreams.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn orb_of_dreams() -> CardIndex {
    card_index("2c24b7c3-7c90-4109-a4aa-d9cae6febf34")
}

/// Orb of Dreams (2c24b7c3-7c90-4109-a4aa-d9cae6febf34) — {3} Artifact:
/// "Permanents enter tapped."
///
/// Causes lands, creatures, and artifacts played by both you and your opponent
/// to enter the battlefield tapped.
#[test]
fn orb_of_dreams_makes_all_permanents_enter_tapped_for_both_players() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[orb_of_dreams(), forest(), forest()])
        .hand(0, &[forest(), llanowar_elves(), sol_ring()])
        .battlefield(1, &[forest()])
        .hand(1, &[forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // 1. p0 plays a land -> enters tapped.
    let land_hand = in_hand(&engine, p0, forest()).unwrap();
    engine
        .apply(p0, PlayerAction::PlayLand { card: land_hand })
        .unwrap();
    assert!(is_tapped(&engine, land_hand), "land enters tapped");

    // 2. p0 casts a creature and an artifact -> both enter tapped.
    let my_forests = forests(&engine, p0, 2);
    tap_mana_where(&mut engine, p0, |id| id == my_forests[0]);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    assert!(is_tapped(&engine, my_elf), "own creature enters tapped");

    tap_mana_where(&mut engine, p0, |id| id == my_forests[1]);
    cast_with_floating(&mut engine, p0, sol_ring());
    pass_until(&mut engine, stack_is_empty);
    let my_ring = on_battlefield(&engine, p0, sol_ring()).unwrap();
    assert!(is_tapped(&engine, my_ring), "own artifact enters tapped");

    // 3. Opponent plays a land and casts a creature -> both enter tapped.
    reach_their_main_phase(&mut engine, p1);
    let their_land = in_hand(&engine, p1, forest()).unwrap();
    engine
        .apply(p1, PlayerAction::PlayLand { card: their_land })
        .unwrap();
    assert!(
        is_tapped(&engine, their_land),
        "opponent's land enters tapped"
    );

    cast_from_hand(&mut engine, p1, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    assert!(
        is_tapped(&engine, their_elf),
        "opponent's creature enters tapped"
    );
}
