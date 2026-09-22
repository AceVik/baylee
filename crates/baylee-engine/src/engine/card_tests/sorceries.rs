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
/// spell goes (CR 608.2n), which is also what puts the second half in reach.
///
/// **What it does not do.** `Beam me up` is unwritten: `AlternativeCost`
/// carries no zone and the one graveyard cast a face can print is
/// `FaceDef::disturb` (CR 702.146), which pays the *face's* own mana cost —
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
        .expect("the resolved sorcery is a card in its owner's graveyard (CR 608.2n)");

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

// oracle_id = "8a2e53f9-8100-488f-8504-b59e9bd1cc29"
fn rite_of_flame() -> CardIndex {
    card_index("8a2e53f9-8100-488f-8504-b59e9bd1cc29")
}

/// Stingcaster Mage, wanted here for its price and nothing else: `{1}{R}` is
/// the cheapest cost in the pool that one Mountain cannot pay and a resolved
/// Rite of Flame can.
///
/// Under a name of its own because the handle in `creatures` is a sibling's
/// private item and a sibling module reaches nothing at all — the module doc
/// on `card_tests` is the reason every shared helper sits in `mod.rs`.
fn a_two_mana_red_spell() -> CardIndex {
    card_index("056b651e-e0e2-4333-9235-d1ffe8fcca29")
}

/// Rite of Flame ({R} sorcery): "Add {R}{R}, …".
///
/// A ritual is the one spell whose whole effect is a number, and a number is
/// exactly what reading the card cannot check: `Effect::mana(ManaColor::Red,
/// 2)` says two red and says nothing about the `{R}` that was paid for it, so
/// a card that added its mana without ever taking the cost, or took the cost
/// and added it to the wrong colour, reads identically. The pool is measured
/// at three moments for that reason — one red floating off the Mountain, an
/// **empty** pool the instant the spell is on the stack, and two red when it
/// has resolved — and the middle one is the half a test that only looked at
/// the end would miss.
///
/// The board is one Mountain and nothing else, and `legal.mana_abilities` is
/// asserted empty once it is tapped: every red counted afterwards came out of
/// the spell, because there is nothing left on the table that could make one.
///
/// Then whether it is *mana* rather than a number in a struct. `{1}{R}` is
/// not castable on the single red the Mountain made and is castable on what
/// the Rite leaves behind, at the same priority, off the same tapped board —
/// so the two the pool reports are two the engine will let a spell spend.
///
/// The second printed clause — "then add {R} for each card named Rite of
/// Flame in each graveyard" — is the `Coverage::Partial` this card declares
/// and is not what this test is about. No graveyard holds a Rite of Flame
/// while this one resolves (the card is on the stack, and nothing else was
/// cast), so the clause would add nothing here even if it were written: the
/// two below is the printed answer under either reading.
#[test]
fn rite_of_flame_spends_one_red_and_leaves_two_spendable_in_the_pool() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(83, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[rite_of_flame(), a_two_mana_red_spell()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the Mountain is tapped"
    );
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "the Mountain's own {{R}}, which is all the mana this board has"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.mana_abilities.is_empty(),
        "the Mountain is tapped and there is no second land: nothing on this \
         board can add mana any more, so every red counted below came out of \
         the spell — {legal:?}"
    );
    let rite = in_hand(&engine, p0, rite_of_flame()).expect("the Rite is in hand");
    let two_drop = in_hand(&engine, p0, a_two_mana_red_spell()).expect("the two-drop is in hand");
    assert!(
        legal.castable.contains(&rite),
        "one floating red pays the printed {{R}}: {legal:?}"
    );
    assert!(
        !legal.castable.contains(&two_drop),
        "and does not pay {{1}}{{R}} — the before half of the offer this test \
         reads again after the Rite has resolved: {legal:?}"
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: rite })
        .expect("the spell the offer just named");
    assert!(
        !stack_is_empty(&engine),
        "a sorcery goes on the stack (CR 601.2a)"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the Mountain's {{R}} was spent on the cost: a ritual that added \
         without ever taking payment would look the same at the end and is \
         only distinguishable here"
    );

    pass_until(&mut engine, |e| at_rest(e, p0));

    let pool = &engine.state().players[0].mana_pool;
    for color in ManaColor::ALL {
        let expected: u16 = if color == ManaColor::Red { 2 } else { 0 };
        assert_eq!(
            pool.available(color),
            expected,
            "`Add {{R}}{{R}}` put two red in a pool the cast had just emptied \
             and touched no other colour — {color:?} disagrees"
        );
    }
    assert_eq!(
        pool.total(),
        2,
        "two and no third: nothing restricted was added beside the plain \
         red, and the clause about cards named Rite of Flame in graveyards \
         has nothing to count while the only copy is the one resolving"
    );
    assert!(
        in_graveyard(&engine, p0, rite_of_flame()).is_some(),
        "a resolved sorcery is put into its owner's graveyard (CR 608.2n)"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&two_drop),
        "the same {{1}}{{R}} card that was unaffordable one priority ago is \
         castable now, off the same tapped Mountain: what the Rite added is \
         mana a spell can be paid with and not a number in a pool: {legal:?}"
    );
}

// oracle_id = "1d67f5ff-1fce-45e5-b6a1-416c569351e2"
fn gitaxian_probe() -> CardIndex {
    card_index("1d67f5ff-1fce-45e5-b6a1-416c569351e2")
}

/// Gitaxian Probe — `{U/P}` sorcery: "({U/P} can be paid with either {U} or
/// 2 life.) Look at target player's hand. Draw a card."
///
/// The card is `Coverage::Partial` with the *look* missing and the target
/// requirement and the draw written, so the three claims here are exactly
/// that half. First the question: "target player" enumerates both seats —
/// and it arrives as `ChoosePlayer` rather than as the object-and-player
/// `ChooseTargets`, because a requirement that is only a player never
/// reaches the object half of targeting — and the probe is aimed at the
/// opponent, the seat whose hand it is printed to look at. Second the cantrip: the spell
/// leaves the hand, lands in its owner's graveyard, and puts exactly one
/// card off the top of the caster's library into that same hand, so the
/// hand it was cast from is the hand it finishes with. Third the gap: the
/// target's hand and library are the size they were, because no effect
/// reads a hand and the look moves nothing.
#[test]
fn gitaxian_probe_asks_for_any_player_and_replaces_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(17, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[gitaxian_probe()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    let library_before = library_size(&engine, p0);
    let their_hand = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p1))
        .len();
    let their_library = library_size(&engine, p1);

    cast_from_hand(&mut engine, p0, gitaxian_probe());

    // The phyrexian symbol may be asked about before the target is; two
    // Islands are sitting there to pay either way, so the first option the
    // engine lists is taken.
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::ChooseCastMode { player, .. } => {
                engine.apply(player, PlayerAction::ChooseMode(0)).unwrap();
            }
            Pending::ChoosePlayer { .. } => break,
            other => panic!("expected the spell's target question, got {other:?}"),
        }
    }

    // A spell whose whole target requirement is a player is asked as
    // `ChoosePlayer` and not as `ChooseTargets`: the wizard branches on
    // `TargetSpec::AnyPlayer` before the object half of targeting runs, so
    // there is no empty object list to inspect — the enumeration of seats
    // *is* the question.
    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!("the probe never asked what to look at")
    };
    assert_eq!(player, p0, "the caster chooses the target");
    assert_eq!(
        options,
        vec![p0, p1],
        "\"target player\" is every seat at the table, the caster included"
    );

    engine
        .apply(p0, PlayerAction::ChoosePlayer(p1))
        .expect("the opponent is a player the probe may target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, gitaxian_probe()).is_some(),
        "the sorcery resolved and went to its owner's graveyard"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before,
        "one card spent and one drawn: the Probe is a cantrip"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and the card drawn came off the top of the caster's library"
    );

    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p1))
            .len(),
        their_hand,
        "the look at the target's hand is the `Coverage::Partial` gap — \
         nothing was taken, shown or moved"
    );
    assert_eq!(
        library_size(&engine, p1),
        their_library,
        "and nothing left the target's library either, so the draw went to \
         the caster and not to the seat that was pointed at"
    );
}

