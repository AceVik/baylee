//! `cards/creatures/mv_2/sindbad.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sindbad() -> CardIndex {
    card_index("dc81069b-b2cf-44b3-98fc-45bb24b815cb")
}

fn revealed_by(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<Vec<ObjectId>> {
    engine
        .journal()
        .entries()
        .iter()
        .filter_map(|e| match &e.event {
            crate::event::GameEvent::Revealed { player, cards } if *player == seat => {
                Some(cards.clone())
            }
            _ => None,
        })
        .collect()
}

fn hand_len(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(seat))
        .len()
}

/// Returns (hand before, hand after, graveyard has the top card, reveals).
fn tap_sindbad(top: CardIndex) -> (usize, usize, bool, usize) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, top).battlefield(0, &[sindbad()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let before = hand_len(&engine, p0);
    let library = library_size(&engine, p0);
    activate(&mut engine, p0, sindbad(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), library - 1, "one card drawn");
    let reveals = revealed_by(&engine, p0);
    let sindbad_obj = on_battlefield(&engine, p0, sindbad()).expect("still here");
    assert!(is_tapped(&engine, sindbad_obj), "the cost tapped it");
    (
        before,
        hand_len(&engine, p0),
        in_graveyard(&engine, p0, top).is_some(),
        reveals.len(),
    )
}

/// A land stays in hand, and the draw is revealed.
#[test]
fn sindbad_keeps_a_drawn_land() {
    let (before, after, in_yard, reveals) = tap_sindbad(forest());
    assert_eq!(after, before + 1, "the Forest stays in hand");
    assert!(!in_yard, "nothing discarded");
    assert_eq!(reveals, 1, "the drawn card was revealed");
}

/// A nonland is revealed and discarded: the hand ends where it began.
#[test]
fn sindbad_discards_a_drawn_nonland() {
    let (before, after, in_yard, reveals) = tap_sindbad(craw_wurm());
    assert_eq!(after, before, "drawn, then discarded");
    assert!(in_yard, "the Wurm is in the graveyard");
    assert_eq!(reveals, 1, "the drawn card was revealed");
}
