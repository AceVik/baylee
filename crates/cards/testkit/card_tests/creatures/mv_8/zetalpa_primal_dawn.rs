//! `cards/creatures/mv_8/zetalpa_primal_dawn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn zetalpa() -> CardIndex {
    card_index("7da0e5de-3e4c-420a-8685-991206100b9d")
}

/// Zetalpa is a 4/8 for {6}{W}{W} with flying, double strike, vigilance,
/// trample and indestructible.
#[test]
fn zetalpa_is_cast_for_eight_with_five_keywords() {
    let mut engine = at_main(&[plains(); 8], &[zetalpa()]);
    let dino = cast_and_settle(&mut engine, zetalpa());
    assert_eq!(pt(&engine, dino), (4, 8));
    let k = keywords(&engine, dino);
    for (kw, name) in [
        (KeywordSet::FLYING, "flying"),
        (KeywordSet::DOUBLE_STRIKE, "double strike"),
        (KeywordSet::VIGILANCE, "vigilance"),
        (KeywordSet::TRAMPLE, "trample"),
        (KeywordSet::INDESTRUCTIBLE, "indestructible"),
    ] {
        assert!(k.contains(kw), "{name}");
    }
}

/// Double strike: unblocked, 4 power hits twice for 8, and vigilance leaves
/// her untapped.
#[test]
fn zetalpa_unblocked_deals_eight_and_stays_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = at_main(&[zetalpa()], &[]);
    let dino = on_battlefield(&engine, p0, zetalpa()).expect("Zetalpa");
    attack_player(&mut engine, dino);
    assert!(!is_tapped(&engine, dino), "vigilance");
    no_blocks_and_settle(&mut engine);
    assert_eq!(life_of(&engine, 1), 12, "two strikes of 4");
}
