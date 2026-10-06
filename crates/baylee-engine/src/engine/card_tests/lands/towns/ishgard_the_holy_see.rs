//! `cards/lands/towns/ishgard_the_holy_see.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ishgard, the Holy See prints `This land enters tapped.` and `{{T}}: Add {{W}}.`
/// on its front face, alongside the Adventure sorcery Faith & Grief.
///
/// Under `Coverage::Partial`, the adventure's self-exile rider is unsupported.
/// Playing the card as a land puts the Town permanent onto the battlefield tapped
/// as a `TypeSet::LAND` rather than `TypeSet::SORCERY`. Advancing to the next turn
/// cycle untaps it and lets it tap for one white mana.
#[test]
fn ishgard_the_holy_see_enters_tapped_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[ishgard_the_holy_see()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, ishgard_the_holy_see());
    assert!(entered_tapped(&engine, land));
    assert!(types(&engine, land).contains(TypeSet::LAND));
    assert!(!types(&engine, land).contains(TypeSet::SORCERY));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, ishgard_the_holy_see(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}
