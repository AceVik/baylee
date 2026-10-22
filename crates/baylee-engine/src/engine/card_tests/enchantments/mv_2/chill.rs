//! `cards/enchantments/mv_2/chill.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn chill() -> CardIndex {
    card_index("70a93d0f-9a02-4e60-a2b8-827fd2410604")
}

/// Chill (70a93d0f-9a02-4e60-a2b8-827fd2410604) — {1}{U} Enchantment:
/// "Red spells cost {2} more to cast."
#[test]
fn chill_increases_red_spell_cost_by_two() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[chill(), mountain(), mountain()])
        .hand(0, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    let bolt = in_hand(&engine, P0, index::LIGHTNING_BOLT).unwrap();
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: bolt })
            .is_err(),
        "with only two red mana available, Lightning Bolt cannot pay two generic plus one red under Chill"
    );
}
