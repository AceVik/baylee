//! `cards/lands/utility/avengers_tower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Avengers Tower: "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to cast a Hero spell..." / "{4}, {T}: Look at the top three cards..."
/// Under `Coverage::Partial`, both Hero-restricted abilities are omitted because hero source restrictions and filtered picks are inexpressible.
/// Playing Avengers Tower enters untapped and immediately taps for colorless mana.
#[test]
fn avengers_tower_enters_untapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(223, forest())
        .hand(0, &[avengers_tower()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, avengers_tower());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, avengers_tower(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
