#[allow(clippy::wildcard_imports)]
use super::*;

/// Stitcher's Supplier ({B}, 1/1): "When this creature enters **or dies**,
/// mill three cards."
///
/// One printed sentence firing on two events, and the card is only itself
/// when both of them fire: read as an enter-trigger alone it is an ordinary
/// 1/1 that mills three once, which is a different card — and the half that
/// goes missing is the one no board shows, because a Zombie that entered and
/// milled looks right until somebody kills it. So the Zombie is cast off a
/// Swamp, and then killed by the opponent's Vindicate, and the same
/// three-card step is struck twice.
///
/// Both halves are read off the *library*, which is what makes the word
/// "mill": three cards leave the top of the deck, and the graveyard's growth
/// is struck beside the library's loss so that a mill which exiled or drew
/// instead could not pass. After the death that growth is **four** — three
/// milled cards and the Zombie's own corpse, which is lying in the same
/// graveyard by the time the trigger resolves.
///
/// The opponent's library is the counter-half. `PlayerRel::You` is what makes
/// this "mill three cards" rather than "target player mills three", and a
/// card that milled the wrong seat would satisfy every other assertion here.
#[test]
fn a_stitchers_supplier_mills_three_entering_and_three_more_dying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let graveyard = |e: &Engine<RegistryLookup>| {
        e.state()
            .zones
            .list(baylee_engine::zone::ZoneLocation::Graveyard(p0))
            .len()
    };
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[stitcher_s_supplier()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let library_before = library_size(&engine, p0);
    let graveyard_before = graveyard(&engine);
    let their_library = library_size(&engine, p1);

    cast_from_hand(&mut engine, p0, stitcher_s_supplier());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, stitcher_s_supplier()).is_some(),
        "one Swamp pays {{B}} and the Zombie arrives"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 3,
        "entering took three cards off the top of its controller's library"
    );
    assert_eq!(
        graveyard(&engine),
        graveyard_before + 3,
        "and put all three of them into the graveyard"
    );
    assert_eq!(
        library_size(&engine, p1),
        their_library,
        "\"mill three cards\" mills its controller: the opponent's library is untouched"
    );

    // The other half of the same sentence, on the opponent's turn: Vindicate
    // destroys the Zombie, and dying has to mill again.
    reach_their_main_phase(&mut engine, p1);
    let zombie =
        on_battlefield(&engine, p0, stitcher_s_supplier()).expect("the Zombie is still out");
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![zombie],
            },
        )
        .expect("their removal may point at an ordinary creature");

    // Snapshotted with the Zombie still alive, so the four cards counted
    // below are the three it mills plus itself and nothing else.
    let library_alive = library_size(&engine, p0);
    let graveyard_alive = graveyard(&engine);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, stitcher_s_supplier()).is_none(),
        "Vindicate destroyed it"
    );
    assert!(
        in_graveyard(&engine, p0, stitcher_s_supplier()).is_some(),
        "and it fell into the graveyard it had been milling into"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_alive - 3,
        "dying is the second event of the one printed sentence: three more \
         cards leave the library"
    );
    assert_eq!(
        graveyard(&engine),
        graveyard_alive + 4,
        "three milled cards and the Zombie's own corpse"
    );
}

/// Temple of Cyclical Time's "{2}{U}, {T}: Transform this land" turns the
/// same permanent over (CR 701.27a), and a permanent that transforms is not
/// a new object: every effect that applied to it goes on applying
/// (CR 712.18). So the god seat 0 turned its stolen Temple into is still
/// seat 0's by the steal that held the Temple, still seat 1's by default and
/// still seat 1's card (CR 108.3), and still tapped from the activation.
///
/// The stand-in that exiled the land and returned the god (#206) gave back a
/// new object whose own default was the activator, which kept it with the
/// thief after the steal had ended (the next test).
#[test]
fn a_stolen_temple_of_cyclical_time_turns_back_into_the_god_under_the_thief() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (engine, temple) =
        a_temple_stolen_and_turned_over(baylee_cards_dsl::Duration::Indefinitely);
    let god = engine
        .state()
        .object(temple)
        .expect("the same arena handle");
    assert_eq!(
        (god.owner, god.controller, god.base_controller),
        (p1, p0, p1),
        "stolen by the same effect, seat 1's by default, and seat 1's card"
    );
    assert!(
        is_tapped(&engine, temple),
        "tapped for the activation, and turning over is not entering"
    );
}

/// A steal "until end of turn" ends in the cleanup step (CR 514.2) whatever
/// face the stolen permanent shows by then. Seat 0 turns the Temple it stole
/// for the turn into the god, and seat 1 has its god back once the turn is
/// over. The stand-in's god was a new object that the steal no longer held,
/// under seat 0 by default, so it stayed with the thief.
#[test]
fn a_temple_stolen_until_end_of_turn_goes_home_as_the_god_it_became() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, temple) =
        a_temple_stolen_and_turned_over(baylee_cards_dsl::Duration::UntilEndOfTurn);
    assert_eq!(
        engine.state().object(temple).map(|o| o.controller),
        Some(p0),
        "seat 0's for the rest of the turn"
    );
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine
            .state()
            .object(temple)
            .map(|o| (o.zone, o.face_index, o.controller)),
        Some((baylee_engine::zone::Zone::Battlefield, 0, p1)),
        "the steal ended with the turn, and the god went home"
    );
}

