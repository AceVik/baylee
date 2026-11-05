//! Paying a spell's mana cost from basic lands, through the engine: a
//! coloured symbol needs its colour, a generic symbol takes any colour
//! (CR 107.4b), and the total must be there (CR 601.2h). Prodigal Sorcerer
//! costs {2}{U}, Day of Judgment {2}{W}{W}.

use super::testkit::{
    Duel, basic_forest, card_index, in_hand, keep_mulligans, pass_until, tap_all_mana,
};
use super::*;
use baylee_core::ids::CardIndex;

const SEED: u64 = 0xc057_0001;
const P0: PlayerId = PlayerId::new(0);

fn sorcerer() -> CardIndex {
    card_index("5e961d15-5972-4e4b-9385-1cd7cd7c6bbe")
}

fn day_of_judgment() -> CardIndex {
    card_index("d057289d-5e28-43d5-8ff3-4a1bc723477d")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

fn forest() -> CardIndex {
    basic_forest()
}

fn castable_with(card: CardIndex, lands: &[CardIndex]) -> bool {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[card])
        .battlefield(0, lands)
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana(&mut engine, P0);
    let spell = in_hand(&engine, P0, card).unwrap();
    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .is_ok()
}

#[test]
fn generic_mana_is_paid_with_any_colour() {
    assert!(castable_with(sorcerer(), &[island(), forest(), forest()]));
    assert!(castable_with(sorcerer(), &[island(), plains(), forest()]));
    assert!(castable_with(sorcerer(), &[island(), island(), island()]));
}

#[test]
fn the_coloured_part_needs_its_colour() {
    assert!(!castable_with(sorcerer(), &[forest(), forest(), forest()]));
    assert!(!castable_with(
        sorcerer(),
        &[plains(), plains(), plains(), plains()]
    ));
}

#[test]
fn the_total_must_be_there() {
    assert!(!castable_with(sorcerer(), &[island(), forest()]));
    assert!(!castable_with(sorcerer(), &[island()]));
    assert!(!castable_with(sorcerer(), &[]));
}

#[test]
fn two_coloured_symbols_need_two_of_that_colour() {
    assert!(castable_with(
        day_of_judgment(),
        &[plains(), plains(), forest(), island()]
    ));
    assert!(!castable_with(
        day_of_judgment(),
        &[plains(), forest(), forest(), island()]
    ));
}

#[test]
fn more_mana_than_needed_is_fine() {
    assert!(castable_with(
        sorcerer(),
        &[island(), island(), forest(), forest(), plains()]
    ));
}
