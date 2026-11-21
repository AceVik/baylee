//! `cards/sorceries/mv_5/balance_of_power.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn balance_of_power() -> CardIndex {
    card_index("c57ee68e-6832-4d4c-b710-fc03a2c10a9c")
}

/// Balance of Power (c57ee68e-6832-4d4c-b710-fc03a2c10a9c) — {3}{U}{U} Sorcery:
/// "If target opponent has more cards in hand than you, draw cards equal to the
/// difference."
#[test]
fn balance_of_power_draws_difference_when_opponent_holds_more_cards() {
    let mut engine = lk::game(
        SEED,
        &[island(), island(), island(), island(), island()],
        &[],
        &[balance_of_power(), forest()],
        &[forest(), island(), plains(), swamp()],
    );
    let lib_before = library_size(&engine, P0);

    cast_from_hand(&mut engine, P0, balance_of_power());
    match engine.pending().clone() {
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, P0);
            assert_eq!(options, vec![P1], "only opponents may be targeted");
            engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
        }
        Pending::ChooseTargets {
            player,
            player_options,
            options,
            ..
        } => {
            assert_eq!(player, P0);
            assert!(options.is_empty());
            assert_eq!(player_options, vec![P1], "only opponents may be targeted");
            lk::aim(&mut engine, &[], &[P1]);
        }
        other => panic!("expected opponent target prompt, got {other:?}"),
    }
    lk::settle(&mut engine);

    assert_eq!(
        library_size(&engine, P0),
        lib_before - 3,
        "opponent had 4 cards while caster had 1 on resolution -> drew 3"
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(P0)).len(), 4);
}

/// Balance of Power — draws zero when the targeted opponent has equal or fewer
/// cards in hand than the caster on resolution.
#[test]
fn balance_of_power_draws_nothing_when_opponent_holds_equal_or_fewer_cards() {
    let mut equal = lk::game(
        SEED,
        &[island(), island(), island(), island(), island()],
        &[],
        &[balance_of_power(), forest(), plains()],
        &[forest(), island()],
    );
    let lib_equal = library_size(&equal, P0);
    tap_all_mana(&mut equal, P0);
    lk::announce(&mut equal, P0, balance_of_power(), 0, 0, &[], &[P1]);
    lk::settle(&mut equal);
    assert_eq!(library_size(&equal, P0), lib_equal);

    let mut fewer = lk::game(
        SEED,
        &[island(), island(), island(), island(), island()],
        &[],
        &[balance_of_power(), forest(), plains(), swamp()],
        &[forest()],
    );
    let lib_fewer = library_size(&fewer, P0);
    tap_all_mana(&mut fewer, P0);
    lk::announce(&mut fewer, P0, balance_of_power(), 0, 0, &[], &[P1]);
    lk::settle(&mut fewer);
    assert_eq!(library_size(&fewer, P0), lib_fewer);
}