/// Past in Flames — {3}{R} sorcery: "Each instant and sorcery card in your
/// graveyard gains flashback until end of turn. The flashback cost is equal
/// to its mana cost." (Its own flashback {4}{R} is the `Coverage::Partial`
/// gap, and nothing here presses it.)
///
/// The scenario is one card in one zone read twice. A Dark Ritual is cast so
/// that it lies in its owner's graveyard, and the offer — the only way this
/// engine ever says a card may be cast — holds nothing for it; the sorcery
/// resolves, and the same card in the same zone is offered as castable for
/// its own {B}. Casting it there is the second half: a flashed-back card is
/// exiled rather than handed back to the graveyard, and its `{B}{B}{B}`
/// arrives, so the spell resolved rather than merely being offered.
///
/// The control is a Llanowar Elves in the same graveyard, put there by
/// `seed_graveyard`. "Each instant and sorcery card" is a filter, and a
/// creature card in that zone is what says so — a grant that had lost its
/// type filter would offer it too, and every assertion about the Ritual
/// would still pass.
///
/// The rule's own test is [`super::super::flashback_tests`], which injects
/// both shapes of grant directly; this one is the card, and it is here
/// because a filtered grant is the only kind Past in Flames makes.
#[test]
fn past_in_flames_grants_a_graveyard_ritual_flashback_and_that_cast_exiles_it() {
    let p0 = PlayerId::new(0);
    // The filler deck is Llanowar Elves so that the graveyard seeding below
    // has a *creature* card to offer the filter's other half.
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[swamp(), swamp(), mountain(), mountain(), mountain()])
        .hand(0, &[past_in_flames(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    seed_graveyard(&mut engine, p0, 1);
    let elf = in_graveyard(&engine, p0, llanowar_elves()).expect("the seeded Elf");

    // A Dark Ritual is what the sorcery's sentence is about, and it gets
    // there by being cast: the Swamps pay its {B} while every Mountain is
    // held back for the sorcery itself.
    tap_all_mana_but(&mut engine, p0, Some(mountain()));
    let ritual = in_hand(&engine, p0, dark_ritual()).expect("the Ritual is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .expect("{B} is in the pool for it");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        in_graveyard(&engine, p0, dark_ritual()),
        Some(ritual),
        "it resolved into the graveyard, which is the zone the sentence names"
    );

    // Before the sorcery: the same card in the same zone is offered nowhere.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&ritual),
        "a card in a graveyard is not castable until something says it is: {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&elf),
        "and a creature card there is never a spell at all"
    );

    // Past in Flames {3}{R}: the four black already in the pool are still
    // inside their phase (CR 500.4) and pay the {3}, and a Mountain pays the
    // {R}.
    tap_all_mana(&mut engine, p0);
    cast_from_hand(&mut engine, p0, past_in_flames());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&ritual),
        "\"each instant and sorcery card in your graveyard gains flashback\": {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&elf),
        "\"each instant and sorcery card\" is a filter and not the whole \
         graveyard: {:?}",
        legal.castable
    );

    // Cast it from the graveyard for its own mana cost. "Then exile it" is
    // the half that tells a flashback cast from an ordinary one, and the
    // Ritual's own three black say the spell resolved on the way.
    let black_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Black);
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .expect("the offer named it, so it is castable");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black)
            >= black_before + 2,
        "its own \"{{B}}{{B}}{{B}}\" resolved: three black arrived and at most \
         one paid the cost"
    );
    assert!(
        in_graveyard(&engine, p0, dark_ritual()).is_none(),
        "\"then exile it\" — a flashed-back card does not come back to the \
         graveyard to be cast again"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&ritual),
        "it is in exile, where a card cast out of a graveyard goes"
    );
}

/// Gamble — {R} sorcery: "Search your library for a card, put that card into your
/// hand, discard a card at random, then shuffle."
///
/// Discarding a card at random is the `Coverage::Partial` gap because no random discard
/// effect exists in the DSL. This scenario proves the search half: casting Gamble offers
/// a mandatory search (min: 1, max: 1) from the library directly into the player's
/// hand, and the card stays in hand without being discarded.
#[test]
fn gamble_searches_library_for_a_card_to_hand_without_random_discard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[gamble()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let lib_size_before = library_size(&engine, p0);
    let spell = in_hand(&engine, p0, gamble()).expect("gamble is in hand");

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("one Mountain pays {R} for Gamble");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on ChooseCards")
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "search prompt indicates library tutor"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "Gamble searches for exactly one card (mandatory search)"
    );
    assert!(
        !options.is_empty(),
        "library has basic Forests to search from"
    );

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("choosing the card from library is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(chosen).map(|o| o.zone),
        Some(Zone::Hand),
        "the searched card arrived in hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "hand holds exactly the tutored card and no random discard occurred"
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_size_before - 1,
        "library is one card smaller after tutoring"
    );
    assert_eq!(
        in_graveyard(&engine, p0, gamble()),
        Some(spell),
        "Gamble resolved into the graveyard"
    );
}

/// Grim Tutor — {1}{B}{B} sorcery: "Search your library for a card, put that
/// card into your hand, then shuffle. You lose 3 life."
///
/// Two effects with a library between them, so the only reading worth playing
/// is both halves off one cast: three Swamps pay the cost, the search question
/// is answered with the card that was on top, and the three life are gone
/// whatever the search found. Naming the top card before anything is cast is
/// what makes the search a *move* rather than a question that was asked — the
/// library is one card shorter and that exact object is in hand, where a tutor
/// that asked and then did nothing would leave both where they were. The
/// opponent's life is the control for "you".
#[test]
fn grim_tutor_finds_the_card_it_names_and_charges_three_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(19, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[grim_tutor()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The card the search will be answered with, named before anything is
    // cast: the *last* entry of the list is the top of the library.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library to search");
    let top_card = engine
        .state()
        .object(top)
        .and_then(|o| o.card)
        .expect("the top of a library is a card")
        .index;
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, grim_tutor());

    // Let the spell resolve; the first thing it asks is which card.
    let mut asked_for_the_card = false;
    for _ in 0..12 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
            } => {
                assert_eq!(player, p0, "the caster searches their own library");
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::SearchLibrary,
                    "a tutor is a search and not a discard or a scry"
                );
                assert!(min >= 1 && max >= 1, "a card is found, not looked at");
                assert!(
                    options.contains(&top),
                    "`Filter::Any` puts the whole library on the menu: {options:?}"
                );
                engine
                    .apply(p0, PlayerAction::ChooseObjects { objects: vec![top] })
                    .expect("the card the question offered is one of its answers");
                asked_for_the_card = true;
                break;
            }
            other => panic!("unexpected while the Tutor resolves: {other:?}"),
        }
    }
    assert!(
        asked_for_the_card,
        "Grim Tutor asks which card before it moves one anywhere"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        17,
        "\"you lose 3 life\", off the same resolution as the search"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you\" is the caster: the opponent pays nothing"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "the found card left the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and it is the exact object that was offered, now in hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the Tutor left the hand and the card it found took its place"
    );
    assert!(
        in_hand(&engine, p0, top_card).is_some(),
        "read by printing too, so the object identity is not the only evidence"
    );
}

