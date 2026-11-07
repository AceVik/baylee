//! `cards/instants/mv_3/circular_logic.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn circular_logic() -> CardIndex {
    card_index("b071bbaf-79fa-413b-807b-2cc86c8a8125")
}

/// "Counter target spell unless its controller pays {1} for each card in your
/// graveyard." The controller of Circular Logic supplies the count, and the
/// target spell's controller can pay it to let that spell resolve.
#[test]
fn circular_logic_charges_its_controllers_graveyard_and_allows_payment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[index::GRIZZLY_BEARS])
        .battlefield(0, &[forest(); 5])
        .hand(1, &[circular_logic()])
        .battlefield(1, &[island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 4);
    seed_graveyard(&mut engine, p1, 2);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::GRIZZLY_BEARS);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    let spell = engine.state().zones.list(ZoneLocation::Stack)[0];
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, circular_logic());
    aim_at(&mut engine, p1, spell);
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana: 2 },
                ..
            } if *player == p0
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(in_graveyard(&engine, p1, circular_logic()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        3,
        "Circular Logic joins its two seeded cards only after resolving"
    );
}

/// Declining the dynamic tax counters the target spell and puts both spells
/// into their owners' graveyards.
#[test]
fn circular_logic_counters_when_the_target_controller_declines() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[index::GRIZZLY_BEARS])
        .battlefield(0, &[forest(), forest()])
        .hand(1, &[circular_logic()])
        .battlefield(1, &[island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p1, 3);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::GRIZZLY_BEARS);
    let spell = engine.state().zones.list(ZoneLocation::Stack)[0];
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, circular_logic());
    aim_at(&mut engine, p1, spell);

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana: 3 },
                ..
            } if *player == p0
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p0, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p1, circular_logic()).is_some());
}

/// Madness casts the discarded incarnation for {U} while an opposing creature
/// spell is still on the stack. Prying Eyes supplies a real discard and leaves
/// exactly one blue mana; its resolution and the other discard make the tax 2.
#[test]
fn circular_logic_madness_counters_a_spell_during_an_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[circular_logic(), index::PRYING_EYES])
        .battlefield(0, &[island(); 7])
        .hand(1, &[index::GRIZZLY_BEARS])
        .battlefield(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, index::GRIZZLY_BEARS);
    let bears = engine.state().zones.list(ZoneLocation::Stack)[0];
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 7);
    cast_with_floating(&mut engine, p0, index::PRYING_EYES);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, min: 2, max: 2, .. } if *player == p0),
    );
    let logic = in_hand(&engine, p0, circular_logic()).unwrap();
    let other = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|card| *card != logic)
        .expect("Prying Eyes drew another card to discard");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![logic, other],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::Madness { card },
                ..
            } if *player == p0 && *card == logic
        )
    });
    assert_eq!(engine.state().object(logic).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    aim_at(&mut engine, p0, bears);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana: 2 },
                ..
            } if *player == p1
        )
    });
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().turn.active, p1, "madness ignored timing");
    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p0, index::PRYING_EYES).is_some());
    assert_eq!(engine.state().object(logic).unwrap().zone, Zone::Graveyard);
    assert!(mine(&engine, p0, circular_logic(), Zone::Exile).is_empty());
}
