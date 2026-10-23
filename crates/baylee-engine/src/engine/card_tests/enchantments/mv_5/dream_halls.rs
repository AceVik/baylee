//! `cards/enchantments/mv_5/dream_halls.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn dream_halls() -> CardIndex {
    card_index("d901e17f-a5f4-49d9-8a6d-ab771949af24")
}

/// Dream Halls (`d901e17f-a5f4-49d9-8a6d-ab771949af24`) — {3}{U}{U} Enchantment (Partial):
/// Casts and enters the battlefield as an enchantment.
#[test]
fn dream_halls_casts_and_enters_battlefield() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[dream_halls()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, dream_halls());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, dream_halls()).is_some());
}