/// Sylvan Scrying — {1}{G} sorcery: "Search your library for a land card,
/// reveal it, put it into your hand, then shuffle."
///
/// The filter is unreadable on this board — the harness' backing deck is sixty
/// basic Forests — so the scenario plays the half that is: the question arrives
/// as `SearchLibrary`, every option is an object that was *in the library* and
/// none of the two Forests seeded into the graveyard beside them, and the card
/// that is taken lies in hand with the battlefield exactly as long as it was.
/// A search that milled or put the land onto the battlefield would satisfy
/// "something left the library" and fail every assertion below.
#[test]
fn sylvan_scrying_finds_a_land_in_the_library_and_puts_it_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[sylvan_scrying()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Two Forests on the floor of the graveyard, so "the search offers the
    // library" is a claim about *which* objects are named and not merely how
    // many: a Forest in a graveyard and a Forest in a library share a printing
    // and nothing else.
    seed_graveyard(&mut engine, p0, 2);
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let graveyard = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let battlefield_before = engine.state().zones.list(ZoneLocation::Battlefield).len();
    assert_eq!(
        graveyard.len(),
        2,
        "the seed put two lands under the library"
    );

    cast_from_hand(&mut engine, p0, sylvan_scrying());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the caster searches their own library");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "a tutor, and not a discard or a scry, is all a client has to tell these apart"
    );
    assert!(
        options.iter().all(|id| library.contains(id)),
        "every option is a card that was in the library: {options:?}"
    );
    assert!(
        !options.iter().any(|id| graveyard.contains(id)),
        "and a card in a graveyard is not a card to search for: {options:?}"
    );
    assert_eq!(
        options.len(),
        library.len(),
        "the whole library, because every card in it is a land"
    );

    let found = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .expect("the card the question offered is the card the search takes");
    pass_until(&mut engine, stack_is_empty);

    let landed = engine
        .state()
        .object(found)
        .expect("the found card is still an object");
    assert_eq!(
        landed.zone,
        Zone::Hand,
        "`Find::HAND`: into hand, and not onto the battlefield"
    );
    assert_eq!(landed.controller, p0, "under the searching seat's control");
    assert!(
        landed.characteristics().types.contains(TypeSet::LAND),
        "and it is a land card, which is what the search was for"
    );
    assert_eq!(
        library_size(&engine, p0),
        library.len() - 1,
        "one card left the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the sorcery left the hand and exactly one card came back to it"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Battlefield).len(),
        battlefield_before,
        "and the search added nothing to the battlefield"
    );
    assert!(
        graveyard.iter().all(|id| engine
            .state()
            .object(*id)
            .is_some_and(|o| o.zone == Zone::Graveyard)),
        "the lands under the library were never on the menu"
    );
}

