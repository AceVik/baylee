//! Dead-Iron Sledge — `{1}` Artifact — Equipment (MRD).
//! Oracle: "Whenever equipped creature blocks or becomes blocked by a creature, destroy both creatures. Equip {2}"

use super::*;

fn dead_iron_sledge() -> CardIndex {
    card_index("e1ee79de-e4c3-48f9-87a7-18f72e2f6116")
}

#[test]
fn dead_iron_sledge_destroys_both_creatures_when_equipped_creature_becomes_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let sledge = dead_iron_sledge();
    let elf = llanowar_elves();
    let hunter = fangren_hunter();
    let mut engine = rig(2, &[], &[sledge, elf], &[hunter]);
    let host = on_battlefield(&engine, p0, elf).unwrap();
    let blocker = on_battlefield(&engine, p1, hunter).unwrap();

    equip(&mut engine, sledge, 2, host);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(host, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    block_and_finish(&mut engine, vec![(blocker, host)]);

    assert!(on_battlefield(&engine, p0, elf).is_none());
    assert!(in_graveyard(&engine, p0, elf).is_some());
    assert!(on_battlefield(&engine, p1, hunter).is_none());
    assert!(in_graveyard(&engine, p1, hunter).is_some());
}

#[test]
fn dead_iron_sledge_destroys_both_creatures_when_equipped_creature_blocks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let sledge = dead_iron_sledge();
    let elf = llanowar_elves();
    let hunter = fangren_hunter();
    let mut engine = rig(2, &[], &[sledge, elf], &[hunter]);
    let host = on_battlefield(&engine, p0, elf).unwrap();
    let attacker = on_battlefield(&engine, p1, hunter).unwrap();

    equip(&mut engine, sledge, 2, host);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(host, attacker)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(on_battlefield(&engine, p0, elf).is_none());
    assert!(in_graveyard(&engine, p0, elf).is_some());
    assert!(on_battlefield(&engine, p1, hunter).is_none());
    assert!(in_graveyard(&engine, p1, hunter).is_some());
}
