//! `cards/creatures/mv_4/snapping_drake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "e15060c3-3773-4548-8747-ff59dcf2b519"

/// Snapping Drake is `{3}{U}` for a 3/2 Drake whose entire printed text is
/// flying, so the card is only itself when both halves are played: the cast
/// that spends four Islands down to an empty pool, and the combat step the
/// keyword is about. The other side of the table holds a Drake and a ground
/// creature, and that pair is the reading — a block list taken off a lone
/// flier would pass whether flying existed or not, while the ground creature's
/// absence from the same pairing is the rule itself (CR 702.9b). The attack
/// waits a turn, because the Drake arrived this turn and a creature with
/// summoning sickness may not attack (CR 302.6).
#[test]
fn snapping_drake_flies_over_the_ground_and_is_blocked_only_by_another_flier() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[snapping_drake()])
        // One creature that flies and one that does not, so the published
        // pairing says which of the two the keyword reaches.
        .battlefield(1, &[snapping_drake(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, snapping_drake());
    pass_until(&mut engine, stack_is_empty);
    let drake = on_battlefield(&engine, p0, snapping_drake()).expect("the Drake resolved");
    let flier = on_battlefield(&engine, p1, snapping_drake()).expect("their Drake is out");
    let ground = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    assert_eq!(pt(&engine, drake), (3, 2), "the body the card prints");
    assert!(
        types(&engine, drake).contains(TypeSet::CREATURE),
        "a 3/2 creature and not a spell left sitting on the stack"
    );
    assert!(
        keywords(&engine, drake).contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent through the layers"
    );
    assert!(
        keywords(&engine, flier).contains(KeywordSet::FLYING),
        "and the control across the table is a flier, or the block list below \
         would say nothing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "four Islands paid {{3}}{{U}} to the last mana"
    );

    // A creature that arrived this turn is summoning sick (CR 302.6), so the
    // attack has to wait for its controller's own next main phase.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, drake),
        "the untap step stood the Drake back up"
    );

    // What flying *does* is a pairing the engine enumerates and validates
    // against, so the offer is the only place the rule can be read.
    let blocks = attack_and_collect_blocks(&mut engine, drake, p1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == flier && b.attackers.contains(&drake)),
        "a creature with flying may block a creature with flying: {blocks:?}"
    );
    assert!(
        !blocks
            .iter()
            .any(|b| b.blocker == ground && b.attackers.contains(&drake)),
        "CR 702.9b: a creature with neither flying nor reach may not block the \
         Drake, so it is nowhere in the pairing: {blocks:?}"
    );
}
