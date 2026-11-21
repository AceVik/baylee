//! `cards/sorceries/mv_2/elephant_resurgence.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_cards_dsl::{Color, ColorSet};
use baylee_core::generated::subtypes;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn elephant_resurgence() -> CardIndex {
    card_index("e2d54c59-7722-4520-adca-cd02a0871ab2")
}

fn threaten() -> CardIndex {
    card_index("3dc8d15a-3ca7-4d1f-a819-c1b7f947a59e")
}

/// Elephant Resurgence (e2d54c59-7722-4520-adca-cd02a0871ab2) — {1}{G} Sorcery:
/// "Each player creates a green Elephant creature token. Those creatures have
/// 'This token's power and toughness are each equal to the number of creature
/// cards in its controller's graveyard.'"
#[test]
fn elephant_resurgence_gives_each_player_an_elephant_sized_by_controllers_graveyard() {
    let mut engine = lk::game(
        SEED,
        &[forest(), forest(), mountain(), mountain(), mountain()],
        &[],
        &[
            elephant_resurgence(),
            threaten(),
            llanowar_elves(),
            aurochs(),
            sol_ring(),
        ],
        &[llanowar_elves(), aurochs(), fangren_hunter()],
    );
    hand_to_graveyard(&mut engine, P0, llanowar_elves());
    hand_to_graveyard(&mut engine, P0, aurochs());
    hand_to_graveyard(&mut engine, P0, sol_ring());
    hand_to_graveyard(&mut engine, P1, llanowar_elves());
    hand_to_graveyard(&mut engine, P1, aurochs());
    hand_to_graveyard(&mut engine, P1, fangren_hunter());

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, elephant_resurgence());
    lk::settle(&mut engine);

    let p0_tokens = tokens_of(&engine, P0);
    let p1_tokens = tokens_of(&engine, P1);
    assert_eq!(p0_tokens.len(), 1);
    assert_eq!(p1_tokens.len(), 1);

    let my_elephant = p0_tokens[0];
    let their_elephant = p1_tokens[0];
    let ch = engine
        .state()
        .object(my_elephant)
        .unwrap()
        .characteristics();
    assert_eq!(ch.colors, ColorSet::from_slice(&[Color::Green]));
    assert!(ch.subtypes.contains(subtypes::creature::ELEPHANT));
    assert_eq!(
        pt(&engine, my_elephant),
        (2, 2),
        "noncreature Sol Ring ignored"
    );
    assert_eq!(pt(&engine, their_elephant), (3, 3));

    // Stealing P1's Elephant with Threaten ({2}{R}) updates its CDA to P0's graveyard (2/2)!
    lk::cast_at(&mut engine, P0, threaten(), &[their_elephant], &[]);
    assert_eq!(pt(&engine, their_elephant), (2, 2));

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, their_elephant), (3, 3));
}
