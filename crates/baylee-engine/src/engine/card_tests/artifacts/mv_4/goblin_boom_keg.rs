//! `cards/artifacts/mv_4/goblin_boom_keg.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn goblin_boom_keg() -> CardIndex {
    card_index("8e819d68-3a6e-449a-8392-e072724b80f1")
}

/// "At the beginning of your upkeep, sacrifice this artifact." and "When this
/// artifact is put into a graveyard from the battlefield, it deals 3 damage
/// to any target." The opponent's upkeep leaves it alone; its controller's
/// sacrifices it, and the trigger asks for a target.
#[test]
fn the_keg_is_sacrificed_on_its_controllers_upkeep_and_then_explodes() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[goblin_boom_keg()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, goblin_boom_keg()).is_some(),
        "the opponent's upkeep is not \"your upkeep\""
    );

    let life = life_of(&engine, p1);
    let options = pass_until_targets(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, goblin_boom_keg()).is_none(),
        "the Keg is gone by the time its trigger asks for a target"
    );
    assert!(in_graveyard(&engine, p0, goblin_boom_keg()).is_some());
    let _ = options;
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("any target includes a player");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p1), life - 3, "3 damage to the target");
}
