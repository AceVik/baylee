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

/// `Bridgeworks Battle` // `Tanglespan Bridgeworks`: "Target creature you
/// control gets +2/+2 until end of turn. It fights up to one target creature
/// you don't control."
///
/// The first menu is my creatures only, the second theirs only and "up to
/// one" (`min` 0). A 1/1 of mine is pumped to 3/3 and fights their 1/1: theirs
/// dies, mine keeps one damage — at the pumped size, which is the resolution
/// seeing its own pump. The declined and the gone-in-response halves are in
/// `fight_tests`.
#[test]
fn bridgeworks_battle_pumps_my_creature_and_it_fights_theirs() {
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
        "opponent creature cannot be pumped"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![my_elf],
                players: vec![],
            },
        )
        .unwrap();

    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the fight's target prompt, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![their_elf],
        "only a creature I don't control is fought"
    );
    assert_eq!((min, max), (0, 1), "\"up to one\"");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![their_elf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, my_elf), (3, 3), "+2/+2 until end of turn");
    assert_eq!(
        engine.state().object(my_elf).map(|o| o.damage),
        Some(1),
        "the 1/1 across the table dealt its one back"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "three damage from the pumped Elves kill theirs"
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

/// `Stump Stomp` // `Burnwillow Clearing`: "Target creature you control deals
/// damage equal to its power to target creature or planeswalker you don't
/// control. // This land enters tapped. {T}: Add {R} or {G}."
///
/// The back face: the test plays the land face, verifies it enters tapped,
/// advances to the next turn so it untaps, and activates its mana ability
/// choosing `{G}` from `Pending::ChooseColor`. The front face is the two
/// tests below.
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

/// Stump Stomp, the front face, at a creature: my 4/4 deals four to their 2/2
/// and is dealt nothing back, because this is one-sided and not a fight.
#[test]
fn stump_stomp_has_my_creature_deal_its_power_and_take_nothing_back() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(2190, forest())
        .battlefield(0, &[forest(), mountain(), fangren_hunter()])
        .battlefield(1, &[wild_colos()])
        .hand(0, &[stump_stomp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let hunter = on_battlefield(&engine, p0, fangren_hunter()).expect("my Hunter is out");
    let colos = on_battlefield(&engine, p1, wild_colos()).expect("their Colos is out");
    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, stump_stomp());
    for chosen in [hunter, colos] {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![chosen],
                    players: vec![],
                },
            )
            .expect("each creature is on its own menu");
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, wild_colos()).is_some(),
        "four damage kill the 2/2"
    );
    assert_eq!(
        engine.state().object(hunter).map(|o| o.damage),
        Some(0),
        "nothing is dealt back"
    );
}

/// Stump Stomp at a planeswalker: "target creature **or planeswalker** you
/// don't control". Damage to a planeswalker removes that much loyalty
/// (CR 306.8) — a 4/4 takes Karn from five to one — and my own creatures are
/// not on the second menu at all.
#[test]
fn stump_stomp_can_hit_a_planeswalker_and_takes_its_loyalty() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(2191, forest())
        .battlefield(
            0,
            &[forest(), mountain(), fangren_hunter(), llanowar_elves()],
        )
        .battlefield(1, &[karn_the_great_creator()])
        .hand(0, &[stump_stomp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let hunter = on_battlefield(&engine, p0, fangren_hunter()).expect("my Hunter is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let karn = on_battlefield(&engine, p1, karn_the_great_creator()).expect("their Karn is out");
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_front_face(&mut engine, p0, stump_stomp());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![hunter],
                players: vec![],
            },
        )
        .expect("my Hunter deals the damage");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the second target question, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&karn),
        "a planeswalker I don't control is a target"
    );
    assert!(
        !options.contains(&elves) && !options.contains(&hunter),
        "and nothing of mine is"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![karn],
                players: vec![],
            },
        )
        .expect("Karn was offered");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(karn)
            .map(|o| o.counters.get(CounterKind::Loyalty)),
        Some(1),
        "five loyalty less four damage"
    );
}

/// `Sundering Eruption` // `Volcanic Fissure`: "Destroy target land. Its
/// controller may search their library for a basic land card, put it onto
/// the battlefield tapped, then shuffle. Creatures without flying can't
/// block this turn. // As this land enters, you may pay 3 life. If you
/// don't, it enters tapped. {T}: Add {R}."
///
/// The first two sentences here: the land is destroyed, a creature is not
/// a legal target for it, and the destroyed land's controller — not the
/// caster — is the seat asked to search, who puts a basic onto the
/// battlefield tapped. The third sentence is the test below this one.
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
    assert!(
        keywords(&engine, elf).contains(KeywordSet::CANT_BLOCK),
        "and the rider reached the opponent's ground creature on the way \
         past, which is the sentence the test below this one is about"
    );
}

/// `Sundering Eruption`'s third sentence: "Creatures without flying can't
/// block this turn."
///
/// Board-wide and not a target, so it is an `Effect::PumpFilter` with no
/// `controlled_by`: every creature the filter matches, at both seats. The
/// filter is what this is really about, and a flier beside a ground
/// creature is the only board that can show it — one keeps the defence and
/// one does not, and both of them belong to the same player.
///
/// CR 611.2c fixes that set as the spell resolves, which is why both
/// creatures are seated before the cast rather than arriving after it.
#[test]
fn sundering_eruption_grounds_the_defence_and_leaves_a_flier_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(477, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), rootbreaker_wurm()])
        .battlefield(1, &[forest(), llanowar_elves(), baleful_strix()])
        .hand(0, &[sundering_eruption()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let doomed = on_battlefield(&engine, p1, forest()).expect("a land to destroy");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the ground creature");
    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("the flier");
    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the 6/6 attacks");

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, sundering_eruption());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the land came out of the menu");

    // The controller of the destroyed land declines the search: this test is
    // about the rider, and a basic arriving tapped cannot block anyway.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("\"may search\" — naming nothing is how a player declines");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, elf).contains(KeywordSet::CANT_BLOCK),
        "a creature without flying"
    );
    assert!(
        !keywords(&engine, strix).contains(KeywordSet::CANT_BLOCK),
        "and one with it is not in the set at all — which is the filter, \
         not the seat: both creatures are the opponent's"
    );

    let blocks = attack_and_collect_blocks(&mut engine, wurm, p1);
    assert_eq!(
        blocks.iter().map(|o| o.blocker).collect::<Vec<_>>(),
        vec![strix],
        "only the flier is offered a block, and it is offered one: {blocks:?}"
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

fn angelic_blessing() -> CardIndex {
    card_index("d3758fca-0522-4b5a-a1cc-3b2b3ab299ba")
}

/// Angelic Blessing — {2}{W} sorcery: "Target creature gets +3/+3 and gains
/// flying until end of turn."
///
/// Both halves of the sentence are read off one play, on a board built so that
/// neither can be mistaken for something else: the named 1/1 Elf becomes a
/// (4, 4) with flying, while a second Elf under the same seat and an Elf
/// across the table stay printed (1, 1)s with no evasion — "target creature"
/// is one creature, and the grant is no anthem. Then the turn ends, which is
/// the printed "until end of turn": the same Elf is a 1/1 on the ground again,
/// from a board where nothing else has changed.
#[test]
fn angelic_blessing_pumps_and_grants_flying_to_the_creature_it_names_only_this_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[angelic_blessing()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "a printed 1/1 before the Blessing"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "and nothing has granted it flying yet"
    );

    // Three Plains pay the {2}{W}; the mana is in the pool before the cast.
    cast_from_hand(&mut engine, p0, angelic_blessing());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the Blessing targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it is the one that aims it");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, host),
        (4, 4),
        "+3/+3 on the creature the spell named"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::FLYING),
        "\"and gains flying\" reaches the same creature"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf beside it is still the 1/1 it was printed as"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FLYING),
        "the grant reaches the target and no other"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and it never reaches across the table"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "\"target creature\" is not \"creatures\", let alone anyone else's"
    );

    // "Until end of turn" is the half no reading inside the turn can see: the
    // opponent's main phase is past p0's cleanup, where the effect expires.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "the pump lasted only the turn it was cast on"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "and the flying went with it"
    );
}

fn bargain() -> CardIndex {
    card_index("a0dd88f6-6e36-40ce-bac2-a0db2b0117b6")
}

