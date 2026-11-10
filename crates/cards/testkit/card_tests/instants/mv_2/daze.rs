//! `cards/instants/mv_2/daze.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn daze_card() -> CardIndex {
    card_index("70486bee-6ee7-41ea-b834-8caf4699302b")
}

fn cast_daze_at(engine: &mut Engine<RegistryLookup>, player: PlayerId, spell: ObjectId) {
    cast_with_floating(engine, player, daze_card());
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let alternative = choose_cast_kind(engine, CastModeKind::Alternative(0));
        engine
            .apply(player, PlayerAction::ChooseMode(alternative))
            .unwrap();
    }
    aim_at(engine, player, spell);
}

/// Daze ({1}{U} Instant): return a controlled Island to its owner's hand
/// instead of paying mana, then counter the targeted spell if its controller
/// declines to pay {1}.
#[test]
fn daze_returns_an_island_instead_of_paying_mana_and_counters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[daze_card()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, grizzly_bears());
    let spell = on_stack(&engine, grizzly_bears()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let returned = on_battlefield(&engine, p0, island()).unwrap();
    let other = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            *id != returned
                && engine.state().object(*id).is_some_and(|object| {
                    object.controller == p0
                        && object.card.is_some_and(|card| card.index == island())
                })
        })
        .unwrap();
    let daze = in_hand(&engine, p0, daze_card()).unwrap();
    cast_daze_at(&mut engine, p0, spell);

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the Island return cost, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(
        (player, min, max, prompt),
        (p0, 1, 1, ChoicePrompt::CostReturn)
    );
    assert_eq!(options, vec![returned, other]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![returned],
            },
        )
        .unwrap();

    assert!(
        engine
            .state()
            .zones
            .contains(returned, ZoneLocation::Hand(p0))
    );
    assert!(
        engine
            .state()
            .zones
            .contains(other, ZoneLocation::Battlefield)
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(engine.state().object(daze).unwrap().zone, Zone::Stack);
    assert!(engine.state().object(daze).unwrap().alt_cast);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
}

/// The targeted spell's controller may instead pay {1}; the pitch cost is
/// already paid and Daze does not counter that spell.
#[test]
fn daze_does_not_counter_when_the_spells_controller_pays_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[daze_card()])
        .battlefield(1, &[forest(), forest(), forest()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, grizzly_bears());
    let spell = on_stack(&engine, grizzly_bears()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let island = on_battlefield(&engine, p0, island()).unwrap();
    cast_daze_at(&mut engine, p0, spell);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![island],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());
    assert!(
        engine
            .state()
            .zones
            .contains(island, ZoneLocation::Hand(p0))
    );
}

/// An opponent's Island cannot pay the caster's cost, and a Forest is not an
/// Island. With no blue mana there is no legal way to cast Daze here.
#[test]
fn daze_cannot_return_an_island_its_caster_does_not_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[forest()])
        .hand(0, &[daze_card()])
        .battlefield(1, &[island(), forest()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, grizzly_bears());
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    let daze = in_hand(&engine, p0, daze_card()).unwrap();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.castable.contains(&daze));
    assert!(!legal.payable.contains(&daze));
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: daze })
            .is_err()
    );
    assert!(engine.state().zones.contains(daze, ZoneLocation::Hand(p0)));
}

/// The ordinary mana cost remains available and returns no Island.
#[test]
fn daze_can_still_be_cast_for_its_printed_mana_cost() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[daze_card()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, grizzly_bears());
    let spell = on_stack(&engine, grizzly_bears()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let daze = in_hand(&engine, p0, daze_card()).unwrap();
    cast_from_hand(&mut engine, p0, daze_card());
    let normal = choose_cast_kind(&engine, CastModeKind::Normal);
    engine.apply(p0, PlayerAction::ChooseMode(normal)).unwrap();
    aim_at(&mut engine, p0, spell);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter(|id| engine.state().object(**id).unwrap().controller == p0)
            .count(),
        2
    );
    assert!(!engine.state().object(daze).unwrap().alt_cast);
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
}
