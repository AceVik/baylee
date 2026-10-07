//! `cards/creatures/mv_1/ragavan_nimble_pilferer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ragavan, Nimble Pilferer — {R} — 2/1 legendary Monkey Pirate: "Whenever
/// Ragavan deals combat damage to a player, create a Treasure token and exile
/// the top card of that player's library. Until end of turn, you may cast that
/// card." The card is cast for {R} (not its dash), handed haste by an
/// equipped Lightning Greaves — a Monkey cast this turn may not attack
/// otherwise (CR 302.6) — and swung into an empty board.
///
/// The token is the proof the trigger reached the *player* and not merely the
/// combat damage step: the board is read empty before the swing and holds one
/// Treasure afterwards, and the defending library is one card shorter, that
/// card in its owner's exile. Until the impulse half was written this test
/// asserted the opposite, the missing clause as a non-move.
#[allow(clippy::too_many_lines)] // a combat played to damage, and the token counted after it
#[test]
fn ragavan_makes_a_treasure_for_connecting_and_exiles_their_top_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(17, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[ragavan_nimble_pilferer(), lightning_greaves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {R} for the Monkey and {2} for the Greaves, off three Mountains. Its
    // dash {1}{R} is offered beside {R}, and not taken.
    cast_from_hand(&mut engine, p0, ragavan_nimble_pilferer());
    let normal = choose_cast_kind(&engine, CastModeKind::Normal);
    engine.apply(p0, PlayerAction::ChooseMode(normal)).unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));
    cast_from_hand(&mut engine, p0, lightning_greaves());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let monkey =
        on_battlefield(&engine, p0, ragavan_nimble_pilferer()).expect("the Monkey resolved");
    let greaves = on_battlefield(&engine, p0, lightning_greaves()).expect("the Greaves resolved");
    assert_eq!(pt(&engine, monkey), (2, 1), "the body the card prints");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing has connected yet"
    );

    // Equip {0} — ability 0 is the static that grants, 1 is the equip.
    activate(&mut engine, p0, lightning_greaves(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![monkey], "the only creature you control");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![monkey],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(greaves)
            .is_some_and(|o| o.attached_to == Some(monkey))
    });
    assert!(
        keywords(&engine, monkey).contains(KeywordSet::HASTE),
        "the Greaves are what let a Monkey cast this turn attack at all (CR 302.6)"
    );

    let their_library = library_size(&engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert!(
        attackers.contains(&monkey),
        "untapped, hasty and no longer sick: {attackers:?}"
    );
    assert_eq!(
        defenders.len(),
        1,
        "the other seat's board is empty, so there is one thing to attack: {defenders:?}"
    );
    let target = defenders.into_iter().next().expect("asserted above");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(monkey, target)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| !tokens_of(e, p0).is_empty());

    assert_eq!(
        engine.state().players[1].life,
        18,
        "the 2/1 got through: damage to the *player* is what the trigger waits for"
    );
    let treasures = tokens_of(&engine, p0);
    assert_eq!(treasures.len(), 1, "one hit, one Treasure");
    assert!(
        types(&engine, treasures[0]).contains(TypeSet::ARTIFACT),
        "and it is the artifact token the card creates"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p1),
        their_library - 1,
        "the top card of their library left it"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p1)).len(),
        1,
        "into its owner's exile, to be cast this turn"
    );
}

