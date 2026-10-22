//! `cards/artifacts/mv_2/emerald_medallion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn emerald_medallion() -> CardIndex {
    card_index("d683024d-8fe7-4509-851b-7655fadc1de7")
}

/// Emerald Medallion (`d683024d-8fe7-4509-851b-7655fadc1de7`) — {2} Artifact:
/// "Green spells you cast cost {1} less to cast."
#[test]
fn emerald_medallion_reduces_green_spell_cost_by_one() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[emerald_medallion(), forest()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Grizzly Bears ({1}{G}) can be cast with a single Forest thanks to Emerald Medallion.
    cast_from_hand(&mut engine, P0, index::GRIZZLY_BEARS);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
}
