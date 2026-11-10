//! `cards/lands/undiscovered_paradise.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Undiscovered Paradise: "{T}: Add one mana of any color. During your next untap step, as you untap your permanents, return this land to its owner's hand."
/// Under `Coverage::Partial`, the delayed return to hand at untap is unsupported.
/// Activating ability 0 prompts for a mana color and adds the chosen mana to the pool.
#[test]
fn undiscovered_paradise_taps_for_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(126, forest())
        .battlefield(0, &[undiscovered_paradise()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let paradise = on_battlefield(&engine, p0, undiscovered_paradise()).expect("Paradise deployed");
    activate(&mut engine, p0, undiscovered_paradise(), 0);

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, paradise));
}
