//! Life as a cost, with real cards: Adanto Vanguard ("Pay 4 life: gains
//! indestructible until end of turn"; +2/+0 while attacking) and
//! Cutthroat Contender ("Pay 1 life: +1/+0 until end of turn; activate
//! only once each turn"). Life is paid as the cost, before the ability
//! resolves (CR 119.4); it can be paid only out of what the player has
//! (CR 119.4); and "only once each turn"
//! is held to.

use super::testkit::{
    Duel, basic_forest, card_index, keep_mulligans, on_battlefield, pass_until, pt, stack_is_empty,
};
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 0x11fe_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn vanguard() -> CardIndex {
    card_index("91c380f6-2c41-48b7-9f07-104e8659e397")
}

fn contender() -> CardIndex {
    card_index("43192553-0db9-48ec-ae88-a2605a6608d8")
}

fn duel(mine: CardIndex, life: i32) -> (Game, ObjectId) {
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[mine])
        .life(0, life)
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    let id = on_battlefield(&engine, P0, mine).unwrap();
    (engine, id)
}

fn activate(engine: &mut Game, source: ObjectId, index: u32) -> bool {
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: index,
            },
        )
        .is_ok()
}

fn life(engine: &Game) -> i32 {
    engine.state().players[0].life
}

fn indestructible(engine: &mut Game, id: ObjectId) -> bool {
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .refresh_characteristics();
    engine
        .state()
        .object(id)
        .unwrap()
        .characteristics()
        .keywords
        .contains(baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE)
}

#[test]
fn the_life_is_paid_on_activation() {
    let (mut engine, knight) = duel(vanguard(), 20);
    assert!(activate(&mut engine, knight, 1));
    assert_eq!(life(&engine), 16, "paid before anything resolves");
    assert!(!indestructible(&mut engine, knight));
    pass_until(&mut engine, stack_is_empty);
    assert!(indestructible(&mut engine, knight));
}

#[test]
fn exactly_enough_life_can_be_paid() {
    let (mut engine, knight) = duel(vanguard(), 4);
    assert!(activate(&mut engine, knight, 1));
    assert_eq!(life(&engine), 0);
}

#[test]
fn less_life_than_the_cost_cannot_pay_it() {
    let (mut engine, knight) = duel(vanguard(), 3);
    assert!(!activate(&mut engine, knight, 1));
    assert_eq!(life(&engine), 3);
}

#[test]
fn indestructible_from_the_ability_survives_destroy() {
    let (mut engine, knight) = duel(vanguard(), 20);
    assert!(activate(&mut engine, knight, 1));
    pass_until(&mut engine, stack_is_empty);
    crate::sba::destroy(engine.dev_state_mut(P0).expect("dev board"), knight);
    assert!(on_battlefield(&engine, P0, vanguard()).is_some());
}

#[test]
fn the_vanguard_is_bigger_only_while_attacking() {
    let (mut engine, knight) = duel(vanguard(), 20);
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .refresh_characteristics();
    assert_eq!(pt(&engine, knight), (1, 1));
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(knight, baylee_core::ids::Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().players[1].life,
        17,
        "a 3/1 while it attacked"
    );
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .refresh_characteristics();
    assert_eq!(pt(&engine, knight), (1, 1), "back to 1/1 after combat");
}

#[test]
fn once_each_turn_is_once() {
    let (mut engine, rogue) = duel(contender(), 20);
    assert!(activate(&mut engine, rogue, 0));
    pass_until(&mut engine, stack_is_empty);
    assert!(!activate(&mut engine, rogue, 0));
    assert_eq!(life(&engine), 19);
}

#[test]
fn once_each_turn_comes_back_next_turn() {
    let (mut engine, rogue) = duel(contender(), 20);
    assert!(activate(&mut engine, rogue, 0));
    pass_until(&mut engine, stack_is_empty);
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number > turn
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert!(activate(&mut engine, rogue, 0));
}
