//! Tokens and sweepers with real cards: Raise the Alarm ({1}{W}, two 1/1
//! Soldier tokens), Day of Judgment and Wrath of God ({2}{W}{W}, destroy
//! all creatures; Wrath's can't be regenerated) and Pyroclasm ({1}{R}, 2
//! damage to each creature). A token that leaves the battlefield ceases to
//! exist (CR 704.5d); "each" and "all" reach both sides of the table.

use super::testkit::{
    Duel, basic_forest, card_index, cast_from_hand, in_graveyard, keep_mulligans, on_battlefield,
    pass_until, quiet_creature, stack_is_empty,
};
use super::*;
use crate::zone::ZoneLocation;
use baylee_core::ids::CardIndex;
use baylee_core::types::TypeSet;

const SEED: u64 = 0x3a55_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn raise_the_alarm() -> CardIndex {
    card_index("5b2364d7-a811-4595-a1b4-224c70555ffa")
}

fn day_of_judgment() -> CardIndex {
    card_index("d057289d-5e28-43d5-8ff3-4a1bc723477d")
}

fn wrath() -> CardIndex {
    card_index("34515b16-c9a4-4f98-8c77-416a7a523407")
}

fn pyroclasm() -> CardIndex {
    card_index("e4bcd4ea-e7cd-4471-8f3b-18bb51d3d70c")
}

fn troll() -> CardIndex {
    card_index("c2f86723-4765-4dac-a9f6-9cfa543613b2")
}

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn duel(hand: &[CardIndex], mine: &[CardIndex], theirs: &[CardIndex]) -> Game {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

fn creatures(engine: &Game, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine.state().object(**id).is_some_and(|o| {
                o.controller == seat && o.characteristics().types.contains(TypeSet::CREATURE)
            })
        })
        .count()
}

fn tokens(engine: &Game, seat: PlayerId) -> Vec<baylee_core::ids::ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat && o.card.is_none())
        })
        .collect()
}

fn resolve(engine: &mut Game, card: CardIndex) {
    cast_from_hand(engine, P0, card);
    pass_until(engine, stack_is_empty);
}

#[test]
fn raise_the_alarm_makes_two_soldiers_for_its_caster() {
    let mut engine = duel(&[raise_the_alarm()], &[plains(), plains()], &[]);
    resolve(&mut engine, raise_the_alarm());
    assert_eq!(tokens(&engine, P0).len(), 2);
    assert_eq!(tokens(&engine, P1).len(), 0);
    for token in tokens(&engine, P0) {
        assert_eq!(super::testkit::pt(&engine, token), (1, 1));
    }
}

#[test]
fn a_destroyed_token_goes_nowhere() {
    let mut engine = duel(&[raise_the_alarm()], &[plains(), plains()], &[]);
    resolve(&mut engine, raise_the_alarm());
    let token = tokens(&engine, P0)[0];
    {
        let state = engine.dev_state_mut(P0).expect("dev board");
        crate::sba::destroy(state, token);
        let _ = crate::sba::run(state, &super::testkit::RegistryLookup);
    }
    assert!(engine.state().object(token).is_none(), "it ceased to exist");
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(P0))
            .contains(&token)
    );
    assert_eq!(tokens(&engine, P0).len(), 1);
}

#[test]
fn day_of_judgment_destroys_every_creature_on_both_sides() {
    let mut engine = duel(
        &[day_of_judgment()],
        &[plains(), plains(), plains(), plains(), quiet_creature()],
        &[quiet_creature(), quiet_creature()],
    );
    resolve(&mut engine, day_of_judgment());
    assert_eq!(creatures(&engine, P0), 0);
    assert_eq!(creatures(&engine, P1), 0);
    assert!(in_graveyard(&engine, P1, quiet_creature()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Battlefield).len(),
        4,
        "the four lands stay"
    );
}

#[test]
fn wrath_of_god_cannot_be_regenerated_from() {
    let mut engine = duel(
        &[wrath()],
        &[plains(), plains(), plains(), plains()],
        &[troll()],
    );
    let troll_id = on_battlefield(&engine, P1, troll()).unwrap();
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .object_mut(troll_id)
        .unwrap()
        .regeneration_shields = 1;
    resolve(&mut engine, wrath());
    assert!(on_battlefield(&engine, P1, troll()).is_none());
}

#[test]
fn day_of_judgment_respects_a_regeneration_shield() {
    let mut engine = duel(
        &[day_of_judgment()],
        &[plains(), plains(), plains(), plains()],
        &[troll()],
    );
    let troll_id = on_battlefield(&engine, P1, troll()).unwrap();
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .object_mut(troll_id)
        .unwrap()
        .regeneration_shields = 1;
    resolve(&mut engine, day_of_judgment());
    assert!(on_battlefield(&engine, P1, troll()).is_some());
}

#[test]
fn pyroclasm_kills_the_small_and_marks_the_big() {
    let mut engine = duel(
        &[pyroclasm()],
        &[mountain(), mountain(), quiet_creature()],
        &[troll()],
    );
    let troll_id = on_battlefield(&engine, P1, troll()).unwrap();
    resolve(&mut engine, pyroclasm());
    assert!(
        on_battlefield(&engine, P0, quiet_creature()).is_none(),
        "its caster's too"
    );
    assert!(
        on_battlefield(&engine, P1, troll()).is_some(),
        "a 4/3 survives 2"
    );
    assert_eq!(engine.state().object(troll_id).unwrap().damage, 2);
}

#[test]
fn pyroclasm_does_not_touch_players() {
    let mut engine = duel(&[pyroclasm()], &[mountain(), mountain()], &[]);
    resolve(&mut engine, pyroclasm());
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}

/// Tokens made this turn are summoning sick like any creature (CR 302.6)
/// and attack on their controller's next turn.
#[test]
fn new_tokens_wait_a_turn_to_attack() {
    let mut engine = duel(&[raise_the_alarm()], &[plains(), plains()], &[]);
    resolve(&mut engine, raise_the_alarm());
    let soldiers = tokens(&engine, P0);
    for token in &soldiers {
        assert!(!crate::combat::can_attack(engine.state(), P0, *token));
    }
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number > turn
            && e.state().turn.phase == Phase::FirstMain
    });
    for token in &soldiers {
        assert!(crate::combat::can_attack(engine.state(), P0, *token));
    }
}

/// Glorious Anthem reaches tokens like any creature its controller has.
#[test]
fn an_anthem_pumps_tokens_too() {
    let anthem = card_index("e3886fe8-9b76-4613-8891-4ec74657c087");
    let mut engine = duel(&[raise_the_alarm()], &[plains(), plains(), anthem], &[]);
    resolve(&mut engine, raise_the_alarm());
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .refresh_characteristics();
    for token in tokens(&engine, P0) {
        assert_eq!(super::testkit::pt(&engine, token), (2, 2));
    }
}