/// Bargain is `{2}{W}` for two printed sentences: "Target opponent draws a
/// card" and "You gain 7 life". Both are read off one resolution, because the
/// card is nothing without either — a draw that went to the caster would still
/// show a card moving between zones, and seven is the only life number that
/// tells the printed clause from the one a cantrip would carry. The target
/// question is asserted too: `AnyOpponent` has to decline the caster, which is
/// the half a bare "target player" would lose, and the life has to land on the
/// seat that cast it and not on the seat whose library got shorter.
#[test]
fn bargain_makes_an_opponent_draw_and_its_caster_gain_seven_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[bargain()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let their_library = library_size(&engine, p1);
    let their_hand = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    let my_hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, bargain());
    // "Target opponent" names a seat and no object at all, so the question
    // is `ChoosePlayer` and not the `ChooseTargets` a spell with an object
    // in its sights asks — `TargetSpec::AnyPlayer` is not `AnyTarget`.
    let Pending::ChoosePlayer {
        player,
        options: player_options,
    } = engine.pending().clone()
    else {
        panic!("Bargain targets an opponent, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        player_options.contains(&p1),
        "\"target opponent\" reaches the seat across the table: {player_options:?}"
    );
    assert!(
        !player_options.contains(&p0),
        "and never the caster — an opponent is not a target for their own \
         spell: {player_options:?}"
    );
    // The absence is the assertion: the pending carries seats and no object
    // list at all, which is what "target opponent" means and what a spell
    // reaching a permanent would not be.
    assert_eq!(
        player_options.len(),
        1,
        "one opponent at a duel, and no permanent among them: {player_options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChoosePlayer(p1))
        .expect("the seat the question offered is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p1),
        their_library - 1,
        "\"target opponent draws a card\" — one card off the top of *their* \
         library, not the caster's"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        their_hand + 1,
        "and it is in the opponent's hand"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        my_hand - 1,
        "the caster's hand shrank by the spell it cast and grew by nothing"
    );
    assert_eq!(
        engine.state().players[0].life,
        27,
        "\"You gain 7 life\" — seven, on the seat that cast it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and none of the life belongs to the opponent who drew"
    );
}

fn blaze() -> CardIndex {
    card_index("0596920f-9946-42f4-a03b-24aab67f9f1b")
}

/// Blaze is `{X}{R}` for "Blaze deals X damage to any target", and the amount
/// is the whole card: the same sorcery aimed at a seat for X of 3 must read
/// three life off one total, and aimed at a creature for X of 1 must read a
/// dead 1/1 off the board — one cast can prove only one of those, so the hand
/// carries two copies. X is announced as the spell is cast (CR 601.2b) and the
/// target right after it (CR 601.2c), and the range the engine offers for X is
/// bounded by mana that is already *floating*, which is why the Mountains are
/// tapped before any claim is made. The Elf is offered as an option on the
/// first cast and is left standing, so "never named" is a control rather than
/// an absence.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn blaze_deals_its_announced_x_to_the_target_it_names_and_to_no_other() {
    /// Answers one Blaze: the announced X, then the target — `at` when
    /// `creature` is `None`, that creature when it is not. Hands back both
    /// option lists the target question published, so the caller can read what
    /// "any target" meant on this board.
    fn cast_blaze(
        engine: &mut Engine<RegistryLookup>,
        seat: PlayerId,
        x: u32,
        at: PlayerId,
        creature: Option<ObjectId>,
    ) -> (Vec<ObjectId>, Vec<PlayerId>) {
        cast_with_floating(engine, seat, blaze());
        let mut announced = false;
        let mut offered: Option<(Vec<ObjectId>, Vec<PlayerId>)> = None;
        for _ in 0..12 {
            if announced && offered.is_some() {
                break;
            }
            match engine.pending().clone() {
                Pending::ChooseNumber { player, min, max } => {
                    assert!(
                        (min..=max).contains(&x),
                        "X of {x} is inside the range the cast offered: {min}..={max}"
                    );
                    engine.apply(player, PlayerAction::ChooseNumber(x)).unwrap();
                    announced = true;
                }
                Pending::ChooseTargets {
                    player,
                    options,
                    player_options,
                    ..
                } => {
                    let objects: Vec<ObjectId> = creature.into_iter().collect();
                    let players: Vec<PlayerId> = if creature.is_some() {
                        Vec::new()
                    } else {
                        vec![at]
                    };
                    engine
                        .apply(player, PlayerAction::ChooseTargets { objects, players })
                        .unwrap();
                    offered = Some((options, player_options));
                }
                Pending::Priority { player, .. } => {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                other => panic!("unexpected while casting Blaze: {other:?}"),
            }
        }
        assert!(
            announced && offered.is_some(),
            "the cast asked for its X and its target, then {:?}",
            engine.pending()
        );
        offered.expect("checked above")
    }

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[blaze(), blaze()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves =
        on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf stands across the table");

    // Both Mountains counts are read off the *pool*, because that is where the
    // engine reads what an `{X}{R}` can pay: six for the first cast, and the
    // two left over for the second.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Mountains tapped, so the cast has four mana to announce an X of 3 with"
    );

    let (objects, players) = cast_blaze(&mut engine, p0, 3, p1, None);
    assert!(
        players.contains(&p0) && players.contains(&p1),
        "CR 115.4: \"any target\" counts both seats in the same choice: {players:?}"
    );
    assert!(
        objects.contains(&elves),
        "and the creature across the table is one of the object options: {objects:?}"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().players[1].life,
        17,
        "X of 3 is three damage to the seat that was named"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the seat that aimed it is untouched"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the Elf was offered as an option and never named, so it still stands"
    );
    assert!(
        in_graveyard(&engine, p0, blaze()).is_some(),
        "the sorcery resolved and went to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "{{X}}{{R}} with X of 3 is four of the six Mountains"
    );

    let (objects, players) = cast_blaze(&mut engine, p0, 1, p1, Some(elves));
    assert!(
        objects.contains(&elves),
        "the same creature is still on the menu, which is the answer this cast takes: {objects:?}"
    );
    assert!(
        players.contains(&p0) && players.contains(&p1),
        "and the two seats are still there, as the answer it declines: {players:?}"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "X of 1 on a printed 1/1 is lethal (CR 704.5f)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and the creature left the battlefield"
    );
    assert_eq!(
        engine.state().players[1].life,
        17,
        "\"any target\" aimed at a creature does not touch the seat behind it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the second {{X}}{{R}} with X of 1 spent the last two Mountains"
    );
}

fn bloodcurdling_scream() -> CardIndex {
    card_index("3ee2060a-5152-4a53-8183-cbd642e4cc29")
}

/// Bloodcurdling Scream prints one line — "Target creature gets +X/+0 until
/// end of turn" for {X}{B} — and the whole card is the two halves of that
/// sentence happening in the right order: the value of X is announced at
/// CR 601.2b and the target is named afterwards at CR 601.2c, so the four
/// Swamps' black is still in the pool while the target question stands and
/// only leaves it at CR 601.2h. X = 3 on a printed 1/1 Elf reads (4, 1) and
/// nothing else would: (1, 1) means the pump never landed, and a touched
/// toughness means the +0 was read as a +X. The second Elf across the table is
/// the control — "target creature" is not "every creature" — and walking to
/// the opponent's main phase reads the "until end of turn" off the very
/// creature that had been pumped.
#[test]
fn bloodcurdling_scream_pumps_the_target_it_names_for_the_x_it_was_given() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        // Two creatures on the other side: the scream is aimed at one of
        // them, and the other is the control. Neither belongs to the seat
        // that taps, so anything that taps leaves both of them alone.
        .battlefield(1, &[quiet_creature(), quiet_creature()])
        .hand(0, &[bloodcurdling_scream()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p1, quiet_creature());
    assert_eq!(elves.len(), 2, "two creatures to choose between");
    let (victim, bystander) = (elves[0], elves[1]);
    assert_eq!(pt(&engine, victim), (1, 1), "a printed 1/1 before the pump");

    // Four Swamps, and only the Swamps: no creature of p0's is on this board,
    // so the pool is exactly four black and the {X}{B} can come from nowhere
    // else.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Swamps, and nothing else on this board makes mana"
    );

    cast_with_floating(&mut engine, p0, bloodcurdling_scream());

    // CR 601.2b: the value of X is announced before anything is targeted.
    let Pending::ChooseNumber { player, min, max } = engine.pending().clone() else {
        panic!("an {{X}} spell asks for its X, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the casting seat names its own X");
    assert!(
        (min..=max).contains(&3),
        "four black pays for X = 3 and the {{B}}: the question offered {min}..={max}"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(3))
        .expect("three was within the range the question offered");

    // CR 601.2c: and only then is the target chosen.
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the scream targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the casting seat aims it");
    assert!(
        options.contains(&victim) && options.contains(&bystander),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "CR 601.2h pays last: the mana is still floating while the question stands"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the creature the question offered was chosen");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}} and the {{B}} both came out of the four Swamps' pool"
    );
    assert!(!stack_is_empty(&engine), "and the spell is on the stack");

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, victim),
        (4, 1),
        "+X/+0 with X = 3: power up three, toughness untouched"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the creature the scream did not name is still the 1/1 it was printed as"
    );

    // "until end of turn", read off the same creature once the turn that cast
    // the spell is over.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, victim),
        (1, 1),
        "the pump is gone with the turn that made it"
    );
}

// oracle_id = "910ff092-7c9d-49d3-a6df-497683e45bbd"
fn cateran_summons() -> CardIndex {
    card_index("910ff092-7c9d-49d3-a6df-497683e45bbd")
}

/// Cateran Summons — {B} sorcery: "Search your library for a Mercenary card,
/// reveal that card, put it into your hand, then shuffle."
///
/// The library is built hostile on purpose: it is the kit's filler deck, so
/// every card in it is a Forest and not one of them is a Mercenary. That is
/// exactly what the search's own option list is asked about — a list that is
/// non-empty and yet offers nothing is the printed subtype filter doing the
/// excluding, where a search that had lost `Filter::HasSubtype` would offer
/// all of the fillers instead. With no legal find the spell resolves the way a
/// failable search does: no card changes zone, and the Summons itself is the
/// only card that moved, to its owner's graveyard, off a {B} the pool really
/// paid rather than one a board reading assumed.
#[test]
fn cateran_summons_offers_nothing_from_a_library_that_holds_no_mercenary() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[cateran_summons()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spell = in_hand(&engine, p0, cateran_summons()).expect("the Summons is in hand");
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Mana before the claim: `castable` is filtered through the pool, so the
    // Swamp is tapped first and what is asserted is the printed {B}.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the lone Swamp taps for the {{B}} the sorcery costs"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.castable.contains(&spell),
        "with {{B}} in the pool the sorcery is castable: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, cateran_summons());

    // The search, and the rest of the spell behind it. Answering with nothing
    // is what a player does when the filter has nothing to offer; the option
    // list itself is the assertion, and fillers listed here would be a search
    // that never read the word "Mercenary" at all.
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseCards {
                player,
                options,
                prompt,
                ..
            } => {
                assert_eq!(
                    prompt,
                    ChoicePrompt::SearchLibrary,
                    "the card's one instruction is a library search"
                );
                assert!(
                    options.is_empty(),
                    "the library holds {library_before} filler cards and not one \
                     of them is a Mercenary: {options:?}"
                );
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: vec![] })
                    .expect("a search with nothing to find is answered with nothing");
            }
            Pending::Priority { .. } if stack_is_empty(&engine) => break,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Summons resolves: {other:?}"),
        }
    }

    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "nothing was found: the library is the length it was, shuffled or not"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the Summons left the hand and no card arrived in its place"
    );
    assert!(
        in_graveyard(&engine, p0, cateran_summons()).is_some(),
        "a sorcery that resolved is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{B}} was really spent on it"
    );
}

fn cloak_of_feathers() -> CardIndex {
    card_index("cb4baf53-51ed-468b-a468-5d7d45a6dc26")
}

