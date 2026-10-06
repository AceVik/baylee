//! `cards/creatures/mv_7/jin_gitaxias_progress_tyrant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn a_copied_rebound_spell_does_not_schedule_a_cast_of_a_vanished_copy() {
    let p0 = PlayerId::new(0);
    let ephemerate = card_index("0fd57894-b917-41c8-a394-360d1d31b236");
    let mut engine = Duel::new(21, forest())
        .battlefield(0, &[plains(), jin_gitaxias(), ondu_cleric()])
        .hand(0, &[ephemerate])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).unwrap();
    let jin = on_battlefield(&engine, p0, jin_gitaxias()).unwrap();
    let original = in_hand(&engine, p0, ephemerate).unwrap();
    cast_from_hand(&mut engine, p0, ephemerate);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();
    let options = options_offered_including(&mut engine, jin);
    assert!(options.contains(&jin));
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![jin] })
        .unwrap();
    assert!(matches!(drive_to_rest(&mut engine, p0), Rest::Reached));
    let rebounds: Vec<_> = engine
        .state
        .delayed
        .iter()
        .filter_map(|d| match d.action {
            crate::state::DelayedAction::CastFromExileWithoutPaying { card, .. } => Some(card),
            _ => None,
        })
        .collect();
    assert_eq!(
        rebounds,
        vec![original],
        "only the spell actually cast from hand can rebound"
    );
}

/// Jin-Gitaxias, Progress Tyrant: "Whenever an opponent casts an artifact,
/// instant, or sorcery spell, counter that spell. This ability triggers only
/// once each turn." The first Giant Growth is countered, the second, the
/// same turn, is not.
#[test]
fn jin_gitaxias_counters_an_opponents_first_instant_each_turn_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let bears = card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0");
    let mut engine = Duel::new(2202, forest())
        .battlefield(0, &[jin_gitaxias()])
        .battlefield(1, &[forest(), forest(), forest(), bears])
        .hand(1, &[giant_growth(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let b = on_battlefield(&engine, p1, bears).expect("bears");

    cast_from_hand(&mut engine, p1, giant_growth());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target, got {:?}", engine.pending())
    };
    assert!(options.contains(&b));
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![b] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, b), (2, 2), "the first spell was countered");

    cast_with_floating(&mut engine, p1, giant_growth());
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![b] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, b),
        (5, 5),
        "the second resolves: once each turn"
    );
    let _ = p0;
}
