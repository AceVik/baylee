//! `cards/creatures/mv_4/norwood_archers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "4e4e96aa-2f05-4f1d-96f8-6c42cd3be589"

/// Norwood Archers — {3}{G} for a 3/3 Elf Archer whose entire printed text is
/// reach. Nothing about the card is readable anywhere but in the combat step,
/// so it is cast for real off four Forests and then a Sphinx of the Final Word
/// attacks across the table with a Festering Goblin standing beside the
/// Archers under the same seat. The declare-blockers offer is the only place
/// the word means anything: the Archers has to be paired with the flier and
/// the Goblin must not be — both are untapped, unsick and on the battlefield
/// together, so nothing but the keyword tells them apart.
#[test]
fn norwood_archers_reach_lets_it_block_a_flier_the_goblin_beside_it_cannot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), festering_goblin()],
        )
        .hand(0, &[norwood_archers()])
        .battlefield(1, &[sphinx_of_the_final_word()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {3}{G} out of the four Forests, which leaves the pool empty and the
    // Goblin standing: it is the control the block offer below is read against.
    cast_from_hand(&mut engine, p0, norwood_archers());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, norwood_archers()).is_some()
    });

    let archers = on_battlefield(&engine, p0, norwood_archers()).expect("the Archers resolved");
    let goblin = on_battlefield(&engine, p0, festering_goblin()).expect("the Goblin is out");
    let flier = on_battlefield(&engine, p1, sphinx_of_the_final_word()).expect("the flier is out");
    assert_eq!(pt(&engine, archers), (3, 3), "the body the card prints");
    assert!(
        keywords(&engine, archers).contains(KeywordSet::REACH),
        "the printed reach reaches the permanent",
    );
    assert!(
        keywords(&engine, flier).contains(KeywordSet::FLYING),
        "and the attacker this is about really does fly",
    );
    assert!(
        !keywords(&engine, goblin).contains(KeywordSet::REACH),
        "while the creature beside it has no reach to block with",
    );

    // Across the opponent's turn and into their combat. The Archers blocking
    // here is not a summoning-sickness violation: CR 302.6 restricts attacking
    // and {T} costs, and a blocker is neither.
    reach_their_main_phase(&mut engine, p1);
    let blocks = attack_and_collect_blocks(&mut engine, flier, p0);

    let paired_with_the_flier: Vec<ObjectId> = blocks
        .iter()
        .filter(|b| b.attackers.contains(&flier))
        .map(|b| b.blocker)
        .collect();
    assert!(
        paired_with_the_flier.contains(&archers),
        "\"Reach (This creature can block creatures with flying.)\" — the \
         offer pairs it with the flier: {blocks:?}",
    );
    assert!(
        !paired_with_the_flier.contains(&goblin),
        "the Goblin is untapped, unsick and on the same board, and still no \
         legal blocker for a flier: {blocks:?}",
    );
}