/// "If it's a land card, put it onto the battlefield." The top card is a
/// Forest: shown to everyone, then on the battlefield, untapped.
#[test]
fn coiling_oracle_puts_a_revealed_land_onto_the_battlefield() {
    let p0 = PlayerId::new(0);
    let (engine, top) = coil(forest());
    assert!(revealed(&engine, top), "revealed before it moved");
    let land = engine.state().object(top).expect("the revealed Forest");
    assert_eq!(land.zone, baylee_engine::zone::Zone::Battlefield);
    assert!(
        !land.status.contains(baylee_engine::object::Status::TAPPED),
        "put onto the battlefield, not tapped"
    );
    let lands = engine
        .state()
        .zones
        .list(baylee_engine::zone::ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.controller == p0 && o.card.is_some_and(|c| c.index == forest()))
        })
        .count();
    assert_eq!(lands, 2, "the Forest it paid with and the one it revealed");
    let hand = engine
        .state()
        .zones
        .list(baylee_engine::zone::ZoneLocation::Hand(p0))
        .len();
    assert_eq!(hand, 0, "nothing went to the hand");
}

/// "…that creature deals damage equal to its power to any target." It
/// enters a 2/3 and deals 2 to the opponent.
#[test]
fn pyrogoyf_deals_its_power_to_any_target_as_it_enters() {
    let (mut engine, goyf) = a_pyrogoyf_aimed_at_the_opponent();
    assert_eq!(pt(&engine, goyf), (2, 3));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 18);
}

/// A Counterspell discovered after the trigger has left the stack has
/// nothing to counter, so it cannot be cast (CR 601.2c): nobody is asked,
/// and it goes to the hand.
#[test]
fn trumpeting_carnosaur_asks_nothing_about_a_spell_with_no_target() {
    let p0 = PlayerId::new(0);
    let (engine, ids) = carnosaur_discovers(&[counterspell()], &[]);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }) && stack_is_empty(&engine),
        "{:?}",
        engine.pending()
    );
    assert_eq!(in_hand(&engine, p0, counterspell()), Some(ids[0]));
}

/// Casting without paying the mana cost is an alternative cost, and a
/// spree mode's cost is an additional one, which is still paid (CR 118.9d,
/// 702.172a). Three Steps Ahead discovered with three Islands' mana still
/// floating: the draw, "+ {2}", is two of it, and the {U} the card prints
/// is not paid. The copy, "+ {3}", is offered too, since the Carnosaur is a
/// creature to copy; both at once, {5}, is more than floats.
#[test]
fn trumpeting_carnosaur_discovers_three_steps_ahead_and_pays_its_spree() {
    let p0 = PlayerId::new(0);
    let (mut engine, _) = carnosaur_discovers_beside(&[island(); 3], &[three_steps_ahead()], &[]);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let offered = choose_modes(&mut engine, p0, 0b100);
    let cost = baylee_core::mana::ManaCost::parse;
    assert_eq!(
        offered.iter().map(|o| (o.kind, o.cost)).collect::<Vec<_>>(),
        [
            (CastModeKind::Modes(0b010), cost("{3}")),
            (CastModeKind::Modes(0b100), cost("{2}")),
        ]
    );
    assert!(on_stack(&engine, three_steps_ahead()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
}

/// Ten lands revealed (two Mountains on the Plains the deck is made of) are
/// one question, about lands, and nothing is asked about a type no revealed
/// card has.
#[test]
fn atraxa_grand_unifier_asks_only_about_types_revealed() {
    let p0 = PlayerId::new(0);
    let (mut engine, ids) = atraxa_reveals(&[mountain(), mountain()]);
    let (asked, offered) = atraxa_question(&engine).expect("the land question");
    assert_eq!(asked, TypeSet::LAND);
    // Two Mountains and eight of the Plains the deck is made of.
    assert_eq!(offered.len(), 10);
    assert!(ids.iter().all(|id| offered.contains(id)));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ids[1]],
            },
        )
        .unwrap();
    assert!(atraxa_question(&engine).is_none());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&ids[1])
    );
}

/// An opponent with no creature is passed over, and the Balrog is exiled all
/// the same: "up to one" asks nothing of a player with nothing to name.
#[test]
fn the_balrog_of_moria_is_exiled_with_no_creature_to_name() {
    let p0 = PlayerId::new(0);
    let (mut engine, _) = balrog_dies(&[&[]]);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(balrog_in_exile(&engine));
}

#[test]
fn master_of_the_hunt_creates_wolves_that_band_with_each_other_until_shelkin_brownie_strips_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                master_of_the_hunt(),
                shelkin_brownie(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Create two Wolves of the Hunt tokens.
    tap_all_mana_but(&mut engine, p0, None);
    activate(&mut engine, p0, master_of_the_hunt(), 0);
    pass_until(&mut engine, stack_is_empty);
    activate(&mut engine, p0, master_of_the_hunt(), 0);
    pass_until(&mut engine, stack_is_empty);

    let wolves: Vec<ObjectId> = engine
        .state()
        .battlefield_seen()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.token.is_some()
                    && engine.state().names.get(o.characteristics().name) == "Wolves of the Hunt"
            })
        })
        .collect();
    assert_eq!(wolves.len(), 2, "two Wolves of the Hunt tokens created");
    assert!(
        baylee_engine::combat::can_band_with_other(engine.state(), wolves[0], wolves[1]),
        "Wolves of the Hunt can band with each other"
    );

    // Activate Shelkin Brownie targeting the first Wolf token.
    activate(&mut engine, p0, shelkin_brownie(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Shelkin Brownie");
    };
    assert!(options.contains(&wolves[0]));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wolves[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !baylee_engine::combat::can_band_with_other(engine.state(), wolves[0], wolves[1]),
        "Shelkin Brownie removed bands-with-other from the targeted Wolf until end of turn"
    );
    assert!(
        baylee_engine::combat::can_band_with_other(engine.state(), wolves[1], wolves[0]),
        "the untargeted Wolf still has bands-with-other"
    );
    let _ = p1;
}
