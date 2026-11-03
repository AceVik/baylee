//! A sacrifice cost and a free spell, with real cards: Mogg Fanatic
//! ("Sacrifice this creature: it deals 1 damage to any target") and
//! Ornithopter ({0}). A sacrifice is paid at once, before the ability
//! resolves (CR 601.2h, 701.21a); it needs no untapped or unsick creature,
//! because only {T} cares about summoning sickness (CR 302.6); and a
//! spell costing {0} is cast with no mana at all.

use super::testkit::{
    Duel, basic_forest, card_index, in_graveyard, in_hand, keep_mulligans, on_battlefield,
    pass_until, stack_is_empty, tap_all_mana,
};
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 0x5ac2_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn fanatic() -> CardIndex {
    card_index("7efd5d62-6da2-428a-98e3-5b842f668656")
}

fn ornithopter() -> CardIndex {
    card_index("a3a98bc9-caa0-49b7-951c-fe4e4f54e4ba")
}

fn duel(library: CardIndex, hand: &[CardIndex], mine: &[CardIndex]) -> Game {
    let mut engine = Duel::new(SEED, library)
        .hand(0, hand)
        .battlefield(0, mine)
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

fn sacrifice_at(engine: &mut Game, goblin: ObjectId, at: PlayerId) -> bool {
    if engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: goblin,
                ability_index: 0,
            },
        )
        .is_err()
    {
        return false;
    }
    while !matches!(engine.pending(), Pending::Priority { .. }) {
        let action = match engine.pending() {
            Pending::ChooseTargets { .. } => PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![at],
            },
            _ => super::testkit::answer_one(engine).expect("answerable").1,
        };
        engine.apply(P0, action).unwrap();
    }
    true
}

#[test]
fn the_sacrifice_is_paid_before_the_ability_resolves() {
    let mut engine = duel(basic_forest(), &[], &[fanatic()]);
    let goblin = on_battlefield(&engine, P0, fanatic()).unwrap();
    assert!(sacrifice_at(&mut engine, goblin, P1));
    assert!(
        on_battlefield(&engine, P0, fanatic()).is_none(),
        "gone as the cost"
    );
    assert!(in_graveyard(&engine, P0, fanatic()).is_some());
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage waits on the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 19);
}

#[test]
fn a_sacrifice_needs_no_creature_that_has_been_there_all_turn() {
    // A library of Fanatics; one is put onto the battlefield this turn.
    let mut engine = duel(fanatic(), &[], &[]);
    let goblin = {
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
    // The offer was made before the Fanatic arrived; let the game ask
    // again in the next step of the same turn.
    let step = engine.state().turn.step;
    pass_until(&mut engine, |e| {
        e.state().turn.step != step
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert_eq!(engine.state().turn.active, P0, "still the turn it arrived");
    assert!(sacrifice_at(&mut engine, goblin, P1));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 19);
}

#[test]
fn a_tapped_creature_can_still_be_sacrificed() {
    let mut engine = duel(basic_forest(), &[], &[fanatic()]);
    let goblin = on_battlefield(&engine, P0, fanatic()).unwrap();
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .object_mut(goblin)
        .unwrap()
        .status
        .insert(crate::object::Status::TAPPED);
    assert!(sacrifice_at(&mut engine, goblin, P1));
}

#[test]
fn a_creature_can_be_sacrificed_only_once() {
    let mut engine = duel(basic_forest(), &[], &[fanatic()]);
    let goblin = on_battlefield(&engine, P0, fanatic()).unwrap();
    assert!(sacrifice_at(&mut engine, goblin, P1));
    assert!(!sacrifice_at(&mut engine, goblin, P1));
}

#[test]
fn a_zero_cost_spell_is_cast_with_no_mana() {
    let mut engine = duel(basic_forest(), &[ornithopter()], &[]);
    assert_eq!(tap_all_mana(&mut engine, P0), 0, "no lands at all");
    let card = in_hand(&engine, P0, ornithopter()).unwrap();
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, ornithopter()).is_some());
}

fn elder() -> CardIndex {
    card_index("e3afc704-220f-498f-9eaa-0821b17dc24c")
}

fn forests_on_battlefield(engine: &Game) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == P0 && o.card.is_some_and(|c| c.index == basic_forest())
            })
        })
        .collect()
}

/// Sakura-Tribe Elder: sacrifice it to search the library for a basic
/// land and put it onto the battlefield tapped, then shuffle.
#[test]
fn the_elder_trades_itself_for_a_tapped_basic_land() {
    let mut engine = duel(basic_forest(), &[], &[elder()]);
    let snake = on_battlefield(&engine, P0, elder()).unwrap();
    let library = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(P0))
        .len();
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: snake,
                ability_index: 0,
            },
        )
        .unwrap();
    assert!(in_graveyard(&engine, P0, elder()).is_some());
    for _ in 0..20 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        if let Pending::Priority { player, .. } = engine.pending().clone() {
            engine.apply(player, PlayerAction::PassPriority).unwrap();
        } else {
            let (player, action) = super::testkit::answer_one(&engine).expect("answerable");
            engine.apply(player, action).unwrap();
        }
    }
    let lands = forests_on_battlefield(&engine);
    assert_eq!(lands.len(), 1);
    assert!(
        engine
            .state()
            .object(lands[0])
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED),
        "it enters tapped"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Library(P0))
            .len(),
        library - 1
    );
}
