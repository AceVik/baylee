//! `cards/enchantments/mv_3/forsaken_wastes.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn forsaken_wastes() -> CardIndex {
    card_index("b9e61e68-9dc8-4295-95dc-dd66a0907c8c")
}

/// Forsaken Wastes (b9e61e68-9dc8-4295-95dc-dd66a0907c8c) — {2}{B} World Enchantment:
/// "Players can't gain life. At the beginning of each player's upkeep, that
/// player loses 1 life."
#[test]
fn forsaken_wastes_makes_active_player_lose_one_life_at_upkeep() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[forsaken_wastes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, forsaken_wastes());
    pass_until(&mut engine, stack_is_empty);

    // Advance to P1's main phase (passing through P1's upkeep trigger).
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(engine.state().players[0].life, 20);
}

/// "At the beginning of each player's upkeep, that player loses 1 life": the
/// controller loses 1 at its own next upkeep too, and the opponent's loss was
/// one, not two.
#[test]
fn forsaken_wastes_drains_each_player_once_at_their_own_upkeep() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[forsaken_wastes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, forsaken_wastes());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "its own turn-1 upkeep is past"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 19);
}
