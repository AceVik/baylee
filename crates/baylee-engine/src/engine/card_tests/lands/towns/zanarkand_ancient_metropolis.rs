//! `cards/lands/towns/zanarkand_ancient_metropolis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Zanarkand, Ancient Metropolis prints `This land enters tapped.` and `{{T}}: Add {{G}}.`
/// on its front face, alongside the Adventure sorcery Lasting Fayth.
///
/// Under `Coverage::Partial`, the adventure half is unsupported: a card whose front face is a
/// land cannot be cast as its Adventure, and no effect puts counters on a token the same
/// resolution created. Playing the card as a land
/// enters the battlefield tapped as a `TypeSet::LAND` rather than `TypeSet::SORCERY`.
/// Advancing to the next turn untaps the Town, where activating ability 0 produces one green mana.
#[test]
fn zanarkand_ancient_metropolis_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[zanarkand_ancient_metropolis()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, zanarkand_ancient_metropolis());
    assert!(entered_tapped(&engine, land));
    assert!(types(&engine, land).contains(TypeSet::LAND));
    assert!(!types(&engine, land).contains(TypeSet::SORCERY));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, zanarkand_ancient_metropolis(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}
