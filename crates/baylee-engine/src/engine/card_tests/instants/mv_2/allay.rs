//! `cards/instants/mv_2/allay.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn allay() -> CardIndex {
    card_index("664d982a-5cd8-4289-afb8-fa2e083042c9")
}

/// Allay (`664d982a-5cd8-4289-afb8-fa2e083042c9`) — {1}{W} Instant (Partial):
/// "Buyback {3}. Destroy target enchantment."
#[test]
fn allay_destroys_target_enchantment() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[index::CRUSADE])
        .hand(0, &[allay()])
        .start();
    assert!(walk_to_own_main(&mut engine, P0));

    let ench = on_battlefield(&engine, P1, index::CRUSADE).unwrap();
    cast_from_hand(&mut engine, P0, allay());
    let land = on_battlefield(&engine, P0, plains()).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("targets")
    };
    assert!(!options.contains(&land));
    aim_at(&mut engine, P0, ench);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::CRUSADE).is_none());
    assert!(in_graveyard(&engine, P1, index::CRUSADE).is_some());
    assert!(
        in_graveyard(&engine, P0, allay()).is_some(),
        "without buyback it goes to the graveyard"
    );
}
