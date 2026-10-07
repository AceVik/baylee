//! `cards/lands/caves/volatile_fault.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Volatile Fault prints `{{T}}: Add {{C}}` and `{{1}}, {{T}}, Sacrifice this land: Destroy target nonbasic land an opponent controls. That player may search their library for a basic land card, put it onto the battlefield, then shuffle. You create a Treasure token.`
/// The card is marked `Coverage::Partial` because the searched land enters tapped instead of untapped.
/// Activating the sacrifice ability destroys the opponent's nonbasic land, allows them to search for a basic land, and creates a Treasure token for the activator.
#[test]
fn volatile_fault_destroys_opponent_nonbasic_and_creates_treasure() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[volatile_fault(), forest()])
        .battlefield(1, &[badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target = on_battlefield(&engine, p1, badlands()).expect("Badlands deployed");
    let fault = on_battlefield(&engine, p0, volatile_fault()).expect("Fault deployed");

    tap_mana_except(&mut engine, p0, fault);
    activate(&mut engine, p0, volatile_fault(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice");
    };
    assert!(options.contains(&target));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();

    assert!(in_graveyard(&engine, p0, volatile_fault()).is_some());

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });

    let Pending::ChooseCards {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected search");
    };
    assert_eq!(player, p1);
    assert_eq!((min, max), (0, 1));
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, badlands()).is_some());
    assert_eq!(tokens_of(&engine, p0).len(), 1);
}
