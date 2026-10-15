//! `cards/creatures/artifacts/mv_2/attentive_sunscribe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn attentive_sunscribe() -> CardIndex {
    card_index("0f85714b-e731-45a3-aa65-8cd59cd48280")
}

/// Declares the Sunscribe as seat 0's attacker, which taps it, and runs to
/// the scry question its becoming tapped asks.
#[track_caller]
fn attack_and_scry(engine: &mut Engine<RegistryLookup>, sunscribe: ObjectId) -> Vec<ObjectId> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(sunscribe, Defender::Player(p1))],
            },
        )
        .expect("the Sunscribe attacks");
    assert!(is_tapped(engine, sunscribe), "attacking taps it");
    pass_until(engine, |e| matches!(e.pending(), Pending::Arrange { .. }));
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, crate::choice::ArrangePrompt::Scry);
    assert_eq!(piles, scry_piles(1));
    assert_eq!(cards.len(), 1);
    cards
}

/// Attentive Sunscribe: a 2/2; "Whenever this creature becomes tapped, scry
/// 1". Attacking taps it and asks a scry of one; the card may go to the bottom.
#[test]
fn attentive_sunscribe_scries_one_when_it_becomes_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[attentive_sunscribe()], &[]);
    let sunscribe = on_battlefield(&engine, p0, attentive_sunscribe()).expect("seated");
    assert_eq!(pt(&engine, sunscribe), (2, 2));
    let cards = attack_and_scry(&mut engine, sunscribe);
    engine
        .apply(p0, look_answer(&cards, &[cards[0]]))
        .expect("to the bottom");
    pass_until(&mut engine, stack_is_empty);
    let library = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .clone();
    assert_eq!(
        library.first().copied(),
        Some(cards[0]),
        "the scried card went to the bottom"
    );
}

/// Scry may keep the card on top.
#[test]
fn attentive_sunscribe_may_keep_the_card_on_top() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[attentive_sunscribe()], &[]);
    let sunscribe = on_battlefield(&engine, p0, attentive_sunscribe()).expect("seated");
    let cards = attack_and_scry(&mut engine, sunscribe);
    engine.apply(p0, look_answer(&cards, &[])).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let library = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .clone();
    assert_eq!(library.last().copied(), Some(cards[0]), "still on top");
}

/// Being cast and untapped asks nothing: only becoming tapped scries.
#[test]
fn attentive_sunscribe_does_not_scry_while_untapped() {
    let mut engine = table(&[attentive_sunscribe()], &[plains(), forest()], &[]);
    cast_and_land(&mut engine, attentive_sunscribe());
    pass_until(&mut engine, stack_is_empty);
    assert!(matches!(engine.pending(), Pending::Priority { .. }));
}
