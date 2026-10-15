//! `cards/creatures/artifacts/mv_4/anvilwrought_raptor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn anvilwrought_raptor() -> CardIndex {
    card_index("d87ebd8c-ad2c-45c9-8c97-f046aeda861c")
}

/// Anvilwrought Raptor — {4} 2/1 artifact creature with flying and first
/// strike. Flying: a Grizzly Bears is not offered the block of it, while the
/// same Bears may block a creature without flying.
#[test]
fn anvilwrought_raptor_flies_over_a_ground_blocker() {
    assert!(!bears_may_block(anvilwrought_raptor()), "flying");
    assert!(
        bears_may_block(quiet_creature()),
        "the control: a creature without flying may be blocked by the Bears"
    );
}

/// First strike: the Raptor blocks an attacking Grizzly Bears (2/2). It
/// deals its 2 first, the Bears die, and the 2/1 takes nothing back; without
/// first strike both would have died together.
#[test]
fn anvilwrought_raptor_kills_a_bear_before_it_strikes_back() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (engine, raptor, _bears) = they_attack_and_we_block(grizzly_bears(), anvilwrought_raptor());
    let keywords = keywords(&engine, raptor);
    assert!(keywords.contains(KeywordSet::FLYING));
    assert!(keywords.contains(KeywordSet::FIRST_STRIKE));
    assert!(
        in_graveyard(&engine, p1, grizzly_bears()).is_some(),
        "the Bears died in the first-strike step"
    );
    assert!(
        on_battlefield(&engine, p0, anvilwrought_raptor()).is_some(),
        "and the Raptor was not dealt damage"
    );
    assert_eq!(engine.state().object(raptor).expect("raptor").damage, 0);
    assert_eq!(pt(&engine, raptor), (2, 1));
}
