//! `cards/creatures/mv_3/argivian_archaeologist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Argivian Archaeologist — {1}{W}{W} — 1/1 Human Artificer: "{W}{W},
/// {T}: Return target artifact card from your graveyard to your hand."
///
/// The menu is two restrictions read together: the card must be an artifact
/// and it must be in **your** graveyard, where "you" is the ability's
/// controller (CR 109.5). Three cards wait in graveyards here — the
/// artifact seeded into this controller's, a land beside it, and an
/// artifact in the opponent's — and only the first may be named (CR 115.1).
/// The returned card is found in hand by its card index, since the zone
/// change makes it a new object (CR 400.7).
#[test]
fn argivian_archaeologist_returns_only_your_artifact_from_your_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, quiet_artifact())
        .battlefield(0, &[argivian_archaeologist(), plains(), plains(), forest()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The library is Sol Rings, so the seed is an artifact card; the Forest
    // beside it and the opponent's Sol Ring are the controls.
    seed_graveyard(&mut engine, p0, 1);
    let their_artifact = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact");
    let artifact_card = in_graveyard(&engine, p0, quiet_artifact()).expect("the seeded artifact");
    let land_card = on_battlefield(&engine, p0, forest()).expect("the land to bury");
    bury(&mut engine, &[land_card, their_artifact]);
    assert!(in_graveyard(&engine, p0, forest()).is_some());
    assert!(in_graveyard(&engine, p1, quiet_artifact()).is_some());

    let archaeologist = on_battlefield(&engine, p0, argivian_archaeologist()).expect("seated");
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2, "two Plains");
    activate(&mut engine, p0, argivian_archaeologist(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the return targets, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![artifact_card],
        "only the artifact card in your graveyard is a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![artifact_card],
                players: vec![],
            },
        )
        .expect("it is the only legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, quiet_artifact()).is_some(),
        "the artifact card came back to hand"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_none(),
        "and left the graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "the land card beside it did not move"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "nor did the opponent's artifact"
    );
    assert!(is_tapped(&engine, archaeologist), "{{T}} was paid");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{W}}{{W}} was paid"
    );
}
