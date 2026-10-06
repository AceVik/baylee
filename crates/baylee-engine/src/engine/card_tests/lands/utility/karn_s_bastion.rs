//! `cards/lands/utility/karn_s_bastion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Karn's Bastion prints `{T}: Add {C}` and `{4}, {T}: Proliferate.`
/// The card is marked `Coverage::Partial` because the `DSL` does not support proliferate.
/// With four `forest` lands floating four mana and Karn's Bastion untapped, the engine offers only
/// ability index 0 for colorless mana and never the proliferate ability, tapping for `{C}` cleanly.
#[test]
fn karns_bastion_taps_for_colorless_and_omits_proliferate_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[karn_s_bastion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, karn_s_bastion());
    assert!(on_battlefield(&engine, p0, karn_s_bastion()).is_some());

    tap_all_mana_but(&mut engine, p0, Some(karn_s_bastion()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        4
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    let offered: Vec<(ObjectId, u32)> = legal
        .abilities
        .iter()
        .copied()
        .filter(|(source, _)| *source == land)
        .collect();
    assert_eq!(offered, vec![(land, 0)]);

    activate(&mut engine, p0, karn_s_bastion(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.total(), 5);
    assert!(is_tapped(&engine, land));
}