/// Cloak of Feathers is `{U}` for "Target creature gains flying until end of
/// turn" and "Draw a card", and the two halves are read off two different
/// places: the keywords the layers hand the creature that was named, and the
/// library and hand the draw moved. The Elf across the table is the control
/// for "target creature" — it is offered and it must end the turn exactly as
/// it started — while p0's own Elf is named as the source kept back, so the
/// creature the spell lands on was not tapped for its own `{T}` first. The
/// spell is also read on the stack between its target and its resolution,
/// which is CR 601.2c before CR 601.2h.
#[test]
fn cloak_of_feathers_lifts_the_creature_it_names_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), llanowar_elves()])
        .hand(0, &[cloak_of_feathers()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "a sorcery wants p0's own main phase with an empty stack"
    );

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FLYING),
        "a printed 1/1 on the ground before the spell"
    );

    // The Elf is named as the source kept back: it prints its own
    // "{T}: Add {G}", so `tap_all_mana` would have spent the very creature
    // this spell is about to aim at.
    tap_mana_except(&mut engine, p0, mine);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the Island's blue, and nothing off the Elf"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_with_floating(&mut engine, p0, cloak_of_feathers());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" reaches either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered is the one it lifts");

    // CR 601.2c named the target and CR 601.2h paid afterwards, so the spell
    // now stands on the stack and has reached neither of its later zones.
    assert!(
        on_stack(&engine, cloak_of_feathers()).is_some(),
        "the {{U}} is spent and the spell is waiting to resolve"
    );
    assert!(
        in_graveyard(&engine, p0, cloak_of_feathers()).is_none(),
        "a sorcery reaches the graveyard when it resolves, not when it is cast"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, mine).contains(KeywordSet::FLYING),
        "\"target creature gains flying until end of turn\""
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "and nothing but the keyword: the pump the card carries prints 0/0"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "the spell lifts the creature it named and never across the table"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the spell left the hand and the draw filled the seat back up, so a \
         cast that never drew would read one short"
    );
    assert!(
        in_graveyard(&engine, p0, cloak_of_feathers()).is_some(),
        "and the sorcery is in its owner's graveyard"
    );

    // The duration is "until end of turn" and not "for the rest of the game":
    // the opponent's main phase lies past p0's cleanup, where it ends.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FLYING),
        "the ground is where the Elf started the turn and where it ends it"
    );
}

fn death_stroke() -> CardIndex {
    card_index("3ebaa91f-5cbf-4a82-9759-9bd4d93a3e87")
}

/// Death Stroke is `{B}{B}` for "Destroy target tapped creature", and the whole
/// card is the word "tapped": the filter is `CREATURE` **and** `Tapped`, so a
/// board that holds both readings at once is what separates them. Two Llanowar
/// Elves — the same card, the same 1/1 body and even the same mana ability —
/// stand one on each side of the table, and only this seat's is tapped, for its
/// own mana. The `{B}{B}` is floated and the board read *before* the cast, so a
/// Death Stroke that could point at any creature would be castable a step too
/// early; afterwards the untapped Elf is the counter-half of the offer and the
/// tapped one is what has to be in a graveyard.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn death_stroke_destroys_the_tapped_creature_and_no_untapped_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[death_stroke()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is on the table");
    let their_elf =
        on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is on the table");
    let spell = in_hand(&engine, p0, death_stroke()).expect("Death Stroke is in hand");

    // The two Swamps alone, with the Elf named as the source to keep back: two
    // black is exactly the printed {B}{B}, so the only thing this board is
    // missing is a creature that is already tapped.
    tap_mana_except(&mut engine, p0, elf);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "two Swamps tapped, and the Elf left standing"
    );
    assert!(
        !is_tapped(&engine, elf) && !is_tapped(&engine, their_elf),
        "no creature in the game is tapped yet"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&spell),
        "{{B}}{{B}} is paid for and there is still nowhere to point it: every \
         creature in the game is untapped, so the spell is not offered: {:?}",
        legal.castable
    );

    // Tapping the Elf for its own {G} is what puts a tapped creature on the
    // board; the green it makes is a side effect of the price it pays.
    activate(&mut engine, p0, llanowar_elves(), 0);
    assert!(
        is_tapped(&engine, elf),
        "a mana ability's whole price is its own {{T}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds priority again: {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "with a tapped creature to point at, the spell is offered: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, death_stroke());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target tapped creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster aims it");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert!(
        options.contains(&elf),
        "the tapped creature is the whole of the filter: {options:?}"
    );
    assert!(
        !options.contains(&their_elf),
        "\"tapped\" is read: the creature across the table is the same card \
         and still no legal target while it stands up: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and my Elf is the only tapped creature in the game: {options:?}"
    );

    // CR 601.2c before CR 601.2h: the target is named while the mana is still
    // in the pool and the creature is still on the battlefield.
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "the {{B}}{{B}} is spent only once the target has been named"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the creature the question offered is the one it dies to");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the targeted creature left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and a destroyed creature goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell declined never moved"
    );
    assert!(
        in_graveyard(&engine, p0, death_stroke()).is_some(),
        "the sorcery itself is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{B}}{{B}} was paid, and the Elf's own {{G}} is all that is left"
    );
}

fn deconstruct() -> CardIndex {
    card_index("36a8ceb1-148b-41e0-a7bf-ceb879bf08e7")
}

/// Deconstruct — {2}{G} sorcery: "Destroy target artifact. Add {G}{G}{G}."
///
/// Both halves are one scenario because the second is what makes the first
/// worth paying for. Three Forests are tapped to cover the {2}{G}, and the
/// pool they filled is empty again the moment the spell's cost is paid at
/// CR 601.2h — so the three green standing in it after the spell resolves can
/// only be the card's own effect, and "Add {G}{G}{G}" is read as a number
/// rather than as a shuffle of whatever was already floating.
///
/// The artifact destroyed is the *opponent's* Sol Ring with an Elf beside it,
/// which is the other half of the target filter: "target artifact" reaches
/// across the table and still declines a creature. The Sol Ring is read out
/// of its owner's graveyard and not merely gone from the battlefield, and the
/// Elf that was never named never moves.
#[test]
fn deconstruct_destroys_the_artifact_it_targets_and_leaves_three_green_behind() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[deconstruct()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");

    // `legal.castable` is filtered through `can_afford`, and that reads the
    // pool and not the untapped lands: three Forests into the pool first, so
    // the claim below is about the card and not about the board behind it.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, three green, and nothing else on this board makes mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let spell = in_hand(&engine, p0, deconstruct()).expect("the sorcery is in hand");
    assert!(
        legal.castable.contains(&spell),
        "an artifact stands across the table, so the sorcery has the target it \
         needs: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, deconstruct());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target artifact\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat aims it");
    assert!(
        options.contains(&rock),
        "\"target artifact\" reaches across the table: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "an Elf is a creature and no artifact: {options:?}"
    );
    // CR 601.2c before CR 601.2h: while the question stands the mana is still
    // floating and the Sol Ring is still on the battlefield.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the cost is the last step of the cast, so nothing is spent yet"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and the artifact is still standing while the target is chosen"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("the artifact the question offered is the one that dies");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{G}} came out of the pool the moment the cost was paid"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying an artifact is no mana ability, so the sorcery is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the targeted artifact left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "and it is in its owner's graveyard, not the caster's"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        3,
        "\"Add {{G}}{{G}}{{G}}\" — three green off the resolving spell"
    );
    assert_eq!(
        pool.total(),
        3,
        "and nothing else in the pool: the {{2}}{{G}} was spent and the effect \
         put exactly its own three back"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved"
    );
}

fn eerie_procession() -> CardIndex {
    card_index("58dde1b0-bb01-4f72-9408-21c0404c1cfd")
}

/// Eerie Procession is `{2}{U}` for exactly one sentence: "Search your library
/// for an Arcane card, reveal that card, put it into your hand, then shuffle."
///
/// The whole backing deck is Eerie Procession itself — an Arcane sorcery — so
/// every card the search can see matches the printed filter and the spell has
/// something to find; one copy of the same printing is seeded into the
/// graveyard first, which makes "your library" a claim about a *zone* and not
/// merely about a card. Reading the card file cannot replace playing it: that
/// the search is asked at all is the engine's answer, and only the card the
/// question offered turning up in hand tells a tutor from a spell that finds
/// nothing.
#[test]
fn eerie_procession_tutors_an_arcane_card_out_of_the_library_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, eerie_procession())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[eerie_procession()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The same printing, in a zone the spell does not name.
    seed_graveyard(&mut engine, p0, 1);
    let buried = in_graveyard(&engine, p0, eerie_procession()).expect("the seed landed");
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert!(!library_before.is_empty(), "there is a library to search");

    // {2}{U} off the three Islands, tapped before anything is claimed: what a
    // spell may be cast with is read off the pool and not off the board.
    cast_from_hand(&mut engine, p0, eerie_procession());

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
    assert_eq!(
        player, p0,
        "the seat that cast the spell does the searching"
    );
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "a tutor's own question, and not a scry, a discard or a sacrifice"
    );
    assert!(
        !options.is_empty(),
        "every card left in this library is an Arcane card, so the search \
         finds some"
    );
    assert!(
        !options.contains(&buried),
        "\"search your *library*\": the Arcane card in the graveyard is a \
         different zone and no find: {options:?}"
    );
    assert_eq!(
        options.len(),
        library_before.len(),
        "the filter is \"an Arcane card\" and nothing in this library is \
         anything else, so the whole of it is on the menu: {options:?}"
    );
    let chosen = options[0];

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("a card the search offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&chosen),
        "\"put that card into your hand\": the very card the search offered, \
         and not some other copy of the same printing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card up: the spell left the hand and the found card replaced it, \
         so a reveal that left the card in the library would read one short"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2,
        "the seeded Arcane card and the spell itself, which resolved rather \
         than being countered or left on the stack"
    );
}

fn eye_of_nowhere() -> CardIndex {
    card_index("27ffdf11-bb7f-40bf-94c4-6bfbc41668e5")
}

