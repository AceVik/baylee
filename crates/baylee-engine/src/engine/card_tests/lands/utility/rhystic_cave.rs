//! `cards/lands/utility/rhystic_cave.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rhystic Cave: "{T}: Choose a color. Add one mana of that color unless any player pays {1}. Activate only as an instant."
/// Under `Coverage::Partial`, the payment prevention condition is omitted.
/// Activating the land prompts for a color choice; selecting Blue adds {U} to the mana pool.
#[test]
fn rhystic_cave_adds_chosen_color_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(135, forest())
        .battlefield(0, &[rhystic_cave()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cave = on_battlefield(&engine, p0, rhystic_cave()).expect("Cave deployed");
    activate(&mut engine, p0, rhystic_cave(), 0);

    let Pending::ChooseColor { player, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, cave));
}
