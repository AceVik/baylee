//! `cards/enchantments/mv_1/tortured_existence.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tortured_existence() -> CardIndex {
    card_index("fa9dfcf7-069c-4441-9ea0-133de47ecada")
}

/// "{B}, Discard a creature card: Return target creature card from your
/// graveyard to your hand."
#[test]
fn tortured_existence_trades_a_creature_card_in_hand_for_one_in_the_graveyard() {
    let mut e = table(
        7241,
        &[index::WIND_DRAKE],
        &[tortured_existence(), index::GRIZZLY_BEARS],
        &[],
    );
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    bury(&mut e, &[bears]);
    let gy = in_graveyard(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let drake = in_hand(&e, P0, index::WIND_DRAKE).unwrap();
    fill_pool(&mut e);
    activate(&mut e, P0, tortured_existence(), 0);
    drive(&mut e, &[gy], &[], &[drake], true);
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, P0, index::GRIZZLY_BEARS).is_some(), "returned");
    assert!(
        in_graveyard(&e, P0, index::WIND_DRAKE).is_some(),
        "discarded"
    );
    assert!(in_hand(&e, P0, index::WIND_DRAKE).is_none());
}
