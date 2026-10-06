//! `cards/artifacts/mv_1/pithing_needle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pithing Needle naming a Room leaves its unlock alone: unlocking a door
/// is a special action (CR 116.2m) and not an activated ability, and the
/// Needle stops only the second (CR 602.5). Walk-In Closet is cast as its
/// left half, so the permanent's name is the one named, and Forgotten
/// Cellar is then unlocked with the Needle on the table.
#[test]
fn pithing_needle_naming_a_room_leaves_its_doors_to_unlock() {
    let p0 = PlayerId::new(0);
    let mut board = vec![forest(); 9];
    board.push(llanowar_elves());
    let mut engine = Duel::new(601, forest())
        .battlefield(0, &board)
        .hand(0, &[walk_in_closet(), pithing_needle()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    float_green(&mut engine, p0, 3);
    cast_with_floating(&mut engine, p0, walk_in_closet());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let room = on_battlefield(&engine, p0, walk_in_closet()).expect("the Closet resolved");

    // Everything else is tapped for it: six Forests and the Elf, one of
    // which pays for the Needle.
    needle_naming(&mut engine, walk_in_closet());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        unlocks_offered(&engine, room),
        [1],
        "the Needle names the Room, and the unlock is not an ability it stops"
    );
    unlock(&mut engine, p0, room, 1).expect("Forgotten Cellar unlocks under the Needle");
    assert_eq!(doors_of(&engine, room), crate::object::Doors::room(0b11));
}

/// Pithing Needle: "As this artifact enters, choose a card name. Activated
/// abilities of sources with the chosen name can't be activated unless
/// they're mana abilities."
///
/// One name, two sources with it and one without. The Desert on the table
/// still taps for {G}, because that is a mana ability (CR 605.1a); the copy
/// in hand no longer cycles, because cycling is an activated ability of a
/// card in a hand (CR 602.1, 113.6b) and "sources" is not "permanents"; and
/// Lay Waste beside it, another name, still does, which is what makes the
/// second reading one about the name and not about the hand. The question is
/// asked of the Needle's controller as it enters, and a face the card does
/// not print, a card the pool does not have and the other seat's answer are
/// each refused with the question left open.
#[allow(clippy::too_many_lines)] // One printed card, played end to end.
#[test]
fn pithing_needle_stops_the_named_card_activating_but_not_tapping_for_mana() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let desert = desert_of_the_indomitable();
    let mut engine = Duel::new(601, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[pithing_needle()])
        .battlefield(1, &[desert, forest(), forest()])
        .hand(1, &[desert, lay_waste()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, pithing_needle());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    assert!(
        matches!(engine.pending(), Pending::ChooseCardName { player } if *player == p0),
        "the Needle's controller names the card, as it enters: {:?}",
        engine.pending()
    );

    for (seat, card, face, what) in [
        (p0, desert, 1, "a face the Desert does not print"),
        (
            p0,
            CardIndex::new(0x0FFF_FFFF),
            0,
            "a card the pool does not have",
        ),
        (p1, desert, 0, "the other seat's answer"),
    ] {
        assert!(
            engine
                .apply(seat, PlayerAction::ChooseCardName { card, face })
                .is_err(),
            "{what} is refused"
        );
        assert!(
            matches!(engine.pending(), Pending::ChooseCardName { player } if *player == p0),
            "and the question is still open after {what}: {:?}",
            engine.pending()
        );
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseCardName {
                card: desert,
                face: 0,
            },
        )
        .expect("any card of the pool may be named");
    let needle = on_battlefield(&engine, p0, pithing_needle()).expect("the Needle is out");
    assert_eq!(
        engine
            .state()
            .object(needle)
            .and_then(|o| o.chosen_name)
            .map(|n| (n.card(), n.face())),
        Some((desert, 0)),
        "the name is kept on the Needle, as the card and face it was read off"
    );

    // Their own turn, because the Desert was seated tapped (it enters
    // tapped) and stands again only in their untap step.
    reach_their_main_phase(&mut engine, p1);
    let on_table = on_battlefield(&engine, p1, desert).expect("their Desert is out");
    tap_mana_except(&mut engine, p1, on_table);
    let in_their_hand = in_hand(&engine, p1, desert).expect("the second Desert is in hand");
    let waste = in_hand(&engine, p1, lay_waste()).expect("Lay Waste is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(on_table, 0)),
        "{{T}}: Add {{G}} is a mana ability, which the Needle spares: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(in_their_hand, 1)),
        "cycling the named card is an activated ability, and it is not \
         offered with {{1}}{{G}} floating: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(waste, 0)),
        "Lay Waste is another name, so its cycling is: {:?}",
        legal.abilities
    );
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::ActivateAbility {
                    source: in_their_hand,
                    ability_index: 1,
                },
            )
            .is_err(),
        "and cycling it anyway is refused"
    );
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: on_table,
                ability_index: 0,
            },
        )
        .expect("the named Desert still taps for mana");
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Green),
        3,
        "two Forests and the Desert"
    );
}

