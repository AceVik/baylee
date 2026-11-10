//! `cards/creatures/mv_2/storm_crow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Storm Crow is `{1}{U}` for a 1/2 Bird with flying, and "flying" is
/// not a line one can read off the card file, but a block rule
/// (CR 702.9b): a creature without flying or reach cannot block it.
/// The scenario therefore puts **both** kinds on the opposite side —
/// a Llanowar Elves as ground control and a second Storm Crow as flyer —,
/// so that the absence of the Elves in the block offer says something
/// about `flying` and not about an empty board.
/// Body and keyword are read on the same creature that was cast;
/// the attack itself waits for the untap step, because a creature cast
/// just now is summoning sick (CR 302.6).
#[test]
fn storm_crow_flies_over_ground_creatures_and_only_a_flier_may_block_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4242, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[storm_crow()])
        .battlefield(1, &[llanowar_elves(), storm_crow()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, storm_crow());
    pass_until(&mut engine, stack_is_empty);
    let crow = on_battlefield(&engine, p0, storm_crow()).expect("der Crow ist gelandet");
    assert_eq!(pt(&engine, crow), (1, 2), "die gedruckten Zahlen");
    assert!(
        keywords(&engine, crow).contains(KeywordSet::FLYING),
        "flying is on the creature after the layers have run"
    );

    let their_crow = on_battlefield(&engine, p1, storm_crow()).expect("der zweite Crow steht");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("der Elf steht");

    // Summoning sickness: the just-cast Crow may only attack on the next
    // own turn.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(crow, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until hält bei der Blockdeklaration")
    };
    let offered: Vec<ObjectId> = blockers
        .iter()
        .filter(|b| b.attackers.contains(&crow))
        .map(|b| b.blocker)
        .collect();
    assert!(
        offered.contains(&their_crow),
        "ein Flieger darf den Crow blocken (CR 702.9b): {offered:?}"
    );
    assert!(
        !offered.contains(&elf),
        "der Llanowar Elves hat weder flying noch reach und ist darum kein \
         legaler Blocker: {offered:?}"
    );
}
