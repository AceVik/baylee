//! Destroy Evil: both choices, target restrictions, and resolution legality.

#[allow(clippy::wildcard_imports)] // Shared card fixtures and gameplay helpers.
use super::*;

fn destroy_evil() -> CardIndex {
    card_index("b75e667a-1ceb-445a-acf4-ae1fd2dae9e7")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn choose_destroy_mode(engine: &mut Engine<RegistryLookup>, seat: PlayerId, mode: usize) {
    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("Destroy Evil must ask for its mode: {:?}", engine.pending());
    };
    assert_eq!(player, seat);
    let slot = options
        .iter()
        .position(|option| option.kind == CastModeKind::Mode(mode))
        .expect("the printed mode is available");
    engine.apply(seat, PlayerAction::ChooseMode(slot)).unwrap();
}

#[test]
fn destroy_evil_each_mode_offers_only_its_printed_targets_and_destroys_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for mode in 0..2 {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[plains(), plains(), giant_spider()])
            .battlefield(
                1,
                &[
                    giant_spider(),
                    hill_giant(),
                    their_enchantment(),
                    quiet_artifact(),
                ],
            )
            .hand(0, &[destroy_evil()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let mine = on_battlefield(&engine, p0, giant_spider()).unwrap();
        let four = on_battlefield(&engine, p1, giant_spider()).unwrap();
        let three = on_battlefield(&engine, p1, hill_giant()).unwrap();
        let enchantment = on_battlefield(&engine, p1, their_enchantment()).unwrap();
        let artifact = on_battlefield(&engine, p1, quiet_artifact()).unwrap();
        let land = on_battlefield(&engine, p0, plains()).unwrap();
        let (target, target_card) = if mode == 0 {
            (four, giant_spider())
        } else {
            (enchantment, their_enchantment())
        };
        cast_from_hand(&mut engine, p0, destroy_evil());
        choose_destroy_mode(&mut engine, p0, mode);
        let menu = aim_at(&mut engine, p0, target);
        assert_eq!(
            menu.contains(&four),
            mode == 0,
            "toughness exactly four qualifies only for creature mode"
        );
        assert_eq!(
            menu.contains(&mine),
            mode == 0,
            "own creatures can be targeted too"
        );
        assert_eq!(menu.contains(&enchantment), mode == 1);
        assert!(
            !menu.contains(&three),
            "toughness three is below the boundary"
        );
        assert!(!menu.contains(&artifact));
        assert!(!menu.contains(&land));
        assert_eq!(menu.len(), if mode == 0 { 2 } else { 1 });
        pass_until(&mut engine, stack_is_empty);
        assert!(in_graveyard(&engine, p1, target_card).is_some());
        assert!(on_battlefield(&engine, p1, target_card).is_none());
        assert!(on_battlefield(&engine, p0, giant_spider()).is_some());
        assert!(on_battlefield(&engine, p1, hill_giant()).is_some());
        assert!(on_battlefield(&engine, p1, quiet_artifact()).is_some());
        let survivor = if mode == 0 {
            their_enchantment()
        } else {
            giant_spider()
        };
        assert!(on_battlefield(&engine, p1, survivor).is_some());
        assert!(in_graveyard(&engine, p0, destroy_evil()).is_some());
    }
}

#[test]
fn destroy_evil_does_not_destroy_a_creature_returned_to_hand_in_response() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), island()])
        .battlefield(1, &[giant_spider(), their_enchantment()])
        .hand(0, &[destroy_evil(), unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spider = on_battlefield(&engine, p1, giant_spider()).unwrap();
    cast_from_hand(&mut engine, p0, destroy_evil());
    choose_destroy_mode(&mut engine, p0, 0);
    aim_at(&mut engine, p0, spider);
    cast_with_floating(&mut engine, p0, unsummon());
    aim_at(&mut engine, p0, spider);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p1, giant_spider()).is_some());
    assert!(on_battlefield(&engine, p1, their_enchantment()).is_some());
    assert!(in_graveyard(&engine, p1, giant_spider()).is_none());
    assert!(in_graveyard(&engine, p0, destroy_evil()).is_some());
}
