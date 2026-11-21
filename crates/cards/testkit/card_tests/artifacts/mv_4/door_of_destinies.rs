//! `cards/artifacts/mv_4/door_of_destinies.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn door_of_destinies() -> CardIndex {
    card_index("9b6d3dcf-aa5a-4516-bd48-a0723e86bfd1")
}

fn mons_s_goblin_raiders() -> CardIndex {
    card_index("a37159df-f6d7-4db6-85de-0ea77f425993")
}

fn woodland_changeling() -> CardIndex {
    card_index("338f0ab6-f744-4e5a-a358-05e71b93758d")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn cast_door_naming(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
    chosen: baylee_core::ids::SubtypeId,
) -> ObjectId {
    let door_card = in_hand(engine, p0, door_of_destinies()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: door_card })
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(chosen))
        .expect("choose creature type for Door of Destinies");
    pass_until(engine, stack_is_empty);
    on_battlefield(engine, p0, door_of_destinies()).unwrap()
}

/// Door of Destinies (9b6d3dcf-aa5a-4516-bd48-a0723e86bfd1) — {4} Artifact:
/// "As this artifact enters, choose a creature type."
/// "Whenever you cast a spell of the chosen type, put a charge counter on this
/// artifact."
/// "Creatures you control of the chosen type get +1/+1 for each charge counter
/// on this artifact."
///
/// Casting Goblin and Changeling spells adds charge counters and buffs your
/// Goblins accordingly, leaving non-Goblins and opponent's Goblins unbuffed.
#[test]
fn door_of_destinies_counts_your_chosen_type_and_changeling_casts() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                forest(),
                forest(),
                mons_s_goblin_raiders(),
                grizzly_bears(),
            ],
        )
        .hand(
            0,
            &[
                door_of_destinies(),
                mons_s_goblin_raiders(),
                woodland_changeling(),
            ],
        )
        .battlefield(1, &[mons_s_goblin_raiders()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p0_goblin = on_battlefield(&engine, p0, mons_s_goblin_raiders()).unwrap();
    let p0_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let p1_goblin = on_battlefield(&engine, p1, mons_s_goblin_raiders()).unwrap();

    tap_all_mana(&mut engine, p0);
    let door = cast_door_naming(
        &mut engine,
        p0,
        baylee_core::generated::subtypes::creature::GOBLIN,
    );
    assert_eq!(counters_on(&engine, door, CounterKind::Charge), 0);

    let g_hand = in_hand(&engine, p0, mons_s_goblin_raiders()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: g_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(counters_on(&engine, door, CounterKind::Charge), 1);
    assert_eq!(pt(&engine, p0_goblin), (2, 2), "your Goblin gets +1/+1");
    assert_eq!(
        pt(&engine, p0_bears),
        (2, 2),
        "non-Goblin Grizzly Bears unbuffed"
    );
    assert_eq!(pt(&engine, p1_goblin), (1, 1), "opponent's Goblin unbuffed");

    let changeling = in_hand(&engine, p0, woodland_changeling()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: changeling })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let changeling_bf = on_battlefield(&engine, p0, woodland_changeling()).unwrap();
    assert_eq!(counters_on(&engine, door, CounterKind::Charge), 2);
    assert_eq!(pt(&engine, p0_goblin), (3, 3), "1/1 + 2/+2 = 3/3");
    assert_eq!(
        pt(&engine, changeling_bf),
        (4, 4),
        "Changeling 2/2 + 2/+2 = 4/4"
    );
}

/// Non-chosen-type spells you cast and chosen-type spells cast by an opponent
/// do NOT put charge counters on Door of Destinies.
#[test]
fn door_of_destinies_ignores_non_chosen_and_opponent_casts() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains(), mountain()])
        .hand(0, &[door_of_destinies(), lightning_bolt()])
        .battlefield(1, &[mountain()])
        .hand(1, &[mons_s_goblin_raiders()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let door = cast_door_naming(
        &mut engine,
        p0,
        baylee_core::generated::subtypes::creature::GOBLIN,
    );

    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, door, CounterKind::Charge), 0);

    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    let p1_g = in_hand(&engine, p1, mons_s_goblin_raiders()).unwrap();
    engine
        .apply(p1, PlayerAction::CastSpell { card: p1_g })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, door, CounterKind::Charge),
        0,
        "opponent's Goblin cast does not trigger your Door of Destinies"
    );
}
