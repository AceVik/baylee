//! `cards/artifacts/mv_3/thunderstaff.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn thunderstaff() -> CardIndex {
    card_index("1a88a378-051c-42b4-bd0e-9c40ae8efea6")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn attack_p0_unblocked(engine: &mut Engine<RegistryLookup>, attackers: &[ObjectId]) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|&id| (id, Defender::Player(p0)))
                    .collect(),
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
}

/// Thunderstaff (1a88a378-051c-42b4-bd0e-9c40ae8efea6) — {3} Artifact:
/// "As long as this artifact is untapped, if a creature would deal combat
///  damage to you, prevent 1 of that damage."
/// "{2}, {T}: Attacking creatures get +1/+0 until end of turn."
///
/// While untapped, prevents 1 combat damage per attacking creature to you
/// (two 2/2s deal 1 each = 2), does NOT prevent noncombat Bolt damage (full 3),
/// and once tapped via `{2}, {T}`, stops preventing combat damage while pumping
/// attacking creatures +1/+0 (sparing non-attacking creatures).
#[test]
fn thunderstaff_prevents_combat_damage_while_untapped_and_pumps_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[thunderstaff(), grizzly_bears(), forest(), forest()])
        .battlefield(1, &[grizzly_bears(), grizzly_bears(), mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);

    // 1. Opponent's turn while Thunderstaff is UNTAPPED: Bolt deals full 3 (20 -> 17).
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, p0),
        17,
        "Thunderstaff prevents combat damage from creatures, not Lightning Bolt"
    );

    // Two 2/2 Bears attack p0 unblocked -> each deals 2 - 1 = 1 damage (17 -> 15).
    let their_bears = all_on_battlefield(&engine, p1, grizzly_bears());
    attack_p0_unblocked(&mut engine, &their_bears);
    assert_eq!(
        life_of(&engine, p0),
        15,
        "untapped Thunderstaff prevented 1 combat damage from each 2/2 (17 -> 15)"
    );

    // 2. p0's turn: declare own Bear attacking, then activate Thunderstaff ({2}, {T}).
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    let my_bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(my_bear, Defender::Player(p1))],
            },
        )
        .unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, thunderstaff(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, my_bear),
        (3, 2),
        "attacking creature gets +1/+0"
    );
    assert_eq!(
        pt(&engine, their_bears[0]),
        (2, 2),
        "non-attacking untouched"
    );

    // 3. On p1's next turn, Thunderstaff is still TAPPED: 2/2 deals full 2 damage (15 -> 13).
    let staff = on_battlefield(&engine, p0, thunderstaff()).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        is_tapped(&engine, staff),
        "Thunderstaff is still tapped on p1's turn"
    );
    attack_p0_unblocked(&mut engine, &[their_bears[0]]);
    assert_eq!(
        life_of(&engine, p0),
        13,
        "while tapped, Thunderstaff prevents 0 combat damage (15 -> 13)"
    );
}
