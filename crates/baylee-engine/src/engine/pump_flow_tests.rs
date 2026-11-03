//! An "until end of turn" effect with a real card (Giant Growth, {G}:
//! target creature gets +3/+3 until end of turn): it applies at once, it
//! may target anyone's creature, it saves a creature from damage that
//! would otherwise be lethal, and it ends in the cleanup step together
//! with the damage being removed (CR 514.2).

use super::testkit::{
    Duel, basic_forest, card_index, in_hand, keep_mulligans, on_battlefield, pass_until, pt,
    quiet_creature, stack_is_empty, tap_all_mana,
};
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 0x9a0f_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn giant_growth() -> CardIndex {
    card_index("5748ebf1-24e3-499d-ab7c-c2cebd462a24")
}

fn duel() -> Game {
    let forest = basic_forest();
    let mut engine = Duel::new(SEED, forest)
        .hand(0, &[giant_growth()])
        .battlefield(0, &[forest, quiet_creature()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

fn grow(engine: &mut Game, target: ObjectId) {
    tap_all_mana(engine, P0);
    let card = in_hand(engine, P0, giant_growth()).expect("in hand");
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    while !matches!(engine.pending(), Pending::Priority { .. }) {
        let (player, action) = super::testkit::answer_one(engine).expect("answerable");
        engine.apply(player, action).unwrap();
    }
    pass_until(engine, stack_is_empty);
}

fn refreshed_pt(engine: &mut Game, id: ObjectId) -> (i16, i16) {
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .refresh_characteristics();
    pt(engine, id)
}

#[test]
fn the_pump_applies_as_the_spell_resolves() {
    let mut engine = duel();
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    assert_eq!(pt(&engine, elf), (1, 1));
    grow(&mut engine, elf);
    assert_eq!(refreshed_pt(&mut engine, elf), (4, 4));
}

#[test]
fn the_pump_may_go_on_an_opponents_creature() {
    let mut engine = duel();
    let theirs = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    let mine = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    grow(&mut engine, theirs);
    assert_eq!(refreshed_pt(&mut engine, theirs), (4, 4));
    assert_eq!(refreshed_pt(&mut engine, mine), (1, 1), "only the target");
}

#[test]
fn the_pump_ends_with_the_turn() {
    let mut engine = duel();
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    grow(&mut engine, elf);
    pass_until(&mut engine, |e| e.state().turn.active == P1);
    assert_eq!(refreshed_pt(&mut engine, elf), (1, 1));
}

#[test]
fn damage_marked_on_a_pumped_creature_leaves_with_the_pump() {
    let mut engine = duel();
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    grow(&mut engine, elf);
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .object_mut(elf)
        .unwrap()
        .damage = 3;
    pass_until(&mut engine, |e| e.state().turn.active == P1);
    assert!(
        on_battlefield(&engine, P0, quiet_creature()).is_some(),
        "three damage on a 4/4 is removed in the same cleanup the pump ends in"
    );
    assert_eq!(engine.state().object(elf).unwrap().damage, 0);
}

#[test]
fn reading_a_pumped_body_twice_gives_one_answer() {
    let mut engine = duel();
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    grow(&mut engine, elf);
    let first = refreshed_pt(&mut engine, elf);
    let again = refreshed_pt(&mut engine, elf);
    assert_eq!(first, again, "reading the projection twice changes nothing");
}
