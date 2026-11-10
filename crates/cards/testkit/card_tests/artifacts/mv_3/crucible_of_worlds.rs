//! `cards/artifacts/mv_3/crucible_of_worlds.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crucible of Worlds: "You may play lands from your graveyard."
/// With Crucible of Worlds on the battlefield, a land card in the graveyard is offered as a legal land play.
/// Playing the land moves it directly from the graveyard to the battlefield and spends the turn's land drop.
#[test]
fn crucible_of_worlds_allows_playing_land_from_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest())
        .battlefield(0, &[crucible_of_worlds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let gy_forest = in_graveyard(&engine, p0, forest()).expect("forest in graveyard");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.lands.contains(&gy_forest),
        "graveyard land offered as legal land play"
    );

    engine
        .apply(p0, PlayerAction::PlayLand { card: gy_forest })
        .unwrap();

    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "forest moved to battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_none(),
        "forest left graveyard"
    );
}
