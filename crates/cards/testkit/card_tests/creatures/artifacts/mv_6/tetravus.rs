//! `cards/creatures/artifacts/mv_6/tetravus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn tetravus_splits_into_uncantable_tetravite_tokens_and_reabsorbs_them_on_upkeep() {
    let tetravus = baylee_core::generated::index::TETRAVUS;
    let holy_strength = baylee_core::generated::index::HOLY_STRENGTH;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .hand(0, &[tetravus, holy_strength])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tetra = cast_resolved(&mut engine, p0, tetravus);
    assert_eq!(counters_on(&engine, tetra, CounterKind::P1P1), 3);
    assert_eq!(pt(&engine, tetra), (4, 4));

    // Pass through p1's turn to p0's next upkeep.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    // Remove 2 +1/+1 counters to create 2 Tetravite tokens.
    engine
        .apply(p0, PlayerAction::ChooseNumber(2))
        .expect("choose to remove 2 counters");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(counters_on(&engine, tetra, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, tetra), (2, 2));

    let tetravites: Vec<ObjectId> = engine
        .state()
        .battlefield_seen()
        .filter(|&id| engine.state().object(id).is_some_and(|o| o.card.is_none()))
        .collect();
    assert_eq!(tetravites.len(), 2, "two Tetravite tokens created");
    for &tok in &tetravites {
        assert!(keywords(&engine, tok).contains(KeywordSet::FLYING));
    }

    // In main phase, casting Holy Strength cannot target the Tetravite tokens
    // ("This token can't be enchanted."), only Tetravus itself.
    reach_their_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, holy_strength);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets for Holy Strength, got {:?}",
            engine.pending()
        );
    };
    assert!(options.contains(&tetra), "Tetravus can be enchanted");
    for &tok in &tetravites {
        assert!(
            !options.contains(&tok),
            "Tetravite token can't be enchanted"
        );
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![tetra],
            },
        )
        .expect("target Tetravus with Holy Strength");
    pass_until(&mut engine, stack_is_empty);

    // Advance to p0's next upkeep to re-absorb the 2 Tetravite tokens.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseNumber { .. } | Pending::ChooseCards { .. }
        )
    });
    for _ in 0..2 {
        match engine.pending().clone() {
            Pending::ChooseNumber { .. } => {
                engine
                    .apply(p0, PlayerAction::ChooseNumber(0))
                    .expect("remove 0 counters this upkeep");
            }
            Pending::ChooseCards { options, .. } => {
                engine
                    .apply(p0, PlayerAction::ChooseObjects { objects: options })
                    .expect("exile both Tetravite tokens for +1/+1 counters");
            }
            _ => break,
        }
        pass_until(&mut engine, |e| {
            stack_is_empty(e)
                || matches!(
                    e.pending(),
                    Pending::ChooseNumber { .. } | Pending::ChooseCards { .. }
                )
        });
        if stack_is_empty(&engine) {
            break;
        }
    }

    assert_eq!(
        counters_on(&engine, tetra, CounterKind::P1P1),
        3,
        "Tetravus regains two +1/+1 counters from exiling its two Tetravites"
    );
}