/// Wheel of Fortune: "Each player discards their hand, then draws seven cards."
/// Under `Coverage::Partial`, whole-hand discarding is unsupported, so each player draws seven cards.
/// Three Mountains pay {2}{R} to cast the sorcery; both players draw seven cards from their libraries.
#[test]
fn wheel_of_fortune_each_player_draws_seven_cards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(39, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[wheel_of_fortune()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let wheel = in_hand(&engine, p0, wheel_of_fortune()).expect("Wheel of Fortune in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: wheel })
        .expect("three Mountains pay {2}{R}");

    let p0_hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    let p0_lib_before = library_size(&engine, p0);
    let p1_lib_before = library_size(&engine, p1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        p0_hand_before + 7,
        "p0 drew seven cards"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        p1_hand_before + 7,
        "p1 drew seven cards"
    );
    assert_eq!(
        library_size(&engine, p0),
        p0_lib_before - 7,
        "p0 library reduced by seven"
    );
    assert_eq!(
        library_size(&engine, p1),
        p1_lib_before - 7,
        "p1 library reduced by seven"
    );
    assert!(
        in_graveyard(&engine, p0, wheel_of_fortune()).is_some(),
        "Wheel of Fortune went to graveyard upon resolution"
    );
}

/// Hunger of the Nim: "Target creature gets +1/+0 until end of turn for each artifact you control."
/// Cast with two artifacts on the battlefield targeting a 1/1 Llanowar Elves, the pump counts both artifacts.
/// Upon resolution, the creature receives +2/+0 and its power and toughness become 3/1.
#[test]
fn hunger_of_the_nim_pumps_target_creature_per_artifact_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(135, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                myr_retriever(),
                myr_retriever(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[hunger_of_the_nim()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    assert_eq!(pt(&engine, elves), (1, 1));

    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, hunger_of_the_nim()).expect("Hunger of the Nim in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves), "Elves is a legal target creature");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elves), (3, 1), "two artifacts give +2/+0");
}

/// Imperial Seal: "Search your library for a card, then shuffle and put that card on top. You lose 2 life."
/// Cast off a Swamp, the sorcery prompts a search of the library and places the chosen card on top.
/// Upon resolution, the player's life total is reduced by two from 20 to 18.
#[test]
fn imperial_seal_tutors_card_to_top_and_costs_two_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(134, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[imperial_seal()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, imperial_seal());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected search choice, got {:?}", engine.pending());
    };
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

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(found),
        "chosen card was placed on top of library"
    );
    assert_eq!(engine.state().players[0].life, 18, "player lost 2 life");
}

/// Green Sun's Zenith — {X}{G} sorcery: "Search your library for a green
/// creature card with mana value X or less, put it onto the battlefield, then
/// shuffle. Shuffle Green Sun's Zenith into its owner's library." The file is
/// `Coverage::Partial`: the search is written and the shuffle-back is not, so
/// the halves a board can hold the card to are the announced X, the creature
/// that arrives, and where the spell itself ends up.
///
/// The library is sixty Llanowar Elves — one green creature card, mana value
/// 1 — and X is announced as **2**. Reading "mana value X or less" against a
/// card strictly cheaper than X is the only reading that separates the printed
/// bound from a search for mana value *X*, and a library of a single printing
/// makes the menu the searching player's own library rather than a hand-picked
/// pair. CR 608.2m then puts the Zenith in the graveyard, which is exactly the
/// sentence the printing replaces.
#[test]
fn green_suns_zenith_finds_a_green_creature_of_mana_value_x_or_less() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[green_suns_zenith()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "nothing but the three Forests stands before the Zenith looks"
    );

    // {X}{G} with X announced as 2 is three mana, and the three Forests are
    // the whole of it: `cast_from_hand` taps them first, so the pool the
    // engine prices X against is the pool the card is paid out of.
    cast_from_hand(&mut engine, p0, green_suns_zenith());
    let Pending::ChooseNumber { player, min, max } = engine.pending().clone() else {
        panic!(
            "a spell with an X in its cost announces it, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster announces their own X");
    assert!(
        min <= 2 && 2 <= max,
        "three green in the pool leaves X somewhere in {min}..={max}"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(2))
        .expect("two is inside the range the engine published");

    // The spell is on the stack now, and the search is asked as it resolves.
    // Not `pass_until`: that walker panics on a `SearchLibrary` question, and
    // this is the question under test.
    let mut steps = 0;
    while !matches!(engine.pending(), Pending::ChooseCards { .. }) {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "expected priority while the Zenith resolves, got {:?}",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
        steps += 1;
        assert!(steps <= 20, "the Zenith never reached its search");
    }

    let Pending::ChooseCards {
        player,
        options,
        min,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the loop only leaves on a card choice")
    };
    assert_eq!(player, p0, "the caster searches their own library");
    assert_eq!(prompt, crate::choice::ChoicePrompt::SearchLibrary);
    assert_eq!(min, 1, "the search is not optional and asks for one card");
    assert!(
        !options.is_empty(),
        "every card in this library is a green creature of mana value 1, so \
         a bound of 2 cannot leave the menu empty"
    );
    let cards_in_library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert!(
        options.iter().all(|id| cards_in_library.contains(id)),
        "and every option comes out of the searching player's own library: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("one of the cards the question offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "\"put it onto the battlefield\": a mana value 1 creature arrived off \
         an announced X of 2, which a search for mana value *X* would never \
         have offered"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the found card left the library, and the shuffle returned no other"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{X}}{{G}} took the whole three the Forests made"
    );
    assert!(
        in_graveyard(&engine, p0, green_suns_zenith()).is_some(),
        "CR 608.2m: the resolving spell lies in the graveyard, because \
         \"shuffle Green Sun's Zenith into its owner's library\" is the clause \
         the card's file leaves off"
    );
}

/// Reshape — {X}{U}{U}: "Search your library for an artifact card with mana
/// value X or less, put it onto the battlefield, then shuffle."
///
/// `Filter::CmcAtMostX` is what says that bound, and a bound is only a bound
/// if some value falls outside it — so the spell is cast twice over a
/// library of Sol Rings, mana value 1. At **X = 1** the search offers them;
/// at **X = 0** it offers nothing, which is the assertion that separates
/// reading the announced number from ignoring it. A filter that always
/// matched would pass the first half and hand the player a Sol Ring for
/// {U}{U}.
///
/// The file is `Coverage::Partial` for the additional cost — "sacrifice an
/// artifact" is a `CostPart` no spell cost list can carry — and nothing
/// below pays it, so both readings of the search agree here.
#[test]
fn reshape_finds_an_artifact_within_the_x_it_announced_and_nothing_above_it() {
    let p0 = PlayerId::new(0);

    // X = 1: {1}{U}{U} is three mana, and three Islands are what is here.
    let mut engine = Duel::new(41, quiet_artifact())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[reshape()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, reshape());

    // CR 601.2b: X is announced before any cost is paid.
    let Pending::ChooseNumber { player, min, max } = engine.pending().clone() else {
        panic!("a spell with {{X}} asks for X, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster announces the value");
    assert!(min <= 1 && 1 <= max, "X = 1 is on offer: {min}..={max}");
    engine
        .apply(p0, PlayerAction::ChooseNumber(1))
        .expect("the value the question enumerated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        !options.is_empty(),
        "Sol Ring is mana value 1, and 1 is `X or less` for X = 1"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{X}}{{U}}{{U}} with X = 1 is three mana, which is what the Islands \
         made: an engine that never charged the X would have one floating"
    );
    let library_before = library_size(&engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a card the search offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let found = on_battlefield(&engine, p0, quiet_artifact()).expect("the artifact it found");
    assert!(
        !is_tapped(&engine, found),
        "\"put it onto the battlefield\" — the printing says nothing about tapped"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card left the library"
    );

    // X = 0: the same library, and nothing in it is `0 or less`.
    let mut zero = Duel::new(41, quiet_artifact())
        .battlefield(0, &[island(), island()])
        .hand(0, &[reshape()])
        .start();
    keep_mulligans(&mut zero);
    assert!(walk_to_own_main(&mut zero, p0), "p0 reaches its own main");
    tap_all_mana(&mut zero, p0);
    cast_with_floating(&mut zero, p0, reshape());
    zero.apply(p0, PlayerAction::ChooseNumber(0))
        .expect("X = 0 is a legal announcement");
    let zero_library = library_size(&zero, p0);
    pass_until(&mut zero, |e| at_rest(e, p0));
    assert_eq!(
        library_size(&zero, p0),
        zero_library,
        "Sol Ring is mana value 1, and the bound the spell announced was 0"
    );
    assert!(
        on_battlefield(&zero, p0, quiet_artifact()).is_none(),
        "so nothing arrived"
    );
}

/// Finale of Devastation — {X}{G}{G}: "Search your library and/or graveyard
/// for a creature card with mana value X or less and put it onto the
/// battlefield."
///
/// The same `Filter::CmcAtMostX` bound over creatures, and the assertion
/// beside it is the **mana**: {X}{G}{G} at X = 1 is three, and three Forests
/// is exactly what the board holds, so an engine that fetched without
/// charging the announced number would leave one floating where this reads
/// zero.
///
/// The file is `Coverage::Partial` for two things this scenario cannot
/// reach: the search is library-only, the printed "and/or graveyard" having
/// no zone to name, and the "if X is 10 or more" rider needs a branch on the
/// announced number. Nothing is in the graveyard here and X is 1.
#[test]
fn finale_of_devastation_fetches_a_creature_within_the_announced_x() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, llanowar_elves())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[finale_of_devastation()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests and nothing else on the board"
    );
    cast_with_floating(&mut engine, p0, finale_of_devastation());
    engine
        .apply(p0, PlayerAction::ChooseNumber(1))
        .expect("X = 1");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    for id in &options {
        assert!(
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves())),
            "a creature card of mana value 1, which is `X or less` for X = 1"
        );
    }
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{X}}{{G}}{{G}} with X = 1 is three mana, and three is what was made"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a card the search offered");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "\"and put it onto the battlefield\""
    );
}

/// Mizzix's Mastery exiles a card out of its controller's own graveyard and
/// then exiles **itself**, and the second half is the one worth playing: a
/// sorcery that resolved normally would be in the graveyard it just emptied
/// a slot in, so "Exile Mizzix's Mastery" is checked by where the spell
/// itself ends up rather than by what it did.
///
/// The library is built out of Brainstorms so the graveyard the spell reads
/// holds instants and nothing else, and the target is asserted against the
/// object that actually left.
///
/// The file is `Coverage::Partial` for the copy — nothing copies a card in
/// exile and lets its controller cast the copy for free — and for overload.
/// Neither is reachable from here, and the exile is the half that is built.
#[test]
fn mizzixs_mastery_exiles_a_card_from_your_graveyard_and_then_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, brainstorm())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[mizzix_s_mastery()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    seed_graveyard(&mut engine, p0, 2);

    let graveyard_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    assert_eq!(graveyard_before, 2, "two instants are buried");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, mizzix_s_mastery());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target card that's an instant or sorcery from your graveyard\" \
             asks which, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options.len(), 2, "both buried instants are on the offer");
    let named = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![named],
                players: vec![],
            },
        )
        .expect("a card the spell offered");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&named),
        "the card it named left the graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, mizzix_s_mastery()).is_none(),
        "\"Exile Mizzix's Mastery\" — a sorcery that merely resolved would be \
         lying in the graveyard it just took a card out of"
    );
}

