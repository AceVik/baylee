//! `cards/enchantments/mv_2/oath_of_druids.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn oath_of_druids() -> CardIndex {
    card_index("cf0dcb27-f02e-4065-81dc-75fbbb09ea3f")
}

fn target_opponent(engine: &mut Engine<RegistryLookup>, active: PlayerId, target: PlayerId) {
    pass_until(engine, |e| {
        e.state().turn.active == active && matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, active, "the player whose upkeep it is chooses");
    assert!(
        options.is_empty(),
        "this is a player target, not an object target"
    );
    assert_eq!(player_options, vec![target]);
    engine
        .apply(
            active,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![target],
            },
        )
        .unwrap();
}

fn accept_reveal(engine: &mut Engine<RegistryLookup>, active: PlayerId, accept: bool) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::MayDo,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the active player must receive Oath's optional reveal: {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, active);
    engine.apply(active, PlayerAction::YesNo(accept)).unwrap();
    pass_until(engine, stack_is_empty);
}

#[test]
fn oath_reveals_through_the_first_creature_and_moves_each_card_to_its_printed_zone() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[oath_of_druids()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[forest(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let creature = hand_to_library_top(&mut engine, P0, llanowar_elves());
    let lower = hand_to_library_top(&mut engine, P0, forest());
    let upper = hand_to_library_top(&mut engine, P0, forest());
    let journal_from = engine.state().journal.len();

    target_opponent(&mut engine, P0, P1);
    accept_reveal(&mut engine, P0, true);

    assert_eq!(
        engine.state().object(creature).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
    assert_eq!(
        engine.state().object(creature).map(|o| o.controller),
        Some(P0)
    );
    for card in [lower, upper] {
        assert_eq!(
            engine.state().object(card).map(|o| o.zone),
            Some(Zone::Graveyard)
        );
        assert_eq!(
            engine.state().object(card).and_then(|o| o.zone_owner),
            Some(P0)
        );
    }
    assert!(
        engine.state().journal.entries()[journal_from..]
            .iter()
            .any(|entry| {
                matches!(&entry.event, GameEvent::Revealed { player, cards }
            if *player == P0 && cards == &vec![upper, lower, creature])
            })
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Library(P0))
            .is_empty()
    );
}

#[test]
fn oath_only_targets_an_opponent_with_strictly_more_creatures() {
    for opponent_creatures in [0, 1] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[oath_of_druids(), llanowar_elves()])
            .battlefield(1, &vec![llanowar_elves(); opponent_creatures])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, P0);
        assert!(engine.state().zones.list(ZoneLocation::Stack).is_empty());
        assert_eq!(all_on_battlefield(&engine, P0, llanowar_elves()).len(), 1);
    }
}

#[test]
fn oath_declining_leaves_the_library_and_graveyard_unchanged() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[oath_of_druids()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let creature = hand_to_library_top(&mut engine, P0, llanowar_elves());
    let graveyard_before = engine.state().zones.list(ZoneLocation::Graveyard(P0)).len();
    let journal_from = engine.state().journal.len();

    target_opponent(&mut engine, P0, P1);
    accept_reveal(&mut engine, P0, false);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P0)).last(),
        Some(&creature)
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        graveyard_before
    );
    assert!(
        !engine.state().journal.entries()[journal_from..]
            .iter()
            .any(|entry| matches!(entry.event, GameEvent::Revealed { .. }))
    );
}

#[test]
fn oath_without_a_creature_reveals_the_whole_library_into_the_graveyard() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[oath_of_druids()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    let cards = [
        hand_to_library_top(&mut engine, P0, forest()),
        hand_to_library_top(&mut engine, P0, forest()),
        hand_to_library_top(&mut engine, P0, forest()),
    ];
    let graveyard_before = engine.state().zones.list(ZoneLocation::Graveyard(P0)).len();
    let library_before = engine.state().zones.list(ZoneLocation::Library(P0)).len();

    target_opponent(&mut engine, P0, P1);
    accept_reveal(&mut engine, P0, true);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P0))
            .is_empty()
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        graveyard_before + library_before,
    );
    for card in cards {
        assert_eq!(
            engine.state().object(card).map(|o| o.zone),
            Some(Zone::Graveyard)
        );
    }
    assert!(all_on_battlefield(&engine, P0, llanowar_elves()).is_empty());
}

#[test]
fn oath_on_an_opponents_upkeep_uses_that_players_comparison_choice_and_library() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[oath_of_druids(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let creature = hand_to_library_top(&mut engine, P1, llanowar_elves());

    target_opponent(&mut engine, P1, P0);
    accept_reveal(&mut engine, P1, true);

    assert_eq!(
        engine.state().object(creature).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
    assert_eq!(
        engine.state().object(creature).map(|o| o.controller),
        Some(P1)
    );
    assert_eq!(all_on_battlefield(&engine, P1, llanowar_elves()).len(), 2);
}
