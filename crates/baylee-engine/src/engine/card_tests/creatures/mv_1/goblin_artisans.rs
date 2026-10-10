//! `cards/creatures/mv_1/goblin_artisans.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::event::GameEvent;

fn goblin_artisans() -> CardIndex {
    card_index("ec85a375-fe38-4b11-af0e-f2b466181dd7")
}

/// Seeds whose first coin flip, in the game below, is won and lost.
const WON_SEED: u64 = 3;
const LOST_SEED: u64 = 1;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Every flip the journal holds, in order, as "won".
fn flips(engine: &Engine<RegistryLookup>) -> Vec<bool> {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|e| match e.event {
            GameEvent::CoinFlipped { won, .. } => Some(won),
            _ => None,
        })
        .collect()
}

/// Every Goblin Artisans seat 0 controls.
fn artisans(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
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
                .is_some_and(|o| o.card.is_some_and(|c| c.index == goblin_artisans()))
        })
        .collect()
}

/// Whether `source`'s ability 0 is on offer to the player with priority.
fn offered(engine: &Engine<RegistryLookup>, source: ObjectId) -> bool {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal.abilities.contains(&(source, 0))
}

/// Activates `source`'s ability 0 and returns the targets it may name.
#[track_caller]
fn activate_and_see_options(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    source: ObjectId,
) -> Vec<ObjectId> {
    assert!(offered(engine, source), "the ability is offered");
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .expect("the ability activates");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    options
}

/// Seat 0 has `count` Goblin Artisans and three Forests in play and `hand`
/// in hand; returns the engine at its main phase.
fn setup(seed: u64, count: usize, hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut board = vec![goblin_artisans(); count];
    board.extend([forest(), forest(), forest()]);
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &board)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

fn galvanic_key() -> CardIndex {
    card_index("d8a552ca-2c7b-410e-bd7e-1bb81465277a")
}

/// Casts a Sol Ring and keeps priority with it on the stack.
#[track_caller]
fn ring_on_the_stack(engine: &mut Engine<RegistryLookup>) -> ObjectId {
    cast_with_floating(engine, P0, sol_ring());
    on_stack(engine, sol_ring()).expect("the ring is on the stack")
}

/// Only an artifact spell its controller controls is on offer: Sol Ring yes;
/// Giant Growth (not an artifact) no.
#[test]
fn only_your_own_artifact_spells_are_offered() {
    let mut engine = setup(WON_SEED, 1, &[sol_ring(), giant_growth()]);
    let artisans = artisans(&engine)[0];
    tap_all_mana(&mut engine, P0);
    let ring = ring_on_the_stack(&mut engine);
    cast_with_floating(&mut engine, P0, giant_growth());
    aim_at(&mut engine, P0, artisans);
    assert!(
        on_stack(&engine, giant_growth()).is_some(),
        "the pump is up"
    );
    let options = activate_and_see_options(&mut engine, P0, artisans);
    assert_eq!(options, vec![ring], "the ring alone, not the Giant Growth");
}

/// An opponent's artifact spell is not a target for my Artisans: with it the
/// only spell on the stack the ability is not offered at all.
#[test]
fn an_opponents_artifact_spell_is_not_offered() {
    let mut engine = Duel::new(WON_SEED, forest())
        .battlefield(0, &[goblin_artisans()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);
    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, sol_ring());
    let ring = on_stack(&engine, sol_ring()).expect("their ring is on the stack");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    assert_eq!(on_stack(&engine, sol_ring()), Some(ring), "still waiting");
    let artisans = artisans(&engine)[0];
    assert!(
        !offered(&engine, artisans),
        "no artifact spell of mine to counter, so no ability"
    );
}

/// Casts a Sol Ring and flips with the Artisans; returns the engine once the
/// stack is empty.
fn flip_at_my_ring(seed: u64) -> Engine<RegistryLookup> {
    let mut engine = setup(seed, 1, &[sol_ring()]);
    let artisans = artisans(&engine)[0];
    tap_all_mana(&mut engine, P0);
    let ring = ring_on_the_stack(&mut engine);
    let options = activate_and_see_options(&mut engine, P0, artisans);
    assert!(options.contains(&ring));
    aim_at(&mut engine, P0, ring);
    pass_until(&mut engine, |e| !flips(e).is_empty() && stack_is_empty(e));
    engine
}

/// "If you win the flip, draw a card." The ring resolves.
#[test]
fn a_won_flip_draws_and_the_ring_resolves() {
    let before = {
        let e = setup(WON_SEED, 1, &[sol_ring()]);
        e.state().zones.list(ZoneLocation::Hand(P0)).len()
    };
    let engine = flip_at_my_ring(WON_SEED);
    assert_eq!(flips(&engine), vec![true], "one flip, won");
    assert!(on_battlefield(&engine, P0, sol_ring()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        before - 1 + 1,
        "the ring left the hand, a card came in"
    );
}

/// "If you lose the flip, counter target artifact spell you control."
#[test]
fn a_lost_flip_counters_my_own_ring() {
    let before = {
        let e = setup(LOST_SEED, 1, &[sol_ring()]);
        e.state().zones.list(ZoneLocation::Hand(P0)).len()
    };
    let engine = flip_at_my_ring(LOST_SEED);
    assert_eq!(flips(&engine), vec![false], "one flip, lost");
    assert!(on_battlefield(&engine, P0, sol_ring()).is_none());
    assert!(in_graveyard(&engine, P0, sol_ring()).is_some(), "countered");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        before - 1,
        "no card drawn"
    );
}

/// "...that isn't the target of an ability from another creature named Goblin
/// Artisans": once the first one aims at a ring the second may not, but may
/// aim at another.
#[test]
fn a_second_artisans_cannot_aim_at_the_same_spell() {
    let mut engine = setup(WON_SEED, 2, &[sol_ring(), galvanic_key()]);
    let both = artisans(&engine);
    assert_eq!(both.len(), 2);
    tap_all_mana(&mut engine, P0);
    let first = ring_on_the_stack(&mut engine);

    // Alone, the first ring is an ordinary target for either one.
    assert!(activate_and_see_options(&mut engine, P0, both[0]).contains(&first));
    aim_at(&mut engine, P0, first);

    // Taken: the second finds nothing to aim at.
    assert!(
        !offered(&engine, both[1]),
        "the only artifact spell is already aimed at by the other Artisans"
    );

    // A flashed-in Galvanic Key is a fresh target.
    cast_with_floating(&mut engine, P0, galvanic_key());
    let second = on_stack(&engine, galvanic_key()).expect("the key is on the stack");
    let options = activate_and_see_options(&mut engine, P0, both[1]);
    assert_eq!(options, vec![second], "the claimed ring is not offered");
}
