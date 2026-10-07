//! Sorceries, the door `cards/sorceries/` puts them behind.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

mod legends;
mod mv_0;
mod mv_1;
mod mv_2;
mod mv_3;
mod mv_4;
mod mv_5;
mod mv_6;
mod mv_7;
mod mv_8;

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

// oracle_id = "1d67f5ff-1fce-45e5-b6a1-416c569351e2"
fn gitaxian_probe() -> CardIndex {
    card_index("1d67f5ff-1fce-45e5-b6a1-416c569351e2")
}

/// Moves the named cards from the seat's hand into its graveyard, the
/// harness way.
#[track_caller]
fn discard_by_hand(engine: &mut Engine<RegistryLookup>, seat: PlayerId, cards: &[CardIndex]) {
    for &card in cards {
        let id = in_hand(engine, seat, card).expect("the card starts in hand");
        engine
            .dev_state_mut(seat)
            .expect("the harness may set boards up")
            .move_object(
                id,
                ZoneLocation::Graveyard(seat),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("into the graveyard");
    }
}

/// Casts Finale of Devastation for `x` off the Forests on the board and
/// passes until its first question.
fn cast_finale(engine: &mut Engine<RegistryLookup>, seat: PlayerId, x: u32) -> Pending {
    tap_all_mana(engine, seat);
    cast_with_floating(engine, seat, finale_of_devastation());
    engine
        .apply(seat, PlayerAction::ChooseNumber(x))
        .expect("the announced X");
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine.pending().clone()
}

fn angelic_blessing() -> CardIndex {
    card_index("d3758fca-0522-4b5a-a1cc-3b2b3ab299ba")
}

fn bargain() -> CardIndex {
    card_index("a0dd88f6-6e36-40ce-bac2-a0db2b0117b6")
}

fn blaze() -> CardIndex {
    card_index("0596920f-9946-42f4-a03b-24aab67f9f1b")
}

fn bloodcurdling_scream() -> CardIndex {
    card_index("3ee2060a-5152-4a53-8183-cbd642e4cc29")
}

// oracle_id = "910ff092-7c9d-49d3-a6df-497683e45bbd"
fn cateran_summons() -> CardIndex {
    card_index("910ff092-7c9d-49d3-a6df-497683e45bbd")
}

fn cloak_of_feathers() -> CardIndex {
    card_index("cb4baf53-51ed-468b-a468-5d7d45a6dc26")
}

fn death_stroke() -> CardIndex {
    card_index("3ebaa91f-5cbf-4a82-9759-9bd4d93a3e87")
}

fn deconstruct() -> CardIndex {
    card_index("36a8ceb1-148b-41e0-a7bf-ceb879bf08e7")
}

fn eerie_procession() -> CardIndex {
    card_index("58dde1b0-bb01-4f72-9408-21c0404c1cfd")
}

fn eye_of_nowhere() -> CardIndex {
    card_index("27ffdf11-bb7f-40bf-94c4-6bfbc41668e5")
}

fn fabricate() -> CardIndex {
    card_index("422e1869-134f-463d-9fa1-86b66a998b3e")
}

fn farseek() -> CardIndex {
    card_index("495e52e6-4c2b-4574-9474-eadbdcc8b4ac")
}

fn fire_ambush() -> CardIndex {
    card_index("50463946-1ce3-4ff0-ad68-2fb87adbe2fd")
}

fn fit_of_rage() -> CardIndex {
    card_index("69d08521-74e1-4215-9ed4-3f12137332a4")
}

fn goblin_offensive() -> CardIndex {
    card_index("25cb5c86-83cd-4a06-b0d1-a0f6fc9158a6")
}

fn howling_fury() -> CardIndex {
    card_index("eda60752-d225-4fd0-9f0f-9b99e321b8fa")
}

// oracle_id = "cedc52eb-66a6-4b43-87f1-9bb9f4d4871e"
fn lay_of_the_land() -> CardIndex {
    card_index("cedc52eb-66a6-4b43-87f1-9bb9f4d4871e")
}

fn monstrous_growth() -> CardIndex {
    card_index("35a05836-38d7-45c7-ac9a-996a682c2129")
}

fn natures_lore() -> CardIndex {
    card_index("78826359-fe63-44ad-adc4-a17ffcd710e4")
}

fn night_s_whisper() -> CardIndex {
    card_index("7ffae8f8-3006-4969-a339-6d30678f87ea")
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

fn sinkhole() -> CardIndex {
    card_index("5a46ad2a-35b1-4dd5-b7c3-fec36b7c67ab")
}

fn steelshaper_s_gift() -> CardIndex {
    card_index("d9abda7e-6ca2-42ea-ab24-c542e57014f1")
}

// oracle_id = "1b882a0e-0ede-4d1a-bd1a-9b7cffbcde8e"
fn three_visits() -> CardIndex {
    card_index("1b882a0e-0ede-4d1a-bd1a-9b7cffbcde8e")
}

fn time_of_need() -> CardIndex {
    card_index("4f2adfd0-c8ab-4bcc-ae55-cb0e798aec7f")
}

// oracle_id = "38ea22cd-2c5d-4f66-a111-207aca4c67c3"
fn vicious_hunger() -> CardIndex {
    card_index("38ea22cd-2c5d-4f66-a111-207aca4c67c3")
}

fn volcanic_hammer() -> CardIndex {
    card_index("98fa5a06-0553-40fd-999c-bc31c9b3f4db")
}

fn wielding_the_green_dragon() -> CardIndex {
    card_index("22fa9020-783e-4292-857e-9d45a3458d71")
}

fn scorching_spear() -> CardIndex {
    card_index("9342fbb8-ab35-4895-946d-951ba6a2b067")
}

fn serum_visions() -> CardIndex {
    card_index("56956afd-db53-4542-816b-490c8b0bbcf7")
}

fn pillage() -> CardIndex {
    card_index("0b137853-7cb9-424b-8285-12938991eafb")
}

fn damn() -> CardIndex {
    card_index("b01d61cc-9844-4191-86a0-f2db6d42d6e5")
}

// oracle_id = "2de6c3d9-1759-40a2-99c6-8cbe17b4bcdd"
fn entreat_the_dead() -> CardIndex {
    card_index("2de6c3d9-1759-40a2-99c6-8cbe17b4bcdd")
}

/// Spirit Water Revival on `board`, main phase, every land tapped for mana.
fn revival_table(board: &[CardIndex]) -> Engine<RegistryLookup> {
    let seat = PlayerId::new(0);
    let mut engine = Duel::new(229, island())
        .hand(0, &[spirit_water_revival()])
        .battlefield(0, board)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat);
    tap_all_mana(&mut engine, seat);
    engine
}

fn revival_is_offered(engine: &Engine<RegistryLookup>) -> bool {
    let seat = PlayerId::new(0);
    let revival = in_hand(engine, seat, spirit_water_revival()).expect("in hand");
    matches!(engine.pending(), Pending::Priority { legal, .. } if legal.castable.contains(&revival))
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

/// Answers the `CostSacrifice` question a spell's "as an additional cost,
/// sacrifice …" asks at cast, with `paid`, after checking it was offered.
/// Returns the objects that were offered.
#[track_caller]
fn sacrifice_as_cast(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    paid: ObjectId,
) -> Vec<ObjectId> {
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "an additional sacrifice is asked at cast, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, seat);
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostSacrifice);
    assert!(options.contains(&paid), "{paid:?} is not among {options:?}");
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![paid],
            },
        )
        .unwrap();
    options
}

