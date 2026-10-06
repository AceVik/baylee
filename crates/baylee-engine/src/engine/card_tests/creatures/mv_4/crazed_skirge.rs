//! `cards/creatures/mv_4/crazed_skirge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crazed Skirge prints two words — flying and haste — and one combat step
/// reads both, because each is what the other cannot be mistaken for. Haste is
/// only visible on the turn the creature arrives, so it is cast and declared as
/// an attacker in the same main phase, and the four Swamps that pay its
/// `{3}{B}` are spent down to an empty pool so the cast is a real payment.
/// Flying is the block question: the opponent's ground Elf is as much an
/// untapped creature as the Sphinx beside it and must stay off the offer, which
/// is the half a merely non-empty block list would not tell apart.
#[test]
fn crazed_skirge_attacks_the_turn_it_arrives_and_only_a_flier_may_block_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[crazed_skirge()])
        // A ground creature that may not block a flier, and a flier that may:
        // both are seated rather than cast, because the combat step is the
        // subject here and neither is the card under test.
        .battlefield(1, &[llanowar_elves(), sphinx_of_the_final_word()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Swamps are exactly `{3}{B}`, and the pool is read where the engine
    // reads it: before the claim, and empty again once the Skirge is paid for.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Swamps tapped, four black, and nothing else on this board makes mana"
    );
    cast_with_floating(&mut engine, p0, crazed_skirge());
    pass_until(&mut engine, stack_is_empty);
    let skirge = on_battlefield(&engine, p0, crazed_skirge()).expect("the Skirge resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}}{{B}} came out of the pool"
    );
    assert_eq!(pt(&engine, skirge), (2, 2), "the body the card prints");
    let granted = keywords(&engine, skirge);
    assert!(
        granted.contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent"
    );
    assert!(
        granted.contains(KeywordSet::HASTE),
        "and the printed haste beside it"
    );

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let sphinx =
        on_battlefield(&engine, p1, sphinx_of_the_final_word()).expect("their flier is out");

    // Haste is the attack declaration itself: `attack_and_collect_blocks`
    // asserts that the creature it names came out of `ChooseAttackers`, and a
    // creature that entered this very turn is in that list only because the
    // card says it may attack the turn it comes down.
    let blocks = attack_and_collect_blocks(&mut engine, skirge, p1);
    assert!(
        blocks
            .iter()
            .any(|o| o.blocker == sphinx && o.attackers.contains(&skirge)),
        "a flier may block a flier: {blocks:?}"
    );
    assert!(
        !blocks.iter().any(|o| o.blocker == elf),
        "the ground Elf is untapped, is a creature, and is no blocker for a \
         flier: {blocks:?}"
    );

    // No blocks, so the 2/2 connects: the life total is the proof that the
    // attack was declared and not merely offered.
    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        panic!(
            "expected the declare-blockers question, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("declaring no blockers is always legal");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        18,
        "an unblocked 2/2 takes two life off the seat it attacked"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the life belongs to the defender, not to the attacker"
    );
    assert!(
        on_battlefield(&engine, p0, crazed_skirge()).is_some(),
        "nothing blocked it, so the Skirge is still on the battlefield"
    );
}
