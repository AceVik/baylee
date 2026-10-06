//! `cards/lands/fetch/demolition_field.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Demolition Field: "{T}: Add {C}." / "{2}, {T}, Sacrifice this land: Destroy target nonbasic land an opponent controls..."
/// Under `Coverage::Partial`, the opponent's basic land enters tapped via `Effect::OptionalBasicLandSearchFor`.
/// Activating ability 1 destroys an opponent's target nonbasic land and puts Demolition Field into its owner's graveyard.
#[test]
fn demolition_field_destroys_opponent_nonbasic_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(210, forest())
        .battlefield(0, &[demolition_field(), forest(), forest()])
        .battlefield(1, &[badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target = on_battlefield(&engine, p1, badlands()).expect("Badlands deployed");
    let field = on_battlefield(&engine, p0, demolition_field()).expect("Field deployed");

    tap_mana_except(&mut engine, p0, field);
    activate(&mut engine, p0, demolition_field(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
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

    assert!(in_graveyard(&engine, p0, demolition_field()).is_some());

    for _ in 0..8 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "unexpected while waiting for search: {:?}",
                engine.pending()
            );
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }

    let Pending::ChooseCards {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected p1 basic search, got {:?}", engine.pending());
    };
    assert_eq!(player, p1);
    assert_eq!((min, max), (0, 1));
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();

    let Pending::ChooseCards {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected p0 basic search, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (0, 1));
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, badlands()).is_some());
    assert!(on_battlefield(&engine, p1, badlands()).is_none());
}