/// Eye of Nowhere is `{U}{U}` for one line: "Return target permanent to its
/// owner's hand." "Permanent" is the whole card — a creature and a land, on
/// either side of the table, are all legal targets — and "its owner's" is the
/// word that decides which hand the card lands in once the spell is aimed
/// across the table, which a bounce of one's own creature could not tell from
/// "controller's". Two Islands pay the `{U}{U}` with the Elf kept back (its
/// own `{T}: Add {G}` is a mana route too), so the blue is real mana out of
/// the pool, and it is still floating while the target question stands
/// (CR 601.2c before CR 601.2h).
#[test]
fn eye_of_nowhere_returns_any_permanent_to_its_owners_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), forest()])
        .hand(0, &[eye_of_nowhere()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_lands = all_on_battlefield(&engine, p0, island());
    assert_eq!(my_lands.len(), 2, "two Islands are the {{U}}{{U}}");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    // The two Islands, and the Elf named as the printing kept back: it is one
    // of the permanents the spell may name, and a source tapped for its own
    // mana would make "two blue" a claim about a board nothing accounts for.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands tapped, and no creature on this board paid in"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let spell = in_hand(&engine, p0, eye_of_nowhere()).expect("the spell is in hand");
    assert!(
        legal.castable.contains(&spell),
        "two blue in the pool pay {{U}}{{U}}: {:?}",
        legal.castable
    );
    cast_with_floating(&mut engine, p0, eye_of_nowhere());

    // "Any permanent": every permanent in the game is on the menu, on both
    // sides of the table and of both types the board offers.
    let options = options_offered_including(&mut engine, theirs);
    for id in [my_lands[0], my_lands[1], mine, theirs, their_land] {
        assert!(
            options.contains(&id),
            "\"target permanent\" reaches {id:?}: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "and those five are the whole battlefield: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "CR 601.2c names the target before CR 601.2h pays, so the mana is \
         still floating while the question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{U}}{{U}} came out of the pool"
    );
    assert!(
        on_stack(&engine, eye_of_nowhere()).is_some(),
        "and the spell is on the stack, waiting to resolve"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\": the Elf goes to the seat that owns it, not \
         to the seat that aimed the spell"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "the caster keeps nothing of what it bounced"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the targeted permanent left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "the permanent the spell did not name never moved"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, island()).len(),
        2,
        "and neither did the two Islands that paid for it"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "nor the creature that was kept back"
    );
    assert!(
        in_graveyard(&engine, p0, eye_of_nowhere()).is_some(),
        "a resolved sorcery goes to its owner's graveyard"
    );
}

fn fabricate() -> CardIndex {
    card_index("422e1869-134f-463d-9fa1-86b66a998b3e")
}

/// Fabricate — {2}{U} sorcery: "Search your library for an artifact card,
/// reveal it, put it into your hand, then shuffle."
///
/// The backing deck is a pile of Sol Rings rather than the usual Forests,
/// because the filter is `Filter::ARTIFACT`: over basic lands the search
/// offers nothing and an empty menu would satisfy "it looked at the library"
/// while the card found no card at all. The found object is followed by
/// identity and not by printing — the seat has drawn Sol Rings in its opening
/// hand already, so "a Sol Ring is in hand" was true before the spell
/// resolved and proves nothing. The library count is read with it because
/// "then shuffle" reorders what is left without changing how much of it there
/// is, and the hand is one card up only if the card actually moved.
#[test]
fn fabricate_tutors_an_artifact_card_out_of_the_library_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, quiet_artifact())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[fabricate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let library_before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, fabricate());
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
    assert_eq!(
        player, p0,
        "the seat that cast the spell does the searching"
    );
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    assert!(
        !options.is_empty(),
        "the library holds artifact cards to find"
    );

    // The spell has already left the hand for the stack, so this is the hand
    // the tutored card is about to join.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let in_library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    for id in &options {
        assert!(
            in_library.contains(id),
            "\"search your library\": {id:?} is no card in it"
        );
        assert!(
            types(&engine, *id).contains(TypeSet::ARTIFACT),
            "\"an artifact card\": {id:?} is not one, so the filter was skipped"
        );
    }

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("a card the search offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&chosen),
        "the very card the search offered reached the hand, and not another \
         copy of the same printing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "one card up — a reveal that left the card where it was could not do \
         this"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        in_graveyard(&engine, p0, fabricate()).is_some(),
        "the sorcery resolved and went to its owner's graveyard"
    );
}

fn farseek() -> CardIndex {
    card_index("495e52e6-4c2b-4574-9474-eadbdcc8b4ac")
}

/// Farseek — {1}{G} sorcery: "Search your library for a Plains, Island, Swamp,
/// or Mountain card, put it onto the battlefield tapped, then shuffle."
///
/// The library is a pile of Badlands: a *nonbasic* original dual carrying the
/// subtypes Swamp and Mountain and no enter modifier of its own, with one
/// Forest dropped on top of it by the harness. That single board reads the
/// whole card — every Badlands is on offer, so the filter is the printed land
/// *types* rather than "a basic land card" or a list of names, while the
/// Forest, the fifth basic land type and the top card of the library, is the
/// one card the offer leaves out. The fetched permanent is the very card the
/// question offered, standing tapped, where the same printing played from hand
/// as the turn's land drop is untapped — so the tapped clause is Farseek's
/// doing and not the land's.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn farseek_fetches_the_land_types_it_names_and_puts_one_down_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(173, badlands())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        // The second Badlands is the control below: the kit deals no opening
        // hand, so a land that is going to be *played* has to be named here.
        .hand(0, &[farseek(), badlands()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A Forest at the top of the library, where a search reading "a basic land
    // card" would find it first: the one card the printed filter does not name.
    let stowed = all_on_battlefield(&engine, p0, forest())[0];
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        state
            .move_object(
                stowed,
                ZoneLocation::Library(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("the harness moves a card");
    }
    // The offer was computed when priority was granted, which was before the
    // Forest left the battlefield: without this the mana walk is handed an
    // ability whose source is in the library and the engine refuses what it
    // just listed. `seed_graveyard` does the same thing for the same reason.
    engine.refresh_offer();
    let in_library = |engine: &Engine<RegistryLookup>, card: CardIndex| -> bool {
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .iter()
            .any(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
            })
    };
    assert!(
        in_library(&engine, forest()),
        "the library holds a card the filter does not name"
    );
    let library_before = library_size(&engine, p0);
    assert!(library_before > 1, "and the cards it does name beside it");

    // Mana before the claim: `castable` reads the pool and not the three
    // Forests still standing, and the {1}{G} comes out of what they fill.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, three green"
    );
    cast_with_floating(&mut engine, p0, farseek());

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
        ChoicePrompt::SearchLibrary,
        "a search of the library and not a scry or a discard"
    );
    assert_eq!(
        options.len(),
        library_before - 1,
        "every land the filter names is on offer and the Forest is the one it \
         does not: {library_before} cards in the library, {} in the offer",
        options.len()
    );
    for id in &options {
        let is_forest = engine
            .state()
            .object(*id)
            .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()));
        assert!(
            !is_forest,
            "a Forest is the fifth basic land type, which this card does not \
             name: {id:?}"
        );
    }

    let fetched = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fetched],
            },
        )
        .expect("a card the search offered is a legal answer");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine
            .state()
            .object(fetched)
            .expect("the fetched card is still an object")
            .zone,
        Zone::Battlefield,
        "\"put it onto the battlefield\" — the very card the question offered, \
         and not a card in hand"
    );
    assert!(
        types(&engine, fetched).contains(TypeSet::LAND),
        "what arrived is the land it was printed as"
    );
    assert!(
        is_tapped(&engine, fetched),
        "\"tapped\": the permanent the search put down is down"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "exactly one card left the library — the shuffle after the search \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        in_library(&engine, forest()),
        "and the card the filter does not name is still in it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{1}}{{G}} came out of the three green the Forests made"
    );

    // The control for the tapped clause: the same printing played from hand as
    // the turn's land drop enters untapped, because a Badlands prints no enter
    // modifier at all.
    let played = play_land(&mut engine, p0, badlands());
    assert!(
        !is_tapped(&engine, played),
        "a Badlands enters untapped on its own, so nothing about the card \
         explains the fetched one being tapped"
    );
}

fn fire_ambush() -> CardIndex {
    card_index("50463946-1ce3-4ff0-ad68-2fb87adbe2fd")
}

/// Fire Ambush — {1}{R} sorcery: "Fire Ambush deals 3 damage to any target."
///
/// "Any target" is the whole card (CR 115.4), so the board is built to tell
/// the two halves of that word apart: a creature under each seat shows up in
/// the object list and both players in the player list, and the damage is then
/// aimed at the opponent's creature rather than at the seat whose board it
/// stands on. Three damage to a printed 1/1 is lethal (CR 704.5f) while the
/// opponent's life total stays where it was — which is what says the number
/// landed on the creature and not on the player — and p0's own Elf, untouched,
/// is the control that the spell hit what it was aimed at and nothing else.
#[test]
fn fire_ambush_deals_three_damage_to_the_creature_it_targets_and_not_to_its_controller() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[fire_ambush()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "a 1/1 for three damage to kill"
    );

    // Two Mountains pay {1}{R}. The Elf is named as the thing kept back: it
    // prints its own {T}: Add {G}, so `tap_all_mana` would have spent it too
    // and every mana number below would be a claim about a third source.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Mountains tapped, and the Elf contributed nothing"
    );
    let life_before_p1 = engine.state().players[1].life;
    cast_with_floating(&mut engine, p0, fire_ambush());

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster is the one that aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"any target\" reaches either side of the table: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4 counts players in the same choice: {player_options:?}"
    );
    // The card is still in hand: `cast_wizard` asks every question
    // CR 601.2b–h poses and moves it to the stack last, at CR 601.2i, so the
    // announcement is atomic from the outside. What this reads instead is
    // the thing the assertion was really about — the damage has not been
    // dealt while the question is open (CR 608.2b).
    assert!(
        in_hand(&engine, p0, fire_ambush()).is_some(),
        "the card has not reached the stack yet (CR 601.2i comes last)"
    );
    assert_eq!(
        engine.state().players[1].life,
        life_before_p1,
        "and nothing has been dealt while the target is being named"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the creature the question offered was chosen");
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the target is still there while the sorcery sits on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "three damage to a printed 1/1 is lethal (CR 704.5f)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and the creature left the battlefield"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was named and never to its \
         controller"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the creature the spell did not name never moved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{R}} came out of the pool the two Mountains filled"
    );
    assert!(
        in_graveyard(&engine, p0, fire_ambush()).is_some(),
        "and the sorcery is in its owner's graveyard once it has resolved"
    );
}

fn fit_of_rage() -> CardIndex {
    card_index("69d08521-74e1-4215-9ed4-3f12137332a4")
}

