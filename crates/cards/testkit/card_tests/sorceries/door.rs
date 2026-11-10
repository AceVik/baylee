#[allow(clippy::wildcard_imports)]
use super::*;

/// "Draw two cards, then discard two cards" is one spell with a choice in
/// the middle of it, and the word that carries the rule is *then*: both
/// draws have already happened when the discard is asked, so a card this
/// spell drew is one of the cards it may be told to throw away. That is what
/// is discarded here — one freshly drawn card and the Elves that were in
/// hand before the sorcery was cast — which says the second thing too: the
/// selection is the player's and not "the first two the engine found", since
/// the other drawn card is still in hand afterwards.
///
/// The same spell can then be cast for its printed flashback cost and is
/// exiled after resolving its second draw/discard sequence.
#[test]
fn faithless_looting_discards_a_card_it_just_drew_and_flashes_back() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(83, forest())
        .battlefield(0, &[mountain(); 4])
        .hand(0, &[faithless_looting(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // One Mountain pays {R}; the other three are left standing for the
    // flashback half below, which is why this is not `cast_from_hand`.
    let source = all_on_battlefield(&engine, p0, mountain())[0];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source })
        .unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    let library_before = library_size(&engine, p0);
    let elf = in_hand(&engine, p0, llanowar_elves()).expect("the Elves are in hand");
    let spell = in_hand(&engine, p0, faithless_looting()).expect("the sorcery is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("one Mountain pays for it");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on a card choice")
    };
    assert_eq!((min, max), (2, 2), "the card says two, not up to two");
    let drawn: Vec<ObjectId> = options
        .iter()
        .copied()
        .filter(|id| !hand_before.contains(id))
        .collect();
    assert_eq!(drawn.len(), 2, "the draws came first: {options:?}");
    assert!(options.contains(&elf), "a card already in hand may go too");
    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "the two drawn cards came off the library"
    );

    let chosen = vec![drawn[0], elf];
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: chosen })
        .expect("two cards out of the hand it was offered");
    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    let zone_of = |id: ObjectId| engine.state().object(id).expect("it still exists").zone;
    assert_eq!(
        (zone_of(drawn[0]), zone_of(drawn[1])),
        (Zone::Graveyard, Zone::Hand),
        "the drawn card that was picked is discarded, the one that was not stays"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some()
            && in_hand(&engine, p0, llanowar_elves()).is_none(),
        "the Elves were the second card picked"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before.len() - 1,
        "two drawn, two discarded, and the sorcery itself gone from hand"
    );
    let looting = in_graveyard(&engine, p0, faithless_looting())
        .expect("the sorcery finished resolving and went to the graveyard");

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: looting })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((min, max), (2, 2));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: options[..2].to_vec(),
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(engine.state().object(looting).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "flashback costs three mana"
    );
}

/// The hand cast cantrips normally. Its graveyard cast pays {2}{U}, returns
/// a controlled creature as a cost, draws again, then exiles the spell.
#[test]
fn open_communications_draws_then_beams_up_a_creature_to_cast_from_the_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(76, forest())
        .battlefield(
            0,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .hand(0, &[open_communications(), open_communications()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // One Island, and only one: the other three stay untapped so that the
    // graveyard half below is asked on a board that can pay {2}{U}.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let source = legal
        .mana_abilities
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == island()))
        })
        .expect("an untapped Island makes the {U}");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source })
        .unwrap();

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let spell = in_hand(&engine, p0, open_communications()).expect("the sorcery is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("one Island pays {U}");
    // `at_rest` and not `stack_is_empty`: the assertions below read p0's own
    // offer, so the walk has to end with p0 holding priority and not merely
    // with the stack empty.
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "`Draw a card.` took exactly one card off the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "and the hand is the size it was: the cantrip spent itself and drew \
         its replacement"
    );
    let grave = in_graveyard(&engine, p0, open_communications())
        .expect("the resolved sorcery is a card in its owner's graveyard (CR 608.2n)");

    let creature = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    let floating = engine.state().players[0].mana_pool.total();
    assert!(floating >= 3);
    let hand_copy = in_hand(&engine, p0, open_communications()).unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(legal.castable.contains(&hand_copy));
    assert!(
        legal.castable.contains(&grave),
        "beam me up is offered with its mana and return cost available"
    );
    let library_before_beam = library_size(&engine, p0);
    let hand_before_beam = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine
        .apply(p0, PlayerAction::CastSpell { card: grave })
        .unwrap();
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the return-cost choice, got {:?}",
            engine.pending()
        );
    };
    assert_eq!((min, max), (1, 1));
    assert_eq!(options, vec![creature]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(creature).unwrap().zone,
        Zone::Hand,
        "the return is paid before the spell resolves"
    );
    assert_eq!(engine.state().object(grave).unwrap().zone, Zone::Stack);
    assert_eq!(engine.state().players[0].mana_pool.total(), floating - 3);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(library_size(&engine, p0), library_before_beam - 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before_beam + 2,
        "the returned creature and the drawn card both enter the hand"
    );
    assert_eq!(engine.state().object(grave).unwrap().zone, Zone::Exile);
}

