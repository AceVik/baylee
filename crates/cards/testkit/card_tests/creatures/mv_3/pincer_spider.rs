//! `cards/creatures/mv_3/pincer_spider.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn pincer_spider() -> CardIndex {
    card_index("fbe25c32-db20-464c-a0ed-d90e75b111a9")
}

/// Pincer Spider ({2}{G} 2/3 Spider):
/// "Kicker {3}; Reach; If this creature was kicked, it enters with a +1/+1
/// counter on it."
#[test]
fn pincer_spider_kicker_adds_counter_and_reach_blocks_flying_attackers() {
    let mut e_base = lk::game(SEED, &[], &[], &[pincer_spider()], &[]);
    lk::float(&mut e_base, lk::P0, &[(ManaColor::Green, 6)]);
    cast_with_floating(&mut e_base, lk::P0, pincer_spider());
    lk::yes_no(&mut e_base, false);
    lk::settle(&mut e_base);
    assert_eq!(
        pt(&e_base, lk::obj(&e_base, lk::P0, pincer_spider())),
        (2, 3)
    );

    let mut e = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::WIND_DRAKE],
        &[pincer_spider()],
        &[],
    );
    let bear = lk::obj(&e, lk::P0, index::GRIZZLY_BEARS);
    let drake = lk::obj(&e, lk::P1, index::WIND_DRAKE);
    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 6)]);
    cast_with_floating(&mut e, lk::P0, pincer_spider());
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);
    let spider = lk::obj(&e, lk::P0, pincer_spider());
    assert_eq!(counters_on(&e, spider, CounterKind::P1P1), 1);
    assert_eq!(pt(&e, spider), (3, 4));

    reach_their_main_phase(&mut e, lk::P1);
    let offer = lk::attack(&mut e, &[drake]);
    assert!(lk::may_block(&offer, spider, drake));
    assert!(!lk::may_block(&offer, bear, drake));
    lk::block(&mut e, &[(spider, drake)]);
    lk::through_combat(&mut e);
    assert!(in_graveyard(&e, lk::P1, index::WIND_DRAKE).is_some());
    assert!(on_battlefield(&e, lk::P0, pincer_spider()).is_some());
}
