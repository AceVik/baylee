//! `cards/lands/utility/gods_eye_gate_to_the_reikai.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gods' Eye, Gate to the Reikai: "{T}: Add {C}." / "When Gods' Eye is put into a graveyard from the battlefield, create a 1/1 colorless Spirit creature token."
/// Activating the mana ability adds {C} to the pool and leaves Gods' Eye tapped; the Spirit
/// trigger is played in `gods_eye_leaves_a_colorless_spirit_when_it_is_destroyed`.
#[test]
fn gods_eye_gate_to_the_reikai_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(128, forest())
        .battlefield(0, &[gods_eye_gate_to_the_reikai()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land =
        on_battlefield(&engine, p0, gods_eye_gate_to_the_reikai()).expect("Gods Eye deployed");
    activate(&mut engine, p0, gods_eye_gate_to_the_reikai(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Gods' Eye, Gate to the Reikai: "When Gods' Eye is put into a graveyard
/// from the battlefield, create a 1/1 colorless Spirit creature token."
/// The land is destroyed by a Vindicate rather than moved by the harness,
/// because the trigger is on the zone change and a board written into place
/// journals none. The Spirit's colourlessness is the assertion that matters:
/// the pool carries three Spirit tokens and two of them are white.
#[test]
fn gods_eye_leaves_a_colorless_spirit_when_it_is_destroyed() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9103, forest())
        .battlefield(0, &[gods_eye_gate_to_the_reikai(), plains(), swamp()])
        .hand(0, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let eye =
        on_battlefield(&engine, p0, gods_eye_gate_to_the_reikai()).expect("the Gate is seated");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing has died yet, so nothing has been made"
    );

    cast_from_hand(&mut engine, p0, vindicate());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&eye), "\"destroy target permanent\"");
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![eye] })
        .expect("the Gate is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, gods_eye_gate_to_the_reikai()).is_none(),
        "the Gate was destroyed"
    );
    let made = tokens_of(&engine, p0);
    assert_eq!(made.len(), 1, "one Gate, one Spirit");
    let spirit = made[0];
    assert_eq!(pt(&engine, spirit), (1, 1));
    let printed = engine
        .state()
        .object(spirit)
        .expect("the Spirit is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Spirit");
    assert!(
        printed.colors.is_empty(),
        "\"a 1/1 colorless Spirit\" — not one of the pool's white ones"
    );
}
