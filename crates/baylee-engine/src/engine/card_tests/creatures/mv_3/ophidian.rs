//! `cards/creatures/mv_3/ophidian.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ophidian() -> CardIndex {
    card_index("953bdefa-4639-4524-af5c-5055d1289f40")
}

fn wall_of_ice() -> CardIndex {
    card_index("1945e165-7636-4727-89ba-3811251ef175")
}

fn hand_size(e: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    e.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// The Ophidian attacks alone and nobody blocks; its trigger's question is
/// pending for player 0.
fn unblocked_attack(e: &mut Engine<RegistryLookup>) {
    let oph = lk::obj(e, P0, ophidian());
    lk::attack(e, &[oph]);
    lk::block(e, &[]);
    pass_until(e, |e| matches!(e.pending(), Pending::YesNo { .. }));
}

/// Ophidian (953bdefa-4639-4524-af5c-5055d1289f40) — "Whenever this creature
/// attacks and isn't blocked, you may draw a card. If you do, this creature
/// assigns no combat damage this turn." Yes: a card is drawn and the defender
/// takes nothing. On the next turn, the same attack declined hurts for 1, so
/// "this turn" ended.
#[test]
fn ophidian_draws_and_deals_no_damage_when_you_say_yes_for_this_turn_only() {
    let mut e = lk::game(SEED, &[ophidian()], &[], &[], &[]);
    assert_eq!(hand_size(&e, P0), 0);

    unblocked_attack(&mut e);
    lk::yes_no(&mut e, true);
    lk::through_combat(&mut e);
    assert_eq!(hand_size(&e, P0), 1, "the card is drawn");
    assert_eq!(
        lk::life(&e, P1),
        20,
        "and the Ophidian assigned no combat damage"
    );

    // Their turn passes; on player 0's next turn they decline.
    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let oph = lk::obj(&e, P0, ophidian());
    lk::declare(&mut e, &[oph]);
    lk::block(&mut e, &[]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    let hand_before = hand_size(&e, P0);
    lk::yes_no(&mut e, false);
    lk::through_combat(&mut e);
    assert_eq!(hand_size(&e, P0), hand_before, "declined: no draw");
    assert_eq!(
        lk::life(&e, P1),
        19,
        "the no-damage effect was only for the first turn"
    );
}

/// Ophidian — "you may": declined on the first attack there is no card and
/// the full 1 damage goes through.
#[test]
fn ophidian_deals_its_damage_when_you_decline_to_draw() {
    let mut e = lk::game(SEED, &[ophidian()], &[], &[], &[]);

    unblocked_attack(&mut e);
    lk::yes_no(&mut e, false);
    lk::through_combat(&mut e);

    assert_eq!(hand_size(&e, P0), 0, "no card");
    assert_eq!(lk::life(&e, P1), 19, "1 combat damage");
}

/// Ophidian — "attacks and isn't blocked": when the Wall of Ice blocks it, there
/// is no question, no card, and it deals its 1 damage to the Wall.
#[test]
fn ophidian_asks_nothing_when_it_is_blocked() {
    let mut e = lk::game(SEED, &[ophidian()], &[wall_of_ice()], &[], &[]);
    let oph = lk::obj(&e, P0, ophidian());
    let wall = lk::obj(&e, P1, wall_of_ice());

    lk::attack(&mut e, &[oph]);
    lk::block(&mut e, &[(wall, oph)]);
    lk::through_combat(&mut e);

    assert_eq!(hand_size(&e, P0), 0, "blocked: no trigger, no card");
    assert_eq!(lk::life(&e, P1), 20);
    assert_eq!(
        lk::damage_on(&e, wall),
        1,
        "its combat damage was assigned as usual"
    );
}
