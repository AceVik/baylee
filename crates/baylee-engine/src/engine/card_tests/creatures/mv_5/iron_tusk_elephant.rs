//! `cards/creatures/mv_5/iron_tusk_elephant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Iron Tusk Elephant is a vanilla {4}{W} 3/3 whose entire printed text is
/// "Trample", and trample is only a rule where damage is assigned: a 3/3
/// blocked by a 1/1 must kill the blocker with one damage and let the other
/// two through to the defending player (CR 702.19b). Reading the card file
/// cannot tell a keyword that was merely listed from one the combat step
/// actually reads, so the scenario attacks with the Elephant, blocks it with a
/// real 1/1 read off the engine's own block offer, and asserts both the dead
/// Elf and the two life the block did not absorb.
#[test]
fn iron_tusk_elephant_tramples_the_excess_damage_over_whatever_blocks_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[iron_tusk_elephant()])
        .battlefield(1, &[llanowar_elves()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elephant =
        on_battlefield(&engine, p0, iron_tusk_elephant()).expect("the Elephant is on the table");
    let blocker = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is on the table");
    assert_eq!(pt(&engine, elephant), (3, 3), "the body the card prints");
    assert!(
        keywords(&engine, elephant).contains(KeywordSet::TRAMPLE),
        "the printed Trample reaches the permanent"
    );
    assert_eq!(
        pt(&engine, blocker),
        (1, 1),
        "one toughness for a printed 3/3 to run over"
    );

    // The attack declaration is the only place the engine lists what may
    // attack and what may block what, so the pairing used below is read out of
    // an offer the engine itself published rather than derived from the board.
    let offered = attack_and_collect_blocks(&mut engine, elephant, p1);
    let pairing = offered
        .iter()
        .find(|o| o.blocker == blocker)
        .unwrap_or_else(|| panic!("the Elf may block the Elephant: {offered:?}"));
    assert!(
        pairing.attackers.contains(&elephant),
        "and the Elephant is one of the attackers it may be assigned to: {pairing:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, elephant)],
            },
        )
        .expect("the pairing came out of the list that offered it");

    // Not `stack_is_empty`: the stack is already empty the moment blockers are
    // declared, which is *before* the combat damage step (CR 510.2), so that
    // predicate would stop the walk with every life total still at twenty. The
    // end step is past the damage.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "lethal damage to a 1/1 puts it in its owner's graveyard (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[1].life,
        18,
        "three damage: one assigned to the blocker and the other two trampling \
         over it (CR 702.19b). Twenty here would mean the keyword was listed on \
         the card and never read by the damage step"
    );
    assert!(
        on_battlefield(&engine, p0, iron_tusk_elephant()).is_some(),
        "a 1/1's damage is not lethal to a 3/3, so the Elephant is still standing"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing in the card touches its controller's life"
    );
}
