//! `cards/enchantments/mv_3/hidden_retreat.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn hidden_retreat() -> CardIndex {
    card_index("a116329a-343e-4f10-a122-38bf8b5ac2c8")
}

/// Hidden Retreat (`a116329a-343e-4f10-a122-38bf8b5ac2c8`) — {2}{W} Enchantment (Partial).
#[test]
fn hidden_retreat_casts_and_enters_battlefield() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[hidden_retreat()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, hidden_retreat());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, hidden_retreat()).is_some());
}
