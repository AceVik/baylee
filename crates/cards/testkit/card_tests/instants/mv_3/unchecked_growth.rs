//! `cards/instants/mv_3/unchecked_growth.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "ea4052bc-cd37-467c-9fba-3f1ed537b781"
fn unchecked_growth() -> CardIndex {
    card_index("ea4052bc-cd37-467c-9fba-3f1ed537b781")
}

/// Unchecked Growth ({2}{G}): target creature gets +4/+4 until EOT, and gains
/// trample until EOT ONLY if it is a Spirit.
#[test]
fn unchecked_growth_pumps_four_four_and_grants_trample_only_to_spirit() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::PAIN_KAMI],
        &[],
        &[unchecked_growth(), unchecked_growth()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let spirit = lk::obj(&engine, P0, index::PAIN_KAMI);

    lk::float(&mut engine, P0, &[(ManaColor::Green, 6)]);

    // 1. Non-Spirit gets +4/+4 without trample.
    cast_with_floating(&mut engine, P0, unchecked_growth());
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);
    assert_eq!(pt(&engine, bears), (6, 6));
    assert!(
        !keywords(&engine, bears).contains(KeywordSet::TRAMPLE),
        "non-Spirit does not gain trample"
    );

    // 2. Spirit gets +4/+4 AND trample until EOT.
    cast_with_floating(&mut engine, P0, unchecked_growth());
    lk::aim(&mut engine, &[spirit], &[]);
    lk::settle(&mut engine);
    assert_eq!(pt(&engine, spirit), (6, 6));
    assert!(
        keywords(&engine, spirit).contains(KeywordSet::TRAMPLE),
        "Spirit gains trample until EOT"
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, bears), (2, 2));
    assert_eq!(pt(&engine, spirit), (2, 2));
    assert!(!keywords(&engine, spirit).contains(KeywordSet::TRAMPLE));
}
