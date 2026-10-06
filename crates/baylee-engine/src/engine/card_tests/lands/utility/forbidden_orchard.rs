//! `cards/lands/utility/forbidden_orchard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Forbidden Orchard: "{T}: Add one mana of any color." / "Whenever you tap this land for mana, target opponent creates a 1/1 colorless Spirit creature token."
/// Under `Coverage::Partial`, the tap-for-mana trigger is unsupported and omitted.
/// Activating ability 0 prompts for a color, produces that mana, and taps the land.
#[test]
fn forbidden_orchard_taps_for_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(128, forest())
        .battlefield(0, &[forbidden_orchard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let orchard = on_battlefield(&engine, p0, forbidden_orchard()).expect("Orchard deployed");
    activate(&mut engine, p0, forbidden_orchard(), 0);

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, orchard));
}
