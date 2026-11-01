//! `cards/creatures/mv_3/leering_gargoyle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::KeywordSet;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

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

/// The ability costs {T}, and "until end of turn" ends: the Gargoyle is a
/// tapped 0/4 now and a 2/2 flier again in the opponent's main phase.
#[test]
fn leering_gargoyle_taps_to_activate_and_the_change_ends_with_the_turn() {
    let mut engine = lk::game(SEED, &[leering_gargoyle()], &[], &[], &[]);
    let gargoyle = lk::obj(&engine, P0, leering_gargoyle());

    assert!(!is_tapped(&engine, gargoyle));
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
    assert!(is_tapped(&engine, gargoyle), "{{T}} is the cost");
    assert_eq!(pt(&engine, gargoyle), (0, 4));
    assert!(
        !lk::offered(&engine, leering_gargoyle(), P0, 0),
        "a tapped Gargoyle cannot activate again"
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, gargoyle), (2, 2));
    assert!(
        engine
            .state()
            .object(gargoyle)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING),
        "flying returns when the turn ends"
    );
}
