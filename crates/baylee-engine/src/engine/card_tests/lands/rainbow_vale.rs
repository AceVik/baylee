//! `cards/lands/rainbow_vale.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rainbow Vale: "{T}: Add one mana of any color. An opponent gains control of this land at the beginning of the next end step."
/// Under `Coverage::Partial`, the end-step control change trigger is omitted.
/// Activating the land prompts for a color choice; selecting Green adds {G} to the mana pool.
#[test]
fn rainbow_vale_adds_chosen_color_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(138, forest())
        .battlefield(0, &[rainbow_vale()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vale = on_battlefield(&engine, p0, rainbow_vale()).expect("Vale deployed");
    activate(&mut engine, p0, rainbow_vale(), 0);

    let Pending::ChooseColor { player, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, vale));
}
