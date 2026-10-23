//! `cards/creatures/mv_2/merfolk_raiders.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::KeywordSet;
use baylee_core::generated::index;
use baylee_core::ids::Defender;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "19edc98c-edc2-47bf-b2da-00f536d7c158"
fn merfolk_raiders() -> CardIndex {
    card_index("19edc98c-edc2-47bf-b2da-00f536d7c158")
}

#[test]
fn merfolk_raiders_has_islandwalk_and_phasing() {
    let mut engine = lk::game(
        SEED,
        &[merfolk_raiders()],
        &[island(), index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let raiders = lk::obj(&engine, P0, merfolk_raiders());
    assert!(
        engine
            .state()
            .object(raiders)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::ISLANDWALK.union(KeywordSet::PHASING))
    );

    // Turn 1 untap step phased out Merfolk Raiders.
    assert!(
        engine
            .state()
            .object(raiders)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT)
    );

    // Advance through P1's turn to P0's next turn -> Merfolk Raiders phases back in.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(
        !engine
            .state()
            .object(raiders)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT)
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(raiders, Defender::Player(P1))],
            },
        )
        .unwrap();

    // P1 controls an Island, so Grizzly Bears cannot block Merfolk Raiders.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(engine.state().players[1].life, 18);
}

/// Islandwalk needs an Island: against a Plains the Bears may block, and the
/// 2/3 Raiders kills them without the defender losing life.
#[test]
fn merfolk_raiders_can_be_blocked_when_the_defender_has_no_island() {
    let mut engine = lk::game(
        SEED,
        &[merfolk_raiders()],
        &[plains(), index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let raiders = lk::obj(&engine, P0, merfolk_raiders());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    // Phased out through turn 1; back in on P0's next turn.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    let offer = lk::attack(&mut engine, &[raiders]);
    assert!(
        lk::may_block(&offer, bears, raiders),
        "no Island, so islandwalk does not apply"
    );
    lk::block(&mut engine, &[(bears, raiders)]);
    lk::through_combat(&mut engine);

    assert_eq!(engine.state().players[1].life, 20);
    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, P0, merfolk_raiders()).is_some());
}