/// How many copies of `card` lie in `seat`'s exile.
fn exiled(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .count()
}

/// How many copies of `card` `seat` controls on the battlefield.
fn fielded(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .count()
}

fn expressive_iteration() -> CardIndex {
    card_index("c7aecca5-2f67-4245-ab2d-e723d8b23a67")
}

/// Casts Expressive Iteration off the Island and the Mountain, keeps the
/// first card looked at, bottoms the second, and returns the three in that
/// order — the third is the one exiled.
fn iterate(engine: &mut Engine<RegistryLookup>, p0: PlayerId) -> [ObjectId; 3] {
    let island = on_battlefield(engine, p0, island()).unwrap();
    let mountain = on_battlefield(engine, p0, mountain()).unwrap();
    tap_mana_where(engine, p0, |id| id == island || id == mountain);
    cast_with_floating(engine, p0, expressive_iteration());
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = pass_to_card_choice(engine)
    else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(prompt, ChoicePrompt::PutIntoHand);
    assert_eq!((min, max), (1, 1));
    assert_eq!(options.len(), 3, "the top three");
    let (keep, bottom, exiled) = (options[0], options[1], options[2]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![keep],
            },
        )
        .unwrap();
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("expected the bottom question, got {:?}", engine.pending())
    };
    assert_eq!(prompt, ChoicePrompt::PutOnBottom);
    assert_eq!(options.len(), 2, "the two not kept");
    assert!(!options.contains(&keep));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bottom],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
    [keep, bottom, exiled]
}

