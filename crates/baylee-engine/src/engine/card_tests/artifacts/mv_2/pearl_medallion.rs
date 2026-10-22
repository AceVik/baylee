//! `cards/artifacts/mv_2/pearl_medallion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn pearl_medallion() -> CardIndex {
    card_index("19015380-1332-4960-8cc6-0732009525a2")
}

/// Pearl Medallion (`19015380-1332-4960-8cc6-0732009525a2`) — {2} Artifact:
/// "White spells you cast cost {1} less to cast."
#[test]
fn pearl_medallion_reduces_white_spell_cost_by_one() {
    // Mesa Pegasus costs {1}{W}; with Pearl Medallion in play it is castable for {W}.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[pearl_medallion(), plains()])
        .hand(0, &[index::MESA_PEGASUS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, index::MESA_PEGASUS);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, index::MESA_PEGASUS).is_some());
}
