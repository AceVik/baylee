//! `cards/creatures/mv_3/bird_maiden.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bird Maiden — {2}{R} — Creature — Human Bird: 1/2 with flying.
///
/// The whole printed card is a body and one keyword, so the scenario has to
/// make both of them load-bearing. The board is built so the Maiden arrives
/// the way the card arrives — cast off three Mountains for {2}{R} and nothing
/// else — and then reads the two numbers that could be wrong: the printed 1/2
/// and the flying the keyword line grants. The control that turns "it flies"
/// into a claim about the *card* is the combat step: an untapped Llanowar
/// Elves beside the Maiden is offered as an attacker while the Maiden is held
/// back only by whatever the declaration says, so the offer is read to show
/// the 1/2 flier is a legal attacker at all and the opponent's ground creature
/// is what flying is *for* — the pair of blockers offered against an attack
/// says which attacker the ground body can block.
#[test]
fn bird_maiden_lands_as_a_flying_one_two_and_attacks_where_the_ground_cannot_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), quiet_creature()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[bird_maiden()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{R} off the three Mountains, with the Elf named as the printing kept
    // back: it is the creature the attack declaration below is read against,
    // and a mana creature tapped for the cost could not attack.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains, three red — the {{2}}{{R}} is paid below, not here"
    );
    cast_with_floating(&mut engine, p0, bird_maiden());
    pass_until(&mut engine, stack_is_empty);

    let maiden = on_battlefield(&engine, p0, bird_maiden()).expect("the Maiden resolved");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    let their_elf = on_battlefield(&engine, p1, quiet_creature()).expect("an Elf across the table");
    assert_eq!(pt(&engine, maiden), (1, 2), "the printed 1/2 body");
    assert!(
        keywords(&engine, maiden).contains(KeywordSet::FLYING),
        "the keyword line reaches the permanent"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FLYING),
        "and the keyword is on the Maiden and not on the board"
    );
    assert!(
        types(&engine, maiden).contains(TypeSet::CREATURE),
        "and it is a creature: {:#?}",
        types(&engine, maiden)
    );

    // The attack declaration is where the keyword is worth anything: the
    // Maiden is a legal attacker, and the only creature that may block it is
    // one that flies — which the Elf across the table does not.
    // CR 302.6: a creature that arrived this turn cannot attack, so the turn
    // goes round once first. Both halves are needed — `walk_to_own_main` on
    // its own returns where it stands, because this already *is* p0's own
    // main phase.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the turn came back round, and the summoning sickness with it"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on the attack declaration")
    };
    assert!(
        attackers.contains(&maiden),
        "an untapped 1/2 with flying is a legal attacker: {attackers:?}"
    );
    assert!(
        attackers.contains(&elf),
        "and so is the ground Elf beside it: {attackers:?}"
    );
    assert!(
        defenders.contains(&Defender::Player(p1)),
        "the opponent is what may be attacked: {defenders:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(maiden, Defender::Player(p1))],
            },
        )
        .unwrap();

    // The defending seat is offered its blockers, and the pairing is the
    // reading: the ground Elf across the table is not a legal blocker for a
    // flying attacker (CR 509.1b).
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on the block declaration")
    };
    // The Elf is not on the menu *at all*, which is the same sentence read
    // one level up: `progress` keeps a `BlockOption` only while it has an
    // attacker it could legally be paired with, so a ground creature facing
    // nothing but fliers is omitted rather than offered with an empty list.
    // Asserting "it is there and cannot block the Maiden" would have been
    // asserting a shape this engine never builds.
    assert!(
        !blockers.iter().any(|option| option.blocker == their_elf),
        "the ground Elf may not block a flier (CR 509.1b): {blockers:?}"
    );
    assert!(
        blockers.is_empty(),
        "and it is the only creature that seat has: {blockers:?}"
    );

    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        19,
        "the unblocked 1/2 dealt its one damage to the player it attacked"
    );
}
