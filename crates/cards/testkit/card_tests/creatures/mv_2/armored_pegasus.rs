//! `cards/creatures/mv_2/armored_pegasus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Armored Pegasus — {1}{W} — Creature — Pegasus, 1/2, flying: that keyword
/// is the whole of its rules text, so the card is only itself when the body
/// and the evasion both land. Casting it off two Plains proves the cost and
/// the arrival; flying is read off the *projected* characteristics, because a
/// `KeywordSet` in the card def is not something a permanent has; and since
/// flying restricts blockers rather than attackers, it is proved where it
/// means something — in the block declaration, where the defending seat's Elf
/// is offered as the blocker of p0's ground Elf on the same prompt and is not
/// offered against the Pegasus.
#[test]
fn armored_pegasus_arrives_as_a_one_two_flier_a_ground_blocker_may_not_touch() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        // Two Plains pay {1}{W}; the Elf beside them is the *ground* attacker
        // the same blocker is offered against below, which is what keeps "not
        // offered" from being an empty offer.
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .hand(0, &[armored_pegasus()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, armored_pegasus());
    pass_until(&mut engine, stack_is_empty);
    let pegasus = on_battlefield(&engine, p0, armored_pegasus()).expect("the Pegasus resolved");
    assert_eq!(pt(&engine, pegasus), (1, 2), "the body the card prints");
    assert!(
        keywords(&engine, pegasus).contains(KeywordSet::FLYING),
        "and the layer projection is where its one line of rules text lives"
    );

    // Summoning sickness (CR 302.6) keeps it home on the turn it arrived, so
    // the attack step that reads the evasion is its controller's next one.
    // `answer_one` walks there, answering whatever the turn in between asks.
    let mut at_its_attack_step = false;
    for _ in 0..400 {
        if matches!(
            engine.pending(),
            Pending::ChooseAttackers { player, attackers, .. }
                if *player == p0 && attackers.contains(&pegasus)
        ) {
            at_its_attack_step = true;
            break;
        }
        let (player, action) = answer_one(&engine)
            .unwrap_or_else(|rest| panic!("the walk to p0's next attack step hit {rest:?}"));
        if let Err(err) = engine.apply(player, action.clone()) {
            panic!("the walk was refused {action:?}: {err:?}");
        }
    }
    assert!(
        at_its_attack_step,
        "the Pegasus is offered as an attacker once it is no longer summoning sick: {:?}",
        engine.pending()
    );

    let ground = on_battlefield(&engine, p0, llanowar_elves()).expect("p0's Elf is still out");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (pegasus, Defender::Player(p1)),
                    (ground, Defender::Player(p1)),
                ],
            },
        )
        .expect("both are untapped creatures past their first turn");

    // The blocker menu is not the next thing the engine says: CR 508.2 hands
    // priority round once the attack is declared, so the walk has to answer
    // that before the question this test is about is asked.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the seat being attacked declares blockers: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the defending seat is the one asked");
    let blockers_for = |attacker: ObjectId| -> Vec<ObjectId> {
        blockers
            .iter()
            .filter(|option| option.attackers.contains(&attacker))
            .map(|option| option.blocker)
            .collect()
    };
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        blockers_for(ground),
        vec![their_elf],
        "a ground 1/1 may block a ground 1/1, so blocking is on the table: {blockers:?}"
    );
    assert!(
        blockers_for(pegasus).is_empty(),
        "and the same Elf may not block a flier (CR 702.9b): {blockers:?}"
    );

    // Nobody blocks, so both attackers connect: 1 from the Pegasus and 1 from
    // the Elf nothing declined to block.
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        18,
        "the flier went over the blocker, and the Elf was let through by choice"
    );
}
