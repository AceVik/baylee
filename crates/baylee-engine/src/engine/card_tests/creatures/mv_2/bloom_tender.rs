//! `cards/creatures/mv_2/bloom_tender.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bloom Tender: "{T}: For each color among permanents you control, add one mana of that color."
/// Controlled alongside Baleful Strix (blue and black), permanents you control exhibit three distinct colors.
/// Activating Bloom Tender prompts for three color choices, producing one mana of each color into the pool.
#[test]
fn bloom_tender_adds_one_mana_per_distinct_color_among_permanents() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(102, forest())
        .battlefield(0, &[bloom_tender(), baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tender = on_battlefield(&engine, p0, bloom_tender()).expect("Bloom Tender deployed");

    activate(&mut engine, p0, bloom_tender(), 0);

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected first color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected second color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected third color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 3, "three distinct colors produce 3 mana");
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, tender));
}
