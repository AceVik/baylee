//! `cards/sorceries/mv_2/cannibalize.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cannibalize() -> CardIndex {
    card_index("bf044267-92b9-4983-964f-2bc4ecf76bce")
}

/// Cannibalize (`bf044267-92b9-4983-964f-2bc4ecf76bce`) — {1}{B} Sorcery (Partial):
/// "Exile target creature."
#[test]
fn cannibalize_exiles_target_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[cannibalize()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, cannibalize());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(P1)).len(), 1);
}
