//! `cards/lands/caves/secret_tunnel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Secret Tunnel: "This land can't be blocked." / "{T}: Add {C}." / "{4}, {T}: Two target creatures you control that share a creature type can't be blocked this turn."
/// Under `Coverage::Partial`, the activated ability requiring two targets of shared creature type is omitted.
/// The static ability grants unblockable to the land itself, and activating ability 1 adds `{C}` to the mana pool.
#[test]
fn secret_tunnel_is_unblockable_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(101, forest())
        .battlefield(0, &[secret_tunnel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tunnel = on_battlefield(&engine, p0, secret_tunnel()).expect("Secret Tunnel deployed");
    assert!(keywords(&engine, tunnel).contains(KeywordSet::UNBLOCKABLE));

    activate(&mut engine, p0, secret_tunnel(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, tunnel));
}
