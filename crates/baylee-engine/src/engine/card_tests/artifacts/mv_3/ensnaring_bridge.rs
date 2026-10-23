//! `cards/artifacts/mv_3/ensnaring_bridge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn ensnaring_bridge() -> CardIndex {
    card_index("dade5835-ccaf-45e3-b9a3-46017e0384b3")
}

/// Ensnaring Bridge (`dade5835-ccaf-45e3-b9a3-46017e0384b3`) — {3} Artifact (Partial):
/// "Creatures with power greater than the number of cards in your hand can't attack."
/// In Partial mode, prevents creatures with power 1 or greater from attacking while your hand is empty.
#[test]
fn ensnaring_bridge_stops_creatures_from_attacking_when_hand_is_empty() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), index::GRIZZLY_BEARS])
        .hand(0, &[ensnaring_bridge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    assert!(!keywords(&engine, bears).contains(KeywordSet::CANT_ATTACK));

    cast_from_hand(&mut engine, P0, ensnaring_bridge());
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bears).contains(KeywordSet::CANT_ATTACK));
}

#[test]
fn empty_hand_restriction_is_read_by_both_players_attack_offers() {
    let mut e = lk::game(
        SEED,
        &[ensnaring_bridge(), index::GRIZZLY_BEARS, index::ORNITHOPTER],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let my_bear = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    let zero = lk::obj(&e, P0, index::ORNITHOPTER);
    let their_bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    let offered = lk::attackers_offered(&mut e);
    assert!(!offered.contains(&my_bear));
    assert!(offered.contains(&zero));
    e.apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    reach_their_main_phase(&mut e, lk::P1);
    assert!(!lk::attackers_offered(&mut e).contains(&their_bear));
}
#[test]
fn safe_partial_does_not_apply_empty_hand_rule_to_a_nonempty_hand() {
    let mut e = lk::game(
        SEED,
        &[ensnaring_bridge(), index::GRIZZLY_BEARS],
        &[],
        &[forest()],
        &[],
    );
    let bear = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    assert!(lk::attackers_offered(&mut e).contains(&bear));
}