/// Fit of Rage is one line — a `{1}{R}` sorcery: "target creature gets +3/+3
/// and gains first strike until end of turn" — and each of its three parts is
/// readable only off a board that carries a control for it. Two Elves stand
/// under the caster and a third across the table, so the target question has
/// to name all three (the card says "target creature", not "you control")
/// while the pump and the keyword may land on exactly the one that was
/// answered. Walking on into the following turn's main phase then reads the
/// "until end of turn" off that same creature, which is what separates a pump
/// from a permanent.
#[test]
fn fit_of_rage_pumps_and_arms_the_one_creature_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), llanowar_elves(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[fit_of_rage()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves of mine, one of which stays bare");
    let (target, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        pt(&engine, target),
        (1, 1),
        "a printed 1/1 before the spell"
    );
    assert!(
        !keywords(&engine, target).contains(KeywordSet::FIRST_STRIKE),
        "and no keyword at all yet"
    );

    // The two Mountains go into the pool first: castability is read off the
    // pool and not off the untapped lands.
    cast_from_hand(&mut engine, p0, fit_of_rage());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast the spell aims it");
    assert!(
        options.contains(&target) && options.contains(&bystander) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the creature the question offered is the one it was aimed at");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, target),
        (4, 4),
        "+3/+3 on the creature Fit of Rage named"
    );
    assert!(
        keywords(&engine, target).contains(KeywordSet::FIRST_STRIKE),
        "and the printed keyword arrives with it"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody named is still the 1/1 it was printed as, so the pump \
         is a target and not \"creatures you control\""
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FIRST_STRIKE),
        "and the keyword reaches no other body either"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the effect reaches the target and never across the table"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "nor does the keyword"
    );

    // "until end of turn" is a clause, not decoration: the same object read
    // after the turn has rolled over is the printed 1/1 again.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, target),
        (1, 1),
        "the pump expires with the turn it was cast in"
    );
    assert!(
        !keywords(&engine, target).contains(KeywordSet::FIRST_STRIKE),
        "and the granted keyword goes with it"
    );
}

fn goblin_offensive() -> CardIndex {
    card_index("25cb5c86-83cd-4a06-b0d1-a0f6fc9158a6")
}

/// Goblin Offensive is a sorcery reading "Create X 1/1 red Goblin creature
/// tokens." — X *is* the card, so the board is five Mountains, which is
/// exactly `{X}{1}{R}{R}` at X = 2 and nothing more: the spell is answered at
/// its X question (CR 601.2b draws X out of the announcer before any cost is
/// paid), the pool reads empty afterwards, and the tokens are counted on the
/// battlefield. Two Goblins is the only number that reads both halves of the
/// cost — an X of 1 would have left a Mountain's worth of red floating, and a
/// flood of tokens would mean the mana was never spent at all.
#[test]
fn goblin_offensive_creates_one_goblin_token_for_each_point_of_x() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[goblin_offensive()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spell = in_hand(&engine, p0, goblin_offensive()).expect("the spell is in hand");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing is on the board before the spell resolves"
    );

    // Mana into the pool first: `legal.castable` is filtered through
    // `can_afford`, which reads the pool and not the untapped lands.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Mountains tapped and the board holds nothing else that makes mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.castable.contains(&spell),
        "five red pays {{X}}{{1}}{{R}}{{R}} with X = 2: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, goblin_offensive());
    let Pending::ChooseNumber { player, min, max } = engine.pending().clone() else {
        panic!(
            "an X spell asks for its X before its cost is paid, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0, "the seat casting the spell names X");
    assert_eq!(min, 0, "X may be zero");
    assert!(
        max >= 2,
        "five Mountains reach exactly X = 2, so 2 is on the menu: {max}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "CR 601.2h pays last: the mana is untouched while the question stands"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(2))
        .expect("X = 2 is what the floating mana pays for");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{X}}{{1}}{{R}}{{R}} with X = 2 spends all five red"
    );
    let goblins = tokens_of(&engine, p0);
    assert_eq!(
        goblins.len(),
        2,
        "one 1/1 Goblin per point of X — and not one per red spent"
    );
    for goblin in goblins {
        let token = engine
            .state()
            .object(goblin)
            .and_then(|o| o.token)
            .expect("a created token knows which token it is");
        assert_eq!((token.power, token.toughness), (Some(1), Some(1)));
        assert!(
            token.colors.contains(baylee_core::color::Color::Red),
            "a 1/1 *red* Goblin"
        );
    }
}

fn howling_fury() -> CardIndex {
    card_index("eda60752-d225-4fd0-9f0f-9b99e321b8fa")
}

/// Howling Fury prints one line — "Target creature gets +4/+0 until end of
/// turn" — and the board is built so that a single play of it settles every
/// word at once. A 1/1 Elf stands under the caster and another across the
/// table, so the target question has to offer both and the `(5, 1)` that
/// follows has to land on exactly the one that was named: a pump that had
/// lost its target would raise both, and one that read the wrong half of the
/// pair would leave a printed 1/1 a 1/1. `+4/+0` and not `+4/+4` is the other
/// half — a toughness pumped with the power reads `(5, 5)`. The last
/// assertion is the duration: by the following main phase the pump is gone,
/// which is what separates "until end of turn" (CR 514.2) from a counter
/// that would still be sitting there.
#[test]
fn howling_fury_pumps_only_the_creature_it_targets_and_only_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[howling_fury()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the spell");
    assert_eq!(pt(&engine, theirs), (1, 1), "and one across the table");

    // `cast_from_hand` taps first: three Swamps and the Elves' own `{G}` are
    // four mana, and `{2}{B}` is three of them.
    cast_from_hand(&mut engine, p0, howling_fury());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the target is named first (CR 601.2c) and the cost is the last step \
         of the cast (CR 601.2h), so the whole four are still floating"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{2}}{{B}} comes out of the pool only now, leaving the \
         Elves' one green as the change"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (5, 1),
        "+4/+0 on the creature it named — a (5, 5) would be a toughness the \
         card does not print"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and no other"
    );

    // The duration. A pump that never expired would read (5, 1) here too, so
    // only the next turn's main phase can tell the two apart.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "\"until end of turn\": the cleanup step of the turn it was cast in \
         took the +4/+0 back, so the Elf is the 1/1 it was printed as"
    );
}

// oracle_id = "cedc52eb-66a6-4b43-87f1-9bb9f4d4871e"
fn lay_of_the_land() -> CardIndex {
    card_index("cedc52eb-66a6-4b43-87f1-9bb9f4d4871e")
}

/// Lay of the Land — {G} sorcery: "Search your library for a basic land card,
/// reveal it, put it into your hand, then shuffle."
///
/// The printed sentence is a *move* and the test reads it as one: the card the
/// search offered is the very object in hand afterwards, the library is one
/// shorter for it, and the hand grew by exactly one. A question that was asked
/// and answered satisfies none of those three, which is why the assertion is
/// on identity and on the two zone counts rather than on the prompt alone. The
/// backing deck is sixty basic lands, so the filter has something to find, and
/// the sorcery's own graveyard entry is the control that says the spell
/// resolved instead of being answered into a fizzle.
#[test]
fn lay_of_the_land_finds_a_basic_land_and_puts_it_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[lay_of_the_land()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The {G} comes out of the Forest for real, and the sorcery goes on the
    // stack; the search below is asked during its resolution.
    cast_from_hand(&mut engine, p0, lay_of_the_land());
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
    assert_eq!(
        player, p0,
        "the seat that cast the sorcery does the searching"
    );
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "a tutor's own question, and not a scry or a discard"
    );
    assert!(
        !options.is_empty(),
        "the filler library is sixty basic lands, so there is something to find"
    );
    let in_library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    for id in &options {
        assert!(
            in_library.contains(id),
            "\"search your library\": {id:?} is no card in it"
        );
    }

    // Both counts are taken while the question stands: the sorcery has already
    // left the hand for the stack, so what the search adds is read against the
    // hand the cast left behind.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&engine, p0);
    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("a card the search offered is a legal answer");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&chosen),
        "\"put it into your hand\": the very card the search offered, and not \
         some other copy of the same printing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "one card up, which a reveal that left the card where it was could not do"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        in_graveyard(&engine, p0, lay_of_the_land()).is_some(),
        "the sorcery resolved and went to its owner's graveyard, which is \
         where a sorcery goes — a spell that fizzled would still be on the \
         stack or nowhere at all"
    );
}

fn monstrous_growth() -> CardIndex {
    card_index("35a05836-38d7-45c7-ac9a-996a682c2129")
}

/// Monstrous Growth prints one sentence — "{1}{G} sorcery: Target creature
/// gets +4/+4 until end of turn." — and the board makes every word of it
/// load-bearing. "Target creature" reaches either side of the table, so the
/// opponent's Elf is the option list's other half and the pump must land only
/// on the creature that was named; +4/+4 on a printed 1/1 reads (5, 5), which a
/// +4/+0 or a +0/+4 could not produce; and the {1}{G} is a real payment out of
/// a pool two Forests filled, with nothing floating while the target question
/// stands (CR 601.2c before CR 601.2h).
#[test]
fn monstrous_growth_pumps_the_creature_it_targets_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[monstrous_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the spell");

    // Three Forests pay the {2} with the Elves named as the printing kept
    // back: they are the creatures this test reads afterwards, and a host
    // tapped for its own mana reads wrong down the page.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three tapped Forests and neither Elf"
    );

    cast_with_floating(&mut engine, p0, monstrous_growth());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures under my own control are offered: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is not \"creature you control\": the Elf across \
         the table is a legal target too: {options:?}"
    );
    assert!(
        !options.contains(&on_battlefield(&engine, p0, forest()).expect("my Forest is out")),
        "a land is no creature at all: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "CR 601.2h pays **after** CR 601.2c chooses, so the three green are \
         still floating while the target question is open"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, host),
        (5, 5),
        "+4/+4 on the creature that was named"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody targeted is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the static never reaches across the table to a creature it did \
         not name"
    );
    assert!(
        in_graveyard(&engine, p0, monstrous_growth()).is_some(),
        "the sorcery resolved and went to its owner's graveyard"
    );
}

fn natures_lore() -> CardIndex {
    card_index("78826359-fe63-44ad-adc4-a17ffcd710e4")
}

/// Nature's Lore ({1}{G}) prints one sentence: search your library for a
/// **Forest card**, put it onto the battlefield, then shuffle. Two Forests pay
/// for the spell before anything is claimed about the search, and the search
/// is read as a move rather than as a question that was asked: the card the
/// offer named is the card now standing on the battlefield, the library is one
/// shorter, and the sorcery itself left the hand. The fetched land arrives
/// **untapped**, which is exactly the word the card prints and the one a
/// `Find::TAPPED` spelling would have quietly lost beside a board that already
/// had two tapped Forests on it.
#[test]
fn natures_lore_fetches_a_forest_onto_the_battlefield_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[natures_lore()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        lands_of(&engine, p0).len(),
        2,
        "two Forests are the whole board the spell is paid with"
    );

    cast_from_hand(&mut engine, p0, natures_lore());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the caster does the searching");
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "one card, and the search is not optional"
    );
    assert_eq!(
        options.len(),
        library_before,
        "\"a Forest card\": every card left in this library qualifies, so the \
         offer is the whole library and nothing was silently dropped from it"
    );

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("a card the search offered is a legal answer");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(chosen)
            .expect("the fetched card exists")
            .zone,
        Zone::Battlefield,
        "\"put that card onto the battlefield\""
    );
    assert!(
        engine
            .state()
            .object(chosen)
            .expect("the fetched card exists")
            .characteristics()
            .types
            .contains(TypeSet::LAND),
        "and what landed is the land the search offered"
    );
    assert!(
        !is_tapped(&engine, chosen),
        "\"onto the battlefield\" untapped — a `Find::TAPPED` spelling would \
         have put it down sideways beside the two Forests that paid"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        3,
        "it joined the two Forests the spell was paid with"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "and the sorcery itself left the hand — a search that never resolved \
         would leave every pile exactly where it was"
    );
}

