//! `cards/creatures/mv_3/land_leeches.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Land Leeches — {1}{G}{G}, a 2/2 Leech whose whole printed text is first
/// strike, and first strike is a claim about *when* the body deals damage and
/// not about the body. So the scenario is a trade that does not happen: a
/// Llanowar Elves under a Rancor is a 3/2 attacker — three power to kill the
/// Leech, two toughness for the Leech to kill — and simultaneous damage would
/// kill both while trampling a point through to the player. Because the Leech
/// strikes first, the Elf is dead in the first-strike combat damage step
/// before it can deal anything at all, and the Leech is still standing with no
/// damage marked on it and the defending player's life untouched.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn land_leeches_strikes_first_so_the_attacker_it_blocks_never_deals_its_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(1919, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[land_leeches()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .hand(1, &[rancor()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {1}{G}{G} off the three Forests. The body is read off the layer
    // projection, which is the only reading that sees a printed keyword as a
    // keyword the game will act on.
    cast_from_hand(&mut engine, p0, land_leeches());
    pass_until(&mut engine, stack_is_empty);
    let leech = on_battlefield(&engine, p0, land_leeches()).expect("the Leech resolved");
    assert_eq!(pt(&engine, leech), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, leech).contains(KeywordSet::FIRST_STRIKE),
        "first strike is the whole of the card's text"
    );
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is out");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FIRST_STRIKE),
        "an ordinary creature beside it has none, so the reading above is \
         about this card and not about the reader"
    );

    // p1's turn: Rancor on the Elf turns a printed 1/1 into the 3/2 the trade
    // needs. The Forest pays for it and the Elf is named as the source kept
    // back, because the Elf is this turn's attacker.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana_but(&mut engine, p1, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        1,
        "one Forest tapped, and the Elf still standing"
    );
    cast_with_floating(&mut engine, p1, rancor());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("an Aura is cast at a creature, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&elf),
        "\"enchant creature\" offers the only creature on p1's side: {options:?}"
    );
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, elf),
        (3, 1),
        "Rancor prints +2/+0 and not +2/+1: a 1/1 becomes a 3/1, which is \
         three power to kill the Leech and one toughness for first strike to \
         kill first"
    );
    assert!(
        keywords(&engine, elf).contains(KeywordSet::TRAMPLE),
        "and the trample that would carry the odd point through to the player"
    );

    // The attack and the block. Both creatures die to one simultaneous damage
    // step; the printed first strike is what decides that only one of them
    // does.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p0))],
            },
        )
        .expect("an untapped 3/2 with no text of its own may attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(leech, elf)],
            },
        )
        .expect("the Leech may block the attacker it can kill");

    // Past both damage steps and the post-combat main: the end step is where
    // the death, the marked damage and the trample would all have shown.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the first-strike damage step killed the attacker (CR 510.4)"
    );
    assert!(
        on_battlefield(&engine, p0, land_leeches()).is_some(),
        "and the Leech is still on the battlefield: without first strike both \
         would have died in the same damage step"
    );
    assert_eq!(
        pt(&engine, leech),
        (2, 2),
        "nothing was dealt back to it, so no damage is marked on it"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the trample never had a combat damage step to carry anything \
         through in"
    );
}

/// Imperial Recruiter — {2}{R} creature: "When this creature enters, search
/// your library for a creature card with power 2 or less, reveal it, put it
/// into your hand, then shuffle."
///
/// A library of Land Leeches (power 2) offers every card, and the find goes
/// to the hand while the battlefield stays as it was.
#[test]
fn imperial_recruiter_finds_a_creature_with_power_two_or_less_for_the_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, land_leeches())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[imperial_recruiter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library_before = library_size(&engine, p0);

    cast_from_hand(&mut engine, p0, imperial_recruiter());
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = pass_to_card_choice(&mut engine)
    else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    assert_eq!(options.len(), library_before, "every Leeches has power 2");
    let found = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&found),
        "\"put it into your hand\""
    );
    assert!(on_battlefield(&engine, p0, land_leeches()).is_none());
    assert_eq!(library_size(&engine, p0), library_before - 1);
}
