//! Those Old Scientists, played through all three chapters.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn those_old_scientists_recover_own_card_and_shuffle_cards_from_different_graveyards() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let scientists = card_index("5ca4e8c4-8358-49be-bafb-85849bf9bd01");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[scientists, unsummon(), plains()])
        .hand(1, &[unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, me);
    let enemy_instant = hand_to_graveyard(&mut engine, them, unsummon());
    let own_instant = in_hand(&engine, me, unsummon()).unwrap();
    let own_land = in_hand(&engine, me, plains()).unwrap();
    let library_before = library_size(&engine, me);
    cast_from_hand(&mut engine, me, scientists);
    pass_until(&mut engine, |engine| {
        matches!(engine.pending(), Pending::ChooseCards { .. })
    });
    assert_eq!(library_size(&engine, me), library_before - 2);
    assert!(
        matches!(engine.pending(), Pending::ChooseCards { player, min: 2, max: 2, .. } if *player == me)
    );
    engine
        .apply(
            me,
            PlayerAction::ChooseObjects {
                objects: vec![own_instant, own_land],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(own_instant).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine.state().object(own_land).unwrap().zone,
        Zone::Graveyard
    );

    reach_their_main_phase(&mut engine, them);
    assert_eq!(pass_until_targets(&mut engine, me), vec![own_instant]);
    assert!(matches!(
        engine.pending(),
        Pending::ChooseTargets { min: 0, max: 1, .. }
    ));
    engine
        .apply(
            me,
            PlayerAction::ChooseTargets {
                objects: vec![own_instant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(own_instant).unwrap().zone, Zone::Hand);
    assert_eq!(
        engine.state().object(enemy_instant).unwrap().zone,
        Zone::Graveyard
    );

    reach_their_main_phase(&mut engine, them);
    let options = pass_until_targets(&mut engine, me);
    assert_eq!(options.len(), 2);
    assert!(options.contains(&own_land));
    assert!(options.contains(&enemy_instant));
    assert!(matches!(
        engine.pending(),
        Pending::ChooseTargets { min: 0, max: 2, .. }
    ));
    engine
        .apply(
            me,
            PlayerAction::ChooseTargets {
                objects: vec![own_land, enemy_instant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(me))
            .contains(&own_land)
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(them))
            .contains(&enemy_instant)
    );
    assert!(in_graveyard(&engine, me, scientists).is_some());
}
