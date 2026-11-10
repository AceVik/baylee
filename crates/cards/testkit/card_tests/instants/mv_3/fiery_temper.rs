//! `cards/instants/mv_3/fiery_temper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn fiery_temper() -> CardIndex {
    card_index("f07bd49d-8e71-4d56-be2a-638514011318")
}

/// "Fiery Temper deals 3 damage to any target." Two ordinary {1}{R}{R} casts
/// reach either a player or a creature and go to the graveyard afterwards.
#[test]
fn fiery_temper_costs_three_and_deals_three_to_a_player_or_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[fiery_temper(), fiery_temper()])
        .battlefield(0, &[mountain(); 6])
        .battlefield(1, &[index::WALL_OF_STONE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 6);
    let wall = on_battlefield(&engine, p1, index::WALL_OF_STONE).unwrap();

    cast_with_floating(&mut engine, p0, fiery_temper());
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
    assert_eq!(life_of(&engine, p1), 17);
    assert_eq!(engine.state().object(wall).unwrap().damage, 0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);

    cast_with_floating(&mut engine, p0, fiery_temper());
    aim_at(&mut engine, p0, wall);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, index::WALL_OF_STONE).is_some());
    assert_eq!(engine.state().object(wall).unwrap().damage, 3);
    assert_eq!(life_of(&engine, p1), 17);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(mine(&engine, p0, fiery_temper(), Zone::Graveyard).len(), 2);
    assert!(mine(&engine, p0, fiery_temper(), Zone::Exile).is_empty());
}

/// Madness {R} casts the exact discarded incarnation during an opponent's
/// turn. Prying Eyes supplies a real discard and leaves exactly one red mana.
#[test]
fn fiery_temper_madness_costs_one_red_during_an_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[fiery_temper(), index::PRYING_EYES])
        .battlefield(
            0,
            &[
                mountain(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[index::WALL_OF_STONE])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    let wall = on_battlefield(&engine, p1, index::WALL_OF_STONE).unwrap();
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 7);

    cast_with_floating(&mut engine, p0, index::PRYING_EYES);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, min: 2, max: 2, .. } if *player == p0),
    );
    let temper = in_hand(&engine, p0, fiery_temper()).unwrap();
    let other = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|card| *card != temper)
        .expect("Prying Eyes drew another card to discard");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![temper, other],
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
            } if *player == p0 && *card == temper
        )
    });
    assert_eq!(engine.state().object(temper).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    aim_at(&mut engine, p0, wall);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().turn.active, p1, "madness ignored timing");
    assert!(on_battlefield(&engine, p1, index::WALL_OF_STONE).is_some());
    assert_eq!(engine.state().object(wall).unwrap().damage, 3);
    assert_eq!(life_of(&engine, p1), 20);
    assert_eq!(engine.state().object(temper).unwrap().zone, Zone::Graveyard);
    assert!(mine(&engine, p0, fiery_temper(), Zone::Exile).is_empty());
}
