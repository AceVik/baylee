//! `cards/artifacts/mv_2/sapphire_medallion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::PHANTASMAL_FORCES;

fn sapphire_medallion() -> CardIndex {
    card_index("f1ef5698-14bd-42bf-aae5-2870699e4186")
}

/// `Sapphire Medallion` ({2}, Artifact, `Coverage::Implemented`):
/// `Blue spells you cast cost {1} less to cast.`
#[test]
fn sapphire_medallion_reduces_the_generic_cost_of_blue_spells_you_cast_by_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        // 3 Islands + Sapphire Medallion: enough to cast Phantasmal Forces ({3}{U} -> {2}{U}).
        .battlefield(0, &[island(), island(), island(), sapphire_medallion()])
        .hand(0, &[PHANTASMAL_FORCES])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, PHANTASMAL_FORCES);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, PHANTASMAL_FORCES).is_some());
}
