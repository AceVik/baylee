//! `cards/enchantments/mv_2/convalescence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn convalescence() -> CardIndex {
    card_index("9ad86d0e-3363-4d98-b0bb-dc491be1ddd5")
}

/// Convalescence (`9ad86d0e-3363-4d98-b0bb-dc491be1ddd5`) — {1}{W} Enchantment (Implemented):
/// "At the beginning of your upkeep, if you have 10 or less life, you gain 1 life."
#[test]
fn convalescence_gains_one_life_on_upkeep_only_when_at_ten_or_less_life() {
    for life in [10, 11, 20] {
        let mut engine = Duel::new(SEED, plains())
            .battlefield(0, &[convalescence()])
            .life(0, life)
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, P0);

        let expected = if life <= 10 { life + 1 } else { life };
        assert_eq!(engine.state().players[0].life, expected);
        reach_their_main_phase(&mut engine, PlayerId::new(1));
        assert_eq!(engine.state().players[0].life, expected);
    }
}
