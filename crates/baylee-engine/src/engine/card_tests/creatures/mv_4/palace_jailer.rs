//! `cards/creatures/mv_4/palace_jailer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Palace Jailer prints two enters-the-battlefield triggers: "you become the
/// monarch", and "exile target creature an opponent controls until an opponent
/// becomes the monarch". Both are played on one board, because each is what the
/// other cannot show — the exile is read as the opponent's Elf leaving the
/// battlefield for exile while my own Elf is never on the target menu, and the
/// designation leaves no marker on the table, so it is read where the rules make
/// it visible: the monarch draws a card at the beginning of their own end step
/// (CR 724.2), which nothing else on this board would do.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn palace_jailer_crowns_its_controller_and_exiles_an_opponents_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .hand(0, &[palace_jailer()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    // Four Plains and only those: the Elf beside them is a creature this test
    // reads afterwards, and a bystander tapped for its own {G} is a board that
    // has already changed for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Plains pay {{2}}{{W}}{{W}}, and the Elf kept back gave nothing"
    );
    cast_with_floating(&mut engine, p0, palace_jailer());

    // The spell resolves, both enters triggers go on the stack, and the one
    // that points at something asks its target question before either resolves
    // (CR 603.3d, CR 601.2c).
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let jailer = on_battlefield(&engine, p0, palace_jailer())
        .expect("the Jailer is on the battlefield before its enters triggers ask");
    assert_eq!(pt(&engine, jailer), (2, 2), "the body the card prints");

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the exile trigger's target question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the Jailer's controller is the one that aims it"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature, and the trigger asks once"
    );
    assert!(
        options.contains(&their_elf),
        "\"target creature an opponent controls\" names the Elf across the table: {options:?}"
    );
    assert!(
        !options.contains(&bystander),
        "my own Elf is a creature, and not one an opponent controls: {options:?}"
    );
    assert!(
        !options.contains(&jailer),
        "and the Jailer is its controller's own creature, so it cannot jail itself: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_elf],
            },
        )
        .expect("the Elf the question offered was a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the creature the trigger named left the battlefield"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&their_elf),
        "and it is in exile under its owner, which is where \"until an opponent \
         becomes the monarch\" leaves it: the seat that became the monarch is \
         the one that cast the Jailer, and no opponent of theirs is the monarch"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_none(),
        "exile is not the graveyard — a creature that had merely died would \
         satisfy the battlefield count above"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature nobody named never moved"
    );
    assert!(
        on_battlefield(&engine, p0, palace_jailer()).is_some(),
        "and the Jailer itself is still standing under its controller"
    );

    // The designation leaves no marker on the table, so it is read where the
    // rules make it visible: the monarch draws a card at the beginning of their
    // own end step (CR 724.2). Everything walked here is p0's combat, p0's end
    // step and p1's untap, upkeep and draw — none of which draws a card for p0
    // unless the first trigger resolved.
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"you become the monarch\": the monarch's own end step arrived and drew \
         for it. A Jailer whose first trigger had never resolved leaves the \
         library exactly as it was"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and the card is in hand, so an emptied library would not satisfy the count above"
    );
}