/// `Agadeem's Awakening` // `Agadeem, the Undercrypt` (`Coverage::Partial`): "Return from
/// your graveyard to the battlefield any number of target creature cards that each have a
/// different mana value X or less. // As this land enters, you may pay 3 life. If you don't,
/// it enters tapped. {T}: Add {B}."
///
/// Under `Coverage::Partial`, the front-face reanimation clause is not expressible in the
/// DSL, but the back face (`Agadeem, the Undercrypt`) is fully modeled. The test plays the
/// land face, pays 3 life on the `EnterModifier::TappedOrPayLife(3)` prompt to have it enter
/// untapped, and immediately activates its mana ability to add `{B}`.
#[test]
fn agadeem_the_undercrypt_pays_three_life_to_enter_untapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let (mut engine, land) =
        play_land_face(agadeem_s_awakening(), 1).expect("plays as Agadeem, the Undercrypt");
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    }

    assert!(
        !is_tapped(&engine, land),
        "3 life was paid, so it entered untapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        17,
        "paid 3 life to enter untapped"
    );

    let black_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Black);
    activate(&mut engine, p0, agadeem_s_awakening(), 0);

    assert!(is_tapped(&engine, land), "tapped to activate mana ability");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        black_before + 1,
        "adds one black mana to the pool"
    );
}

/// `Bala Ged Recovery` // `Bala Ged Sanctuary` (`Coverage::Implemented`): "Return target
/// card from your graveyard to your hand. // This land enters tapped. {T}: Add {G}."
///
/// Marked `Coverage::Implemented`, casting the front face for `{2}{G}` targets a card in
/// your graveyard via `TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::You)`.
/// The test verifies that only cards in the caster's graveyard are offered, choosing the
/// seeded card and confirming it returns to hand upon resolution.
#[test]
fn bala_ged_recovery_returns_target_card_from_own_graveyard_to_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(473, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[bala_ged_recovery()])
        .start();
    keep_mulligans(&mut engine);

    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);

    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let p0_target = engine.state().zones.list(ZoneLocation::Graveyard(p0))[0];
    let p1_other = engine.state().zones.list(ZoneLocation::Graveyard(p1))[0];

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, bala_ged_recovery());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&p0_target),
        "card in caster's graveyard is a legal target"
    );
    assert!(
        !options.contains(&p1_other),
        "card in opponent's graveyard is not a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_target],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&p0_target),
        "targeted card returned to caster's hand"
    );
    assert!(
        in_graveyard(&engine, p0, bala_ged_recovery()).is_some(),
        "Bala Ged Recovery resolved to graveyard"
    );
}

/// `Bridgeworks Battle` // `Tanglespan Bridgeworks` (`Coverage::Partial`): "Target creature
/// you control gets +2/+2 until end of turn. It fights up to one target creature you don't
/// control. // As this land enters, you may pay 3 life. If you don't, it enters tapped.
/// {T}: Add {G}."
///
/// Under `Coverage::Partial`, the front-face pump effect is implemented while the fight
/// clause is omitted. The test casts `Bridgeworks Battle` targeting a controlled creature,
/// confirms that an opponent's creature cannot be targeted by `Filter::YOUR_CREATURE`, and
/// verifies the target receives +2/+2 until end of turn while the opponent's creature survives.
#[test]
fn bridgeworks_battle_pumps_controlled_creature_and_omits_fight() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(474, forest())
        .battlefield(0, &[forest(), forest(), forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[bridgeworks_battle()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("p0 has an elf");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("p1 has an elf");
    assert_eq!(pt(&engine, my_elf), (1, 1));
    assert_eq!(pt(&engine, their_elf), (1, 1));

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_front_face(&mut engine, p0, bridgeworks_battle());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&my_elf),
        "controlled creature is a legal target"
    );
    assert!(
        !options.contains(&their_elf),
        "opponent creature cannot be targeted"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_elf],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, my_elf),
        (3, 3),
        "controlled creature gets +2/+2 until end of turn"
    );
    assert_eq!(
        pt(&engine, their_elf),
        (1, 1),
        "opponent creature is not pumped"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "under `Coverage::Partial` no fight occurred so opponent creature survives"
    );
    assert!(
        in_graveyard(&engine, p0, bridgeworks_battle()).is_some(),
        "Bridgeworks Battle moves to graveyard after resolution"
    );
}

/// `Maelstrom Pulse` (`Coverage::Partial`): "Destroy target nonland permanent and all
/// other permanents with the same name as that permanent."
///
/// Under `Coverage::Partial`, `Maelstrom Pulse` destroys the targeted nonland permanent
/// while the same-name sweep is omitted due to lack of a name filter in the DSL.
/// The test targets one of two opponent `llanowar_elves()`, verifies that lands cannot be
/// targeted, and confirms that only the targeted permanent is destroyed upon resolution.
#[test]
fn maelstrom_pulse_destroys_target_nonland_permanent_and_omits_name_sweep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(475, forest())
        .battlefield(0, &[swamp(), swamp(), forest()])
        .battlefield(1, &[forest(), llanowar_elves(), llanowar_elves()])
        .hand(0, &[maelstrom_pulse()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let their_land = on_battlefield(&engine, p1, forest()).expect("opponent land");
    let their_elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(their_elves.len(), 2, "opponent has two elves");

    cast_from_hand(&mut engine, p0, maelstrom_pulse());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&their_elves[0]),
        "nonland permanent is a legal target"
    );
    assert!(
        !options.contains(&their_land),
        "lands are not legal targets for Maelstrom Pulse"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_elves[0]],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    // By id and not by card: there are several Elves here, so
    // `on_battlefield(.., llanowar_elves())` would find one of the others
    // and say nothing about the one that was targeted. And off the
    // battlefield rather than out of the arena — a destroyed permanent is
    // still an object, which is a token's test and not this one.
    assert!(
        !engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Battlefield)
            .contains(&their_elves[0]),
        "the targeted creature was destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "and it is in its owner's graveyard"
    );
    assert!(
        engine.state().object(their_elves[1]).is_some(),
        "under `Coverage::Partial` the other permanent with the same name survives"
    );
    assert!(
        in_graveyard(&engine, p0, maelstrom_pulse()).is_some(),
        "Maelstrom Pulse moved to graveyard after resolving"
    );
}

