//! `cards/creatures/mv_9/the_unspeakable.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn the_unspeakable() -> CardIndex {
    card_index("64e413c9-96e5-43c3-a182-84d53fef4327")
}

fn eye_of_nowhere_card() -> CardIndex {
    card_index("27ffdf11-bb7f-40bf-94c4-6bfbc41668e5")
}

/// The Unspeakable is a 6/7 for {6}{U}{U}{U} with flying and trample.
#[test]
fn the_unspeakable_is_cast_for_nine_with_flying_and_trample() {
    let mut engine = at_main(&[island(); 9], &[the_unspeakable()]);
    let spirit = cast_and_settle(&mut engine, the_unspeakable());
    assert_eq!(pt(&engine, spirit), (6, 7));
    assert!(keywords(&engine, spirit).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, spirit).contains(KeywordSet::TRAMPLE));
}

fn graveyard_len(engine: &Engine<RegistryLookup>) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(PlayerId::new(0)))
        .len()
}

/// Attacks with the Spirit, unblocked, and stops at the first question the
/// combat-damage trigger asks.
fn connect(engine: &mut Engine<RegistryLookup>) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let spirit = on_battlefield(engine, p0, the_unspeakable()).expect("the Spirit");
    attack_and_collect_blocks(engine, spirit, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
}

/// "Whenever The Unspeakable deals combat damage to a player, you may return
/// target Arcane card from your graveyard to your hand": the Arcane sorcery in
/// the graveyard is offered, the may is accepted and the card is in hand.
#[test]
fn the_unspeakable_returns_an_arcane_card_when_it_connects() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, eye_of_nowhere_card())
        .battlefield(0, &[the_unspeakable()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);
    assert_eq!(graveyard_len(&engine), 1);
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    connect(&mut engine);
    let card = in_graveyard(&engine, p0, eye_of_nowhere_card()).expect("the Arcane card");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
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
    assert_eq!(graveyard_len(&engine), 0, "left the graveyard");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 1,
        "and went to the hand"
    );
}

/// Declining the "may" leaves the card in the graveyard.
#[test]
fn the_unspeakable_may_leave_the_card_in_the_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, eye_of_nowhere_card())
        .battlefield(0, &[the_unspeakable()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);
    connect(&mut engine);
    let card = in_graveyard(&engine, p0, eye_of_nowhere_card()).expect("the Arcane card");
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
    assert_eq!(graveyard_len(&engine), 1);
}
