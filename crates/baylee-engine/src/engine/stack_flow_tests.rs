//! The stack with a real instant (Shock, {R}: 2 damage to any target):
//! "any target" offers players and creatures (CR 115.4), a resolved spell
//! goes to its owner's graveyard (CR 608.2n), the last spell cast resolves
//! first (CR 405.5), and a spell whose only target is gone does not
//! resolve (CR 608.2b).

use super::testkit::{
    Duel, card_index, in_graveyard, in_hand, keep_mulligans, on_battlefield, pass_until,
    quiet_creature, stack_is_empty, tap_all_mana,
};
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 0x57ac_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn shock() -> CardIndex {
    card_index("a9d288b8-cdc1-4e55-a0c9-d6edfc95e65d")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn duel() -> Game {
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[shock()])
        .hand(1, &[shock()])
        .battlefield(0, &[mountain()])
        .battlefield(1, &[mountain(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

/// Casts `seat`'s Shock and answers its target question.
fn shock_at(engine: &mut Game, seat: PlayerId, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    tap_all_mana(engine, seat);
    let card = in_hand(engine, seat, shock()).expect("Shock in hand");
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .unwrap();
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "{:?}",
        engine.pending()
    );
    engine
        .apply(seat, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
    while !matches!(engine.pending(), Pending::Priority { .. }) {
        let (player, action) = super::testkit::answer_one(engine).expect("answerable");
        engine.apply(player, action).unwrap();
    }
}

fn life(engine: &Game, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

#[test]
fn any_target_offers_both_players_and_the_creatures() {
    let mut engine = duel();
    tap_all_mana(&mut engine, P0);
    let card = in_hand(&engine, P0, shock()).unwrap();
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    let Pending::ChooseTargets {
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("{:?}", engine.pending())
    };
    let elf = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    assert!(options.contains(&elf));
    assert!(player_options.contains(&P0) && player_options.contains(&P1));
    assert_eq!((min, max), (1, 1));
}

#[test]
fn shock_to_a_player_costs_two_life_and_goes_to_the_graveyard() {
    let mut engine = duel();
    shock_at(&mut engine, P0, vec![], vec![P1]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, P1), 18);
    assert_eq!(life(&engine, P0), 20);
    assert!(in_graveyard(&engine, P0, shock()).is_some());
}

#[test]
fn shock_to_a_creature_kills_a_one_toughness_creature() {
    let mut engine = duel();
    let elf = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    shock_at(&mut engine, P0, vec![elf], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P1, quiet_creature()).is_none());
    assert!(in_graveyard(&engine, P1, quiet_creature()).is_some());
    assert_eq!(
        life(&engine, P1),
        20,
        "the damage went to the creature only"
    );
}

#[test]
fn the_last_spell_cast_resolves_first() {
    let mut engine = duel();
    shock_at(&mut engine, P0, vec![], vec![P1]);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == P1));
    shock_at(&mut engine, P1, vec![], vec![P0]);
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .len(),
        2
    );
    // Both pass: the top of the stack, seat 1's Shock, resolves alone.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert_eq!(life(&engine, P0), 18, "seat 1's Shock resolved");
    assert_eq!(life(&engine, P1), 20, "seat 0's Shock waits");
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .len(),
        1
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, P1), 18);
}

#[test]
fn a_spell_whose_target_is_gone_does_not_resolve() {
    let mut engine = duel();
    let elf = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    shock_at(&mut engine, P0, vec![elf], vec![]);
    {
        let state = engine.dev_state_mut(P0).expect("dev board");
        state
            .move_object(
                elf,
                crate::zone::ZoneLocation::Hand(P1),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P0, shock()).is_some());
    assert_eq!(life(&engine, P0), 20);
    assert_eq!(life(&engine, P1), 20);
    assert!(in_hand(&engine, P1, quiet_creature()).is_some());
}

#[test]
fn an_instant_may_be_cast_in_the_opponents_turn() {
    let mut engine = duel();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    shock_at(&mut engine, P0, vec![], vec![P1]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, P1), 18);
}

/// After a spell resolves the active player receives priority (CR
/// 117.3b), even when the other player cast it.
#[test]
fn after_resolution_priority_goes_to_the_active_player() {
    let mut engine = duel();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
    });
    shock_at(&mut engine, P1, vec![], vec![P0]);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(stack_is_empty(&engine));
    assert_eq!(life(&engine, P0), 18);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0),
        "{:?}",
        engine.pending()
    );
}

/// Both players passing in a row with an empty stack ends the step
/// (CR 500.2); one pass does not.
#[test]
fn one_pass_with_an_empty_stack_does_not_end_the_step() {
    let mut engine = duel();
    let step = (engine.state().turn.phase, engine.state().turn.step);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert_eq!((engine.state().turn.phase, engine.state().turn.step), step);
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == P1));
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert_ne!((engine.state().turn.phase, engine.state().turn.step), step);
}
