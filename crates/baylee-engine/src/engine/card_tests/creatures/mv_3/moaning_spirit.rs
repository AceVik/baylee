//! `cards/creatures/mv_3/moaning_spirit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Moaning Spirit is `{2}{B}` for a 2/1 Spirit whose entire printed text is
/// "Flying", so the test has to play both halves of it: the body arrives off
/// three Swamps, and the keyword is read where it does something — the block
/// declaration. The grounded Elf across the table is offered against a second,
/// grounded attacker and refused against the Spirit, which is what tells a
/// flier from a creature that merely carries the word in its card file.
#[test]
fn moaning_spirit_arrives_as_a_flying_two_one_a_ground_elf_cannot_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[moaning_spirit()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // `{2}{B}` off the three Swamps. The Elf is named as the printing kept
    // back: it is the grounded attacker the block offer below is measured
    // against, and a creature tapped for mana may not attack.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Swamps, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, moaning_spirit());
    pass_until(&mut engine, stack_is_empty);

    let spirit = on_battlefield(&engine, p0, moaning_spirit()).expect("the Spirit resolved");
    assert!(
        types(&engine, spirit).contains(TypeSet::CREATURE),
        "what arrived is a creature"
    );
    assert_eq!(pt(&engine, spirit), (2, 1), "the printed 2/1 body");
    assert!(
        keywords(&engine, spirit).contains(KeywordSet::FLYING),
        "and the keyword the card prints"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{B}} came out of the pool"
    );

    // Both of p0's creatures attack — the Spirit through the air, the Elf on
    // the ground — so one defender has one legal block and one illegal one.
    // CR 302.6: a creature that arrived this turn cannot attack, so
    // the turn goes round once before the line is pressed. Both halves are
    // needed — `walk_to_own_main` on its own returns where it stands.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the turn came back round, and the summoning sickness with it"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on the attack declaration")
    };
    assert_eq!(player, p0, "the active player declares");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    assert!(
        attackers.contains(&spirit) && attackers.contains(&elves),
        "an untapped flier and an untapped 1/1 may both attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (spirit, Defender::Player(p1)),
                    (elves, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on the block declaration")
    };
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let can_block: Vec<ObjectId> = blockers
        .iter()
        .filter(|option| option.blocker == their_elf)
        .flat_map(|option| option.attackers.iter().copied())
        .collect();
    assert!(
        can_block.contains(&elves),
        "the grounded Elf is a legal block for the grounded attacker, so the \
         offer is not empty of this defender: {can_block:?}"
    );
    assert!(
        !can_block.contains(&spirit),
        "and the same Elf may not block the flier: {can_block:?}"
    );
}
