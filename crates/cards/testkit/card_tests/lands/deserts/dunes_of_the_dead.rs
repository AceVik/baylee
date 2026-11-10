//! `cards/lands/deserts/dunes_of_the_dead.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dunes of the Dead: "{T}: Add {C}." / "When this land is put into a graveyard from the battlefield, create a 2/2 black Zombie creature token."
/// Destroying Dunes of the Dead with Vindicate triggers its death trigger.
/// On resolution, Dunes of the Dead is in the graveyard and a 2/2 Zombie creature token is created on the battlefield.
#[test]
fn dunes_of_the_dead_creates_zombie_token_on_death() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(122, forest())
        .battlefield(0, &[dunes_of_the_dead(), plains(), swamp(), forest()])
        .hand(0, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dunes = on_battlefield(&engine, p0, dunes_of_the_dead()).expect("Dunes deployed");
    tap_mana_except(&mut engine, p0, dunes);

    cast_from_hand(&mut engine, p0, vindicate());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Vindicate");
    };
    assert!(options.contains(&dunes));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![dunes],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, dunes_of_the_dead()).is_some());
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Zombie token created");
    assert_eq!(pt(&engine, tokens[0]), (2, 2));
}
