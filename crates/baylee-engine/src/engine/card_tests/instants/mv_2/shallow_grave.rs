//! `cards/instants/mv_2/shallow_grave.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn shallow_grave() -> CardIndex {
    card_index("9843d8a6-bcbf-40c2-8c41-316e97db9c62")
}

/// Shallow Grave (`{1}{B}`, `Coverage::Partial`): returns a creature card from
/// your graveyard to the battlefield without targeting.
#[test]
fn shallow_grave_returns_creature_card_from_your_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), grizzly_bears()])
        .hand(0, &[shallow_grave()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    bury(&mut engine, &[bear]);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());

    cast_from_hand(&mut engine, p0, shallow_grave());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(options.len(), 1);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
}
