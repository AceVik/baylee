//! `cards/sorceries/mv_1/firebolt.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn firebolt() -> CardIndex {
    card_index("7aa31280-12c3-479d-a6dd-f1c669043083")
}

#[track_caller]
fn start_flashback(engine: &mut Engine<RegistryLookup>, player: PlayerId, bolt: ObjectId) {
    engine
        .apply(player, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let flashback = choose_cast_kind(engine, CastModeKind::Flashback);
        engine
            .apply(player, PlayerAction::ChooseMode(flashback))
            .unwrap();
    }
}

/// The ordinary spell costs exactly {R} and deals two damage to either kind of
/// any target. Neither ordinary cast exiles itself.
#[test]
fn firebolt_costs_one_red_and_deals_two_to_a_player_or_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[firebolt(), firebolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, firebolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p1), 18);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(in_graveyard(&engine, p0, firebolt()).is_some());

    cast_from_hand(&mut engine, p0, firebolt());
    let options = aim_at(&mut engine, p0, bears);
    assert!(options.contains(&bears));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Graveyard);
    assert_eq!(life_of(&engine, p1), 18, "the creature was the only target");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .iter()
            .filter(|&&id| {
                engine
                    .state()
                    .object(id)
                    .is_some_and(|object| object.card.is_some_and(|card| card.index == firebolt()))
            })
            .count(),
        2,
        "both ordinary casts go to the graveyard, not exile"
    );
}

/// Flashback is an alternative cost: five total mana including one red, not
/// the printed {R} plus an additional payment. The second spell also exiles.
#[test]
fn firebolt_flashback_costs_four_generic_and_one_red_then_exiles() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 5])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[firebolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bolt = hand_to_graveyard(&mut engine, p0, firebolt());
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
    start_flashback(&mut engine, p0, bolt);
    aim_at(&mut engine, p0, bears);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(engine.state().object(bolt).unwrap().zone, Zone::Stack);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Graveyard);
    assert_eq!(life_of(&engine, p1), 20);
    assert_eq!(engine.state().object(bolt).unwrap().zone, Zone::Exile);
    assert!(in_graveyard(&engine, p0, firebolt()).is_none());
}

/// Four red mana are one short of {4}{R}; the cheaper ordinary cost does not
/// make the card castable from the graveyard.
#[test]
fn firebolt_flashback_is_not_offered_with_only_four_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 4])
        .hand(0, &[firebolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bolt = hand_to_graveyard(&mut engine, p0, firebolt());
    tap_all_mana(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.castable.contains(&bolt));
    assert!(!legal.payable.contains(&bolt));
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: bolt })
            .is_err()
    );
    assert_eq!(engine.state().object(bolt).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
}
