//! `cards/creatures/mv_3/raging_gorilla.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::MONS_S_GOBLIN_RAIDERS;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn raging_gorilla() -> CardIndex {
    card_index("1dd6df3f-eac4-4f30-b832-4a3f3a97632f")
}

/// P0 with a resolved Gorilla and `goblins` Goblin Raiders on P1's side, at the
/// end of P0's first main phase.
fn board(goblins: usize) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &vec![MONS_S_GOBLIN_RAIDERS; goblins])
        .hand(0, &[raging_gorilla()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, raging_gorilla());
    pass_until(&mut engine, stack_is_empty);
    let gorilla = on_battlefield(&engine, P0, raging_gorilla()).unwrap();
    assert_eq!(pt(&engine, gorilla), (2, 3));
    (engine, gorilla)
}

/// `seat` declares `attacker` alone against `defender` and the defender is
/// asked to block.
fn attack(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    attacker: ObjectId,
    defender: PlayerId,
) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat),
    );
    engine
        .apply(
            seat,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(defender))],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
}

/// `Raging Gorilla` ({2}{R}, 2/3 Creature — Ape, `Coverage::Implemented`):
/// `Whenever this creature blocks or becomes blocked, it gets +2/-2 until end
/// of turn.` Attacking and blocked, it becomes 4/1.
#[test]
fn raging_gorilla_gets_plus_2_minus_2_when_blocked() {
    let (mut engine, gorilla) = board(1);
    let goblin = on_battlefield(&engine, P1, MONS_S_GOBLIN_RAIDERS).unwrap();

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    attack(&mut engine, P0, gorilla, P1);
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(goblin, gorilla)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, gorilla), (4, 1));
}

/// "Becomes blocked" is once, however many creatures block it: two Goblins
/// give one +2/-2 (a second would have killed it).
#[test]
fn raging_gorilla_gets_it_once_when_blocked_by_two_creatures() {
    let (mut engine, gorilla) = board(2);
    let goblins = all_on_battlefield(&engine, P1, MONS_S_GOBLIN_RAIDERS);
    assert_eq!(goblins.len(), 2);

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    attack(&mut engine, P0, gorilla, P1);
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(goblins[0], gorilla), (goblins[1], gorilla)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, gorilla), (4, 1));
}

/// "Blocks": when the Gorilla is the blocker it gets +2/-2 too.
#[test]
fn raging_gorilla_gets_plus_2_minus_2_when_it_blocks() {
    let (mut engine, gorilla) = board(1);
    let goblin = on_battlefield(&engine, P1, MONS_S_GOBLIN_RAIDERS).unwrap();

    attack(&mut engine, P1, goblin, P0);
    engine
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(gorilla, goblin)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, gorilla), (4, 1));
}

/// Neither blocking nor blocked: no trigger, and the Gorilla keeps 2/3 through
/// combat.
#[test]
fn raging_gorilla_stays_two_three_when_nothing_blocks_it() {
    let (mut engine, gorilla) = board(1);

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    attack(&mut engine, P0, gorilla, P1);
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, gorilla), (2, 3));
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "it hit for its printed 2"
    );
}
