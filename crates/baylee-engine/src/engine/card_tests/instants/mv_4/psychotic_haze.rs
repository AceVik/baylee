//! `cards/instants/mv_4/psychotic_haze.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn psychotic_haze() -> CardIndex {
    card_index("4b6aa5e2-b539-46cd-804b-cd9e996a9456")
}

/// "Psychotic Haze deals 1 damage to each creature and each player." Both
/// players and the 1/1 Elves on either side are hit without a target choice.
/// The Wall survives with one damage marked, and the ordinary cast spends {2}{B}{B}.
#[test]
fn psychotic_haze_hits_each_player_and_each_creature_without_targeting() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[psychotic_haze()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), index::WALL_OF_STONE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let own_elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let their_elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let wall = on_battlefield(&engine, p1, index::WALL_OF_STONE).unwrap();

    tap_mana_except(&mut engine, p0, own_elves);
    cast_with_floating(&mut engine, p0, psychotic_haze());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life_of(&engine, p0), 19, "the caster is also a player");
    assert_eq!(life_of(&engine, p1), 19, "the opponent is also a player");
    assert_eq!(
        engine.state().object(own_elves).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine.state().object(their_elves).unwrap().zone,
        Zone::Graveyard
    );
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p1, index::WALL_OF_STONE).is_some());
    assert_eq!(engine.state().object(wall).unwrap().damage, 1);
    assert!(in_graveyard(&engine, p0, psychotic_haze()).is_some());
}

/// Madness {1}{B} casts the exact discarded incarnation during an opponent's
/// turn. Prying Eyes supplies the discard; the two remaining black mana pay
/// the entire alternative cost before Haze damages the Wall and both players.
#[test]
fn psychotic_haze_madness_hits_each_player_and_creature_for_two_mana() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[psychotic_haze(), index::PRYING_EYES])
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
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
    assert_eq!(engine.state().players[0].mana_pool.total(), 8);

    cast_with_floating(&mut engine, p0, index::PRYING_EYES);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, min: 2, max: 2, .. } if *player == p0),
    );
    let haze = in_hand(&engine, p0, psychotic_haze()).unwrap();
    let other = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|card| *card != haze)
        .expect("Prying Eyes drew another card to discard");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![haze, other],
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
            } if *player == p0 && *card == haze
        )
    });
    assert_eq!(engine.state().object(haze).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "Prying Eyes leaves the two black mana needed for Madness"
    );

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().turn.active, p1, "madness ignored timing");
    assert_eq!(life_of(&engine, p0), 19);
    assert_eq!(life_of(&engine, p1), 19);
    assert!(on_battlefield(&engine, p1, index::WALL_OF_STONE).is_some());
    assert_eq!(engine.state().object(wall).unwrap().damage, 1);
    assert!(in_graveyard(&engine, p0, index::PRYING_EYES).is_some());
    assert_eq!(engine.state().object(haze).unwrap().zone, Zone::Graveyard);
    assert!(mine(&engine, p0, psychotic_haze(), Zone::Exile).is_empty());
}