/// Sevinne's Reclamation ({2}{W}, sorcery): "Return target permanent card
/// with mana value 3 or less from your graveyard to the battlefield."
///
/// The first half walks the printed filter clause by clause over the
/// graveyard the builder stocked, takes the one card that survives it, and
/// reads the board afterwards.
///
/// The second half is the card's `Coverage::Partial`. Flashback (CR 702.34)
/// is unwritten — no `FaceDef` field and no `AbilityDef` says "you may cast
/// this from your graveyard for {4}{W}, then exile it" — and
/// `compute_legal`'s graveyard loop only reaches `can_cast` for a card that a
/// granted `Modifier::GrantsFlashback` or a `disturb` face names. So the
/// spell lands in its owner's graveyard (CR 400.7) and stays there: not
/// castable with more than the printed flashback cost floating, so that
/// affordability cannot be mistaken for the reason, and not exiled either.
/// That closes the card's second gap without a second board, because the copy
/// clause is conditioned on having been cast from a graveyard and this card
/// can only be cast from a hand. The day flashback is written this half
/// fails, which is when the test owes a rewrite — and is the whole point of
/// asserting a gap rather than describing one in a comment.
#[test]
fn a_reclamation_returns_one_small_permanent_from_your_own_graveyard_and_is_never_offered_back() {
    let (mut engine, p0, p1) = a_reclamation_over_a_stocked_graveyard();
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    let small = in_graveyard(&engine, p0, llanowar_elves()).expect("p0's Elf died");
    let big =
        in_graveyard(&engine, p0, sheoldred_the_apocalypse()).expect("a four-drop was milled");
    let instant = in_graveyard(&engine, p0, dark_ritual()).expect("the Ritual resolved");
    let theirs = in_graveyard(&engine, p1, llanowar_elves()).expect("their Elf died too");

    assert_eq!((min, max), (1, 1), "\"target permanent card\" is not a may");
    assert!(
        options.contains(&small),
        "a one-mana creature card in your own graveyard is what the spell is \
         for: {options:?}"
    );
    for (card, why) in [
        (big, "mana value 4 is one over \"3 or less\""),
        (instant, "an instant card is not a permanent card"),
        (
            theirs,
            "the same printing, in a graveyard the spell does not read",
        ),
    ] {
        assert!(!options.contains(&card), "{why}: {options:?}");
    }

    let choice = PlayerAction::ChooseObjects {
        objects: vec![small],
    };
    engine.apply(p0, choice).expect("the one legal target");
    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some()
            && in_graveyard(&engine, p0, llanowar_elves()).is_none(),
        "the Elf is on the battlefield under the caster's control, and gone \
         from the graveyard it was returned from",
    );
    assert!(
        in_graveyard(&engine, p0, sheoldred_the_apocalypse()).is_some()
            && in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one target and one card: cast from a hand the copy clause is false, \
         so nothing else moved",
    );

    // The Partial half: a sorcery card in its owner's own graveyard, with
    // every Plains the cast did not spend tapped for white.
    let reclamation = in_graveyard(&engine, p0, sevinne_s_reclamation())
        .expect("the sorcery went to its owner's graveyard");
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&reclamation),
        "six white mana is more than the printed flashback cost of {{4}}{{W}}, \
         so affordability is not the reason: the engine offers no way to cast \
         this card out of a graveyard at all. {:?}",
        legal.castable,
    );
    assert!(
        in_graveyard(&engine, p0, sevinne_s_reclamation()).is_some(),
        "and it is not exiled either, which is the other half of a flashback \
         that never happens",
    );
}

