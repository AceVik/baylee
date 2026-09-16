//! Sorceries, the door `cards/sorceries/` puts them behind.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// A creature cast after a board-wide debuff resolved is not on its list.
///
/// CR 611.2c: a continuous effect created by a resolving spell or ability
/// that modifies characteristics affects the objects it found when it began,
/// and the set does not change afterwards. Toxic Deluge registered a *live
/// filter* instead — "every creature", asked again on every projection —
/// so a creature cast one priority later entered as a 0/0 and was swept up
/// by the next state-based check, killed by a spell that had already
/// finished resolving.
///
/// Both halves are asserted, because either alone can be passed by a
/// mistake: the opponent's Llanowar Elves was there when the Deluge
/// resolved and dies, and the one cast afterwards stands there at 1/1.
#[test]
fn a_creature_cast_after_a_mass_debuff_is_not_shrunk_by_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), forest()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[toxic_deluge(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Only the swamps: the forest is kept back for the creature, so the
    // test does not depend on which land the payment happens to spend.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        if engine
            .state()
            .object(source)
            .is_some_and(|o| o.card.is_some_and(|c| c.index == swamp()))
        {
            engine
                .apply(p0, PlayerAction::ActivateManaAbility { source })
                .unwrap();
        }
    }
    let deluge = in_hand(&engine, p0, toxic_deluge()).expect("the Deluge is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: deluge })
        .unwrap();
    let Pending::ChooseNumber { .. } = engine.pending().clone() else {
        panic!("expected the X choice, got {:?}", engine.pending())
    };
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_none(),
        "the 1/1 that was there when the Deluge resolved took -1/-1 and died",
    );

    cast_from_hand(&mut engine, p0, quiet_creature());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, quiet_creature()).is_some()
    });
    let mine = on_battlefield(&engine, p0, quiet_creature())
        .expect("a creature cast after the Deluge survives its arrival");
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the Deluge had already resolved, so it is not one of its creatures",
    );
}

// oracle_id = "3d6fa57a-aa53-4b5c-b8af-a7612c823117"
fn faithless_looting() -> baylee_core::ids::CardIndex {
    card_index("3d6fa57a-aa53-4b5c-b8af-a7612c823117")
}

/// "Draw two cards, then discard two cards" is one spell with a choice in
/// the middle of it, and the word that carries the rule is *then*: both
/// draws have already happened when the discard is asked, so a card this
/// spell drew is one of the cards it may be told to throw away. That is what
/// is discarded here — one freshly drawn card and the Elves that were in
/// hand before the sorcery was cast — which says the second thing too: the
/// selection is the player's and not "the first two the engine found", since
/// the other drawn card is still in hand afterwards.
///
/// The second half is the `Coverage::Partial` the card admits to. Flashback
/// {2}{R} is not written, and the offer is where that shows:
/// `LegalActions::castable` reaches a graveyard only for a card a
/// `GrantsFlashback` effect points at, or a face that prints disturb, and
/// Faithless Looting is neither. So with three Mountains tapped — exactly
/// what the printed flashback cost takes, and *tapped* rather than merely
/// standing because `casting::can_cast` probes the floating pool — the card
/// sitting in its owner's graveyard is still not offered back, and naming it
/// anyway is refused. If flashback is ever written, this is the assertion
/// that fails, which is the point of playing the gap and not only the half
/// that works.
#[test]
fn faithless_looting_discards_a_card_it_just_drew_and_is_never_offered_back_for_flashback() {
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

    // The half the card does not have. Read from a sorcery-speed priority,
    // because a refusal read at instant speed would be the timing rule
    // talking rather than the missing permission.
    assert!(
        matches!(engine.state().turn.phase, Phase::FirstMain) && engine.state().turn.active == p0,
        "the spell resolved back into p0's own main phase"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal.mana_abilities.len(),
        3,
        "three Mountains still stand, which is what the flashback cost takes"
    );
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&looting),
        "no flashback is written, so the graveyard card is not offered: {:?}",
        legal.castable
    );
    let refused = engine.apply(p0, PlayerAction::CastSpell { card: looting });
    assert!(refused.is_err(), "and naming it anyway is refused");
}

// oracle_id = "b08c5b86-4f84-46a3-877f-bbf71ec17adb"
fn open_communications() -> baylee_core::ids::CardIndex {
    card_index("b08c5b86-4f84-46a3-877f-bbf71ec17adb")
}

