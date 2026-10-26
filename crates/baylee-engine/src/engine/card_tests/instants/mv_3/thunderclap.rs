//! `cards/instants/mv_3/thunderclap.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "02f5ceb4-8b9c-4107-972b-6eec365c4a6c"
fn thunderclap() -> CardIndex {
    card_index("02f5ceb4-8b9c-4107-972b-6eec365c4a6c")
}

/// `Thunderclap` (`Coverage::Partial`: base mana cost `{2}{R}`):
/// "Thunderclap deals 3 damage to target creature."
#[test]
fn thunderclap_deals_three_damage_to_target_creature() {
    let mut engine = lk::game(
        SEED,
        &[index::MOUNTAIN, index::MOUNTAIN, index::MOUNTAIN],
        &[index::TRAINED_ARMODON],
        &[thunderclap()],
        &[],
    );
    let armodon = lk::obj(&engine, P1, index::TRAINED_ARMODON); // 3/3

    cast_from_hand(&mut engine, P0, thunderclap());
    lk::aim(&mut engine, &[armodon], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, index::TRAINED_ARMODON).is_some());
}
