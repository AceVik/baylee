//! `cards/creatures/mv_7/djinn_of_the_lamp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Djinn of the Lamp is `{5}{U}{U}` for a 5/6 Djinn whose only text is flying,
/// so the whole card is one body and one keyword and neither is visible in the
/// card file. The body is read on a Djinn the seven Islands actually paid for,
/// and flying is played rather than looked up: the same defending Elf that the
/// engine offers as a blocker for the ground attacker beside the Djinn is
/// withheld from the flier, so the published pairing — and not the absence of a
/// blocker — is what says the keyword is on the permanent.
#[test]
#[allow(clippy::too_many_lines)]
fn djinn_of_the_lamp_arrives_as_a_five_six_flier_a_ground_creature_cannot_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[djinn_of_the_lamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Seven Islands are exactly `{5}{U}{U}`, and the Elf is named as the
    // printing kept back: it is the ground attacker the blockers are read
    // against below, and a creature tapped for mana may not attack.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "seven Islands tapped for seven blue, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, djinn_of_the_lamp());
    pass_until(&mut engine, stack_is_empty);

    let djinn = on_battlefield(&engine, p0, djinn_of_the_lamp()).expect("the Djinn resolved");
    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    assert_eq!(
        pt(&engine, djinn),
        (5, 6),
        "the 5/6 body the card prints, off the mana the Islands really made"
    );
    assert!(
        keywords(&engine, djinn).contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent, so it is a keyword and not \
         a line of text in a file"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the seven blue were the whole price: nothing is left floating"
    );

    // Summoning sickness (CR 302.6) keeps the Djinn out of the combat of the
    // turn it arrived in, so the attack waits for its controller's next one.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the attack declaration")
    };
    assert_eq!(player, p0, "the active seat declares its attackers");
    assert!(
        attackers.contains(&djinn) && attackers.contains(&mine),
        "both are untapped creatures past their first turn: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(djinn, Defender::Player(p1)), (mine, Defender::Player(p1))],
            },
        )
        .expect("both attackers came out of the list that offered them");

    // The offer is a pairing, not two flat lists, so one defending creature
    // can be read against two attackers at once — which is the only way the
    // keyword shows up as a *difference* rather than as an empty list.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on nothing else")
    };
    assert_eq!(player, p1, "the defending seat is the one asked");
    assert_eq!(blockers.len(), 1, "and it has one creature to block with");
    let offer = &blockers[0];
    assert_eq!(offer.blocker, theirs, "the Elf across the table");
    assert!(
        offer.attackers.contains(&mine),
        "the ground Elf beside the Djinn is a creature it may block: {:?}",
        offer.attackers
    );
    assert!(
        !offer.attackers.contains(&djinn),
        "\"Flying\" is read when the pairings are published: the same Elf is \
         withheld from the flier: {:?}",
        offer.attackers
    );

    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("declaring no blockers is always legal");

    // Not `stack_is_empty`: that is already true the moment attackers are
    // declared, so it would stop the walk before combat damage (CR 510.2).
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        14,
        "5 from the unblockable flier and 1 from the Elf that could have been \
         blocked, so the Djinn's damage is the printed power and not a label"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the defending seat dealt nothing back, because it declared no \
         blocks and attacked on nobody's turn"
    );
    assert!(
        on_battlefield(&engine, p0, djinn_of_the_lamp()).is_some(),
        "the Djinn outlived the combat it was unblockable in"
    );
}