fn night_s_whisper() -> CardIndex {
    card_index("7ffae8f8-3006-4969-a339-6d30678f87ea")
}

/// Night's Whisper prints one line — "You draw two cards and lose 2 life" —
/// on a `{1}{B}` sorcery, and neither half is visible in the other's reading.
/// The draws are asserted off the library *and* the hand, because a library
/// two cards shorter with a hand that never grew would satisfy a count alone;
/// the life is read against both seats so that "you" is checked rather than
/// assumed. And the 2 life is read while the spell still stands on the stack —
/// it is an effect and not a cost, so it is nowhere near CR 601.2h, and a card
/// that had paid it at cast time would show 18 there already.
#[test]
fn nights_whisper_draws_two_cards_and_takes_two_life_from_its_controller() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[night_s_whisper()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the Swamps are tapped"
    );
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert!(
        in_hand(&engine, p0, night_s_whisper()).is_some(),
        "the spell starts where it is cast from"
    );

    // Two Swamps pay `{1}{B}`, so the cast is a real payment and not a label.
    cast_from_hand(&mut engine, p0, night_s_whisper());
    assert!(
        on_stack(&engine, night_s_whisper()).is_some(),
        "the sorcery is on the stack, and neither half of it has happened yet"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "losing the 2 life is the spell's effect, not its cost, so the life \
         is still there while the stack holds it (CR 601.2c before 601.2h)"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the card left the hand for the stack"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "and nothing has been drawn off the top of the library yet"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the `{{1}}{{B}}` came out of the pool the two Swamps filled"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "\"you draw two cards\" — two off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "one card gone to the stack and two drawn brings the hand one up, \
         which a library that merely emptied could not show"
    );
    assert_eq!(
        engine.state().players[0].life,
        18,
        "\"and lose 2 life\" — the life belongs to the seat that cast it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and never to the opponent: the card says \"you\" twice"
    );
    assert!(
        in_graveyard(&engine, p0, night_s_whisper()).is_some(),
        "a resolved sorcery goes to its owner's graveyard"
    );
}

// Sacred Nectar prints one sentence — "{1}{W} sorcery: You gain 4 life." —
// and that number is the whole card, so the board is built to make three
// readings disagree: twenty life that has to become twenty-four, a {1}{W}
// that has to leave both Plains tapped and the pool empty, and a life swing
// that must be *four* rather than one per mana spent. The spell is a
// sorcery, so the life is read twice — still twenty while it sits on the
// stack, and twenty-four only once it has resolved and gone to the
// graveyard.
fn sacred_nectar() -> CardIndex {
    card_index("30870ee5-6ad7-48a9-983e-d3b018f2344f")
}

#[test]
fn sacred_nectar_gains_four_life_when_it_resolves_off_a_generic_and_a_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(311, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[sacred_nectar()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sources = all_on_battlefield(&engine, p0, plains());
    assert_eq!(sources.len(), 2, "two Plains are the whole of {{1}}{{W}}");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing has been gained yet"
    );

    // Mana first, then the claim: `castable` is read off the pool.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two white floating, which is exactly the printed cost"
    );
    cast_with_floating(&mut engine, p0, sacred_nectar());

    assert!(
        !stack_is_empty(&engine),
        "a sorcery uses the stack, so the life is not gained on announcement"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{W}} came out of the pool"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "still twenty while the spell is waiting to resolve"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        24,
        "\"You gain 4 life\" — four, and not one life per mana that paid it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the caster, not to the opponent"
    );
    assert!(
        in_graveyard(&engine, p0, sacred_nectar()).is_some(),
        "a resolved sorcery goes to its owner's graveyard"
    );
}

/// Scorching Spear is one printed line — "Scorching Spear deals 1 damage to
/// any target" — and "any target" (CR 115.4) is the whole of what a board can
/// hold it to: one question, one choice, with the permanents and the seats
/// arriving in the same `ChooseTargets`. The two copies in hand are aimed at
/// one of each because that pair is what pins the answer's list down — the Elf
/// across the table dies with its controller's life untouched, and then the
/// seat itself loses the life while every creature on the board still stands.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn scorching_spear_deals_one_damage_to_a_creature_or_a_player_and_to_nothing_else() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), llanowar_elves()])
        .hand(0, &[scorching_spear(), scorching_spear()])
        .battlefield(1, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, theirs), (1, 1), "a printed 1/1 for one damage");

    // {R} off the two Mountains. The target is named before the cost is paid
    // (CR 601.2c, then CR 601.2h), so the spell is already on the stack while
    // this question stands.
    cast_from_hand(&mut engine, p0, scorching_spear());
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster does the aiming");
    assert_eq!((min, max), (1, 1), "one target, and the spell requires one");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"any target\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: players are the other half of the same choice: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![theirs],
                players: vec![],
            },
        )
        .expect("the Elf across the table was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one damage to a printed 1/1 is lethal (CR 704.5f)"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature the spear did not name never moved"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature, not to the seat whose board it stood on"
    );

    // The same question again, answered out of the other half of the list.
    cast_from_hand(&mut engine, p0, scorching_spear());
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!(
            "the second spear asks the same choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        player_options.contains(&p1),
        "\"any target\" reaches the seat itself: {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "one damage to the player who was named"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the life belongs to that player, not to the caster"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "no creature was named the second time, so none moved"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2,
        "both spears resolved and went to their owner's graveyard"
    );
}

/// Serum Visions — {U} sorcery: "Draw a card. Scry 2."
///
/// The two printed sentences run in one resolution and each leaves its mark
/// somewhere different, which is why they are read off the same cast: the draw
/// is the object that was on top of the library now sitting in hand, and the
/// scry is the question that follows it. The question has to be asked about
/// the top two cards *after* the draw — `second` and `third` of the library as
/// it stood before the cast — so a scry that looked at the pre-draw top, or a
/// draw that happened second, would name the wrong two. The library is one card
/// shorter afterwards and the card the search offered lies on the bottom, which
/// is what tells a reorder with a look from a draw.
#[test]
fn serum_visions_draws_a_card_and_then_scries_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &[serum_visions()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The library as it stands before anything is cast: the list's last entry
    // is the top card, its first is the bottom — the order `Effect::Scry`
    // reads the top `n` in and the end `ZonePosition::Bottom` writes to.
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];
    let third = library_before[library_before.len() - 3];

    cast_from_hand(&mut engine, p0, serum_visions());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the caster does the looking");
    assert_eq!(prompt, crate::choice::ChoicePrompt::ScryBottom);
    assert_eq!(
        options,
        vec![second, third],
        "the top two cards *after* the draw: the card that was on top is in \
         hand and must not be on the scry's menu"
    );
    assert_eq!(
        (min, max),
        (0, 2),
        "either, both or neither may be bottomed"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![second],
            },
        )
        .expect("one of the two the scry just looked at");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library.first().copied(),
        Some(second),
        "the chosen card is bottomed"
    );
    assert_eq!(
        library.last().copied(),
        Some(third),
        "the card left alone is the new top"
    );
    assert_eq!(
        library.len(),
        library_before.len() - 1,
        "\"Draw a card\" took one off the top and a scry only reorders"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "the drawn card is the very object that was on top, which the filler \
         deck's identical printings cannot stand in for"
    );
    assert!(
        in_graveyard(&engine, p0, serum_visions()).is_some(),
        "and the sorcery itself resolved into its owner's graveyard"
    );
}

fn sinkhole() -> CardIndex {
    card_index("5a46ad2a-35b1-4dd5-b7c3-fec36b7c67ab")
}

/// Sinkhole costs {B}{B} and prints one sentence: "Destroy target land."
///
/// The word worth playing is "land", so the board carries one of each thing
/// the filter could have widened into: a Forest across the table with an Elf
/// standing beside it, which a bare `Filter::Any` would have offered as a
/// target too. Both lands *are* offered — "target land" reaches either side of
/// the table — and the permanent that is not named has to be exactly where it
/// was afterwards, which is what separates a destroy aimed at one target from
/// a spell that wrecked the board it landed on.
#[test]
fn sinkhole_destroys_the_land_it_names_and_leaves_the_rest_of_the_table_alone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .hand(0, &[sinkhole()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let my_land = on_battlefield(&engine, p0, swamp()).expect("my Swamp is out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // The two Swamps pay the {B}{B} before the offer is read, because
    // `legal`/the target list is what the pool funds and not what the
    // untapped lands could have funded.
    cast_from_hand(&mut engine, p0, sinkhole());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert!(
        options.contains(&my_land) && options.contains(&their_land),
        "\"target land\" reaches either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&their_elf),
        "an Elf is a creature and no land, so it may not be named: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_land],
            },
        )
        .expect("the Forest was one of the options the question enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "the land the spell named left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "and a destroyed land goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, swamp()).is_some(),
        "the land the spell did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "nor did the creature standing beside it"
    );
    assert!(
        in_graveyard(&engine, p0, sinkhole()).is_some(),
        "and the sorcery itself is in its caster's graveyard"
    );
}

fn steelshaper_s_gift() -> CardIndex {
    card_index("d9abda7e-6ca2-42ea-ab24-c542e57014f1")
}

/// Steelshaper's Gift ({W} sorcery) prints one sentence: "Search your library
/// for an Equipment card, reveal that card, put it into your hand, then
/// shuffle." The library here is a stack of Lightning Greaves — an Equipment,
/// and one that costs {0}, so the same scenario can play what it fetched and
/// show the search delivered a castable card rather than a name.
///
/// `Find::HAND` is the half that separates this from Wayfarer's Bauble and
/// Journeyer's Kite, which put the card onto the battlefield instead: the
/// chosen object is read in *hand*, the library loses exactly one card to a
/// shuffle that reorders without resizing, and the Gift itself is spent into
/// the graveyard.
#[test]
fn steelshaper_s_gift_tutors_an_equipment_card_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, lightning_greaves())
        // Three Plains: {W} for the Gift, and the {2} the Greaves themselves
        // cost — it is the *equip* that is free, not the artifact.
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[steelshaper_s_gift()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, steelshaper_s_gift());

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
    assert_eq!(player, p0, "the seat that cast it does the searching");
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "a tutor's own question, and not a scry or a discard"
    );
    assert!(
        !options.is_empty(),
        "the library is a stack of Equipment cards to find"
    );
    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("a card the search offered is a legal answer");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&chosen),
        "\"put it into your hand\": the very card the search offered, in hand"
    );
    assert_eq!(
        engine
            .state()
            .object(chosen)
            .and_then(|o| o.card)
            .map(|c| c.index),
        Some(lightning_greaves()),
        "and it is the Equipment the library is made of, not some other card"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        in_graveyard(&engine, p0, steelshaper_s_gift()).is_some(),
        "a sorcery that resolved is in its owner's graveyard"
    );

    // The tutor is only worth anything if what it fetched can be played:
    // the {2} left floating after the Gift is exactly the Greaves' own cost,
    // and an artifact needs a main phase its own seat holds priority in
    // (CR 117.1a).
    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.state().turn.phase, Phase::FirstMain | Phase::SecondMain)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    cast_with_floating(&mut engine, p0, lightning_greaves());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, lightning_greaves()).is_some(),
        "the Equipment the Gift fetched is a real card: it arrived on the \
         battlefield without a single source being tapped"
    );
}

