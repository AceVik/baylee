//! `cards/enchantments/mv_1/vampiric_rites.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn vampiric_rites() -> CardIndex {
    card_index("660de988-b6fb-4f36-8006-42af3e7f908d")
}

/// "{1}{B}, Sacrifice a creature: You gain 1 life and draw a card."
#[test]
fn vampiric_rites_sacrifices_a_creature_to_gain_life_and_draw() {
    let mut e = table(7261, &[], &[vampiric_rites(), index::GRIZZLY_BEARS], &[]);
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let hand = hand_size(&e, P0);
    activate(&mut e, P0, vampiric_rites(), 0);
    drive(&mut e, &[], &[], &[bears], true);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(life(&e, P0), 21);
    assert_eq!(hand_size(&e, P0), hand + 1);
    assert!(in_graveyard(&e, P0, index::GRIZZLY_BEARS).is_some());
}
