//! `cards/lands/utility/urborg.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Urborg: "{T}: Add {B}." / "{T}: Target creature loses first strike or swampwalk until end of turn."
/// Under `Coverage::Partial`, the modal keyword loss ability is unsupported and omitted.
/// The legendary land taps to add {B} to the mana pool and offers no second ability.
#[test]
fn urborg_taps_for_black_mana_and_omits_keyword_loss() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(132, forest()).battlefield(0, &[urborg()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let u = on_battlefield(&engine, p0, urborg()).expect("Urborg deployed");

    activate(&mut engine, p0, urborg(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, u));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(s, ai)| *s == u && *ai == 1),
        "no second ability is offered"
    );
}

#[test]
fn urborg_chooses_the_lost_keyword_only_during_resolution() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for mode in 0..2 {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::URBORG, index::GRIZZLY_BEARS])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
        let state = e.dev_state_mut(p0).unwrap();
        std::sync::Arc::make_mut(&mut state.object_mut(bear).unwrap().base).keywords =
            KeywordSet::FIRST_STRIKE
                .union(KeywordSet::SWAMPWALK)
                .union(KeywordSet::VIGILANCE);
        state.invalidate_projections();
        state.refresh_characteristics();
        e.refresh_offer();
        activate(&mut e, p0, index::URBORG, 1);
        assert!(matches!(e.pending(), Pending::ChooseTargets { .. }));
        aim_at(&mut e, p0, bear);
        assert!(matches!(e.pending(), Pending::Priority { .. }));
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCastMode { .. })
        });
        e.apply(p0, PlayerAction::ChooseMode(mode)).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            keywords(&e, bear).contains(KeywordSet::FIRST_STRIKE),
            mode == 1
        );
        assert_eq!(
            keywords(&e, bear).contains(KeywordSet::SWAMPWALK),
            mode == 0
        );
        assert!(keywords(&e, bear).contains(KeywordSet::VIGILANCE));
    }
}
