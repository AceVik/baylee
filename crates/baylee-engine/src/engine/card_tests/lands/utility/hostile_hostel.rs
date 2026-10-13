//! `cards/lands/utility/hostile_hostel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hostile Hostel // Creeping Inn: "{T}: Add {C}." / "{1}, {T}, Sacrifice a creature: Put a soul counter on this land..."
/// Under `Coverage::Partial`, the soul-counter transform and linked-exile attack trigger are omitted.
/// Playing the front face enters untapped and immediately taps for colorless mana.
#[test]
fn hostile_hostel_enters_untapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(227, forest())
        .hand(0, &[hostile_hostel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hostile_hostel());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, hostile_hostel(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Hostile Hostel // Creeping Inn: the land is colorless, and the back face
/// prints no mana cost and a black color indicator ("an object is each color
/// denoted by its color indicator", CR 202.2e). Without the indicator the
/// turned-over Creeping Inn stayed colorless.
#[test]
fn creeping_inn_is_black_after_the_flip() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(8304, forest())
        .battlefield(0, &[hostile_hostel()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let hostel = on_battlefield(&e, p0, hostile_hostel()).expect("Hostile Hostel is on the table");
    let colors =
        |e: &Engine<RegistryLookup>| e.state().object(hostel).unwrap().characteristics().colors;
    assert_eq!(
        colors(&e),
        baylee_core::color::ColorSet::EMPTY,
        "the land is colorless"
    );

    let def = baylee_cards::by_index(hostile_hostel()).unwrap();
    assert!(e.dev_state_mut(p0).unwrap().transform(hostel, def, 1));
    assert_eq!(
        e.state().object(hostel).unwrap().face_index,
        1,
        "Creeping Inn is up"
    );
    assert_eq!(
        colors(&e),
        baylee_core::color::ColorSet::of(baylee_core::color::Color::Black),
        "Creeping Inn has no mana cost and a black color indicator, so it is black and only black (CR 202.2e)"
    );
}
