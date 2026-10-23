//! `cards/creatures/mv_4/maro.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "ef32258a-d567-4fbc-ab85-8c00080f5b36"
fn maro() -> CardIndex {
    card_index("ef32258a-d567-4fbc-ab85-8c00080f5b36")
}

/// Maro (`Coverage::Partial`): the "cards in your hand" power and toughness
/// are not in `PtCount`, so the card plays as a 0/0 and is put into the
/// graveyard as soon as it enters. This test pins that limitation; when
/// `PtCount` can count a hand, it becomes a test of the count.
#[test]
fn maro_enters_as_a_zero_zero_and_dies_until_pt_counts_a_hand() {
    let mut engine = lk::game(SEED, &[], &[], &[maro()], &[]);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Green, 2)],
    );
    cast_with_floating(&mut engine, P0, maro());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, maro()).is_none(),
        "a 0/0 does not stay on the battlefield"
    );
    assert!(
        in_graveyard(&engine, P0, maro()).is_some(),
        "Maro enters as a 0/0 creature and is put into the graveyard by state-based actions"
    );
}
