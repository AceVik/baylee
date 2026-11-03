//! A static ability and a keyword that forbids, with real cards: Glorious
//! Anthem (creatures you control get +1/+1) and Wall of Wood (0/3
//! defender). A static ability applies for as long as its source is on
//! the battlefield (CR 611.3a), to what it names now — creatures that
//! arrive later included — and to nothing else; defender stops an attack
//! and not a block (CR 702.3b).

use super::testkit::{
    Duel, card_index, keep_mulligans, on_battlefield, pass_until, pt, quiet_creature,
};
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 0x57a7_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn anthem() -> CardIndex {
    card_index("e3886fe8-9b76-4613-8891-4ec74657c087")
}

fn wall() -> CardIndex {
    card_index("aa5e6894-8c97-46f8-a0fd-07c6db51e9a7")
}

fn duel(mine: &[CardIndex], theirs: &[CardIndex]) -> Game {
    let mut engine = Duel::new(SEED, quiet_creature())
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

fn body(engine: &mut Game, id: ObjectId) -> (i16, i16) {
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .refresh_characteristics();
    pt(engine, id)
}

#[test]
fn the_anthem_reaches_its_controllers_creatures() {
    let mut engine = duel(&[anthem(), quiet_creature()], &[]);
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    assert_eq!(body(&mut engine, elf), (2, 2));
}

#[test]
fn the_anthem_leaves_the_opponents_creatures_alone() {
    let mut engine = duel(&[anthem()], &[quiet_creature()]);
    let theirs = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    assert_eq!(body(&mut engine, theirs), (1, 1));
}

#[test]
fn a_creature_that_arrives_later_is_pumped_too() {
    let mut engine = duel(&[anthem()], &[]);
    let elf = {
        let state = engine.dev_state_mut(P0).expect("dev board");
        let card = state.draw_cards(P0, 1)[0];
        state
            .move_object(
                card,
                crate::zone::ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap()
    };
    assert_eq!(body(&mut engine, elf), (2, 2));
}

#[test]
fn the_bonus_ends_when_the_anthem_leaves() {
    let mut engine = duel(&[anthem(), quiet_creature()], &[]);
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    let source = on_battlefield(&engine, P0, anthem()).unwrap();
    crate::sba::destroy(engine.dev_state_mut(P0).expect("dev board"), source);
    assert_eq!(body(&mut engine, elf), (1, 1));
}

#[test]
fn two_anthems_add_up() {
    let mut engine = duel(&[anthem(), anthem(), quiet_creature()], &[]);
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    assert_eq!(body(&mut engine, elf), (3, 3));
}

#[test]
fn a_wall_cannot_attack() {
    let engine = duel(&[wall()], &[]);
    let wood = on_battlefield(&engine, P0, wall()).unwrap();
    assert!(!crate::combat::can_attack(engine.state(), P0, wood));
}

#[test]
fn a_wall_can_block() {
    let mut engine = duel(&[quiet_creature()], &[wall()]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    let wood = on_battlefield(&engine, P1, wall()).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, baylee_core::ids::Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(wood, elf)],
                },
            )
            .is_ok()
    );
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 20);
    assert!(
        on_battlefield(&engine, P1, wall()).is_some(),
        "a 0/3 survives one damage"
    );
}
