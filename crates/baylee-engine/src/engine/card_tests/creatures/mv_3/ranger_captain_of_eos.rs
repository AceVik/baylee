//! `cards/creatures/mv_3/ranger_captain_of_eos.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ranger-Captain of Eos — {1}{W}{W} — Creature — Human Soldier Ranger, 3/3:
/// "When this creature enters, you may search your library for a creature card
/// with mana value 1 or less, reveal it, put it into your hand, then shuffle."
/// The backing library here is sixty copies of a one-mana 1/1, so the filter
/// has something to find and the assertions can say exactly where the found
/// card went — the object chosen out of the library is the object that turns up
/// in the hand, with the library one shorter and the hand one longer.
/// The tail is the half that **moved**. It pinned the card's
/// `Coverage::Partial`: "Sacrifice this creature: your opponents can't cast
/// noncreature spells this turn" was on no offer, with the Sol Ring beside
/// it as the control saying the board offered activations at all. The
/// sentence is now `Modifier::OpponentsCantCast`, so the pin is inverted
/// rather than deleted — a limitation that was written down is a test that
/// has to move, and the move is the record of it. What the ability *does*
/// is the test below this one.
#[allow(clippy::too_many_lines)] // one search answered, and the ability that is not there
#[test]
fn ranger_captain_of_eos_searches_up_a_one_mana_creature_and_is_never_offered_its_sacrifice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(2024, llanowar_elves())
        .battlefield(0, &[plains(), plains(), plains(), quiet_artifact()])
        .hand(0, &[ranger_captain_of_eos()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{W}{W} off the three Plains, and the creature's one trigger is the
    // only thing the board has to resolve. The Sol Ring is kept back: it is
    // the control below for "this board does offer activations", and a
    // tapped one is offered nothing.
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    cast_with_floating(&mut engine, p0, ranger_captain_of_eos());
    // "You may search" is **one** question here and not two: an optional
    // `Effect::SearchLibrary` is offered as the search itself with `min: 0`,
    // so declining is answering it with nothing. There is no `YesNo` in
    // front of it, which is what the first draft of this test waited for.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the searching seat is the one choosing");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "a library search, and not a scry, a discard or a sacrifice"
    );
    assert_eq!(
        (min, max),
        (0, 1),
        "\"you may\": nought is a legal answer, and one is the most it takes"
    );

    let library_before: Vec<ObjectId> =
        engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert!(
        !options.is_empty(),
        "every card in this library is a creature with mana value 1, which is \
         exactly what the filter asks for: {library_before:?}"
    );
    assert!(
        options.iter().all(|id| library_before.contains(id)),
        "the menu is that seat's own library and nothing else: {options:?}"
    );

    let found = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .expect("the card the question offered is a legal answer");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&found),
        "the card the search offered is the card that reached the hand"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .contains(&found),
        "and it is no longer in the library it came out of"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "\"a creature card\", singular: exactly one left the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "which the hand reads from the other side"
    );
    let captain =
        on_battlefield(&engine, p0, ranger_captain_of_eos()).expect("the Captain resolved");
    assert_eq!(
        pt(&engine, captain),
        (3, 3),
        "the body the card prints, on the battlefield and not in a graveyard"
    );

    // The `Coverage::Partial` half, with its control: the Sol Ring's printed
    // {T} is on the offer list, the Captain's Sacrifice line is not on the
    // card at all.
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is out");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a quiet priority after the search: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.iter().any(|(source, _)| *source == ring),
        "the board does offer activations: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.iter().any(|(source, _)| *source == captain),
        "\"Sacrifice this creature: …\" is on the card now, and its price is \
         the Captain itself — which is standing right here: {:?}",
        legal.abilities
    );
}

/// What the Ranger-Captain's sacrifice actually does: "Your opponents can't
/// cast noncreature spells this turn."
///
/// One board taken twice, and the only difference between the rows is
/// whether p0 pressed the ability — so the refusal cannot be the price, the
/// phase or the priority round. Brainstorm is the opponent's spell because
/// it needs no target: a counterspell with nothing to counter is refused for
/// a reason that has nothing to do with this card, and the negative would
/// have been true either way.
///
/// The second Island is held back through the first phase on purpose. A
/// pool empties at the end of a phase (CR 500.4), and the row that matters
/// is read after p0 has had a priority of its own — so the mana that pays
/// for the Brainstorm has to be floated *after* that, and is asserted to be
/// floating when the refusal is read.
#[test]
fn the_ranger_captains_sacrifice_takes_an_opponents_noncreature_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for sacrificed in [false, true] {
        let mut engine = Duel::new(2024, llanowar_elves())
            .battlefield(0, &[ranger_captain_of_eos()])
            .battlefield(1, &[island()])
            .hand(1, &[brainstorm()])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

        let captain =
            on_battlefield(&engine, p0, ranger_captain_of_eos()).expect("the Captain is out");
        if sacrificed {
            activate(&mut engine, p0, ranger_captain_of_eos(), 1);
            pass_until(&mut engine, stack_is_empty);
            assert!(
                in_graveyard(&engine, p0, ranger_captain_of_eos()).is_some(),
                "the price is the Captain itself"
            );
            assert!(
                on_battlefield(&engine, p0, ranger_captain_of_eos()).is_none(),
                "and it is off the battlefield: {captain:?}"
            );
        }

        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
                && e.state().zones.stack_is_empty()
        });
        tap_all_mana(&mut engine, p1);
        assert_eq!(
            engine.state().players[1]
                .mana_pool
                .available(ManaColor::Blue),
            1,
            "the {{U}} the Brainstorm costs is floating, sacrificed {sacrificed}"
        );

        let storm = in_hand(&engine, p1, brainstorm()).expect("the Brainstorm is in hand");
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected p1's priority, got {:?}", engine.pending())
        };
        assert_eq!(
            legal.castable.contains(&storm),
            !sacrificed,
            "\"your opponents can't cast noncreature spells this turn\", \
             sacrificed {sacrificed}: {:?}",
            legal.castable
        );
    }
}