fn nyleas_intervention() -> CardIndex {
    card_index("acf388b2-c4e3-4f1b-a16c-88f991d5c17b")
}

/// Casts Nylea's Intervention with the mana already floating, choosing `mode`
/// and `x` in whichever order the cast asks for them.
#[track_caller]
fn cast_nyleas(engine: &mut Engine<RegistryLookup>, seat: PlayerId, mode: usize, x: u32) {
    tap_all_mana(engine, seat);
    cast_with_floating(engine, seat, nyleas_intervention());
    let (mut chose_mode, mut chose_x) = (false, false);
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseCastMode { options, .. } => {
                let slot = options
                    .iter()
                    .position(|o| o.kind == crate::choice::CastModeKind::Mode(mode))
                    .expect("the mode is offered");
                engine.apply(seat, PlayerAction::ChooseMode(slot)).unwrap();
                chose_mode = true;
            }
            Pending::ChooseNumber { min, max, .. } => {
                assert!((min..=max).contains(&x), "X = {x} in {min}..={max}");
                engine.apply(seat, PlayerAction::ChooseNumber(x)).unwrap();
                chose_x = true;
            }
            _ => break,
        }
    }
    assert!(chose_mode && chose_x, "the cast asked for a mode and an X");
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Prismatic Ending.
// ---------------------------------------------------------------------------

fn prismatic_ending() -> CardIndex {
    card_index("2cb98ca9-d7bb-416b-a17e-ee5f8e4d78f2")
}

/// Eternal Witness, mana value 3.
fn prismatic_three_drop() -> CardIndex {
    card_index("30b24e8e-3b0e-4d8e-90f3-f66eb7c1858c")
}

/// Charming Prince, mana value 2.
fn prismatic_two_drop() -> CardIndex {
    card_index("c48d844c-3976-4fa5-8e0d-3f0e535e7619")
}