/// Pithing Needle on a planeswalker. A loyalty ability is an activated
/// ability (CR 606.1) and never a mana ability (CR 605.1a), so naming Karn
/// leaves him nothing to do on his controller's turn. The same board with
/// another name chosen offers his +1, which is what makes the first reading
/// one about the name.
#[test]
fn pithing_needle_named_after_a_planeswalker_leaves_it_nothing_to_activate() {
    let p1 = PlayerId::new(1);
    for (named, offered) in [(karn_the_great_creator(), false), (lay_waste(), true)] {
        let mut engine = Duel::new(602, forest())
            .battlefield(0, &[forest()])
            .hand(0, &[pithing_needle()])
            .battlefield(1, &[karn_the_great_creator()])
            .start();
        keep_mulligans(&mut engine);
        needle_naming(&mut engine, named);
        reach_their_main_phase(&mut engine, p1);
        let karn = on_battlefield(&engine, p1, karn_the_great_creator()).expect("Karn is out");
        let Pending::Priority { player, legal } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        assert_eq!(player, p1, "Karn's controller, in their own main phase");
        assert_eq!(
            legal.abilities.contains(&(karn, 1)),
            offered,
            "Karn's +1 with the Needle naming {:?}: {:?}",
            engine
                .lookup
                .card(named)
                .map(baylee_cards_dsl::CardDef::name),
            legal.abilities
        );
    }
}

/// The name belongs to the permanent that entered (CR 400.7). Boomerang the
/// Needle and the lock goes with it, so the Desert cycles again; the card in
/// the hand carries no name; and cast again, the Needle asks again, and the
/// new name is the one that locks.
#[test]
fn pithing_needle_returned_to_hand_forgets_its_name_and_asks_again() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let desert = desert_of_the_indomitable();
    let mut engine = Duel::new(603, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[pithing_needle(), desert])
        .battlefield(1, &[island(), island()])
        .hand(1, &[boomerang()])
        .start();
    keep_mulligans(&mut engine);
    let cycles = |engine: &Engine<RegistryLookup>| {
        let card = in_hand(engine, p0, desert).expect("the Desert is in hand");
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        legal.abilities.contains(&(card, 1))
    };
    let needle = needle_naming(&mut engine, desert);
    // Four Forests paid {1}, and {G}{G}{G} float: the Desert's cycling
    // now, and a second Needle with its cycling after.
    assert!(!cycles(&engine), "the Needle names the Desert");

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, boomerang());
    aim_at(&mut engine, p1, needle);
    pass_until(&mut engine, stack_is_empty);
    let card = in_hand(&engine, p0, pithing_needle()).expect("the Needle is back in hand");
    assert_eq!(
        engine.state().object(card).and_then(|o| o.chosen_name),
        None,
        "the card in hand is a new object, with no name chosen"
    );
    assert!(
        cycles(&engine),
        "and with the Needle gone, the Desert cycles"
    );

    cast_with_floating(&mut engine, p0, pithing_needle());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseCardName {
                card: boomerang(),
                face: 0,
            },
        )
        .expect("Boomerang is a card of the pool");
    assert!(
        cycles(&engine),
        "cast again, the Needle names Boomerang, and the Desert is free"
    );
}