/// `Makindi Stampede` // `Makindi Mesas` (`Coverage::Implemented`): "Creatures you control
/// get +2/+2 until end of turn. // This land enters tapped. {T}: Add {W}."
///
/// Marked `Coverage::Implemented`, casting the front face for `{3}{W}{W}` executes
/// `Effect::PumpFilter` over `Filter::YOUR_CREATURE`. The test verifies that controlled
/// creatures receive the +2/+2 buff until end of turn while an opponent's creature is
/// untouched, and confirms the spell resolves to the graveyard.
#[test]
fn makindi_stampede_pumps_controlled_creatures_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(478, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[makindi_stampede()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("controlled elf");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent elf");
    assert_eq!(pt(&engine, my_elf), (1, 1));
    assert_eq!(pt(&engine, their_elf), (1, 1));

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_front_face(&mut engine, p0, makindi_stampede());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, my_elf),
        (3, 3),
        "controlled creature gets +2/+2 until end of turn"
    );
    assert_eq!(
        pt(&engine, their_elf),
        (1, 1),
        "opponent creature is not pumped"
    );
    assert!(
        in_graveyard(&engine, p0, makindi_stampede()).is_some(),
        "Makindi Stampede moved to graveyard after resolving"
    );
}

/// `Pelakka Predation` // `Pelakka Caverns` (`Coverage::Partial`): "Target opponent reveals
/// their hand. You choose a card from it with mana value 3 or greater. That player discards
/// that card. // This land enters tapped. {T}: Add {B}."
///
/// Under `Coverage::Partial`, the front-face targeted discard clause is not expressible in
/// the DSL, but the back face (`Pelakka Caverns`) is fully implemented. The test plays the
/// land face, confirms it enters tapped, advances to the next turn so it untaps, and
/// activates its mana ability to add `{B}`.
#[test]
fn pelakka_caverns_enters_tapped_and_taps_for_black_mana() {
    let (mut engine, caverns) =
        play_land_face(pelakka_predation(), 1).expect("plays as Pelakka Caverns");
    assert!(is_tapped(&engine, caverns), "Pelakka Caverns enters tapped");

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, caverns), "untaps on next turn");
    let black_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Black);

    activate(&mut engine, p0, pelakka_predation(), 0);

    assert!(
        is_tapped(&engine, caverns),
        "tapped to activate mana ability"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        black_before + 1,
        "adds one black mana to the pool"
    );
}

/// `Stump Stomp` // `Burnwillow Clearing` (`Coverage::Partial`): "Target creature you control
/// deals damage equal to its power to target creature or planeswalker you don't control.
/// // This land enters tapped. {T}: Add {R} or {G}."
///
/// Under `Coverage::Partial`, the front-face fight-like effect is not expressible in the
/// DSL, but the back face (`Burnwillow Clearing`) is fully implemented. The test plays the
/// land face, verifies it enters tapped, advances to the next turn so it untaps, and
/// activates its mana ability choosing `{G}` from `Pending::ChooseColor`.
#[test]
fn burnwillow_clearing_enters_tapped_and_taps_for_chosen_mana() {
    let (mut engine, land) =
        play_land_face(stump_stomp(), 1).expect("plays as Burnwillow Clearing");
    assert!(
        is_tapped(&engine, land),
        "Burnwillow Clearing enters tapped"
    );

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, land), "untaps on next turn");
    let green_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Green);

    activate(&mut engine, p0, stump_stomp(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![ManaColor::Red, ManaColor::Green],
        "offers Red and Green"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();

    assert!(is_tapped(&engine, land), "tapped to activate mana ability");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        green_before + 1,
        "adds one green mana to the pool"
    );
}

/// `Sundering Eruption` // `Volcanic Fissure` (`Coverage::Partial`): "Destroy target land.
/// Its controller may search their library for a basic land card, put it onto the battlefield
/// tapped, then shuffle. Creatures without flying can't block this turn. // As this land
/// enters, you may pay 3 life. If you don't, it enters tapped. {T}: Add {R}."
///
/// Under `Coverage::Partial`, land destruction and the opponent's basic land search are
/// implemented via `Effect::OptionalBasicLandSearchFor`, while the blocking restriction is
/// omitted. The test destroys an opponent's land, verifies creatures cannot be targeted,
/// resolves the opponent's basic-land search putting it onto the battlefield tapped, and
/// confirms the original land was destroyed.
#[test]
fn sundering_eruption_destroys_land_and_gives_opponent_tapped_basic() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(476, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .hand(0, &[sundering_eruption()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let target_land = on_battlefield(&engine, p1, forest()).expect("opponent land");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent elf");

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, sundering_eruption());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&target_land),
        "target land is a legal target"
    );
    assert!(
        !options.contains(&elf),
        "creature is not a legal target for land destruction"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_land],
            },
        )
        .unwrap();

    for _ in 0..8 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected basic land search prompt, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p1,
        "the basic-land search belongs to the destroyed land's controller"
    );
    assert_eq!((min, max), (0, 1), "may search for up to one basic land");
    let found_land = options[0];
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![found_land],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        !engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Battlefield)
            .contains(&target_land),
        "the targeted land was destroyed — it is off the battlefield, not \
         out of the arena, because a destroyed permanent is still an object"
    );
    assert!(
        is_tapped(&engine, found_land),
        "the searched basic land enters tapped"
    );
    assert_eq!(
        engine.state().object(found_land).map(|o| o.controller),
        Some(p1),
        "the searched basic land is controlled by the opponent"
    );
    assert!(
        in_graveyard(&engine, p0, sundering_eruption()).is_some(),
        "Sundering Eruption moves to graveyard after resolving"
    );
}

/// `Thoughtseize` (`Coverage::Partial`): "Target player reveals their hand. You choose
/// a nonland card from it. That player discards that card. You lose 2 life."
///
/// Under `Coverage::Partial`, the target player requirement and the caster's 2 life loss
/// are implemented via `Effect::LoseLife`. The opponent is targeted, confirming they are
/// among `player_options`, and the caster's life drops by 2 upon resolution, while the
/// unmodeled reveal-and-discard clause leaves the opponent's hand unchanged.
#[test]
fn thoughtseize_targets_player_and_causes_caster_to_lose_two_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(472, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[thoughtseize()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let life_before = engine.state().players[0].life;
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    cast_from_hand(&mut engine, p0, thoughtseize());

    // A spell whose only target is a player asks `ChoosePlayer` and not
    // `ChooseTargets` — the object list would be empty either way, and the
    // engine offers the seats rather than an empty target requirement.
    let Pending::ChoosePlayer { options, .. } = engine.pending().clone() else {
        panic!("expected the player choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&p1),
        "the opponent is one of the seats offered"
    );

    engine
        .apply(p0, PlayerAction::ChoosePlayer(p1))
        .expect("the opponent is a legal choice");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        life_before - 2,
        "caster lost 2 life"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        p1_hand_before,
        "under `Coverage::Partial` the opponent hand is untouched"
    );
    assert!(
        in_graveyard(&engine, p0, thoughtseize()).is_some(),
        "Thoughtseize moves to graveyard after resolving"
    );
}

/// `Yawgmoth's Will` (`Coverage::Partial`): "Until end of turn, you may play lands and
/// cast spells from your graveyard. If a card would be put into your graveyard from
/// anywhere this turn, exile that card instead."
///
/// Under `Coverage::Partial`, the continuous grant of `Modifier::PlayLandsFromGraveyard`
/// is implemented for the turn, while casting spells from the graveyard and the exile
/// replacement rule are omitted. The test verifies that a graveyard land is not offered
/// before casting, is offered in `legal.lands` once `Yawgmoth's Will` resolves, and can be
/// successfully played onto the battlefield.
#[test]
fn yawgmoth_s_will_allows_playing_land_from_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(477, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[yawgmoth_s_will()])
        .start();
    keep_mulligans(&mut engine);

    seed_graveyard(&mut engine, p0, 1);
    let gy_land = in_graveyard(&engine, p0, forest()).expect("forest in graveyard");

    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.lands.contains(&gy_land),
        "before Yawgmoth's Will, graveyard land cannot be played"
    );

    cast_from_hand(&mut engine, p0, yawgmoth_s_will());
    pass_until(&mut engine, stack_is_empty);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "expected priority after resolution, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.lands.contains(&gy_land),
        "after Yawgmoth's Will resolves, graveyard land is offered in legal.lands"
    );

    engine
        .apply(p0, PlayerAction::PlayLand { card: gy_land })
        .unwrap();

    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "forest entered the battlefield from graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_none(),
        "forest is no longer in the graveyard"
    );
}

