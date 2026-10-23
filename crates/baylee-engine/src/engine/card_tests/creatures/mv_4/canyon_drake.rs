//! `cards/creatures/mv_4/canyon_drake.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn canyon_drake() -> CardIndex {
    card_index("b5b46c99-1cc1-465a-a831-3fd6665ef560")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// p0 has a Canyon Drake and a Mountain, `hand` as the whole hand, and their
/// first main phase.
fn a_drake_with_a_hand(hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[canyon_drake(), mountain()])
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

fn hand_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// Canyon Drake (b5b46c99-1cc1-465a-a831-3fd6665ef560) — {2}{R}{R} Creature — Drake:
/// "Flying. {1}, Discard a card at random: This creature gets +2/+0 until end of turn."
#[test]
fn canyon_drake_flies_and_discards_at_random_for_plus_two_plus_zero() {
    let mut engine = a_drake_with_a_hand(&[grizzly_bears()]);
    let drake = on_battlefield(&engine, P0, canyon_drake()).unwrap();
    assert_eq!(pt(&engine, drake), (1, 2));
    assert!(keywords(&engine, drake).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, canyon_drake(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, grizzly_bears()).is_some());
    assert_eq!(pt(&engine, drake), (3, 2), "+2/+0: toughness is untouched");
}

#[test]
fn canyon_drake_discards_exactly_one_card_of_a_larger_hand() {
    let mut engine = a_drake_with_a_hand(&[grizzly_bears(), hill_giant()]);
    let drake = on_battlefield(&engine, P0, canyon_drake()).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, canyon_drake(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(hand_size(&engine, P0), 1, "one card is the price, not both");
    let in_yard = [grizzly_bears(), hill_giant()]
        .into_iter()
        .filter(|&c| in_graveyard(&engine, P0, c).is_some())
        .count();
    assert_eq!(in_yard, 1);
    assert_eq!(pt(&engine, drake), (3, 2));
}

#[test]
fn canyon_drake_pump_ends_with_the_turn() {
    let mut engine = a_drake_with_a_hand(&[grizzly_bears()]);
    let drake = on_battlefield(&engine, P0, canyon_drake()).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, canyon_drake(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, drake), (3, 2));

    reach_their_main_phase(&mut engine, P1);

    assert_eq!(pt(&engine, drake), (1, 2), "until end of turn");
}

#[test]
fn canyon_drake_cannot_pay_its_cost_with_an_empty_hand() {
    let engine = a_drake_with_a_hand(&[]);
    let drake = on_battlefield(&engine, P0, canyon_drake()).unwrap();

    assert!(
        !ability_offered(&engine, drake, 0),
        "{{1}} is available but there is no card to discard"
    );
}
