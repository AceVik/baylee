//! "This turn" is one turn: what the state tallies per turn — life lost,
//! life gained, damage marked on creatures (CR 514.2) — is cleared as the
//! next turn begins, for every seat and not only the active one.

use super::testkit::{Duel, basic_forest, keep_mulligans, pass_until, quiet_creature};
use super::*;

const SEED: u64 = 0x7e12_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn duel() -> Game {
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { .. })
    });
    engine
}

fn next_turn(engine: &mut Game) {
    let turn = engine.state().turn.number;
    pass_until(engine, |e| {
        e.state().turn.number > turn && matches!(e.pending(), Pending::Priority { .. })
    });
}

#[test]
fn life_lost_this_turn_is_forgotten_next_turn() {
    let mut engine = duel();
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .change_life(P1, -3, crate::event::Cause::Effect);
    assert!(engine.state().per_turn.life_lost[1]);
    assert_eq!(engine.state().per_turn.life_lost_amount[1], 3);
    next_turn(&mut engine);
    assert!(!engine.state().per_turn.life_lost[1]);
    assert_eq!(engine.state().per_turn.life_lost_amount[1], 0);
    assert_eq!(
        engine.state().players[1].life,
        17,
        "the life itself stays lost"
    );
}

#[test]
fn life_gained_this_turn_is_forgotten_next_turn() {
    let mut engine = duel();
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .change_life(P0, 4, crate::event::Cause::Effect);
    assert_eq!(engine.state().per_turn.life_gained_amount[0], 4);
    next_turn(&mut engine);
    assert_eq!(engine.state().per_turn.life_gained_amount[0], 0);
    assert_eq!(engine.state().players[0].life, 24);
}

#[test]
fn damage_on_a_creature_is_removed_at_cleanup_of_any_turn() {
    let mut engine = duel();
    let elf = super::testkit::on_battlefield(&engine, P1, quiet_creature()).expect("an elf");
    {
        let state = engine.dev_state_mut(P0).expect("dev board");
        state.object_mut(elf).unwrap().base_mut().toughness = Some(5);
        state.invalidate_projections();
        state.object_mut(elf).unwrap().damage = 3;
    }
    next_turn(&mut engine);
    assert_eq!(
        engine.state().object(elf).unwrap().damage,
        0,
        "seat 1's creature is healed in seat 0's cleanup"
    );
}
