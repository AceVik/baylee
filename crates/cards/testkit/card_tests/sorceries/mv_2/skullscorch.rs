//! `cards/sorceries/mv_2/skullscorch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn skullscorch() -> CardIndex {
    card_index("b52788ff-9956-400a-bf26-17e58f673af0")
}

/// Skullscorch (b52788ff-9956-400a-bf26-17e58f673af0) — {R}{R} Sorcery:
/// "Target player discards two cards at random unless that player has
/// Skullscorch deal 4 damage to them."
#[test]
fn skullscorch_discards_two_at_random_unless_target_player_takes_four_damage() {
    let mut declined = lk::game(
        SEED,
        &[mountain(), mountain()],
        &[],
        &[skullscorch()],
        &[forest(), island(), plains()],
    );
    tap_all_mana(&mut declined, P0);
    lk::announce(&mut declined, P0, skullscorch(), 0, 0, &[], &[P1]);
    pass_until(&mut declined, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = declined.pending().clone() else {
        unreachable!();
    };
    assert_eq!(player, P1);
    lk::yes_no(&mut declined, false);
    lk::settle(&mut declined);
    assert_eq!(
        declined.state().zones.list(ZoneLocation::Hand(P1)).len(),
        1,
        "discarded two cards at random"
    );
    assert_eq!(lk::life(&declined, P1), 20);

    let mut accepted = lk::game(
        SEED,
        &[mountain(), mountain()],
        &[],
        &[skullscorch()],
        &[forest(), island(), plains()],
    );
    tap_all_mana(&mut accepted, P0);
    lk::announce(&mut accepted, P0, skullscorch(), 0, 0, &[], &[P1]);
    pass_until(&mut accepted, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut accepted, true);
    lk::settle(&mut accepted);
    assert_eq!(accepted.state().zones.list(ZoneLocation::Hand(P1)).len(), 3);
    assert_eq!(lk::life(&accepted, P1), 16, "took 4 damage instead");
}
