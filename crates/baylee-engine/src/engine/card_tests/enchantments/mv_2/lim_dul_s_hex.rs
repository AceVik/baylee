//! `cards/enchantments/mv_2/lim_dul_s_hex.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P1: PlayerId = PlayerId::new(1);

fn lim_dul_s_hex() -> CardIndex {
    card_index("7f9116e1-9aab-470b-90eb-f51a3fbb3c0e")
}

/// Lim-Dûl's Hex (7f9116e1-9aab-470b-90eb-f51a3fbb3c0e) — {1}{B} Enchantment:
/// "At the beginning of your upkeep, for each player, this enchantment deals 1 damage to that player unless they pay {B} or {3}."
#[test]
fn lim_dul_s_hex_deals_one_damage_to_each_player_on_your_upkeep_and_not_opponents() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[lim_dul_s_hex()])
        .start();
    keep_mulligans(&mut engine);

    // P0's upkeep (turn 1): deals 1 damage to each player.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 1 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 19);

    // P1's upkeep (turn 2): does not trigger on opponent's upkeep.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2
            && e.state().turn.active == P1
            && e.state().turn.step == Step::Draw
            && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 19);
}
