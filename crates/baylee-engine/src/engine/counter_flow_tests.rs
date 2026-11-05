//! Countering a spell with real cards: Counterspell ({U}{U}, counter
//! target spell) and Essence Scatter ({1}{U}, counter target creature
//! spell). A countered spell goes to its owner's graveyard and does none of
//! what it would have done (CR 701.6a); a counter's target is a spell on
//! the stack and only the kind of spell it names (CR 115.2; 115.1a for "target").

use super::testkit::{
    Duel, basic_forest, card_index, cast_from_hand, in_graveyard, in_hand, keep_mulligans,
    on_battlefield, on_stack, pass_until, quiet_creature, stack_is_empty, tap_all_mana,
};
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 0xc0c0_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn counterspell() -> CardIndex {
    card_index("cc187110-1148-4090-bbb8-e205694a39f5")
}

fn essence_scatter() -> CardIndex {
    card_index("46665089-aa3d-44c3-964d-6638dfbb5782")
}

fn shock() -> CardIndex {
    card_index("a9d288b8-cdc1-4e55-a0c9-d6edfc95e65d")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn duel(answer: CardIndex) -> Game {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[quiet_creature(), shock()])
        .hand(1, &[answer])
        .battlefield(0, &[basic_forest(), mountain()])
        .battlefield(1, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

/// Seat 1 casts `answer` and is shown its target question; returns the
/// offered spells.
fn offer(engine: &mut Game, answer: CardIndex) -> Vec<ObjectId> {
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == P1));
    tap_all_mana(engine, P1);
    let card = in_hand(engine, P1, answer).expect("the answer is in hand");
    engine.apply(P1, PlayerAction::CastSpell { card }).unwrap();
    match engine.pending().clone() {
        Pending::ChooseTargets { options, .. } => options,
        other => panic!("a target question, got {other:?}"),
    }
}

fn aim(engine: &mut Game, target: ObjectId) {
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    while !matches!(engine.pending(), Pending::Priority { .. }) {
        let (player, action) = super::testkit::answer_one(engine).expect("answerable");
        engine.apply(player, action).unwrap();
    }
}

#[test]
fn a_countered_creature_spell_goes_to_the_graveyard_and_never_enters() {
    let mut engine = duel(counterspell());
    cast_from_hand(&mut engine, P0, quiet_creature());
    let spell = on_stack(&engine, quiet_creature()).expect("on the stack");
    let options = offer(&mut engine, counterspell());
    assert!(options.contains(&spell));
    aim(&mut engine, spell);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, quiet_creature()).is_none());
    assert!(in_graveyard(&engine, P0, quiet_creature()).is_some());
    assert!(in_graveyard(&engine, P1, counterspell()).is_some());
}

#[test]
fn a_countered_burn_spell_deals_no_damage() {
    let mut engine = duel(counterspell());
    tap_all_mana(&mut engine, P0);
    let card = in_hand(&engine, P0, shock()).unwrap();
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    let spell = on_stack(&engine, shock()).expect("on the stack");
    let _ = offer(&mut engine, counterspell());
    aim(&mut engine, spell);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 20);
    assert!(in_graveyard(&engine, P0, shock()).is_some());
}

#[test]
fn a_creature_counter_offers_a_creature_spell() {
    let mut engine = duel(essence_scatter());
    cast_from_hand(&mut engine, P0, quiet_creature());
    let spell = on_stack(&engine, quiet_creature()).expect("on the stack");
    assert!(offer(&mut engine, essence_scatter()).contains(&spell));
}

/// With a creature spell and a Shock both on the stack, the creature
/// counter is offered the creature spell and not the instant.
#[test]
fn a_creature_counter_does_not_offer_an_instant() {
    let mut engine = duel(essence_scatter());
    tap_all_mana(&mut engine, P0);
    super::testkit::cast_with_floating(&mut engine, P0, quiet_creature());
    let creature = on_stack(&engine, quiet_creature()).expect("on the stack");
    super::testkit::cast_with_floating(&mut engine, P0, shock());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    let burn = on_stack(&engine, shock()).expect("on the stack");
    let options = offer(&mut engine, essence_scatter());
    assert!(options.contains(&creature));
    assert!(!options.contains(&burn));
}

#[test]
fn a_counter_with_nothing_on_the_stack_cannot_be_cast() {
    let mut engine = duel(counterspell());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
            && e.state().zones.stack_is_empty()
    });
    tap_all_mana(&mut engine, P1);
    let card = in_hand(&engine, P1, counterspell()).unwrap();
    assert!(
        engine.apply(P1, PlayerAction::CastSpell { card }).is_err(),
        "a spell with a target needs one (CR 601.2c)"
    );
}

/// Seat 0 casts an elf; seat 1 counters it; seat 0 counters the
/// counter. The top Counterspell resolves first and the one it targets
/// never does, so the elf resolves (CR 405.5, 701.6a).
#[test]
fn a_counter_on_a_counter_lets_the_first_spell_resolve() {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[quiet_creature(), counterspell()])
        .hand(1, &[counterspell()])
        .battlefield(0, &[basic_forest(), island(), island()])
        .battlefield(1, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana(&mut engine, P0);
    super::testkit::cast_with_floating(&mut engine, P0, quiet_creature());
    let elf_spell = on_stack(&engine, quiet_creature()).unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P1);
    let theirs = in_hand(&engine, P1, counterspell()).unwrap();
    engine
        .apply(P1, PlayerAction::CastSpell { card: theirs })
        .unwrap();
    aim(&mut engine, elf_spell);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    let their_counter = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .last()
        .copied()
        .unwrap();
    let mine = in_hand(&engine, P0, counterspell()).unwrap();
    engine
        .apply(P0, PlayerAction::CastSpell { card: mine })
        .unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![their_counter],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, quiet_creature()).is_some());
    assert!(in_graveyard(&engine, P1, counterspell()).is_some());
    assert!(in_graveyard(&engine, P0, counterspell()).is_some());
}
