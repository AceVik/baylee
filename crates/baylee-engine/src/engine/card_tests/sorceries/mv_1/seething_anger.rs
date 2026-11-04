//! `cards/sorceries/mv_1/seething_anger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn seething_anger() -> CardIndex {
    card_index("2effcd1c-5833-4e0e-bb67-2d2ffb22a014")
}

/// p0 has `lands` Mountains and Grizzly Bears, with Seething Anger in hand;
/// p1 has two Islands and a Counterspell. p0's main phase.
fn anger_in_the_bears(lands: usize) -> Engine<RegistryLookup> {
    let mut board = vec![mountain(); lands];
    board.push(index::GRIZZLY_BEARS);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &board)
        .battlefield(1, &[island(), island()])
        .hand(0, &[seething_anger()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Seething Anger (`2effcd1c-5833-4e0e-bb67-2d2ffb22a014`) — {R} Sorcery:
/// "Target creature gets +3/+0 until end of turn."
#[test]
fn seething_anger_gives_target_creature_plus_three_plus_zero() {
    let mut engine = anger_in_the_bears(1);

    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, seething_anger());
    aim_at(&mut engine, P0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert_not_bought_back(&engine, P0, seething_anger());

    assert_eq!(power_toughness(&engine, bear), (5, 2));
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(power_toughness(&engine, bear), (2, 2));
}

/// "Buyback {3}": the {3} is paid on top, and the card comes home.
#[test]
fn seething_anger_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = anger_in_the_bears(4);
    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, seething_anger());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bear);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{R}} and {{3}} are the four floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(power_toughness(&engine, bear), (5, 2));
    assert_bought_back(&engine, P0, seething_anger());
}

/// Declined, it costs three less and is a Seething Anger that is spent.
#[test]
fn seething_anger_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = anger_in_the_bears(4);
    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, seething_anger());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, bear);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(power_toughness(&engine, bear), (5, 2));
    assert_not_bought_back(&engine, P0, seething_anger());
}

/// Countered, it does not resolve: buyback paid, card gone, the creature as
/// it was.
#[test]
fn seething_anger_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = anger_in_the_bears(4);
    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, seething_anger());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bear);
    counter_the_top_spell(&mut engine, P0, P1);

    assert_eq!(power_toughness(&engine, bear), (2, 2));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, seething_anger());
}

/// With its one target gone it does not resolve either.
#[test]
fn seething_anger_that_loses_its_target_goes_to_the_graveyard() {
    let mut engine = anger_in_the_bears(4);
    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, seething_anger());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bear);
    bury(&mut engine, &[bear]);
    pass_until(&mut engine, stack_is_empty);

    assert_not_bought_back(&engine, P0, seething_anger());
}
