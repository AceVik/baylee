//! `cards/artifacts/mv_2/jet_medallion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn jet_medallion() -> CardIndex {
    card_index("bccb06f1-8839-4104-9ffa-c79a817f8378")
}

/// Jet Medallion (`bccb06f1-8839-4104-9ffa-c79a817f8378`) — {2} Artifact:
/// "Black spells you cast cost {1} less to cast."
#[test]
fn jet_medallion_reduces_black_spell_cost_by_one() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[jet_medallion(), swamp()])
        .hand(0, &[index::SCATHE_ZOMBIES])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Scathe Zombies costs {2}{B}, so with one Jet Medallion and only one Swamp it still needs {1} more;
    // with a second Swamp on the battlefield, {2}{B} reduced by {1} is castable for {1}{B}.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[jet_medallion(), swamp(), swamp()])
        .hand(0, &[index::SCATHE_ZOMBIES])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, index::SCATHE_ZOMBIES);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, index::SCATHE_ZOMBIES).is_some());
}
