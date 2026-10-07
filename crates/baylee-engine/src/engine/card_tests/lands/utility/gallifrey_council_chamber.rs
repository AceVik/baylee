//! `cards/lands/utility/gallifrey_council_chamber.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gallifrey Council Chamber: "When Gallifrey Council Chamber enters, surveil 1." / "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to cast a Time Lord or Alien spell..."
/// Under `Coverage::Partial`, the spend restriction applies to spells only and not activations.
/// Playing this land resolves its surveil trigger, and activating ability 2 produces restricted mana in `pool.restricted()`.
#[test]
fn gallifrey_council_chamber_surveils_and_produces_restricted_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(334, forest())
        .hand(0, &[gallifrey_council_chamber()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, gallifrey_council_chamber()).expect("chamber in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    // The surveil is a *triggered* ability, so it goes on the stack and the
    // question arrives when it resolves, not when the land arrives.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a surveil arrangement, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, crate::choice::ArrangePrompt::Surveil);
    engine.apply(p0, look_answer(&cards, &[])).unwrap();

    pass_until(&mut engine, stack_is_empty);

    activate(&mut engine, p0, gallifrey_council_chamber(), 2);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Blue);
    assert_eq!(pool.available(ManaColor::Blue), 0);
    let land = on_battlefield(&engine, p0, gallifrey_council_chamber()).expect("land deployed");
    assert!(is_tapped(&engine, land));
}
