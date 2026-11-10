//! `cards/artifacts/mv_3/braided_net.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn braided_net() -> CardIndex {
    card_index("86ef06b3-2049-494d-9ecb-ca14766d3b68")
}

/// Braided Net // Braided Quipu: the back face prints no mana cost and a blue
/// color indicator, and "an object is each color denoted by its color
/// indicator" (CR 202.2e). Without the indicator the turned-over artifact was
/// colorless.
#[test]
fn braided_quipu_is_blue_after_the_flip() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(8301, forest())
        .battlefield(0, &[braided_net()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let net = on_battlefield(&e, p0, braided_net()).expect("Braided Net is on the battlefield");
    let colors =
        |e: &Engine<RegistryLookup>| e.state().object(net).unwrap().characteristics().colors;
    assert_eq!(
        colors(&e),
        baylee_core::color::ColorSet::of(baylee_core::color::Color::Blue),
        "the front face is blue by its {{2}}{{U}} cost"
    );

    let def = baylee_cards::by_index(braided_net()).unwrap();
    assert!(e.dev_state_mut(p0).unwrap().transform(net, def, 1));
    assert_eq!(
        e.state().object(net).unwrap().face_index,
        1,
        "Braided Quipu is up"
    );
    assert_eq!(
        colors(&e),
        baylee_core::color::ColorSet::of(baylee_core::color::Color::Blue),
        "Braided Quipu has no mana cost and a blue color indicator, so it is blue and only blue (CR 202.2e)"
    );
}