/// Treasure Cruise ({7}{U} sorcery): "Delve. Draw three cards."
///
/// Both printed sentences in one cast, which is the only way either of them
/// is worth anything: the card is an eight-mana draw spell that nobody would
/// ever pay eight mana for, so a test that cast it off eight lands would
/// prove the half the card is not about.
///
/// The premise is therefore one Island — `{U}` floating and nothing else —
/// and seven cards in the graveyard. `legal.castable` is computed against the
/// pool as it stands, so an offer made here is made on one mana against a
/// cost of eight, and delve (CR 702.66a, "for each generic mana in this
/// spell's total cost, you may exile a card from your graveyard rather than
/// pay that mana") is the only thing that can close the gap. The question
/// that follows says the same thing from the other side: `max == 7` is the
/// generic half of `{7}{U}` and not the size of the graveyard.
///
/// Then the other sentence, which delve says nothing about and which is the
/// whole point of paying for the card: exactly three cards off the top. The
/// library is counted rather than the hand alone, because a hand that grew by
/// three could have grown from anywhere — and the hand is counted *too*,
/// because a library that shrank by three could have been milled. It grows by
/// two, not three: the Cruise itself left the hand to be cast.
///
/// The exiled seven are read again after the draw rather than before it. That
/// is what separates "delve exiled them" from "delve exiled them and the draw
/// gave them back", which is the shape a graveyard cost gets wrong; and the
/// opponent's library is counted for the word *you*, since "draw three cards"
/// is not "each player draws three cards".
#[test]
fn seven_cards_out_of_the_graveyard_and_one_island_draw_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(5, island())
        .hand(0, &[treasure_cruise()])
        .battlefield(0, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 7);

    let cruise = in_hand(&engine, p0, treasure_cruise()).expect("the Cruise is in hand");
    tap_all_mana(&mut engine, p0);
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let theirs_before = library_size(&engine, p1);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&cruise),
        "one Island floats {{U}} against a cost of {{7}}{{U}}, so the seven \
         cards in the graveyard are what makes the Cruise castable at all"
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: cruise })
        .expect("the spell the offer just quoted");

    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected the delve question, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 7, "any card in the graveyard may be exiled");
    assert_eq!(min, 0, "delve is never compulsory");
    assert_eq!(max, 7, "and {{7}} is exactly what seven of them pay for");
    let delved = options.clone();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: options })
        .expect("exiling all seven is a legal answer");

    assert!(
        !stack_is_empty(&engine),
        "the spell never reached the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the Island paid the blue half and the graveyard paid all seven \
         generic — nothing was left floating"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 3,
        "three cards off the top, and no fourth"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2,
        "three drawn into a hand the Cruise itself had just left"
    );
    assert_eq!(
        library_size(&engine, p1),
        theirs_before,
        "\"draw three cards\" is the caster's three, not each player's"
    );
    for id in &delved {
        assert_eq!(
            engine.state().object(*id).map(|o| o.zone),
            Some(Zone::Exile),
            "a card delve exiled is still in exile after the draw"
        );
    }
    assert!(
        in_graveyard(&engine, p0, treasure_cruise()).is_some(),
        "the resolved sorcery lies in the graveyard it just emptied"
    );
}

/// Spirit Water Revival's taps pay for the waterbend `{6}` and for nothing
/// else (CR 701.67b), so `{U}{U}` and a body to tap cannot pay the printed
/// `{1}{U}{U}`. #229: the offer counted the body as the missing `{1}`.
#[test]
fn spirit_water_revival_is_not_offered_on_a_tap_that_could_only_pay_its_printed_cost() {
    assert!(
        revival_is_offered(&revival_table(&[
            island(),
            island(),
            island(),
            ondu_cleric()
        ])),
        "the control: three Islands pay {{1}}{{U}}{{U}}"
    );
    assert!(
        !revival_is_offered(&revival_table(&[island(), island(), ondu_cleric()])),
        "a creature's tap was counted toward the printed cost"
    );
}

/// "Converge — Exile target nonland permanent if its mana value is less
/// than or equal to the number of colors of mana spent to cast this
/// spell." {2}{W} paid with white, blue and black: three colors, and a
/// three-drop is exiled.
#[test]
fn prismatic_ending_exiles_what_its_three_colors_reach() {
    let p1 = PlayerId::new(1);
    let engine = prismatic_ending_at(&[plains(), island(), swamp()], 2, prismatic_three_drop());
    assert!(on_battlefield(&engine, p1, prismatic_three_drop()).is_none());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .iter()
            .any(|&id| engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == prismatic_three_drop())))
    );
}

/// Colorless mana is no color (CR 106.1a): {2}{W} paid with a Plains and
/// Sol Ring's {C}{C} is one color, and a two-drop stays.
#[test]
fn prismatic_ending_counts_no_colorless_mana() {
    let p1 = PlayerId::new(1);
    let engine = prismatic_ending_at(&[plains(), sol_ring()], 2, prismatic_two_drop());
    assert!(on_battlefield(&engine, p1, prismatic_two_drop()).is_some());
}

/// All four: the board is empty of all three types, and every graveyard.
#[test]
fn farewell_in_full_leaves_only_the_lands() {
    let p1 = PlayerId::new(1);
    let engine = farewell_with(0b1111);
    for gone in [
        llanowar_elves(),
        sol_ring(),
        sterling_grove(),
        darksteel_gargoyle(),
    ] {
        assert!(on_battlefield(&engine, p1, gone).is_none());
    }
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(p1)).len(), 7);
    assert_eq!(lands_of(&engine, PlayerId::new(0)).len(), 6);
}
