//! `cards/creatures/mv_5/ravenous_vampire.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::KeywordSet;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "2c029419-3601-4158-b10f-45a19de7faa3"
fn ravenous_vampire() -> CardIndex {
    card_index("2c029419-3601-4158-b10f-45a19de7faa3")
}

#[test]
fn ravenous_vampire_enters_as_3_3_with_flying() {
    let mut engine = lk::game(SEED, &[], &[], &[ravenous_vampire()], &[]);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 3), (ManaColor::Black, 2)],
    );
    cast_with_floating(&mut engine, P0, ravenous_vampire());
    lk::settle(&mut engine);

    let vamp = lk::obj(&engine, P0, ravenous_vampire());
    let ch = engine.state().object(vamp).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(3), Some(3)));
    assert!(ch.keywords.contains(KeywordSet::FLYING));
}
