//! `cards/creatures/mv_2/skulking_ghost.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn skulking_ghost() -> CardIndex {
    card_index("cd97f8b4-e751-4b52-987b-a442aef57809")
}

/// Skulking Ghost (`{1}{B}`, `Coverage::Implemented`): 2/1 flying Spirit that
/// sacrifices itself when it becomes the target of a spell or ability.
#[test]
fn skulking_ghost_sacrifices_itself_when_targeted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[skulking_ghost(), forest()])
        .hand(0, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ghost = on_battlefield(&engine, p0, skulking_ghost()).unwrap();
    assert!(keywords(&engine, ghost).contains(KeywordSet::FLYING));

    cast_from_hand(&mut engine, p0, giant_growth());
    aim_at(&mut engine, p0, ghost);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, skulking_ghost()).is_none());
    assert!(in_graveyard(&engine, p0, skulking_ghost()).is_some());
}
