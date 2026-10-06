//! `cards/lands/check/spymaster_s_vault.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Spymaster's Vault: "This land enters tapped unless you control a Swamp." / "{T}: Add {B}." / "{B}, {T}: Target creature you control connives X..."
/// Under `Coverage::Partial`, the connive ability is omitted because connive has no effect in the `DSL`.
/// Controlling a Swamp allows Spymaster's Vault to enter untapped and immediately tap for black mana.
#[test]
fn spymaster_s_vault_enters_untapped_with_swamp_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(208, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[spymaster_s_vault()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, spymaster_s_vault());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, spymaster_s_vault(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, land));
}