/// Seat 0 floats everything `sources` make, casts Prismatic Ending for X =
/// `x` at `victim` (seat 1's only permanent) and lets it resolve.
fn prismatic_ending_at(sources: &[CardIndex], x: u32, victim: CardIndex) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, sources)
        .battlefield(1, &[victim])
        .hand(0, &[prismatic_ending()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let target = on_battlefield(&engine, p1, victim).unwrap();
    tap_all_mana_but(&mut engine, p0, None);
    cast_with_floating(&mut engine, p0, prismatic_ending());
    let Pending::ChooseNumber { .. } = engine.pending().clone() else {
        panic!("X is announced, got {:?}", engine.pending())
    };
    engine.apply(p0, PlayerAction::ChooseNumber(x)).unwrap();
    let _ = aim_at(&mut engine, p0, target);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the whole pool paid {{{x}}}{{W}}"
    );
    engine
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

// ---------------------------------------------------------------------------
// Maik's European Highlander: Farewell.
// ---------------------------------------------------------------------------

/// Seat 0 casts Farewell off six Plains with the modes `set` names, at seat
/// 1's Llanowar Elves, Sol Ring, Sterling Grove and Darksteel Gargoyle and
/// three cards in seat 1's graveyard, and lets it resolve. Every one of the
/// fifteen sets is offered, each at the printed cost.
fn farewell_with(set: u8) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 6])
        .battlefield(
            1,
            &[
                llanowar_elves(),
                sol_ring(),
                sterling_grove(),
                darksteel_gargoyle(),
            ],
        )
        .hand(0, &[farewell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p1, 3);
    cast_from_hand(&mut engine, p0, farewell());
    let offered = choose_modes(&mut engine, p0, set);
    assert_eq!(
        offered.iter().map(|o| o.kind).collect::<Vec<_>>(),
        (1..16).map(CastModeKind::Modes).collect::<Vec<_>>(),
        "one or more of four modes is fifteen sets, and each is one row"
    );
    assert!(
        offered
            .iter()
            .all(|o| o.cost == baylee_core::mana::ManaCost::parse("{4}{W}{W}")),
        "no mode of Farewell costs anything of its own: {offered:?}"
    );
    pass_until(&mut engine, stack_is_empty);
    engine
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

/// Profane Tutor: "Suspend 2—{1}{B}. Search your library for a card, put
/// that card into your hand, then shuffle."
fn profane_tutor() -> CardIndex {
    card_index("27a1f42c-0b86-4609-9609-1fa9cab7e7c9")
}

// ---------------------------------------------------------------------------
// Alpha cards, played by their Oracle text.
// ---------------------------------------------------------------------------

/// A seat's current life total, read the way most of this batch reads it.
fn life_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

fn armageddon() -> CardIndex {
    card_index("c9ed8b01-959a-47d6-891e-0abbdccf6e4f")
}

fn resurrection() -> CardIndex {
    card_index("837417d8-8260-486d-a3ed-3b5711eaf34a")
}

fn raise_dead() -> CardIndex {
    card_index("cbc9c731-181a-4f00-a7b0-eb7e56eac2ea")
}

fn regrowth() -> CardIndex {
    card_index("e6e4a8bd-5c40-4654-8de1-0da9afed90fd")
}

fn wrath_of_god() -> CardIndex {
    card_index("34515b16-c9a4-4f98-8c77-416a7a523407")
}

fn braingeyser() -> CardIndex {
    card_index("9908e597-9470-4c13-8387-39431b380138")
}

fn time_walk() -> CardIndex {
    card_index("d0209d3f-3f7e-4fd5-bce5-10bce6f29c86")
}

fn timetwister() -> CardIndex {
    card_index("c823e687-6311-4c99-974b-fd77d204141a")
}

fn disintegrate() -> CardIndex {
    card_index("92d6af2f-728e-4e41-87cb-5c90878a2f2f")
}

fn earthquake() -> CardIndex {
    card_index("9a40614b-50a3-422c-849e-53c8b7d3d204")
}

fn flashfires() -> CardIndex {
    card_index("c281f436-8c77-48f7-b31c-d40cd7f9ed6a")
}

fn hurricane() -> CardIndex {
    card_index("9c021685-4017-49c7-9f58-2ae0243361a0")
}

fn tranquility() -> CardIndex {
    card_index("f671e3c3-cd59-4d06-a1af-5d04892cf74d")
}

fn tsunami() -> CardIndex {
    card_index("ef2b1565-7b02-4cab-9031-beb1701ee929")
}

fn stream_of_life() -> CardIndex {
    card_index("9eb2912d-2130-49f2-9529-b58fa5a97a15")
}

fn drain_life() -> CardIndex {
    card_index("e75ba79f-4cc2-4ede-8641-559ab94e7e36")
}

// ---- Abilities no test had fired, second sweep (L4, 2026-10-01) ----

fn ancestral_vision_card() -> CardIndex {
    card_index("9728dec9-d482-4c7a-8cdc-44d010dc878d")
}

fn temporal_mastery_card() -> CardIndex {
    card_index("5c58b8e6-c572-461e-893e-a8c05f20ba17")
}

// oracle_id = "7140d726-0136-43af-84b5-85005a66a186"
fn metamorphosis() -> CardIndex {
    card_index("7140d726-0136-43af-84b5-85005a66a186")
}

fn detonate() -> CardIndex {
    card_index("daa90a75-c600-41bd-9311-ec21cf51480b")
}

/// Living Wall: an artifact creature of mana value 4 whose own `{1}` ability
/// can raise the shield every "it can't be regenerated" test has to ignore.
fn living_wall() -> CardIndex {
    card_index("4844312c-3c9d-4ca1-986d-4ad35e68454e")
}

fn reconstruction() -> CardIndex {
    card_index("ad8fb78b-5ca5-4ef1-8c68-ee57d1e32fec")
}

fn shatterstorm() -> CardIndex {
    card_index("96ce2403-4607-440a-92ae-80aceb458c5d")
}
