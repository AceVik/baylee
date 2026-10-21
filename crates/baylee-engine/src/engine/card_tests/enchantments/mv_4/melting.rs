//! `cards/enchantments/mv_4/melting.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn melting() -> CardIndex {
    card_index("599247f2-7f88-40ab-be08-e284e5f231d9")
}

/// Melting (599247f2-7f88-40ab-be08-e284e5f231d9) — {3}{R} Enchantment:
/// Coverage::Partial — plays as an enchantment with no effect.
#[test]
fn melting_casts_for_three_and_a_red_onto_the_battlefield() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[melting()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, melting());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, melting()).is_some());
}