// oracle_id = "1b882a0e-0ede-4d1a-bd1a-9b7cffbcde8e"
fn three_visits() -> CardIndex {
    card_index("1b882a0e-0ede-4d1a-bd1a-9b7cffbcde8e")
}

/// Three Visits is `{1}{G}` for "Search your library for a Forest card, put
/// it onto the battlefield, then shuffle."
///
/// The words worth playing are *onto the battlefield*: the found land arrives
/// as a land and not as a card in hand, and it arrives **untapped** — the
/// clause the Rampant-Growth-shaped tutors in this pool do not share
/// (Wayfarer's Bauble puts its own fetch in tapped, and its test says so).
/// So one scenario reads the zone the found card went to, its tapped state,
/// the library it left and the hand it did *not* join, with both Forests
/// spent to pay the spell so nothing is floating for a land to have been
/// paid out of.
#[test]
fn three_visits_finds_a_forest_and_puts_it_onto_the_battlefield_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[three_visits()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        lands_of(&engine, p0).len(),
        2,
        "two Forests and nothing else: they are the whole of the {{1}}{{G}}"
    );

    cast_from_hand(&mut engine, p0, three_visits());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the two Forests paid the {{1}}{{G}}, so nothing is left floating"
    );

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
        ChoicePrompt::SearchLibrary,
        "a tutor's own question, and not a scry or a discard"
    );
    assert!(
        !options.is_empty(),
        "the library holds Forest cards to find"
    );
    let in_library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    for id in &options {
        assert!(
            in_library.contains(id),
            "\"search your library\": {id:?} is no card in it"
        );
    }

    let found = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .expect("a card the search offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(found)
            .expect("the found card is still an object")
            .zone,
        Zone::Battlefield,
        "\"put it onto the battlefield\" — not into hand, and no longer in the \
         library"
    );
    assert!(
        !is_tapped(&engine, found),
        "the card prints no \"tapped\", so the Forest arrives standing — the \
         clause Wayfarer's Bauble's own fetch does have"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        3,
        "the battlefield gained exactly one land"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the hand lost the sorcery and gained nothing: a fetch that had put \
         the land into hand would read the same size it started at"
    );
    assert!(
        in_graveyard(&engine, p0, three_visits()).is_some(),
        "the sorcery itself resolved and is in its owner's graveyard"
    );
}

fn time_of_need() -> CardIndex {
    card_index("4f2adfd0-c8ab-4bcc-ae55-cb0e798aec7f")
}

/// Time of Need — {1}{G} sorcery: "Search your library for a legendary creature
/// card, reveal it, put it into your hand, then shuffle."
///
/// The library is fifty-odd copies of Katara, the Fearless, so every card the
/// filter could match is a legendary creature and the question's surface is
/// exactly as wide as the printed sentence — the mana cost is paid by two real
/// Forests, and no source on the board can pay it twice.
///
/// The card is only itself as a *move*, though: the question has to arrive as a
/// `ChoicePrompt::SearchLibrary`, and the card it offered has to be the card in
/// hand afterwards with the library one shorter. A search that showed a card
/// and left it where it was would satisfy a test that only checked that some
/// question had been asked.
#[test]
fn time_of_need_searches_a_legendary_creature_out_of_the_library_into_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, katara_the_fearless())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[time_of_need()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, time_of_need());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(
        player, p0,
        "the seat that cast the sorcery does the searching"
    );
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry, a surveil or a discard"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "\"a legendary creature card\" is one card, and the printing is not \
         optional"
    );
    let in_library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    for id in &options {
        assert!(
            in_library.contains(id),
            "\"search your library\": {id:?} is no card in it"
        );
    }
    assert!(
        !options.is_empty(),
        "the library holds legendary creatures to find: {options:?}"
    );
    let chosen = options[0];

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("a card the search offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    let found = engine
        .state()
        .object(chosen)
        .expect("the searched card is still an object");
    let chars = found.characteristics();
    assert!(
        chars.supertypes.contains(SupertypeSet::LEGENDARY)
            && chars.types.contains(TypeSet::CREATURE),
        "\"a legendary creature card\": the find is a legendary creature and \
         not merely some card: {chars:?}"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&chosen),
        "\"put it into your hand\": the very card the search offered, and not \
         some other copy of the same printing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the hand is where it started: the spell left it and the creature it \
         found took that place, which a search that left the card in the \
         library could not do"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        in_graveyard(&engine, p0, time_of_need()).is_some(),
        "and the sorcery itself is where a resolved instant or sorcery goes"
    );
}

// oracle_id = "38ea22cd-2c5d-4f66-a111-207aca4c67c3"
fn vicious_hunger() -> CardIndex {
    card_index("38ea22cd-2c5d-4f66-a111-207aca4c67c3")
}

/// Vicious Hunger is `{B}{B}` for "Vicious Hunger deals 2 damage to target
/// creature and you gain 2 life." The board is built so that neither half can
/// be borrowed from anything else: the creature it is aimed at is a printed
/// 1/1, which is exactly what two damage is lethal to, and the other two
/// creatures on the table are the controls that say the spell is *aimed*
/// rather than applied to the board — the second Elf across the table and the
/// one under the caster's own control are both still printed 1/1s afterwards.
/// Nothing on the table can move a life total, so the one point of life the
/// caster is up is the printed clause and not a land or a mana creature's.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn vicious_hunger_kills_a_printed_one_one_and_gains_exactly_two_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .hand(0, &[vicious_hunger()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my own Elf is out");
    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two Elves across the table, one of which dies"
    );
    let (prey, spare) = (elves[0], elves[1]);
    assert_eq!(
        pt(&engine, prey),
        (1, 1),
        "a printed 1/1 is exactly what two damage is lethal to"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "and so is the bystander on this side of the table"
    );

    // Both Swamps and the Elf feed the cost, which is why the green is counted
    // below rather than assumed away: a mana creature is a mana route too
    // (#159), and `tap_all_mana` presses it.
    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 2, "two Swamps, two black");
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "and the Elf's own {{G}}, which no black cost can spend"
    );

    cast_with_floating(&mut engine, p0, vicious_hunger());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that cast it is the one that aims it");
    assert_eq!(
        (min, max),
        (1, 1),
        "\"target creature\" is exactly one target"
    );
    assert!(
        options.contains(&prey) && options.contains(&spare) && options.contains(&mine),
        "\"target creature\" reaches every creature on the table, both sides \
         included: {options:?}"
    );
    assert!(
        player_options.is_empty(),
        "and names no player — the damage goes to a creature: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![prey],
            },
        )
        .expect("the Elf was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "two damage on a printed 1/1 is lethal (CR 704.5f)"
    );
    assert_eq!(
        on_battlefield(&engine, p1, llanowar_elves()),
        Some(spare),
        "and the damage is aimed: the Elf nobody named never moved"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "\"target creature\" is not \"the table\": the caster's own Elf is untouched"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "\"you gain 2 life\" — two, and not a point per creature or per damage"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature that was targeted and never to its controller"
    );
    assert!(
        in_graveyard(&engine, p0, vicious_hunger()).is_some(),
        "a resolved sorcery goes to its owner's graveyard"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "the {{B}}{{B}} came out of the pool"
    );
    assert_eq!(
        pool.total(),
        1,
        "and what is left is the Elf's green, which could not have paid it"
    );
}

fn volcanic_hammer() -> CardIndex {
    card_index("98fa5a06-0553-40fd-999c-bc31c9b3f4db")
}

/// Volcanic Hammer is a `{1}{R}` sorcery printing one sentence: "Volcanic
/// Hammer deals 3 damage to any target." The card is played twice off four
/// Mountains in one main phase, because "any target" is two lists in one
/// choice (CR 115.4) and a single cast can only exercise one of them: the
/// first Hammer is aimed at the opponent, whose life total reads the printed
/// number exactly (20 → 17, not 2 and not 4), and the second at the Elf
/// across the table, which dies while that same life total does not move
/// again.
///
/// Every bystander is a control. The Elf is standing while the first target
/// question is open — the offer carries it beside both players, and answering
/// with a player has to leave it where it is — and the second cast is paid
/// with the two red the first one left floating in the same phase (CR 500.5)
/// rather than with a source tapped for it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn volcanic_hammer_deals_three_to_a_player_and_then_to_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(311, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[volcanic_hammer(), volcanic_hammer()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, their_elf), (1, 1), "a printed 1/1");

    // Both casts come out of the same pool: four Mountains, four red, and a
    // pool that survives until the step ends (CR 500.5).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Mountains, and no other mana source on this board"
    );

    cast_with_floating(&mut engine, p0, volcanic_hammer());
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster aims it");
    assert_eq!((min, max), (1, 1), "one target, and the card asks once");
    assert!(
        options.contains(&their_elf),
        "the creature across the table is one of the object options: {options:?}"
    );
    assert!(
        player_options.contains(&p1) && player_options.contains(&p0),
        "CR 115.4: \"any target\" counts players in the same choice: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        17,
        "3 damage to a player is exactly three life"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing was dealt to the seat that cast it"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the Hammer did not name never moved"
    );
    assert!(
        in_graveyard(&engine, p0, volcanic_hammer()).is_some(),
        "a resolved sorcery goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "the {{1}}{{R}} came out of the four red, and the two left are the \
         second cast's"
    );

    // The other half of "any target", off the same floating two.
    cast_with_floating(&mut engine, p0, volcanic_hammer());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&their_elf),
        "the same Elf is a legal target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_elf],
            },
        )
        .expect("the creature was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "three damage to a printed 1/1 is lethal (CR 704.5f)"
    );
    assert_eq!(
        engine.state().players[1].life,
        17,
        "the second Hammer was aimed at the creature, so the life total does \
         not move again"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the two red the first cast left behind paid for it"
    );
}

