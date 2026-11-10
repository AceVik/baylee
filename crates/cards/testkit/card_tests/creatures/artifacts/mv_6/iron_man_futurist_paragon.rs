//! `cards/creatures/artifacts/mv_6/iron_man_futurist_paragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn iron_man_futurist_paragon() -> CardIndex {
    card_index("eabe9a02-ea47-4548-8933-e5d1269fab25")
}

/// Iron Man, Futurist Paragon — flying; "At the beginning of combat on your
/// turn, target artifact or creature becomes an artifact creature with base
/// power and toughness 5/5 and gains flying." A Sol Ring is a legal target, a
/// Forest is not, and the Ring comes out a 5/5 flying artifact creature.
#[test]
fn iron_man_futurist_paragon_animates_an_artifact_at_the_beginning_of_combat() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[iron_man_futurist_paragon(), sol_ring(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let iron = on_battlefield(&engine, p0, iron_man_futurist_paragon()).expect("seated");
    let ring = on_battlefield(&engine, p0, sol_ring()).expect("seated");
    let land = on_battlefield(&engine, p0, forest()).expect("seated");
    assert_eq!(pt(&engine, iron), (5, 5));
    assert!(keywords(&engine, iron).contains(KeywordSet::FLYING));
    assert!(!keywords(&engine, ring).contains(KeywordSet::FLYING));

    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&ring) && options.contains(&iron));
    assert!(
        !options.contains(&land),
        "a land is neither an artifact nor a creature"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_artifact_creature(&engine, ring),
        "the Ring is now an artifact creature"
    );
    assert_eq!(pt(&engine, ring), (5, 5));
    assert!(keywords(&engine, ring).contains(KeywordSet::FLYING));
}
