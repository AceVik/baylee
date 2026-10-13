//! `cards/battles/mv_2/invasion_of_ikoria.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn invasion_of_ikoria() -> CardIndex {
    card_index("21d90735-5e04-4a8c-a546-ff26d4d5998d")
}

/// Invasion of Ikoria // Zilortha, Apex of Ikoria: the back face prints no
/// mana cost and a green color indicator, and "an object is each color
/// denoted by its color indicator" (CR 202.2e). Without the indicator the
/// transformed Zilortha was a colorless 8/8.
#[test]
fn zilortha_apex_of_ikoria_is_green_after_the_flip() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(8302, forest())
        .battlefield(0, &[invasion_of_ikoria()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let siege = on_battlefield(&e, p0, invasion_of_ikoria())
        .expect("Invasion of Ikoria is on the battlefield");
    let colors =
        |e: &Engine<RegistryLookup>| e.state().object(siege).unwrap().characteristics().colors;
    assert_eq!(
        colors(&e),
        baylee_core::color::ColorSet::of(baylee_core::color::Color::Green),
        "the Siege is green by its {{X}}{{G}}{{G}} cost"
    );

    let def = baylee_cards::by_index(invasion_of_ikoria()).unwrap();
    assert!(e.dev_state_mut(p0).unwrap().transform(siege, def, 1));
    assert_eq!(
        e.state().object(siege).unwrap().face_index,
        1,
        "Zilortha is up"
    );
    assert_eq!(
        colors(&e),
        baylee_core::color::ColorSet::of(baylee_core::color::Color::Green),
        "Zilortha has no mana cost and a green color indicator, so it is green and only green (CR 202.2e)"
    );
}
