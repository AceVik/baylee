//! `cards/creatures/artifacts/mv_5/sand_golem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sand_golem() -> CardIndex {
    card_index("0f9b78ca-1556-4157-a91c-55276fd78127")
}

fn grizzly_bears() -> CardIndex {
    baylee_core::generated::index::GRIZZLY_BEARS
}

fn hymn_to_tourach() -> CardIndex {
    baylee_core::generated::index::HYMN_TO_TOURACH
}

/// Sand Golem (`{5}`, `Coverage::Partial`): 3/3 Artifact Creature — Golem that
/// triggers when a spell or ability an opponent controls causes you to discard
/// it and returns a Sand Golem card from your graveyard to the battlefield at
/// the beginning of the next end step.
#[test]
fn sand_golem_returns_to_battlefield_at_next_end_step_when_discarded_by_opponent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(1, &[swamp(), swamp()])
        .hand(0, &[sand_golem(), grizzly_bears()])
        .hand(1, &[hymn_to_tourach()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, hymn_to_tourach());
    engine.apply(p1, PlayerAction::ChoosePlayer(p0)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let golem = in_graveyard(&engine, p0, sand_golem()).unwrap();
    assert!(
        in_graveyard(&engine, p0, grizzly_bears()).is_some(),
        "Hymn to Tourach took both cards of the two-card hand"
    );

    // Advance to the end step: the delayed trigger asks which card to return.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(
        options,
        vec![golem],
        "it returns this card, not any creature card: the discarded Grizzly Bears stay"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: options })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, sand_golem()).is_some());
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
}