/// `Emeria's Call` // `Emeria, Shattered Skyclave` (`Coverage::Partial`): "Create two 4/4 white
/// Angel Warrior creature tokens with flying. Non-Angel creatures you control gain indestructible
/// until your next turn. // As this land enters, you may pay 3 life. If you don't, it enters tapped.
/// {T}: Add {W}."
///
/// Under `Coverage::Partial`, the tokens are created as 4/4 white Angels with flying (omitting
/// the Warrior subtype), and `Modifier::AddKeyword(KeywordSet::INDESTRUCTIBLE)` is applied to
/// all controlled non-Angel creatures. The test casts the front face, verifies that two 4/4
/// flying tokens are created, confirms that the caster's non-Angel creature gains indestructible,
/// and verifies that neither the Angel tokens nor the opponent's creature gain indestructible.
#[test]
fn emerias_call_creates_two_angel_tokens_and_grants_indestructible_to_non_angels() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(471, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[emeria_s_call()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("p0 controls an elf");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("p1 controls an elf");

    assert!(
        !keywords(&engine, my_elf).contains(KeywordSet::INDESTRUCTIBLE),
        "elf does not have indestructible before the spell"
    );

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_front_face(&mut engine, p0, emeria_s_call());

    pass_until(&mut engine, stack_is_empty);

    let created_tokens = tokens_of(&engine, p0);
    assert_eq!(created_tokens.len(), 2, "creates exactly two tokens");
    for token in &created_tokens {
        assert_eq!(pt(&engine, *token), (4, 4), "each Angel token is 4/4");
        assert!(
            keywords(&engine, *token).contains(KeywordSet::FLYING),
            "each Angel token has flying"
        );
        assert!(
            !keywords(&engine, *token).contains(KeywordSet::INDESTRUCTIBLE),
            "Angel tokens do not gain indestructible (non-Angel restriction)"
        );
    }

    assert!(
        keywords(&engine, my_elf).contains(KeywordSet::INDESTRUCTIBLE),
        "controlled non-Angel creature gains indestructible"
    );
    assert!(
        !keywords(&engine, their_elf).contains(KeywordSet::INDESTRUCTIBLE),
        "opponent's creature does not gain indestructible"
    );
    assert!(
        in_graveyard(&engine, p0, emeria_s_call()).is_some(),
        "the sorcery resolved to the graveyard"
    );
}

/// `Ondu Inversion` // `Ondu Skyruins` (`Coverage::Implemented`): "Destroy all nonland
/// permanents. // This land enters tapped. {T}: Add {W}."
///
/// Under `Coverage::Implemented`, casting the front-face sorcery destroys all nonland permanents
/// across both battlefields via `Effect::DestroyAll` with `Filter::NONLAND`. The test sets up
/// creatures and artifacts on both sides alongside lands, casts `Ondu Inversion`, and verifies
/// that all creatures and artifacts are destroyed while all lands remain on the battlefield.
#[test]
fn ondu_inversion_destroys_all_nonland_permanents_on_both_sides() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(718, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                quiet_artifact(),
            ],
        )
        .battlefield(1, &[plains(), llanowar_elves(), quiet_artifact()])
        .hand(0, &[ondu_inversion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "p0 starts with an elf"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "p1 starts with an elf"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "p0 starts with an artifact"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "p1 starts with an artifact"
    );

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, ondu_inversion());

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "p0's creature was destroyed"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "p1's creature was destroyed"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "p0's artifact was destroyed"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "p1's artifact was destroyed"
    );

    assert!(
        on_battlefield(&engine, p0, plains()).is_some(),
        "p0's lands survive the nonland wipe"
    );
    assert!(
        on_battlefield(&engine, p1, plains()).is_some(),
        "p1's lands survive the nonland wipe"
    );
    assert!(
        in_graveyard(&engine, p0, ondu_inversion()).is_some(),
        "the sorcery resolved to the graveyard"
    );
}

/// `Sea Gate Restoration` // `Sea Gate, Reborn` (`Coverage::Partial`): "Draw cards equal to the
/// number of cards in your hand plus one. You have no maximum hand size for the rest of the game.
/// // As this land enters, you may pay 3 life. If you don't, it enters tapped. {T}: Add {U}."
///
/// Under `Coverage::Partial`, the front-face draw clause cannot be expressed in the DSL, but the
/// back face (`Sea Gate, Reborn`) is fully modeled. The test plays the land face via
/// `play_land_face`, pays 3 life on the `EnterModifier::TappedOrPayLife(3)` prompt to enter
/// untapped, and immediately activates its mana ability to add `{U}` to the pool.
#[test]
fn sea_gate_reborn_pays_three_life_to_enter_untapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let (mut engine, land) =
        play_land_face(sea_gate_restoration(), 1).expect("plays as Sea Gate, Reborn");
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    }

    assert!(
        !is_tapped(&engine, land),
        "3 life was paid, so it entered untapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        17,
        "paid 3 life to enter untapped"
    );

    let blue_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Blue);
    activate(&mut engine, p0, sea_gate_restoration(), 0);

    assert!(is_tapped(&engine, land), "tapped to activate mana ability");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        blue_before + 1,
        "adds one blue mana to the pool"
    );
}

/// `Song-Mad Treachery` // `Song-Mad Ruins` (`Coverage::Implemented`): "Gain control of target
/// creature until end of turn. Untap that creature. It gains haste until end of turn. // This
/// land enters tapped. {T}: Add {R}."
///
/// Under `Coverage::Implemented`, the front-face sorcery executes all three clauses in sequence:
/// it gains control of an opponent's creature via `Modifier::GainControl`, untaps that creature
/// via `Effect::UntapTarget`, and grants it `KeywordSet::HASTE` until end of turn. The test
/// lets the opponent tap their creature during their turn, casts `Song-Mad Treachery` on the
/// following turn, and verifies control transfer, the untap state, and granted haste.
#[test]
fn song_mad_treachery_steals_untaps_and_grants_haste_to_opponent_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(815, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[song_mad_treachery()])
        .start();
    keep_mulligans(&mut engine);

    // Advance to p1's main phase so Llanowar Elves loses summoning sickness and taps for mana.
    reach_their_main_phase(&mut engine, p1);
    let victim = on_battlefield(&engine, p1, llanowar_elves()).expect("p1 controls the creature");
    tap_all_mana(&mut engine, p1);
    assert!(is_tapped(&engine, victim), "p1 tapped the elf for mana");

    // Advance to p0's main phase: only p0's permanents untap, leaving the victim tapped.
    reach_their_main_phase(&mut engine, p0);
    assert!(
        is_tapped(&engine, victim),
        "victim remains tapped across the turn boundary"
    );
    assert_eq!(
        engine.state().object(victim).map(|o| o.controller),
        Some(p1),
        "p1 still controls the victim before the spell resolves"
    );

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, song_mad_treachery());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target selection, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&victim),
        "the opponent's creature is a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(victim).map(|o| o.controller),
        Some(p0),
        "p0 gained control of the creature"
    );
    assert!(
        !is_tapped(&engine, victim),
        "the creature was untapped by the spell"
    );
    assert!(
        keywords(&engine, victim).contains(KeywordSet::HASTE),
        "the creature gained haste"
    );
    assert!(
        in_graveyard(&engine, p0, song_mad_treachery()).is_some(),
        "the sorcery resolved to the graveyard"
    );
}

