//! `cards/artifacts/mv_2/cursed_totem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{LLANOWAR_ELVES, PRODIGAL_SORCERER, SOL_RING};

const P0: PlayerId = PlayerId::new(0);

fn cursed_totem() -> CardIndex {
    card_index("6225a704-430a-4f56-ad87-0e8d87f285f5")
}

/// Cursed Totem (6225a704-430a-4f56-ad87-0e8d87f285f5) — {2} Artifact:
/// "Activated abilities of creatures can't be activated."
#[test]
fn cursed_totem_prevents_activating_creature_abilities() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), PRODIGAL_SORCERER])
        .hand(0, &[cursed_totem()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let tim = on_battlefield(&engine, P0, PRODIGAL_SORCERER).unwrap();

    // Before Cursed Totem is on the battlefield, Prodigal Sorcerer can activate.
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(legal.abilities.contains(&(tim, 0)));

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, cursed_totem());
    pass_until(&mut engine, stack_is_empty);

    // With Cursed Totem on the battlefield, Prodigal Sorcerer's ability cannot be activated.
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(!legal.abilities.contains(&(tim, 0)));
}

/// Only "creatures": Sol Ring's mana ability (an artifact's) stays, and
/// Llanowar Elves' mana ability (a creature's) is locked by the Totem. The
/// same Elves without the Totem can activate it.
#[test]
fn cursed_totem_locks_a_creatures_mana_ability_but_not_an_artifacts() {
    let board = |with_totem: bool| {
        let mut field = vec![SOL_RING, LLANOWAR_ELVES];
        if with_totem {
            field.push(cursed_totem());
        }
        let mut engine = Duel::new(SEED, island()).battlefield(0, &field).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, P0);
        engine
    };

    let free = board(false);
    let elves = on_battlefield(&free, P0, LLANOWAR_ELVES).unwrap();
    assert!(ability_offered(&free, elves, 0), "without the Totem");

    let locked = board(true);
    let elves = on_battlefield(&locked, P0, LLANOWAR_ELVES).unwrap();
    let ring = on_battlefield(&locked, P0, SOL_RING).unwrap();
    assert!(
        !ability_offered(&locked, elves, 0),
        "the creature's ability"
    );
    assert!(ability_offered(&locked, ring, 0), "the artifact's ability");
}
