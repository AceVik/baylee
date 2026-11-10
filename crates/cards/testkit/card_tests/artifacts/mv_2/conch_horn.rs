//! `cards/artifacts/mv_2/conch_horn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn conch_horn() -> CardIndex {
    card_index("b019cc20-d32d-4cdf-80cc-eafb3f78db88")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hand(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Hand(seat)).clone()
}

fn library(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Library(seat))
        .clone()
}

/// The Horn activated with the Bears in hand; the engine stands at the
/// question "which card goes back", whose menu is returned.
fn horn_asking() -> (Engine<RegistryLookup>, Vec<ObjectId>, usize) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[conch_horn(), forest()])
        .hand(0, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library_before = library(&engine, p0).len();
    assert_eq!(hand(&engine, p0).len(), 1, "just the Bears");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, conch_horn(), 0);
    assert!(
        on_battlefield(&engine, p0, conch_horn()).is_none(),
        "sacrificed as a cost"
    );
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p0),
    );
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    (engine, options, library_before)
}

/// Conch Horn (b019cc20-d32d-4cdf-80cc-eafb3f78db88) — "{1}, {T}, Sacrifice
/// this artifact: Draw two cards, then put a card from your hand on top of
/// your library."
///
/// The two cards are drawn first, so the question's menu is the whole hand of
/// three. Putting the Bears back leaves the two drawn cards in hand and the
/// Bears on top of the library.
#[test]
fn conch_horn_draws_two_then_puts_the_chosen_card_on_top() {
    let p0 = PlayerId::new(0);
    let (mut engine, options, library_before) = horn_asking();
    let in_hand = hand(&engine, p0);
    assert_eq!(in_hand.len(), 3, "the Bears and two drawn cards");
    assert_eq!(
        options.len(),
        3,
        "any card in hand may go back: {options:?}"
    );
    for card in &in_hand {
        assert!(options.contains(card), "{card:?} is on the menu");
    }
    assert_eq!(
        library(&engine, p0).len(),
        library_before - 2,
        "two cards left the library"
    );

    let bears = in_hand
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == grizzly_bears()))
        })
        .expect("the Bears are in hand");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .expect("the Bears were on the menu");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(hand(&engine, p0).len(), 2);
    assert!(!hand(&engine, p0).contains(&bears), "the Bears went back");
    assert_eq!(
        library(&engine, p0).last(),
        Some(&bears),
        "on top of the library"
    );
    assert_eq!(library(&engine, p0).len(), library_before - 1);
    assert!(in_graveyard(&engine, p0, conch_horn()).is_some());
}

/// The card put back need not be the one that was already in hand: putting
/// back a drawn card keeps the Bears.
#[test]
fn conch_horn_may_put_back_a_card_it_just_drew() {
    let p0 = PlayerId::new(0);
    let (mut engine, _options, library_before) = horn_asking();
    let bears = in_hand_bears(&engine, p0);
    let drawn = hand(&engine, p0)
        .into_iter()
        .find(|id| *id != bears)
        .expect("a drawn card");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![drawn],
            },
        )
        .expect("a drawn card was on the menu");
    pass_until(&mut engine, stack_is_empty);

    assert!(hand(&engine, p0).contains(&bears), "the Bears stayed");
    assert!(!hand(&engine, p0).contains(&drawn));
    assert_eq!(library(&engine, p0).last(), Some(&drawn));
    assert_eq!(library(&engine, p0).len(), library_before - 1);
}

fn in_hand_bears(engine: &Engine<RegistryLookup>, seat: PlayerId) -> ObjectId {
    in_hand(engine, seat, grizzly_bears()).expect("the Bears are in hand")
}
