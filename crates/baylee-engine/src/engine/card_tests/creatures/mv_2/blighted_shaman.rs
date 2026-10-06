//! `cards/creatures/mv_2/blighted_shaman.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blighted Shaman prints two lines off one tap: `{T}, Sacrifice a Swamp:`
/// gives a target creature +1/+1 until end of turn, and `{T}, Sacrifice a
/// creature:` gives it +2/+2. Neither line prints a mana symbol, so the two
/// are told apart only by the menu their cost draws — which is exactly what
/// this board is built to read: a Swamp filter that must refuse the Forest
/// beside it and the Swamp across the table, and a creature filter that must
/// refuse the Elf across the table while eating one of its own. Two Shamans
/// stand on purpose, because the first line spends its own `{T}` and no pool
/// may stand in for a tap, so the whole scenario fits in one main phase. The
/// bodies are read on both sides: each pump lands on the creature that was
/// named and leaves every other creature as printed.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn blighted_shaman_trades_a_swamp_for_one_and_a_creature_for_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(1701, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                forest(),
                blighted_shaman(),
                blighted_shaman(),
                quiet_creature(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[swamp(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let shamans = all_on_battlefield(&engine, p0, blighted_shaman());
    assert_eq!(shamans.len(), 2, "two Shamans, one line each");
    let (swamp_eater, creature_eater) = (shamans[0], shamans[1]);
    let elves = all_on_battlefield(&engine, p0, quiet_creature());
    assert_eq!(elves.len(), 2, "one creature to eat and one to pump");
    let (fodder, other) = (elves[0], elves[1]);
    let my_swamps = all_on_battlefield(&engine, p0, swamp());
    assert_eq!(my_swamps.len(), 2, "one Swamp to eat and one to keep");
    let my_forest = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    let their_elf = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    let their_swamp = on_battlefield(&engine, p1, swamp()).expect("their Swamp is out");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "neither line prints a mana symbol, so there is nothing to float and \
         nothing a missing offer could be blamed on"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(swamp_eater, 0))
            && legal.abilities.contains(&(creature_eater, 1)),
        "an untapped Shaman with a Swamp and a creature to feed it offers both \
         lines: {:?}",
        legal.abilities
    );

    // ---- the Swamp line -------------------------------------------------
    let mut aimed_at: Vec<ObjectId> = Vec::new();
    let mut fed_with: Vec<ObjectId> = Vec::new();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: swamp_eater,
                ability_index: 0,
            },
        )
        .expect("the Swamp line is offered");
    for _ in 0..8 {
        match engine.pending().clone() {
            // CR 601.2c takes the target before CR 601.2h pays the cost; the
            // loop answers whichever arrives first rather than pinning an
            // order the engine is free to pick.
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert_eq!(player, p0, "the activating seat aims its own pump");
                aimed_at = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .expect("the Elf was on the menu the engine published");
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the cost is the activator's to pay");
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostSacrifice,
                    "the variant is what tells a client this is a cost and not \
                     a search"
                );
                assert_eq!((min, max), (1, 1), "one Swamp, no more and no fewer");
                fed_with = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![my_swamps[0]],
                        },
                    )
                    .expect("the Swamp was on the menu the engine published");
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected while the Swamp line resolves: {other:?}"),
        }
    }
    assert!(!aimed_at.is_empty(), "the pump asks for a target");
    assert!(
        aimed_at.contains(&fodder) && aimed_at.contains(&their_elf),
        "\"target creature\" is any creature, on either side of the table: {aimed_at:?}"
    );
    assert!(
        !aimed_at.contains(&my_forest) && !aimed_at.contains(&their_swamp),
        "and a land is no creature: {aimed_at:?}"
    );
    assert!(
        fed_with.contains(&my_swamps[0]) && fed_with.contains(&my_swamps[1]),
        "both Swamps this seat controls are the menu: {fed_with:?}"
    );
    assert!(
        !fed_with.contains(&my_forest),
        "\"a Swamp\" is read rather than skipped: the Forest beside them is no \
         Swamp: {fed_with:?}"
    );
    assert!(
        !fed_with.contains(&their_swamp),
        "`CR 701.21a`: a seat sacrifices only what it controls: {fed_with:?}"
    );
    assert_eq!(fed_with.len(), 2, "and those two are the whole menu");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, fodder),
        (2, 2),
        "the printed 1/1 with the Swamp's +1/+1 on it"
    );
    assert_eq!(
        pt(&engine, their_elf),
        (1, 1),
        "the pump reaches the creature it targeted and no other"
    );
    assert!(is_tapped(&engine, swamp_eater), "{{T}} paid the other half");
    assert!(
        in_graveyard(&engine, p0, swamp()).is_some(),
        "the eaten Swamp is in its owner's graveyard"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, swamp()).len(),
        1,
        "and only the one that was named: the other Swamp is still a land"
    );
    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "the Forest was never an option and never moved"
    );

    // ---- the creature line ----------------------------------------------
    let mut aimed_at: Vec<ObjectId> = Vec::new();
    let mut fed_with: Vec<ObjectId> = Vec::new();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: creature_eater,
                ability_index: 1,
            },
        )
        .expect("the creature line is offered");
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseTargets { options, .. } => {
                aimed_at = options;
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![other],
                        },
                    )
                    .expect("the Elf was on the menu the engine published");
            }
            Pending::ChooseCards {
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostSacrifice,
                    "the variant is what tells a client this is a cost and not \
                     a search"
                );
                assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
                fed_with = options;
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .expect("the Elf was on the menu the engine published");
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected while the creature line resolves: {other:?}"),
        }
    }
    assert!(!aimed_at.is_empty(), "the pump asks for a target");
    assert!(
        aimed_at.contains(&other) && aimed_at.contains(&their_elf),
        "\"target creature\" reaches both sides of the table here too: {aimed_at:?}"
    );
    assert!(
        fed_with.contains(&fodder) && fed_with.contains(&other),
        "both creatures this seat controls are the menu: {fed_with:?}"
    );
    assert!(
        fed_with.contains(&swamp_eater) && fed_with.contains(&creature_eater),
        "\"a creature\" is not \"another creature\": the tapped Shaman that \
         already spent its {{T}} and the one paying right now are both edible: \
         {fed_with:?}"
    );
    assert!(
        !fed_with.contains(&their_elf),
        "`CR 701.21a`: the Elf across the table is not yours to sacrifice: {fed_with:?}"
    );
    assert!(
        !fed_with.contains(&my_forest) && !fed_with.contains(&my_swamps[1]),
        "and a land is no creature: {fed_with:?}"
    );
    assert_eq!(fed_with.len(), 4, "and those four are the whole menu");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, other),
        (3, 3),
        "the printed 1/1 with the creature line's +2/+2 on it"
    );
    assert!(
        is_tapped(&engine, creature_eater),
        "{{T}} paid the other half"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_creature()).is_some(),
        "the Elf that was eaten is in its owner's graveyard"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, quiet_creature()),
        vec![other],
        "and the creature the pump landed on is the one that is left standing"
    );
    assert_eq!(
        pt(&engine, their_elf),
        (1, 1),
        "the Elf across the table was offered as a target and never chosen"
    );
}
