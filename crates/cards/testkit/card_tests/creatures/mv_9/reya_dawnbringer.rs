//! `cards/creatures/mv_9/reya_dawnbringer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn reya() -> CardIndex {
    card_index("08d1a5d1-141d-48e3-ac22-c2529191cb13")
}

/// Reya is a 4/6 flier for {6}{W}{W}{W}.
#[test]
fn reya_is_cast_for_nine_with_flying() {
    let mut engine = at_main(&[plains(); 9], &[reya()]);
    let angel = cast_and_settle(&mut engine, reya());
    assert_eq!(pt(&engine, angel), (4, 6));
    assert!(keywords(&engine, angel).contains(KeywordSet::FLYING));
}

/// A board whose library is all Bears, one of which is already in the
/// graveyard, with Reya about to face her first upkeep.
fn reya_with_a_bear_in_the_graveyard() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, grizzly_bears())
        .battlefield(0, &[reya()])
        .start();
    keep_mulligans(&mut engine);
    seed_graveyard(&mut engine, p0, 1);
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    engine
}

fn upkeep_until_targets(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
}

/// "At the beginning of your upkeep, you may return target creature card
/// from your graveyard to the battlefield": the graveyard Bears is offered as
/// a target, the may is accepted and the Bears is on the battlefield.
#[test]
fn reya_returns_a_creature_card_at_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = reya_with_a_bear_in_the_graveyard();
    upkeep_until_targets(&mut engine);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    let card = in_graveyard(&engine, p0, grizzly_bears()).expect("in the graveyard");
    assert!(options.contains(&card), "{options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![card],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "reanimated"
    );
}

/// Declining the "may" leaves the Bears in the graveyard.
#[test]
fn reya_may_leave_the_creature_in_the_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = reya_with_a_bear_in_the_graveyard();
    upkeep_until_targets(&mut engine);
    let card = in_graveyard(&engine, p0, grizzly_bears()).expect("in the graveyard");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![card],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
}
