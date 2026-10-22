//! `cards/creatures/mv_3/leering_gargoyle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::KeywordSet;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "fc0907eb-a3fc-40ef-950d-b05aee9d5da7"
fn leering_gargoyle() -> CardIndex {
    card_index("fc0907eb-a3fc-40ef-950d-b05aee9d5da7")
}

#[test]
fn leering_gargoyle_tap_gives_minus_2_plus_2_and_loses_flying_until_eot() {
    let mut engine = lk::game(SEED, &[leering_gargoyle()], &[], &[], &[]);
    let gargoyle = lk::obj(&engine, P0, leering_gargoyle());

    assert_eq!(pt(&engine, gargoyle), (2, 2));
    assert!(
        engine
            .state()
            .object(gargoyle)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );

    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: gargoyle,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, gargoyle), (0, 4));
    assert!(
        !engine
            .state()
            .object(gargoyle)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );
}
