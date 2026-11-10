//! `cards/sorceries/mv_4/deep_analysis.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn deep_analysis() -> CardIndex {
    card_index("579cbd92-797f-4cdf-91ed-fca7a523eae5")
}

fn hand_size(engine: &Engine<RegistryLookup>, player: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(player)).len()
}

/// Either player can draw the two cards. Flashback costs {1}{U} and 3 life,
/// not the ordinary {3}{U}, and the card is exiled after its second effect.
#[test]
fn deep_analysis_targets_a_player_and_flashes_back_for_mana_and_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 6])
        .hand(0, &[deep_analysis()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let my_library = library_size(&engine, p0);
    let their_library = library_size(&engine, p1);

    cast_from_hand(&mut engine, p0, deep_analysis());
    let Pending::ChoosePlayer { options, .. } = engine.pending().clone() else {
        panic!("expected a player target, got {:?}", engine.pending());
    };
    assert!(options.contains(&p0) && options.contains(&p1));
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_size(&engine, p1), 2);
    assert_eq!(hand_size(&engine, p0), 0);
    assert_eq!(library_size(&engine, p1), their_library - 2);
    assert_eq!(library_size(&engine, p0), my_library);
    assert_eq!(life_of(&engine, p0), 20);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    let analysis = in_graveyard(&engine, p0, deep_analysis()).unwrap();

    engine
        .apply(p0, PlayerAction::CastSpell { card: analysis })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let flashback = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine
            .apply(p0, PlayerAction::ChooseMode(flashback))
            .unwrap();
    }
    engine.apply(p0, PlayerAction::ChoosePlayer(p0)).unwrap();
    assert_eq!(life_of(&engine, p0), 17, "the life is paid while casting");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_size(&engine, p0), 2);
    assert_eq!(hand_size(&engine, p1), 2);
    assert_eq!(library_size(&engine, p0), my_library - 2);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(engine.state().object(analysis).unwrap().zone, Zone::Exile);
}

/// Two life cannot pay the printed three, even with all the mana available.
#[test]
fn deep_analysis_flashback_is_not_offered_at_two_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .life(0, 2)
        .battlefield(0, &[island(), island()])
        .hand(0, &[deep_analysis()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let analysis = hand_to_graveyard(&mut engine, p0, deep_analysis());
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(!legal.castable.contains(&analysis));
    assert!(!legal.payable.contains(&analysis));
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: analysis })
            .is_err()
    );
    assert_eq!(life_of(&engine, p0), 2);
    assert_eq!(
        engine.state().object(analysis).unwrap().zone,
        Zone::Graveyard
    );
}
