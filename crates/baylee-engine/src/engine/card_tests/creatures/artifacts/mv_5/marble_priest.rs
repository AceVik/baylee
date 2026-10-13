//! `cards/creatures/artifacts/mv_5/marble_priest.rs`, played.
//!
//! Written from the Oracle text and the Comprehensive Rules alone (the card
//! has no Scryfall rulings): CR 509.1b-c (blocking requirements), 510.1
//! (combat damage assignment), 615 (prevention).

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

// oracle_id = "96bcabee-e84e-409f-8439-80f5308d73e9"
fn marble_priest() -> CardIndex {
    card_index("96bcabee-e84e-409f-8439-80f5308d73e9")
}

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// What `actions.rs`' block-requirement check refuses a declaration with.
const MUST_BLOCK: &str = "a creature that must block if able does not";

/// Asserts `result` is the engine's own refusal `why`.
#[track_caller]
fn refused(result: Result<(), EngineError>, why: &str) {
    match result {
        Err(EngineError::IllegalAction(message)) => assert_eq!(message, why),
        other => panic!("expected the refusal {why:?}, got {other:?}"),
    }
}

/// A duel at seat 0's first main phase with the Priest on its side.
#[track_caller]
fn table(theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    lk::game(SEED, &[marble_priest()], theirs, &[], &[])
}

/// Attacks seat 1 with the Priest and stops at the declare-blockers question.
#[track_caller]
fn priest_attacks(engine: &mut Engine<RegistryLookup>) -> ObjectId {
    let priest = on_battlefield(engine, P0, marble_priest()).expect("the Priest is out");
    lk::attack(engine, &[priest]);
    assert!(
        matches!(engine.pending(), Pending::ChooseBlockers { player, .. } if *player == P1),
        "seat 1 is asked to block: {:?}",
        engine.pending()
    );
    priest
}

/// Walks combat to its end, answering the damage divisions a multiple block
/// asks of the attacker (everything to the first blocker listed), and stops in
/// the second main phase, where the damage is still marked.
#[track_caller]
fn through_combat_damage(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..60 {
        if engine.state().turn.phase == Phase::SecondMain {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseNumber { player, max, .. } => {
                engine
                    .apply(player, PlayerAction::ChooseNumber(max))
                    .unwrap();
            }
            other => panic!("unexpected in combat: {other:?}"),
        }
    }
    panic!("combat never ended");
}

fn sorted(mut pairs: Vec<(ObjectId, ObjectId)>) -> Vec<(ObjectId, ObjectId)> {
    pairs.sort_unstable();
    pairs
}

/// "All Walls able to block this creature do so." Two untapped Walls and a
/// Giant face the attacking Priest. The question names the two Walls as the
/// blocks the engine requires and not the Giant; a declaration that leaves
/// either Wall out is refused (CR 509.1c), the one that names both is legal --
/// with or without the Giant beside them, which is free to block or not.
#[test]
fn every_wall_able_to_block_the_priest_must() {
    let mut engine = table(&[index::WALL_OF_HEAT, index::WALL_OF_STONE, index::HILL_GIANT]);
    let heat = on_battlefield(&engine, P1, index::WALL_OF_HEAT).expect("seated");
    let stone = on_battlefield(&engine, P1, index::WALL_OF_STONE).expect("seated");
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).expect("seated");
    let priest = priest_attacks(&mut engine);

    let Pending::ChooseBlockers { obeying, .. } = engine.pending().clone() else {
        unreachable!("priest_attacks stops on exactly this")
    };
    assert_eq!(
        sorted(obeying),
        sorted(vec![(heat, priest), (stone, priest)]),
        "both Walls are required and the Giant is not"
    );

    for short in [
        vec![],
        vec![(heat, priest)],
        vec![(stone, priest)],
        vec![(giant, priest)],
        vec![(heat, priest), (giant, priest)],
    ] {
        refused(
            engine.apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: short.clone(),
                },
            ),
            MUST_BLOCK,
        );
    }

    // Positive control: with the Giant as well, the declaration is legal.
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(heat, priest), (stone, priest), (giant, priest)],
            },
        )
        .expect("both Walls and the Giant: the Walls are in, the Giant is free");
    let mut blocking = engine.state().combat.blockers_of(priest);
    blocking.sort_unstable();
    let mut expected = vec![heat, stone, giant];
    expected.sort_unstable();
    assert_eq!(blocking, expected, "all three block the Priest");
}

/// The same board, the exact requirement and nothing more: the two Walls alone
/// are a legal block, and so is the Giant joining them; what is *not* required
/// is the Giant, and a Wall that cannot block (tapped) is not required either.
#[test]
fn only_walls_able_to_block_are_required_and_non_walls_are_free() {
    let mut engine = table(&[index::WALL_OF_HEAT, index::WALL_OF_STONE, index::HILL_GIANT]);
    let heat = on_battlefield(&engine, P1, index::WALL_OF_HEAT).expect("seated");
    let stone = on_battlefield(&engine, P1, index::WALL_OF_STONE).expect("seated");
    // Wall of Stone is tapped: not "able to block".
    engine.dev_state_mut(P1).unwrap().set_tapped(stone, true);
    engine.refresh_offer();
    let priest = priest_attacks(&mut engine);
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).expect("seated");

    let Pending::ChooseBlockers {
        obeying, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("priest_attacks stops on exactly this")
    };
    assert!(
        lk::may_block(&blockers, heat, priest) && lk::may_block(&blockers, giant, priest),
        "positive control: the untapped Wall and the Giant may block it: {blockers:?}"
    );
    assert!(
        !lk::may_block(&blockers, stone, priest),
        "and the tapped Wall may not: {blockers:?}"
    );
    assert_eq!(
        obeying,
        vec![(heat, priest)],
        "the one Wall that can block it is the one required block"
    );
    refused(
        engine.apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, priest)],
            },
        ),
        MUST_BLOCK,
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(heat, priest)],
            },
        )
        .expect("the one able Wall blocks and nothing else is needed");
    assert_eq!(engine.state().combat.blockers_of(priest), vec![heat]);
}

