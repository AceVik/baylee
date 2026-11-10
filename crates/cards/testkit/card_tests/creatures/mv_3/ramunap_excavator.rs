//! `cards/creatures/mv_3/ramunap_excavator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ramunap Excavator: "You may play lands from your graveyard."
/// A static permission identical to Crucible of Worlds on a 2/3 Snake Cleric creature.
/// With Ramunap Excavator on the battlefield, a Forest seeded in the graveyard is legally played onto the battlefield.
#[test]
fn ramunap_excavator_allows_playing_land_from_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(103, forest())
        .battlefield(0, &[ramunap_excavator()])
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
