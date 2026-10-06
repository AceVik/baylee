//! `cards/sorceries/mv_1/reconstruction.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Reconstruction — {U} — "Return target artifact card from your graveyard
/// to your hand."
///
/// The menu is the card. It holds the artifact card in the caster's own
/// graveyard; it does not hold the creature card beside it (the artifact
/// filter) and does not hold the artifact in the opponent's graveyard
/// (CR 400.3 — a graveyard is its owner's, and the card says "your"). The
/// named card arrives in hand and leaves no card behind, while the two
/// witnesses stay in the graveyards they were in.
#[test]
fn reconstruction_returns_your_own_artifact_card_from_among_the_graveyards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &[reconstruction(), quiet_artifact(), quiet_creature()])
        .hand(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = hand_to_graveyard(&mut engine, p0, quiet_artifact());
    let elf = hand_to_graveyard(&mut engine, p0, quiet_creature());
    let theirs = hand_to_graveyard(&mut engine, p1, quiet_artifact());

    cast_from_hand(&mut engine, p0, reconstruction());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target artifact card from your graveyard\" is a target, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster chooses");
    assert_eq!((min, max), (1, 1), "one card");
    assert!(
        options.contains(&mine),
        "my artifact card is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature card is no artifact: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"your graveyard\", not theirs: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and the artifact card is the whole menu: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the card the question offered is the one that is named");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, quiet_artifact()).is_some(),
        "the artifact card is back in its owner's hand"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_none(),
        "and no longer in the graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_creature()).is_some(),
        "the creature card was not a legal target and stayed"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "and the opponent's artifact card never moved"
    );
}