/// "Defending player" matters: at a three-player table only the Walls of the
/// player being attacked are made to block. Seat 2's Wall is not.
#[test]
fn only_the_defending_players_walls_must_block() {
    let mut engine = Duel::table(SEED, index::FOREST, 3)
        .battlefield(0, &[marble_priest()])
        .battlefield(1, &[index::WALL_OF_HEAT, index::HILL_GIANT])
        .battlefield(2, &[index::WALL_OF_STONE])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P0));
    let priest = on_battlefield(&engine, P0, marble_priest()).expect("the Priest is out");
    let heat = on_battlefield(&engine, P1, index::WALL_OF_HEAT).expect("seated");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(priest, Defender::Player(P1))],
            },
        )
        .expect("the Priest attacks seat 1");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == P1),
    );
    let Pending::ChooseBlockers { obeying, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on exactly this")
    };
    assert_eq!(
        obeying,
        vec![(heat, priest)],
        "the defending player's Wall is required, nobody else's"
    );
}

/// "Prevent all combat damage that would be dealt to this creature by Walls."
/// The Priest (3/3) attacks into a Wall of Swords (3/5, power three: lethal to
/// the Priest were it dealt); the Wall blocks, the Priest is marked
/// with nothing and lives, and the Wall takes the Priest's three. The control:
/// the same Wall blocking an ordinary 2/2 kills it, so the Wall really does hurt
/// what it fights.
#[test]
fn combat_damage_from_a_wall_to_the_priest_is_prevented() {
    let mut engine = table(&[index::WALL_OF_SWORDS]);
    let swords = on_battlefield(&engine, P1, index::WALL_OF_SWORDS).expect("seated");
    assert_eq!(pt(&engine, swords), (3, 5), "the Wall hits for three");
    let priest = priest_attacks(&mut engine);
    let (power, toughness) = pt(&engine, priest);
    assert_eq!((power, toughness), (3, 3), "the Priest is a 3/3");
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(swords, priest)],
            },
        )
        .expect("the Wall blocks the Priest");
    through_combat_damage(&mut engine);

    assert!(
        on_battlefield(&engine, P0, marble_priest()).is_some(),
        "three damage would have killed a 3/3: the Priest is alive"
    );
    assert_eq!(
        lk::damage_on(&engine, priest),
        0,
        "the Wall's three damage to the Priest is prevented"
    );
    assert_eq!(
        lk::damage_on(&engine, swords),
        3,
        "the Priest's own damage to the Wall is not"
    );

    // The control: a vanilla 2/2 attacker, same Wall, same block.
    let mut control = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::WALL_OF_SWORDS],
        &[],
        &[],
    );
    let bears = on_battlefield(&control, P0, index::GRIZZLY_BEARS).expect("seated");
    let swords = on_battlefield(&control, P1, index::WALL_OF_SWORDS).expect("seated");
    lk::attack(&mut control, &[bears]);
    lk::block(&mut control, &[(swords, bears)]);
    through_combat_damage(&mut control);
    assert!(
        in_graveyard(&control, P0, index::GRIZZLY_BEARS).is_some(),
        "an ordinary attacker is killed by the same Wall's damage"
    );
}

/// Only *Walls'* damage is prevented. A Gray Ogre (2/2) blocking the Priest
/// deals its two and the Priest is marked with them; with a Wall of Swords and
/// the Ogre both blocking (the Wall has no choice), the Priest is marked with
/// the Ogre's two and survives, where the Wall's three on top of them would
/// have killed it.
#[test]
fn combat_damage_from_a_non_wall_blocker_is_dealt() {
    // The Ogre alone: no Wall anywhere, so nothing forces anyone.
    let mut alone = table(&[index::GRAY_OGRE]);
    let ogre = on_battlefield(&alone, P1, index::GRAY_OGRE).expect("seated");
    let priest = priest_attacks(&mut alone);
    alone
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(ogre, priest)],
            },
        )
        .expect("a free block");
    through_combat_damage(&mut alone);
    assert_eq!(
        lk::damage_on(&alone, priest),
        2,
        "a non-Wall deals its full two to the Priest"
    );

    // Ogre and Wall together.
    let mut both = table(&[index::GRAY_OGRE, index::WALL_OF_SWORDS]);
    let ogre = on_battlefield(&both, P1, index::GRAY_OGRE).expect("seated");
    let swords = on_battlefield(&both, P1, index::WALL_OF_SWORDS).expect("seated");
    let priest = priest_attacks(&mut both);
    both.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(swords, priest), (ogre, priest)],
        },
    )
    .expect("the Wall (required) and the Ogre (free)");
    through_combat_damage(&mut both);
    assert!(
        on_battlefield(&both, P0, marble_priest()).is_some(),
        "the Priest lives: only the Ogre's two reached it"
    );
    assert_eq!(
        lk::damage_on(&both, priest),
        2,
        "two from the Ogre, nothing from the Wall"
    );
}
