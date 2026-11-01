//! `cards/enchantments/mv_4/squeeze.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "e18d8326-c3c3-43c2-ac13-6861c0ef3888"
fn squeeze() -> CardIndex {
    card_index("e18d8326-c3c3-43c2-ac13-6861c0ef3888")
}

/// `Squeeze` (`Coverage::Implemented`):
/// "Sorcery spells cost {3} more to cast."
#[test]
fn squeeze_taxes_sorcery_spells_by_three_but_not_instants() {
    let mut engine = lk::game(
        SEED,
        &[squeeze()],
        &[index::MOUNTAIN],
        &[index::STONE_RAIN, index::LIGHTNING_BOLT],
        &[],
    );
    resync(&mut engine);

    let their_land = lk::obj(&engine, P1, index::MOUNTAIN);
    let rain = in_hand(&engine, P0, index::STONE_RAIN).expect("Stone Rain in hand");
    let bolt = in_hand(&engine, P0, index::LIGHTNING_BOLT).expect("Lightning Bolt in hand");

    // Stone Rain costs {2}{R}; with Squeeze it costs {5}{R}. Floating only {2}{R} is not enough.
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
    );
    assert!(
        !priority_offer(&engine).castable.contains(&rain),
        "Stone Rain should not be castable with only {{2}}{{R}} while Squeeze is on the battlefield"
    );

    // Instant spells are unaffected: {R} is enough for Lightning Bolt.
    assert!(
        priority_offer(&engine).castable.contains(&bolt),
        "Lightning Bolt (instant) should still cost {{R}}"
    );

    // Add {3} more mana (total {5}{R} available for Stone Rain) and Stone Rain becomes castable.
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 3)]);
    assert!(
        priority_offer(&engine).castable.contains(&rain),
        "Stone Rain becomes castable once {{5}}{{R}} is available"
    );
    cast_with_floating(&mut engine, P0, index::STONE_RAIN);
    lk::aim(&mut engine, &[their_land], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, index::MOUNTAIN).is_some());
}
