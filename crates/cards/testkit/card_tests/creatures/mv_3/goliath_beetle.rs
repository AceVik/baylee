//! `cards/creatures/mv_3/goliath_beetle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goliath Beetle is `{2}{G}` for a 3/1 Insect whose entire printed text is
/// trample, and trample is a claim about damage assignment rather than a
/// label: a 3/1 blocked by a 1/1 spends one damage on the blocker and has two
/// left over for the defending player. So the Beetle is cast off three
/// Forests, sits out its own summoning sickness (CR 302.6 — a 3/1 that cannot
/// attack on the turn it arrives is a body, not a card), and swings into the
/// Elf across the table a turn later. `(3, 1)` read off the layers before and
/// after is what says the printed numbers are the projected ones, and the
/// Elf's graveyard entry beside p1's missing two life is the half that
/// separates trample from a plain 3/1: a blocked attacker that kept all of
/// its damage would leave that life total alone.
#[test]
fn goliath_beetle_tramples_two_damage_past_a_one_one_blocker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[goliath_beetle()])
        .battlefield(1, &[llanowar_elves()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{G} off the three Forests, which are every mana source on this board.
    cast_from_hand(&mut engine, p0, goliath_beetle());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, goliath_beetle()).is_some()
    });
    let beetle = on_battlefield(&engine, p0, goliath_beetle()).expect("the Beetle resolved");
    let elf =
        on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf stands across the table");
    assert_eq!(
        pt(&engine, beetle),
        (3, 1),
        "the printed 3/1, read off the layer system"
    );
    assert!(
        keywords(&engine, beetle).contains(KeywordSet::TRAMPLE),
        "the one word the card prints reaches the permanent"
    );

    // CR 302.6: the Beetle arrived this turn and may not attack, so a whole
    // turn cycle is what makes the swing below legal at all.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, beetle),
        (3, 1),
        "still a 3/1 a turn later, so nothing transient was what was read"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        attackers.contains(&beetle),
        "an untapped, unsick attacker is offered: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(beetle, Defender::Player(p1))],
            },
        )
        .unwrap();

    // One blocker, and whether it may block which attacker is a pairing the
    // engine enumerates rather than a fact to assume (CR 509.1a).
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p1, "the defending seat declares the blockers");
    assert!(
        blockers
            .iter()
            .any(|o| o.blocker == elf && o.attackers.contains(&beetle)),
        "the Elf may block the Beetle: {blockers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, beetle)],
            },
        )
        .unwrap();

    // Past the combat damage step (CR 510.2) and on to the end of the turn:
    // damage is dealt as the step turns, so a walk that stopped at the first
    // quiet priority would still read the life total twenty.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        18,
        "one damage is lethal to a 1/1 and the other two trample over — a \
         blocked attacker that kept its damage would leave this at twenty"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the blocker did not survive the damage that was assigned to it"
    );
    assert!(
        in_graveyard(&engine, p0, goliath_beetle()).is_some(),
        "and the Elf's one point of damage is lethal to a 3/1, so the \
         toughness the card prints is real on both sides of the block"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the damage that trampled over went to the defending player and never \
         to the controller of the attacker"
    );
}

/// The other half of the bound: over a library of Goliath Beetles (power 3)
/// the Recruiter's search has nothing to offer, asks nothing, and every card
/// stays where it was.
#[test]
fn imperial_recruiter_finds_nothing_among_creatures_of_power_three() {
    let p0 = PlayerId::new(0);
    let goliath_beetle = card_index("86ab4400-fbdd-4c18-a893-441286a9d7d0");
    let mut engine = Duel::new(SEED, goliath_beetle)
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[imperial_recruiter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, imperial_recruiter());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, imperial_recruiter()).is_some());
    assert_eq!(library_size(&engine, p0), library_before);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the Recruiter left the hand and nothing came to it"
    );
}
