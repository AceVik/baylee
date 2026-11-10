//! `cards/creatures/mv_8/goliath_spider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn goliath_spider() -> CardIndex {
    card_index("17415789-159e-43cf-93f8-9926405ee8f1")
}

/// Goliath Spider is a 7/6 with reach for {6}{G}{G}.
#[test]
fn goliath_spider_is_cast_for_eight_with_reach() {
    let mut engine = at_main(&[forest(); 8], &[goliath_spider()]);
    let spider = cast_and_settle(&mut engine, goliath_spider());
    assert_eq!(pt(&engine, spider), (7, 6));
    assert!(keywords(&engine, spider).contains(KeywordSet::REACH));
}

/// Reach: against a flying attacker the Spider is offered as a blocker, and
/// a creature with neither flying nor reach is not.
#[test]
fn goliath_spider_may_block_a_flier_and_a_bear_may_not() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[shivan_hellkite()])
        .battlefield(1, &[goliath_spider(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let dragon = on_battlefield(&engine, p0, shivan_hellkite()).expect("the flier");
    let spider = on_battlefield(&engine, p1, goliath_spider()).expect("the Spider");
    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("the bear");
    let blocks = attack_player(&mut engine, dragon);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == spider && b.attackers.contains(&dragon)),
        "reach: the Spider may block the flier: {blocks:?}"
    );
    assert!(
        blocks.iter().all(|b| b.blocker != bear),
        "the bear has neither flying nor reach: {blocks:?}"
    );
}
