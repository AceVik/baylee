//! `cards/lands/fetch/maelstrom_of_the_spirit_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Maelstrom of the Spirit Dragon` taps for `{{C}}`, produces restricted mana of any color
/// spendable only on Dragon or Omen spells, and searches for a Dragon card under `Coverage::Implemented`.
/// Activating ability 1 prompts for a color choice and deposits restricted mana into `pool.restricted()`
/// rather than general available mana.
#[test]
fn maelstrom_of_the_spirit_dragon_produces_restricted_dragon_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1909, forest())
        .hand(0, &[maelstrom_of_the_spirit_dragon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, maelstrom_of_the_spirit_dragon());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, maelstrom_of_the_spirit_dragon(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Red);
    assert!(is_tapped(&engine, land));
}
