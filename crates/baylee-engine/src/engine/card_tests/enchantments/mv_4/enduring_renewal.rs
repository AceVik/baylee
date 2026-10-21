//! `cards/enchantments/mv_4/enduring_renewal.rs`, played.
//!
//! Oracle:
//! - "Play with your hand revealed."
//! - "If you would draw a card, reveal the top card of your library instead. If it's a creature card, put it into your graveyard. Otherwise, draw a card."
//! - "Whenever a creature is put into your graveyard from the battlefield, return it to your hand."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "47a080c4-ff04-4f52-aca0-2b8e4f4d931e"
fn enduring_renewal() -> CardIndex {
    card_index("47a080c4-ff04-4f52-aca0-2b8e4f4d931e")
}

#[test]
fn enduring_renewal_returns_dying_creature_from_your_graveyard_to_your_hand() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[enduring_renewal(), index::LIGHTNING_BOLT],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::White, 2)],
    );
    cast_with_floating(&mut engine, P0, enduring_renewal());
    pass_until(&mut engine, stack_is_empty);

    // Kill Grizzly Bears with Lightning Bolt: Enduring Renewal triggers and returns it to P0's hand.
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "creature put into your graveyard from the battlefield is returned to your hand"
    );
    assert!(
        in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_none(),
        "creature is no longer in the graveyard"
    );
}

#[test]
fn enduring_renewal_leaves_an_opponents_creature_in_their_graveyard() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[enduring_renewal(), index::LIGHTNING_BOLT],
        &[],
    );
    let their_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::White, 2)],
    );
    cast_with_floating(&mut engine, P0, enduring_renewal());
    pass_until(&mut engine, stack_is_empty);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[their_bears], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "a creature put into another player's graveyard is not returned"
    );
    assert!(in_hand(&engine, P1, index::GRIZZLY_BEARS).is_none());
    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_none());
}
