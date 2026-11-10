//! `cards/creatures/mv_3/keen_eyed_archers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Keen-Eyed Archers — {2}{W}, a 2/2 Elf Archer whose whole printed text is
/// reach: "This creature can block creatures with flying."
///
/// Reach is neither a body nor a pump, so the one place it can be read is the
/// pairing `Pending::ChooseBlockers` publishes. p1 attacks with a flier and
/// with a ground 1/1, and beside the Archer stands an untapped Elf of p0's
/// own: the Archer is paired with both attackers while the Elf is paired with
/// the ground one alone — same table, same flier, and the keyword is the only
/// difference. The blocks are then really declared and the attack is stopped,
/// so the offer was a permission the board acted on and not merely a list.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn keen_eyed_archers_blocks_the_flier_where_the_plain_elf_beside_it_cannot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), llanowar_elves()])
        .battlefield(1, &[sphinx_of_the_final_word(), llanowar_elves()])
        .hand(0, &[keen_eyed_archers()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    // Three Plains pay {2}{W}. The Elf is named as the one source kept back:
    // it is the untapped bystander the block offer below is read against, and
    // a creature tapped for mana has a status that changed for its own reason.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, keen_eyed_archers());
    pass_until(&mut engine, stack_is_empty);

    let archers = on_battlefield(&engine, p0, keen_eyed_archers()).expect("the Archers resolved");
    let bystander =
        on_battlefield(&engine, p0, llanowar_elves()).expect("the kept Elf is still standing");
    assert_eq!(pt(&engine, archers), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, archers).contains(KeywordSet::REACH),
        "reach is printed on the card and reached the characteristics through the layers"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::REACH),
        "and the Elf beside it prints none, which is what makes the pairing below a filter"
    );

    // Across the table and into p1's combat. The walk uses the kit's own
    // answerer rather than `pass_until`, because a turn boundary is crossed
    // here: a seat over its hand size is asked to discard on the way, and
    // `answer_one` answers that question too.
    let mut asked_for_attackers = false;
    for _ in 0..600 {
        if matches!(engine.pending(), Pending::ChooseAttackers { player, .. } if *player == p1) {
            asked_for_attackers = true;
            break;
        }
        let (player, action) = answer_one(&engine)
            .expect("a turn cycle on this board asks only questions this kit can answer");
        engine
            .apply(player, action)
            .expect("the answer came out of the question");
    }
    assert!(
        asked_for_attackers,
        "p1's combat step arrives and asks for attackers"
    );

    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the walk stopped on the attack declaration")
    };
    let flier =
        on_battlefield(&engine, p1, sphinx_of_the_final_word()).expect("the flier is on the table");
    let ground = on_battlefield(&engine, p1, llanowar_elves()).expect("their ground 1/1 is out");
    assert!(
        attackers.contains(&flier) && attackers.contains(&ground),
        "both of p1's untapped creatures may attack: {attackers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (flier, Defender::Player(p0)),
                    (ground, Defender::Player(p0)),
                ],
            },
        )
        .expect("both were enumerated as attackers");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the block declaration")
    };
    assert_eq!(player, p0, "the attacked seat is the one asked to block");

    let archer_pairing = blockers
        .iter()
        .find(|o| o.blocker == archers)
        .expect("an untapped creature with reach is a blocker, so the Archer is on the menu");
    assert!(
        archer_pairing.attackers.contains(&flier),
        "reach: the Archer can block the creature with flying: {:?}",
        archer_pairing.attackers
    );
    assert!(
        archer_pairing.attackers.contains(&ground),
        "and the ground attacker is blockable by it too, so the flight rule is \
         read per pairing and not as a filter over the whole list: {:?}",
        archer_pairing.attackers
    );

    let elf_pairing = blockers
        .iter()
        .find(|o| o.blocker == bystander)
        .expect("the Elf beside it may block the ground attacker, so it is on the menu too");
    assert!(
        elf_pairing.attackers.contains(&ground),
        "an untapped 1/1 with no evasion of its own may block a ground 1/1: {:?}",
        elf_pairing.attackers
    );
    assert!(
        !elf_pairing.attackers.contains(&flier),
        "and it may not block the flier — no reach, no flying, no pairing: {:?}",
        elf_pairing.attackers
    );

    // The offer is a permission and this is the act: both attackers are
    // blocked, so no combat damage reaches the player (CR 509.1, CR 510.1).
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(archers, flier), (bystander, ground)],
            },
        )
        .expect("both pairings were enumerated");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[0].life,
        20,
        "every attacker was blocked, so the reach pairing resolved into a real \
         block and nothing got through"
    );
}