/// Open Communications ({U}, sorcery): "Draw a card." and, under it, "Beam me
/// up {2}{U} (You may cast this card from your graveyard for {2}{U} if you
/// also return a creature you control to its owner's hand. Then exile this
/// spell.)"
///
/// The card stands at `Coverage::Partial`, and the two halves of that claim
/// are struck in one scenario because either one alone would mislead. A test
/// that only drew the card would say nothing about the line the file admits
/// it cannot write; a test that only found the graveyard shut would not have
/// shown that the card plays at all.
///
/// **What it does.** One Island pays {U}, the sorcery resolves and the seat's
/// library is one card shorter while their hand is exactly the size it was —
/// which is the signature of a cantrip and not of a spell that merely left
/// the hand, because a `Draw a card.` that did nothing would leave the hand
/// one smaller. The card is then in its owner's graveyard, where a resolved
/// spell goes (CR 608.2m), which is also what puts the second half in reach.
///
/// **What it does not do.** `Beam me up` is unwritten: `AlternativeCost`
/// carries no zone and the one graveyard cast a face can print is
/// `FaceDef::disturb` (CR 702.112), which pays the *face's* own mana cost —
/// {U} here, not the printed {2}{U} — so the card may only ever be cast from
/// hand. The board is built so that nothing else can be the reason: the card
/// is in its owner's graveyard, a creature the additional cost could return
/// stands on the battlefield, and three Islands' worth of blue is floating,
/// which pays {2}{U} over.
///
/// The counter-half is the second copy in hand. It is castable at that very
/// priority, so the mana and the sorcery timing (CR 307.1) are demonstrably
/// not what the graveyard copy is missing — the zone is. And the offer and
/// the door are asked separately, because an offer a test cannot see is worth
/// nothing if pressing the card works anyway.
///
/// Written to **fail** the day a graveyard cast exists: when the DSL can say
/// "from your graveyard, for this price, returning a creature", this breaks,
/// and flipping `Coverage::Partial` to `Implemented` is what closes it.
#[test]
fn open_communications_draws_a_card_and_is_never_castable_out_of_the_graveyard() {
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
        .expect("the resolved sorcery is a card in its owner's graveyard (CR 608.2m)");

    // Everything `Beam me up` asks for is on the table before the question is
    // put: the creature it would return, and the mana it would cost.
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "a creature the additional cost could return is on the battlefield"
    );
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    let floating = engine.state().players[0].mana_pool.total();
    assert!(
        floating >= 3,
        "the Islands held back are floating, which covers the printed \
         {{2}}{{U}}, so nothing below is the graveyard copy being too \
         expensive: {floating}"
    );
    let hand_copy =
        in_hand(&engine, p0, open_communications()).expect("the second copy is still in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&hand_copy),
        "the counter-half: the same card is castable from hand at this very \
         priority, so neither the mana nor the sorcery timing is what the \
         graveyard copy is missing: {legal:?}"
    );
    assert!(
        !legal.castable.contains(&grave),
        "`Beam me up` is not written, so the card is offered from hand and \
         never from the graveyard — if this fires, a graveyard cast exists \
         and Open Communications is no longer Coverage::Partial: {legal:?}"
    );
    // The same rule from the other side. `apply` starts no casting wizard for
    // a card the offer did not name, so this is the shorter half of the pair
    // the Damn case in `cast_face_tests` writes out: an offer a test cannot
    // see is worth nothing if pressing the card works anyway.
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: grave })
            .is_err(),
        "the graveyard copy was refused by the offer and accepted by the wizard"
    );
}

// oracle_id = "1b9f9f5b-8712-4f00-90cb-1b7b9970eccc"
fn sevinne_s_reclamation() -> baylee_core::ids::CardIndex {
    card_index("1b9f9f5b-8712-4f00-90cb-1b7b9970eccc")
}

/// A Sevinne's Reclamation cast off four Plains and standing on its target
/// choice, over a graveyard holding one card for every clause of the printed
/// filter.
///
/// Each of the four is a card that would be a legal target but for the single
/// word under test: Llanowar Elves is the one-mana creature card the spell is
/// for, Sheoldred is a permanent card one mana value over "3 or less", Dark
/// Ritual is a card in that same graveyard that is not a *permanent* card,
/// and the opponent's Llanowar Elves is the very printing being returned,
/// lying in the graveyard the spell does not read. Asserting only that
/// something came back would pass against `Filter::Any`, which is the mistake
/// this shape exists to refuse.
///
/// They get there three different ways because the harness has only one:
/// `seed_graveyard` mills whatever the library is filled with, so the filler
/// is the four-drop, the two Elves are destroyed off the battlefield, and the
/// instant is cast and allowed to resolve. Six of the ten Plains are left
/// untapped on purpose — that is more than the flashback cost the card prints
/// and does not have, which the test spends at the end.
fn a_reclamation_over_a_stocked_graveyard() -> (Engine<RegistryLookup>, PlayerId, PlayerId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![swamp(), llanowar_elves()];
    board.extend([plains(); 10]);
    let mut engine = Duel::new(61, sheoldred_the_apocalypse())
        .battlefield(0, &board)
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[sevinne_s_reclamation(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);

    // A creature card into each graveyard, off the battlefield: the library
    // is filled with the four-drop, so milling cannot produce a small one.
    for seat in [p0, p1] {
        let elf = on_battlefield(&engine, seat, llanowar_elves()).expect("both Elves are out");
        let state = engine
            .dev_state_mut(seat)
            .expect("the harness may set boards up");
        crate::sba::destroy(state, elf);
    }
    seed_graveyard(&mut engine, p0, 1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The instant card, put where it is by being cast for one black mana.
    let swamps = all_on_battlefield(&engine, p0, swamp());
    let tap = PlayerAction::ActivateManaAbility { source: swamps[0] };
    engine.apply(p0, tap).expect("the Swamp taps for black");
    let ritual = in_hand(&engine, p0, dark_ritual()).expect("the Ritual is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .expect("one Swamp pays {B}");
    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    for source in all_on_battlefield(&engine, p0, plains())
        .into_iter()
        .take(4)
    {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .expect("a Plains taps for white");
    }
    let spell = in_hand(&engine, p0, sevinne_s_reclamation()).expect("the spell is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("four Plains pay {2}{W}");
    (engine, p0, p1)
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

// oracle_id = "5b6bdf5a-2742-4851-92cd-a857a3852836"
fn treasure_cruise() -> baylee_core::ids::CardIndex {
    card_index("5b6bdf5a-2742-4851-92cd-a857a3852836")
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