/// `Suppression Ray` // `Orderly Plaza` (`Coverage::Partial`): "Tap all creatures target player
/// controls. You may pay any amount of {E}. If you do, choose up to that many creatures tapped
/// this way. Put a stun counter on each of them. // This land enters tapped. {T}: Add {W} or {U}."
///
/// Under `Coverage::Partial`, the front-face effect is omitted, but the back face (`Orderly Plaza`)
/// is fully implemented. The test plays the land face via `play_land_face`, verifies that it enters
/// tapped, advances two turns until it untaps, and activates its mana ability to choose `{W}`
/// from `Pending::ChooseColor`.
#[test]
fn orderly_plaza_enters_tapped_and_taps_for_chosen_mana() {
    let (mut engine, land) = play_land_face(suppression_ray(), 1).expect("plays as Orderly Plaza");
    assert!(is_tapped(&engine, land), "Orderly Plaza enters tapped");

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, land), "untaps on next turn");
    let white_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::White);

    activate(&mut engine, p0, suppression_ray(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Blue],
        "offers White and Blue"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();

    assert!(is_tapped(&engine, land), "tapped to activate mana ability");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        white_before + 1,
        "adds one white mana to the pool"
    );
}

/// `Zof Consumption` // `Zof Bloodbog` (`Coverage::Implemented`): "Each opponent loses 4 life and
/// you gain 4 life. // This land enters tapped. {T}: Add {B}."
///
/// Under `Coverage::Implemented`, casting `Zof Consumption` causes each opponent to lose 4 life
/// via `Effect::LoseLife` and the caster to gain 4 life via `Effect::gain_life`. The spell has no
/// targets and resolves directly, leaving the card in the caster's graveyard.
#[test]
fn zof_consumption_drains_opponent_for_four_life_and_caster_gains_four() {
    // Only the caster is named: both life totals are read off the seat list
    // by index below, which is what the card is about.
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(984, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[swamp()])
        .hand(0, &[zof_consumption()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(
        engine.state().players[0].life,
        20,
        "caster starts at 20 life"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "opponent starts at 20 life"
    );

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, zof_consumption());

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        16,
        "opponent lost 4 life from Zof Consumption"
    );
    assert_eq!(
        engine.state().players[0].life,
        24,
        "caster gained 4 life from Zof Consumption"
    );
    assert!(
        in_graveyard(&engine, p0, zof_consumption()).is_some(),
        "the sorcery card is in the graveyard after resolving"
    );
}

/// `Bloodsoaked Insight` // `Sanguine Morass` (`Coverage::Partial`):
/// "This spell costs {1} less to cast for each 1 life your opponents have lost this turn.
/// Target opponent exiles the top three cards of their library. Until the end of your next turn,
/// you may play those cards. If you cast a spell this way, mana of any type can be spent to cast it. //
/// This land enters tapped. `{{T}}`: Add `{{B}}` or `{{R}}`."
///
/// Under `Coverage::Partial`, the front-face sorcery is omitted, while the back-face land
/// (`Sanguine Morass`) is implemented in full. The test plays the back face as a land, confirms it
/// enters tapped, advances to the next turn so it untaps, activates its mana ability, chooses
/// `ManaColor::Black`, and asserts that one black mana is produced.
#[test]
fn sanguine_morass_enters_tapped_and_taps_for_black_or_red_mana() {
    let (mut engine, land) =
        play_land_face(bloodsoaked_insight(), 1).expect("plays as Sanguine Morass");
    assert!(is_tapped(&engine, land), "Sanguine Morass enters tapped");

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, land), "untaps on next turn");

    activate(&mut engine, p0, bloodsoaked_insight(), 0);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "expected color choice for dual mana, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&ManaColor::Black), "offers black mana");
    assert!(options.contains(&ManaColor::Red), "offers red mana");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    assert!(is_tapped(&engine, land), "tapped to produce mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "one black mana in pool"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0,
        "no red mana in pool"
    );
}

/// `Shatterskull Smashing` // `Shatterskull, the Hammer Pass` (`Coverage::Partial`):
/// "Shatterskull Smashing deals X damage divided as you choose among up to two target creatures
/// and/or planeswalkers. If X is 6 or more, Shatterskull Smashing deals twice X damage divided
/// as you choose among them instead. // As this land enters, you may pay 3 life. If you don't,
/// it enters tapped. `{{T}}`: Add `{{R}}`."
///
/// Under `Coverage::Partial`, the front-face sorcery is not expressible in the DSL, while the
/// back-face land (`Shatterskull, the Hammer Pass`) is implemented in full. The test plays the back
/// face as a land, pays 3 life to arrive untapped, and activates its mana ability to add `{{R}}`.
#[test]
fn shatterskull_the_hammer_pass_pays_three_life_to_enter_untapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let (mut engine, land) =
        play_land_face(shatterskull_smashing(), 1).expect("plays as Shatterskull, the Hammer Pass");
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    }

    assert!(
        !is_tapped(&engine, land),
        "paid 3 life, so it entered untapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        17,
        "life reduced from 20 to 17"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("the land taps for mana");

    assert!(is_tapped(&engine, land), "tapped for mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "one red mana added to pool"
    );
}

/// `Turntimber Symbiosis` // `Turntimber, Serpentine Wood` (`Coverage::Partial`):
/// "Look at the top seven cards of your library. You may put a creature card from among them onto the
/// battlefield. If that card has mana value 3 or less, it enters with three additional +1/+1 counters
/// on it. Put the rest on the bottom of your library in a random order. // As this land enters, you may
/// pay 3 life. If you don't, it enters tapped. `{{T}}`: Add `{{G}}`."
///
/// Under `Coverage::Partial`, the front-face creature search is omitted, while the back-face land
/// (`Turntimber, Serpentine Wood`) is implemented in full. The test plays the back face as a land,
/// declines paying 3 life so that it enters tapped without life loss, advances to the next turn so it
/// untaps, and activates its mana ability to add `{{G}}`.
#[test]
fn turntimber_serpentine_wood_enters_tapped_on_declined_life_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let (mut engine, land) =
        play_land_face(turntimber_symbiosis(), 1).expect("plays as Turntimber, Serpentine Wood");
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    }

    assert!(
        is_tapped(&engine, land),
        "declined paying 3 life, so it entered tapped"
    );
    assert_eq!(engine.state().players[0].life, 20, "life remains at 20");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, land), "untaps on next turn");

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("the land taps for mana");

    assert!(is_tapped(&engine, land), "tapped for mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one green mana added to pool"
    );
}
