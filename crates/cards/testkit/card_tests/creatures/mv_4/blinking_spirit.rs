//! `cards/creatures/mv_4/blinking_spirit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn blinking_spirit() -> CardIndex {
    card_index("a26f6958-9bc8-46f8-8ca7-7f55098e1291")
}

/// Blinking Spirit (a26f6958-9bc8-46f8-8ca7-7f55098e1291) — {3}{W} 2/2 Creature — Spirit:
/// - "{0}: Return Blinking Spirit to its owner's hand."
#[test]
fn blinking_spirit_returns_to_owners_hand_for_zero_mana() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[blinking_spirit()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let spirit = on_battlefield(&engine, P0, blinking_spirit()).expect("Blinking Spirit seated");
    assert_eq!(pt(&engine, spirit), (2, 2));

    activate(&mut engine, P0, blinking_spirit(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, blinking_spirit()).is_none());
    assert!(
        in_hand(&engine, P0, blinking_spirit()).is_some(),
        "returned to its owner's hand for {{0}}"
    );
}
