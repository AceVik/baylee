//! `cards/creatures/artifacts/mv_2/hedron_crawler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hedron_crawler() -> CardIndex {
    card_index("0b9a4e06-b21d-4cbe-906f-9dbd08dbe5d3")
}

/// Hedron Crawler: a 0/1 with "{T}: Add {C}". Seated, it taps for exactly one
/// colorless mana, not for a color.
#[test]
fn hedron_crawler_taps_for_one_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[hedron_crawler()], &[]);
    let crawler = on_battlefield(&engine, p0, hedron_crawler()).expect("seated");
    assert_eq!(pt(&engine, crawler), (0, 1));
    assert!(offered(&engine, crawler, 0));
    activate(&mut engine, p0, hedron_crawler(), 0);
    assert!(is_tapped(&engine, crawler));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.total(), 1);
    assert!(stack_is_empty(&engine), "a mana ability uses no stack");
    assert!(!offered(&engine, crawler, 0), "tapped, it cannot tap again");
}

/// Summoning sickness: a Crawler cast this turn cannot tap for mana, and
/// the offer does not list it.
#[test]
fn hedron_crawler_cast_this_turn_cannot_tap_yet() {
    let mut engine = table(&[hedron_crawler()], &[forest(), forest()], &[]);
    let crawler = cast_and_land(&mut engine, hedron_crawler());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !offered(&engine, crawler, 0),
        "a {{T}} ability needs a creature under its controller's control since the turn began"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
