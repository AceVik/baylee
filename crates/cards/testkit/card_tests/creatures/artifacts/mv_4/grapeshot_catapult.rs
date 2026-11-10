//! `cards/creatures/artifacts/mv_4/grapeshot_catapult.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Grapeshot Catapult — {4} — 2/3 artifact creature: "{T}: This creature
/// deals 1 damage to target creature with flying."
///
/// "Creature with flying" is a targeting restriction (CR 115.1) read off
/// the flying keyword (CR 702.9), so the menu itself is the card: the
/// ground creature and the Catapult are not on it, and the flyer takes the
/// point — lethal to the 1/1 it was aimed at (CR 704.5g). The menu is
/// asserted and not only the damage, because a filter one creature too
/// wide looks exactly the same from the graveyard.
#[test]
fn grapeshot_catapult_hits_only_a_creature_with_flying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[grapeshot_catapult()])
        .battlefield(1, &[baleful_strix(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let catapult = on_battlefield(&engine, p0, grapeshot_catapult()).expect("seated");
    let flyer = on_battlefield(&engine, p1, baleful_strix()).expect("the flyer is seated");
    let ground = on_battlefield(&engine, p1, gray_ogre()).expect("the ground creature is seated");

    activate(&mut engine, p0, grapeshot_catapult(), 0);
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("the damage targets, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&flyer),
        "a creature with flying is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&ground),
        "a creature without flying is not: {options:?}"
    );
    assert!(
        !options.contains(&catapult),
        "and the Catapult has no flying itself: {options:?}"
    );
    assert_eq!((min, max), (1, 1), "exactly one creature");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![flyer],
            },
        )
        .expect("the flyer is the legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, catapult), "{{T}} was paid");
    assert!(
        in_graveyard(&engine, p1, baleful_strix()).is_some(),
        "one damage is lethal to the 1/1 flyer (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p1, gray_ogre()).is_some(),
        "the creature that was never a target is untouched"
    );
}
