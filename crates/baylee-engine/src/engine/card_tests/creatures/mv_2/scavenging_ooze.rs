//! `cards/creatures/mv_2/scavenging_ooze.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The exile targets cards in either graveyard, never battlefield permanents.
#[test]
fn scavenging_ooze_exiles_a_card_from_either_players_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(3001, forest())
        .battlefield(0, &[scavenging_ooze(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // One card in each graveyard, so the target spec's `EachPlayer` is a
    // question about the table rather than about the caster's own bin. The
    // harness' dev capability is what puts them there (no `SeatSpec` field
    // for a graveyard), and `seed_graveyard` refreshes the offer so an
    // ability that reads a graveyard is not withheld for want of one.
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);
    let mine = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    let theirs = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p1))
        .clone();
    assert_eq!(
        (mine.len(), theirs.len()),
        (1, 1),
        "one seeded card apiece and no discard before it"
    );
    let doomed = theirs[0];

    // Mana into the pool first: the offer is computed against what is
    // floating, and {G} that is still sitting on a Forest pays for nothing.
    tap_all_mana(&mut engine, p0);
    let ooze = on_battlefield(&engine, p0, scavenging_ooze()).expect("the Ooze resolved");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    let offered = deeds(&legal, &[ooze]);
    assert!(
        matches!(offered[..], [(0, Deed::Ability(0))]),
        "{{G}} in the pool and a card in a graveyard: the one line the Ooze \
         prints is offered: {offered:?}"
    );

    activate(&mut engine, p0, scavenging_ooze(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the exile targets, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&mine[0]) && options.contains(&doomed),
        "\"target card from a graveyard\" reaches either side of the table: \
         {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and holds the graveyard cards alone — the Ooze's own 2/2 body is no \
         card in a graveyard: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the card the question enumerated is the one it takes");
    pass_until(&mut engine, stack_is_empty);

    let exiled = engine
        .state()
        .object(doomed)
        .expect("the card still exists");
    assert_eq!(exiled.zone, Zone::Exile, "the named card left for exile");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        0,
        "the opponent's graveyard is one card lighter"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "and the card on this side was never touched"
    );
    assert!(
        on_battlefield(&engine, p0, scavenging_ooze()).is_some(),
        "the Ooze stays where it was: nothing about the ability moves its \
         source"
    );
}

/// #240: the trigger names the card that died and nothing else. A Scavenging
/// Ooze exiles the Rider out of the graveyard in response; exile is a public
/// zone, so the object keeps its handle, and a resolver that did not ask where
/// it was would pull it out of exile onto the library. CR 400.7 makes it a new
/// object there, so it stays exiled.
#[test]
fn murderous_rider_exiled_in_response_stays_in_exile() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(389, island())
        .battlefield(0, &[murderous_rider()])
        .battlefield(1, &[mountain(), forest(), scavenging_ooze()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let rider = on_battlefield(&engine, p0, murderous_rider()).expect("the Rider is seated");

    bolt_to_death(&mut engine, p1, rider);
    assert!(!stack_is_empty(&engine), "the dies trigger is waiting");
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        player, p1,
        "the active player holds priority over the trigger"
    );
    let forests = all_of(&engine, p1, forest());
    tap_mana_where(&mut engine, p1, |id| forests.contains(&id));
    activate(&mut engine, p1, scavenging_ooze(), 0);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![rider],
                players: vec![],
            },
        )
        .expect("the Rider is a card in a graveyard");
    pass_until(&mut engine, stack_is_empty);

    let library = engine.state().zones.list(ZoneLocation::Library(p0));
    assert!(
        library
            .iter()
            .all(|id| !engine_card_is(&engine, *id, murderous_rider())),
        "the trigger found no Rider in the graveyard and moved nothing"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .iter()
            .any(|id| engine_card_is(&engine, *id, murderous_rider())),
        "the Rider is still in exile"
    );
}

#[test]
fn ooze_rewards_only_a_creature_and_never_places_counters_on_a_new_source_incarnation() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (creature, blink, illegal) in [
        (false, false, false),
        (true, false, false),
        (true, true, false),
        (true, false, true),
    ] {
        let victim = if creature {
            grizzly_bears()
        } else {
            index::MIND_STONE
        };
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[scavenging_ooze(), index::DOUBLING_SEASON, forest()])
            .battlefield(1, &[victim])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let ooze = on_battlefield(&e, p0, scavenging_ooze()).unwrap();
        let card = on_battlefield(&e, p1, victim).unwrap();
        bury(&mut e, &[card]);
        tap_all_mana(&mut e, p0);
        activate(&mut e, p0, scavenging_ooze(), 0);
        aim_at(&mut e, p0, card);
        if blink {
            let s = e.dev_state_mut(p0).unwrap();
            s.move_object(
                ooze,
                ZoneLocation::Exile(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
            s.object_mut(ooze).unwrap().kind = crate::object::ObjectKind::Permanent;
            s.move_object(
                ooze,
                ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        }
        if illegal {
            e.dev_state_mut(p0)
                .unwrap()
                .move_object(
                    card,
                    ZoneLocation::Exile(p1),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
        }
        let life = e.state().players[0].life;
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().players[0].life,
            life + i32::from(creature && !illegal)
        );
        assert_eq!(
            counters_on(&e, ooze, CounterKind::P1P1),
            if creature && !blink && !illegal { 2 } else { 0 }
        );
        assert_eq!(e.state().object(card).unwrap().zone, Zone::Exile);
    }
}