/// Palace Jailer's exile ends when "an opponent becomes the monarch", and
/// only then. Skyclave Apparition's has no end at all. Each holds one of
/// seat 1's Elves when seat 1 takes the crown with a third Elf's combat
/// damage (CR 724.2), and only the Jailer's prisoner comes back, under its
/// owner's control (CR 610.3c).
///
/// Every card linked to an exile was released as soon as anyone but its
/// host's controller became the monarch, so the Apparition's Elf walked out
/// with the Jailer's.
#[test]
#[allow(clippy::too_many_lines)] // two exiles, a turn and a combat, told in order
fn palace_jailer_frees_its_prisoner_when_an_opponent_is_crowned_and_skyclave_keeps_its_own() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(616, plains())
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
            ],
        )
        .battlefield(1, &[llanowar_elves(), llanowar_elves(), llanowar_elves()])
        .hand(0, &[palace_jailer(), skyclave_apparition()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let elves: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == llanowar_elves()))
        })
        .collect();
    let [jailed, held, attacker] = elves[..] else {
        panic!("three Elves on seat 1's side: {elves:?}")
    };
    let plains = plains_of(&engine, p0);

    tap_mana_where(&mut engine, p0, |id| plains[..4].contains(&id));
    cast_with_floating(&mut engine, p0, palace_jailer());
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&jailed), "{options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![jailed],
                players: vec![],
            },
        )
        .expect("the first Elf is targeted");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().monarch,
        Some(p0),
        "the Jailer crowned seat 0"
    );

    tap_mana_where(&mut engine, p0, |id| plains[4..].contains(&id));
    cast_with_floating(&mut engine, p0, skyclave_apparition());
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&held), "{options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![held],
                players: vec![],
            },
        )
        .expect("the second Elf is targeted");
    pass_until(&mut engine, stack_is_empty);
    for elf in [jailed, held] {
        assert_eq!(
            engine.state().object(elf).map(|o| o.zone),
            Some(Zone::Exile),
            "both Elves are exiled"
        );
    }

    // Seat 1's turn: the third Elf attacks the monarch and nobody blocks.
    reach_their_main_phase(&mut engine, p1);
    let _ = attack_and_collect_blocks(&mut engine, attacker, p0);
    pass_until(&mut engine, |e| {
        e.state().monarch == Some(p1) && stack_is_empty(e)
    });

    let freed = engine.state().object(jailed).expect("the Jailer's Elf");
    assert_eq!(
        freed.zone,
        Zone::Battlefield,
        "an opponent of the Jailer's controller became the monarch"
    );
    assert_eq!((freed.owner, freed.controller), (p1, p1));
    assert_eq!(
        engine.state().object(held).map(|o| o.zone),
        Some(Zone::Exile),
        "Skyclave Apparition's exile names no event it lasts until"
    );
    assert!(
        on_battlefield(&engine, p0, palace_jailer()).is_some()
            && on_battlefield(&engine, p0, skyclave_apparition()).is_some(),
        "and both hosts are still standing: only the crown moved"
    );
}

/// Palace Jailer at a table of three, and the monarch leaves the game
/// (CR 724.4). Seat 0 is the monarch, with seat 1's Elves in the Jailer's
/// exile, and concedes during seat 2's turn. The crown passes to the active
/// player, seat 2, as seat 0 leaves: not to seat 1, the next seat after the
/// leaver. Seat 2 is an opponent of the player who exiled the Elves, so the
/// Jailer's exile ends and they come back to seat 1 (CR 610.3c).
///
/// The crown stayed with the seat that had left, and the Elves stayed in
/// exile for the rest of the game.
#[test]
fn a_monarch_who_leaves_crowns_the_active_player_and_frees_the_jailers_prisoner() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(619, plains(), 3)
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[palace_jailer()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("p1's Elves");
    let plains = plains_of(&engine, p0);
    tap_mana_where(&mut engine, p0, |id| plains.contains(&id));
    cast_with_floating(&mut engine, p0, palace_jailer());
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&elves), "{options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .expect("the Elves are targeted");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().monarch,
        Some(p0),
        "the Jailer crowned seat 0"
    );
    assert_eq!(
        engine.state().object(elves).map(|o| o.zone),
        Some(Zone::Exile)
    );

    reach_their_main_phase(&mut engine, p2);
    assert_eq!(engine.state().turn.active, p2);
    engine
        .apply(p0, PlayerAction::Concede)
        .expect("a concession is legal at any time");
    assert!(engine.state().has_left(p0));
    assert_eq!(
        engine.state().monarch,
        Some(p2),
        "the active player, and not seat 1, who sits next to the leaver"
    );
    let freed = engine.state().object(elves).expect("seat 1's Elves");
    assert_eq!(freed.zone, Zone::Battlefield, "an opponent was crowned");
    assert_eq!((freed.owner, freed.controller), (p1, p1));
}