/// At a table of three, Ragavan hits seat 2, and "that player" is seat 2:
/// its top card is exiled, seat 1's is not, and seat 0 gets a Treasure.
/// The exiled Llanowar Elves is offered to seat 0 in its second main phase,
/// and cast off a Forest it enters under seat 0's control, still seat 2's
/// card.
#[test]
fn ragavan_exiles_the_top_card_of_the_player_it_hit_and_may_cast_it() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, llanowar_elves(), 3)
        .battlefield(0, &[ragavan_nimble_pilferer(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let ragavan = on_battlefield(&engine, p0, ragavan_nimble_pilferer()).unwrap();
    let (theirs, bystander) = (top_of_library(&engine, p2), top_of_library(&engine, p1));
    ragavan_hits(&mut engine, ragavan, p2);
    assert_eq!(engine.state().players[2].life, 18);
    assert_eq!(tokens_of(&engine, p0).len(), 1, "a Treasure");
    assert_eq!(
        engine.state().object(theirs).map(|o| (o.zone, o.owner)),
        Some((Zone::Exile, p2)),
        "that player's top card, in its owner's exile"
    );
    assert_eq!(
        engine.state().object(bystander).map(|o| o.zone),
        Some(Zone::Library),
        "the other opponent's library is untouched"
    );
    let forest = on_battlefield(&engine, p0, forest()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == forest);
    assert!(priority_offer(&engine).castable.contains(&theirs));
    cast_object_and_resolve(&mut engine, p0, theirs);
    assert_eq!(
        engine
            .state()
            .object(theirs)
            .map(|o| (o.controller, o.owner)),
        Some((p0, p2))
    );
}

/// "You may **cast** that card": a land is not cast (CR 305.9), and the
/// permission is not one to play it (CR 601.1a). Ragavan exiles a Mountain
/// off the top of seat 1's library, and in seat 0's second main phase, its
/// land drop unused, the Mountain is offered neither as a land nor as a
/// spell, and playing it is refused. A permission to play would have
/// offered it.
#[test]
fn ragavan_does_not_let_its_controller_play_an_exiled_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[ragavan_nimble_pilferer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ragavan = on_battlefield(&engine, p0, ragavan_nimble_pilferer()).unwrap();
    let theirs = top_of_library(&engine, p1);
    ragavan_hits(&mut engine, ragavan, p1);
    assert_eq!(
        engine.state().object(theirs).map(|o| o.zone),
        Some(Zone::Exile)
    );
    assert_eq!(engine.state().players[0].lands_played_this_turn, 0);
    assert!(
        crate::casting::play_permission(engine.state(), p0, theirs).is_some(),
        "the permission is there, for casting"
    );
    let offer = priority_offer(&engine);
    assert!(!offer.lands.contains(&theirs), "not a land drop");
    assert!(!offer.castable.contains(&theirs), "and not a spell");
    assert!(
        engine
            .apply(p0, PlayerAction::PlayLand { card: theirs })
            .is_err()
    );
}

/// Dash {1}{R} (CR 702.109a): off two Mountains Ragavan is offered for {R}
/// and for its dash cost. Dashed, it has haste and attacks the turn it is
/// cast, and at the beginning of the end step its delayed trigger returns it
/// to its owner's hand.
#[test]
fn ragavan_dashed_attacks_at_once_and_returns_at_the_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[ragavan_nimble_pilferer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, ragavan_nimble_pilferer());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected the cast options, got {:?}", engine.pending())
    };
    let cost = baylee_core::mana::ManaCost::parse;
    assert_eq!(
        options.iter().map(|o| (o.kind, o.cost)).collect::<Vec<_>>(),
        [
            (CastModeKind::Normal, cost("{R}")),
            (CastModeKind::Dash, cost("{1}{R}")),
        ]
    );
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let ragavan = on_battlefield(&engine, p0, ragavan_nimble_pilferer()).unwrap();
    assert!(keywords(&engine, ragavan).contains(KeywordSet::HASTE));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    ragavan_hits(&mut engine, ragavan, p1);
    assert_eq!(engine.state().players[1].life, 18, "it attacked");
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert!(
        in_hand(&engine, p0, ragavan_nimble_pilferer()).is_some(),
        "returned at the end step"
    );
}

/// Cast for {R} beside the dash it could have paid, Ragavan has no haste,
/// and nothing returns it: it is still on the battlefield on the next turn.
#[test]
fn ragavan_cast_for_its_mana_cost_has_no_haste_and_stays() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[ragavan_nimble_pilferer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, ragavan_nimble_pilferer());
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let ragavan = on_battlefield(&engine, p0, ragavan_nimble_pilferer()).unwrap();
    assert!(!keywords(&engine, ragavan).contains(KeywordSet::HASTE));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{R}} of two"
    );
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert_eq!(
        engine.state().object(ragavan).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
}

/// The permanent a dashed spell became is the one dash returns, and no
/// later object (CR 400.7). Ragavan dashed and then blinked by Ephemerate
/// comes back a new permanent: it has no haste, and the end step's trigger
/// leaves it where it is.
#[test]
fn ragavan_dashed_and_blinked_is_no_longer_dashed() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), plains()])
        .hand(0, &[ragavan_nimble_pilferer(), ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let plains = on_battlefield(&engine, p0, plains()).unwrap();
    tap_mana_except(&mut engine, p0, plains);
    cast_with_floating(&mut engine, p0, ragavan_nimble_pilferer());
    let dash = choose_cast_kind(&engine, CastModeKind::Dash);
    engine.apply(p0, PlayerAction::ChooseMode(dash)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let ragavan = on_battlefield(&engine, p0, ragavan_nimble_pilferer()).unwrap();
    assert!(keywords(&engine, ragavan).contains(KeywordSet::HASTE));
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, ephemerate());
    let _ = aim_at(&mut engine, p0, ragavan);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(ragavan).map(|o| o.zone),
        Some(Zone::Battlefield),
        "blinked back"
    );
    assert!(
        !keywords(&engine, ragavan).contains(KeywordSet::HASTE),
        "a new object, not the dashed one"
    );
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert_eq!(
        engine.state().object(ragavan).map(|o| o.zone),
        Some(Zone::Battlefield),
        "and the dash trigger did not return it"
    );
}
