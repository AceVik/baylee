//! `cards/sorceries/mv_1/goblin_grenade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn goblin_grenade() -> CardIndex {
    card_index("20aecb7b-7e37-4efe-ac27-f9f9042203f4")
}

fn mons_s_goblin_raiders() -> CardIndex {
    card_index("a37159df-f6d7-4db6-85de-0ea77f425993")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Seat 0 holds Goblin Grenade with a Mountain, a Goblin and a Grizzly Bears
/// on the battlefield; seat 1 has a Hill Giant.
fn board(goblin: bool) -> Engine<RegistryLookup> {
    let mut mine = vec![mountain(), grizzly_bears()];
    if goblin {
        mine.push(mons_s_goblin_raiders());
    }
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &mine)
        .battlefield(1, &[hill_giant()])
        .hand(0, &[goblin_grenade()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// The sacrifice prompt: who is asked, and what may be chosen.
fn sacrifice_prompt(engine: &Engine<RegistryLookup>) -> (PlayerId, Vec<ObjectId>) {
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "a sacrifice choice was expected, got {:?}",
            engine.pending()
        );
    };
    (player, options)
}

/// Goblin Grenade (20aecb7b-7e37-4efe-ac27-f9f9042203f4) — "As an additional
/// cost to cast this spell, sacrifice a Goblin. Goblin Grenade deals 5 damage
/// to any target." Aimed at the opponent: the Goblin (and only a Goblin) may
/// be sacrificed, and the opponent takes 5.
#[test]
fn goblin_grenade_sacrifices_a_goblin_and_deals_5_damage_to_a_player() {
    let mut engine = board(true);
    let goblin = on_battlefield(&engine, P0, mons_s_goblin_raiders()).expect("a Goblin");
    let bears = on_battlefield(&engine, P0, grizzly_bears()).expect("a Bears");
    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, goblin_grenade(), 0, 0, &[], &[P1]);

    let (player, options) = sacrifice_prompt(&engine);
    assert_eq!(player, P0);
    assert!(options.contains(&goblin), "the Goblin may be sacrificed");
    assert!(!options.contains(&bears), "a non-Goblin may not");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![goblin],
            },
        )
        .expect("sacrificing the Goblin pays the cost");
    assert!(
        in_graveyard(&engine, P0, mons_s_goblin_raiders()).is_some(),
        "the Goblin is sacrificed as the spell is cast"
    );
    assert_eq!(
        lk::life(&engine, P1),
        20,
        "nothing has happened to the target yet"
    );

    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P1), 15, "5 damage to the target player");
    assert!(on_battlefield(&engine, P0, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, P0, goblin_grenade()).is_some());
}

/// Goblin Grenade — "deals 5 damage to any target": a creature is a target
/// too. A Hill Giant (3/3) is destroyed.
#[test]
fn goblin_grenade_kills_a_creature_with_5_damage() {
    let mut engine = board(true);
    let goblin = on_battlefield(&engine, P0, mons_s_goblin_raiders()).expect("a Goblin");
    let giant = on_battlefield(&engine, P1, hill_giant()).expect("the Giant");
    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, goblin_grenade(), 0, 0, &[giant], &[]);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![goblin],
            },
        )
        .expect("sacrificing the Goblin pays the cost");
    lk::settle(&mut engine);

    assert!(
        on_battlefield(&engine, P1, hill_giant()).is_none(),
        "5 damage kills a 3/3"
    );
    assert_eq!(lk::life(&engine, P1), 20, "the player is untouched");
}

/// Goblin Grenade — the Goblin is a cost, so without one the spell cannot be
/// cast at all, whatever mana is available.
#[test]
fn goblin_grenade_cannot_be_cast_without_a_goblin_to_sacrifice() {
    let mut engine = board(false);
    tap_all_mana(&mut engine, P0);
    assert!(
        !lk::castable(&engine, P0, goblin_grenade()),
        "no Goblin, no spell (a Grizzly Bears is not a Goblin)"
    );
}
