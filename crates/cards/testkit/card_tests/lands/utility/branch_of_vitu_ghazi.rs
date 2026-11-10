//! `cards/lands/utility/branch_of_vitu_ghazi.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Branch of Vitu-Ghazi: "{T}: Add {C}." / "Disguise {3}..." / "When this land is turned face up, add two mana of any one color..."
/// Under `Coverage::Partial`, disguise and the turned-face-up ability are omitted.
/// Playing this land allows it to enter untapped and immediately tap for colorless mana.
#[test]
fn branch_of_vitu_ghazi_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(332, forest())
        .hand(0, &[branch_of_vitu_ghazi()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, branch_of_vitu_ghazi());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, branch_of_vitu_ghazi(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
