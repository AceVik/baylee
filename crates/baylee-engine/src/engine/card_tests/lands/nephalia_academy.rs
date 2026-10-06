//! `cards/lands/nephalia_academy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nephalia Academy prints `If a spell or ability an opponent controls causes you to discard a card,
/// you may reveal that card and put it on top of your library instead of putting it anywhere else`
/// and `{T}: Add {C}.`
/// The card is marked `Coverage::Partial` because discard-replacement triggers are unsupported in the `DSL`.
/// Nephalia Academy enters untapped and its only implemented ability at index 0 taps to add one colorless mana.
#[test]
fn nephalia_academy_enters_untapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[nephalia_academy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, nephalia_academy());
    assert!(!is_tapped(&engine, land));

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

    activate(&mut engine, p0, nephalia_academy(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