fn wielding_the_green_dragon() -> CardIndex {
    card_index("22fa9020-783e-4292-857e-9d45a3458d71")
}

/// Wielding the Green Dragon costs {1}{G} and prints one sentence: "Target
/// creature gets +4/+4 until end of turn." The creature that is chosen is the
/// **opponent's** Elf, because `Filter::CREATURE` reaches across the table and
/// a pump quietly narrowed to "you control" would never have offered it — while
/// the printed 1/1 under the caster's own control is the control that says the
/// +4/+4 landed on the creature the spell named and not on the whole board.
/// `(5, 5)` is the only body that reads both printed numbers: a `(5, 1)` would
/// mean the toughness half was dropped, and a surviving 1/1 anywhere means the
/// effect was a board buff.
#[test]
fn wielding_the_green_dragon_pumps_only_the_creature_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves()])
        .hand(0, &[wielding_the_green_dragon()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");
    assert_eq!(pt(&engine, theirs), (1, 1), "and one across the table");

    cast_from_hand(&mut engine, p0, wielding_the_green_dragon());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and those two creatures are the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, theirs),
        (5, 5),
        "+4/+4 on the creature the spell named"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "and nothing at all for the creature it did not target — a pump that \
         had reached every creature on the board would leave this at (5, 5)"
    );
}

fn scorching_spear() -> CardIndex {
    card_index("9342fbb8-ab35-4895-946d-951ba6a2b067")
}

fn serum_visions() -> CardIndex {
    card_index("56956afd-db53-4542-816b-490c8b0bbcf7")
}

/// Emeria's Call: "Create two 4/4 white Angel Warrior creature tokens with
/// flying. Non-Angel creatures you control gain indestructible until your
/// next turn." The two halves read each other, which is why the token's
/// **Warrior** half of the type line is not decoration: the rider spares
/// Angels, so a pair made from the pool's plain 4/4 white flying Angel
/// would look identical on the board and be the same card — the assertion
/// that separates them is that the Angels do *not* gain indestructible
/// while the Elves beside them do.
#[test]
fn emeria_s_call_makes_two_angel_warriors_its_own_rider_then_spares() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9106, forest())
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
        .hand(0, &[emeria_s_call()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are seated");
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::INDESTRUCTIBLE),
        "nothing has been cast yet"
    );

    cast_from_hand(&mut engine, p0, emeria_s_call());
    pass_until(&mut engine, stack_is_empty);

    let made = tokens_of(&engine, p0);
    assert_eq!(made.len(), 2, "\"Create two … tokens\"");
    for angel in &made {
        assert_eq!(pt(&engine, *angel), (4, 4), "the printed 4/4");
        let wings = keywords(&engine, *angel);
        assert!(
            wings.contains(KeywordSet::FLYING),
            "\"with flying\": {wings:?}"
        );
        assert!(
            !wings.contains(KeywordSet::INDESTRUCTIBLE),
            "the rider reads \"non-Angel creatures\", and these are Angels"
        );
        let printed = engine
            .state()
            .object(*angel)
            .expect("the Angel is on the battlefield")
            .token
            .expect("it knows which token it is");
        assert_eq!(
            printed.name, "Angel Warrior",
            "not the pool's plain Angel, which the rider would have caught"
        );
    }
    assert!(
        keywords(&engine, elves).contains(KeywordSet::INDESTRUCTIBLE),
        "\"Non-Angel creatures you control gain indestructible\""
    );
}

fn pillage() -> CardIndex {
    card_index("0b137853-7cb9-424b-8285-12938991eafb")
}

/// Pillage: "Destroy target artifact or land. It can't be regenerated."
///
/// The clause on a card that names no creature at all, which is the case it
/// looks pointless in until an animated land is standing there. Spawning
/// Pool's `{1}{B}` turns it into a 1/1 Skeleton that keeps every land type
/// it had — so it is a legal target for Pillage *and* it can shield itself,
/// and those two facts meeting is the only board on which this sentence of
/// Pillage does any work.
///
/// Four Swamps and three Mountains: the animation takes `{1}{B}`, the
/// granted regeneration another `{B}`, and `mana_pay::pay_any` settles a
/// generic symbol out of `ManaColor::ALL` in order, so each `{1}` reaches
/// for the black before the red.
#[test]
fn pillage_destroys_an_animated_land_that_shielded_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(513, forest())
        .battlefield(
            0,
            &[
                spawning_pool(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[pillage()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let pool = on_battlefield(&engine, p0, spawning_pool()).expect("the manland is seated");
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, spawning_pool(), 1);
    pass_until(&mut engine, |e| at_rest(e, p0));
    let types = engine.state().object(pool).unwrap().characteristics().types;
    assert!(
        types.contains(TypeSet::CREATURE) && types.contains(TypeSet::LAND),
        "a Skeleton that is still a land, which is what puts it in Pillage's menu"
    );

    raise_a_shield(&mut engine, p0, pool, crate::choice::GRANTED_ABILITY);

    cast_with_floating(&mut engine, p0, pillage());
    let menu = aim_at(&mut engine, p0, pool);
    assert!(menu.contains(&pool), "an animated land is a land: {menu:?}");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, spawning_pool()).is_none(),
        "the shield the land bought itself did not save it"
    );
    assert!(
        in_graveyard(&engine, p0, spawning_pool()).is_some(),
        "and the land is in its owner's graveyard"
    );
}

fn damn() -> CardIndex {
    card_index("b01d61cc-9844-4191-86a0-f2db6d42d6e5")
}

/// Damn, overloaded: "Destroy target creature. A creature destroyed this way
/// can't be regenerated." with "target" read as "each" (CR 702.96a).
///
/// The overload is the sweeping half, so the clause it carries is
/// `destroy_all_no_regen` rather than `destroy_no_regen` — a second door,
/// written the same day and just as able to be wired to the wrong one. The
/// Elves beside the Troll are what say the mode was overloaded at all:
/// nothing was targeted, and both creatures die.
#[test]
fn damn_overloaded_sweeps_a_shielded_creature_away_with_an_unshielded_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(514, forest())
        .battlefield(
            0,
            &[
                lotleth_troll(),
                llanowar_elves(),
                // Three Swamps and not two. The shield eats a black, and
                // with one left the normal mode's {B}{B} is unaffordable —
                // the engine then has one legal mode, asks nothing, and the
                // choice this test is about never happens.
                swamp(),
                swamp(),
                swamp(),
                plains(),
                plains(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[damn()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let troll = on_battlefield(&engine, p0, lotleth_troll()).expect("the Troll is seated");
    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, troll, 1);

    cast_with_floating(&mut engine, p0, damn());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("a modal spell asks which mode, got {:?}", engine.pending())
    };
    assert!(
        options
            .iter()
            .any(|o| matches!(o.kind, crate::choice::CastModeKind::Mode(0))),
        "the normal mode is affordable too, so the overload below is a \
         choice and not the only thing left: {options:?}"
    );
    let overload = options
        .iter()
        .position(|o| matches!(o.kind, crate::choice::CastModeKind::Mode(1)))
        .expect("the overload is the second mode and it is affordable");
    engine
        .apply(p0, PlayerAction::ChooseMode(overload))
        .expect("the overload cost is floating");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the unshielded creature died, which is the sweep happening at all"
    );
    assert!(
        in_graveyard(&engine, p0, lotleth_troll()).is_some(),
        "and the shielded one died with it"
    );
}

// oracle_id = "2de6c3d9-1759-40a2-99c6-8cbe17b4bcdd"
fn entreat_the_dead() -> CardIndex {
    card_index("2de6c3d9-1759-40a2-99c6-8cbe17b4bcdd")
}

/// Entreat the Dead — {X}{X}{B}{B}{B} — "return X target creature cards from
/// your graveyard to the battlefield."
///
/// X is the count of *targets* rather than a number the effect reads, which
/// is the one shape a reanimation written against the first target cannot
/// do at all: the announced two would have brought one card back and the
/// spell would have looked like it worked. So the board holds exactly two
/// creature cards and the assertion is both of them.
///
/// Seven Swamps, because {X}{X} at X = 2 is four mana in front of {B}{B}{B}
/// — the doubled symbol is what makes a wrong X visible as a refused cast
/// rather than as a cheaper one.
#[test]
fn entreat_the_dead_returns_as_many_creatures_as_the_x_it_was_cast_for() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(44, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
                rib_cage_spider(),
            ],
        )
        .hand(0, &[entreat_the_dead()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are seated");
    let spider = on_battlefield(&engine, p0, rib_cage_spider()).expect("the Spider is seated");
    bury(&mut engine, &[elves, spider]);

    cast_from_hand(&mut engine, p0, entreat_the_dead());
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!(
            "a printed {{X}} has to be asked about: {:?}",
            engine.pending()
        )
    };
    assert_eq!(min, 0, "X may always be nothing");
    assert!(
        max >= 2,
        "seven Swamps pay {{X}}{{X}}{{B}}{{B}}{{B}} for X = 2: max = {max}"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(2))
        .expect("X = 2 is inside the range the engine just offered");

    let options = pass_until_targets(&mut engine, p0);
    assert!(
        options.contains(&elves) && options.contains(&spider),
        "both creature cards in my graveyard: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves, spider],
                players: vec![],
            },
        )
        .expect("X targets, and X was announced as two");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the first of the two came back"
    );
    assert!(
        on_battlefield(&engine, p0, rib_cage_spider()).is_some(),
        "and so did the second — X is a count and not a decoration"
    );
    assert!(
        in_graveyard(&engine, p0, entreat_the_dead()).is_some(),
        "and the sorcery itself resolved into the graveyard"
    );
}
