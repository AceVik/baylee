//! `cards/enchantments/mv_3/mana_flare.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mana Flare pays its printed cost and resolves; production is tested below.
#[test]
fn alpha_eval_mana_flare_supported_cast_and_resolution() {
    let p0 = PlayerId::new(0);
    let card = card_index("97159138-c34b-416e-b079-5c952383a243");
    let mut engine = Duel::new(1001, forest())
        .battlefield(0, &[mountain(); 3])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, card);
    assert!(on_stack(&engine, card).is_some(), "the card was cast");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the printed cost was paid"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, card).is_some());
    assert!(in_hand(&engine, p0, card).is_none());
}

#[test]
fn mana_flare_adds_one_for_each_player_and_ignores_nonlands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let flare = card_index("97159138-c34b-416e-b079-5c952383a243");
    let mut engine = Duel::new(1020, forest())
        .battlefield(0, &[flare, forest(), sol_ring(), llanowar_elves()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    activate(&mut engine, p0, sol_ring(), 0);
    activate(&mut engine, p0, llanowar_elves(), 0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    for (seat, card, color) in [
        (p0, forest(), ManaColor::Green),
        (p1, island(), ManaColor::Blue),
    ] {
        pass_until(&mut engine, |e| at_rest(e, seat));
        let before = engine.state().players[usize::from(seat.get())]
            .mana_pool
            .available(color);
        let source = on_battlefield(&engine, seat, card).unwrap();
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
        assert!(stack_is_empty(&engine), "the bonus resolves immediately");
        assert_eq!(
            engine.state().players[usize::from(seat.get())]
                .mana_pool
                .available(color),
            before + 2
        );
    }
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        0
    );
}

#[test]
fn mana_flare_adds_once_per_tap_not_per_mana_and_copies_stack() {
    let p0 = PlayerId::new(0);
    let flare = card_index("97159138-c34b-416e-b079-5c952383a243");
    let tomb = card_index("23467047-6dba-4498-b783-1ebc4f74b8c2");
    let mut engine = Duel::new(1021, forest())
        .battlefield(0, &[flare, flare, tomb])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    activate(&mut engine, p0, tomb, 0);
    assert!(stack_is_empty(&engine));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        4
    );
    assert_eq!(engine.state().players[0].life, 18);
}

#[test]
fn mana_flare_multitype_choices_belong_to_the_lands_player_for_each_copy() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let flare = card_index("97159138-c34b-416e-b079-5c952383a243");
    let turf = card_index("657243dd-e479-4f4b-99d2-09b55d833a35");
    let mut engine = Duel::new(1022, forest())
        .battlefield(0, &[flare, flare])
        .battlefield(1, &[turf])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    activate(&mut engine, p1, turf, 0);
    for color in [ManaColor::Green, ManaColor::Red] {
        let Pending::ChooseColor { player, options } = engine.pending() else {
            panic!(
                "extra mana must offer the produced types: {:?}",
                engine.pending()
            );
        };
        assert_eq!(*player, p1);
        assert_eq!(options, &[ManaColor::Red, ManaColor::Green]);
        assert!(stack_is_empty(&engine));
        assert!(engine.apply(p0, PlayerAction::ChooseColor(color)).is_err());
        assert!(
            engine
                .apply(p1, PlayerAction::ChooseColor(ManaColor::Blue))
                .is_err()
        );
        engine.apply(p1, PlayerAction::ChooseColor(color)).unwrap();
    }
    assert!(at_rest(&engine, p1));
    let pool = &engine.state().players[1].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 2);
    assert_eq!(pool.available(ManaColor::Green), 2);
    assert_eq!(pool.total(), 4);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

#[test]
fn mana_flare_uses_the_chosen_dual_color_not_every_type_the_land_can_make() {
    let p0 = PlayerId::new(0);
    let flare = card_index("97159138-c34b-416e-b079-5c952383a243");
    let taiga = card_index("22e3cf1d-3559-4ce1-954c-8dc815342979");
    let mut engine = Duel::new(1023, forest())
        .battlefield(0, &[flare, taiga])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    activate(&mut engine, p0, taiga, 0);
    assert!(matches!(engine.pending(), Pending::ChooseColor { .. }));
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    assert!(
        at_rest(&engine, p0),
        "there is no second choice for a single produced type"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 2);
    assert_eq!(pool.available(ManaColor::Red), 0);
}

#[test]
fn mana_flare_does_not_copy_spending_restrictions_or_snow_provenance() {
    let p0 = PlayerId::new(0);
    let flare = card_index("97159138-c34b-416e-b079-5c952383a243");
    let ziggurat = card_index("0baabe39-72ae-47bd-a095-cbf7eb8a6361");
    let snow = card_index("5f0d3be8-e63e-4ade-ae58-6b0c14f2ce6d");
    let bolt = card_index("4457ed35-7c10-48c8-9776-456485fdf070");
    let mut engine = Duel::new(1024, forest())
        .battlefield(0, &[flare, ziggurat, snow])
        .hand(0, &[bolt])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    activate(&mut engine, p0, ziggurat, 0);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the bonus is unrestricted"
    );
    assert_eq!(
        pool.restricted().len(),
        1,
        "the land's mana stays restricted"
    );
    cast_with_floating(&mut engine, p0, bolt);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![PlayerId::new(1)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        17,
        "the bonus can pay for a noncreature spell"
    );
    let source = on_battlefield(&engine, p0, snow).unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source })
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 2);
    assert_eq!(pool.snow_available(ManaColor::Green), 1);
}

#[test]
fn mana_flare_waits_for_all_filter_land_choices_before_offering_the_bonus() {
    let p0 = PlayerId::new(0);
    let flare = card_index("97159138-c34b-416e-b079-5c952383a243");
    let gate = card_index("e9f5feb2-2c1a-46ce-885a-4f378d7d10af");
    let mut engine = Duel::new(1025, forest())
        .battlefield(0, &[flare, gate, island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let source = on_battlefield(&engine, p0, island()).unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source })
        .unwrap();
    activate(&mut engine, p0, gate, 1);
    for color in [ManaColor::White, ManaColor::Blue] {
        assert!(matches!(engine.pending(), Pending::ChooseColor { player, .. } if *player == p0));
        engine.apply(p0, PlayerAction::ChooseColor(color)).unwrap();
    }
    let Pending::ChooseColor { options, .. } = engine.pending() else {
        panic!(
            "Flare now offers both produced types: {:?}",
            engine.pending()
        );
    };
    assert_eq!(options, &[ManaColor::White, ManaColor::Blue]);
    assert!(stack_is_empty(&engine));
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    assert!(at_rest(&engine, p0));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 2);
    assert_eq!(
        pool.available(ManaColor::Blue),
        2,
        "one leftover from Island, one from Gate"
    );
}

#[test]
fn mana_flare_stops_after_its_source_is_destroyed() {
    let p0 = PlayerId::new(0);
    let flare = card_index("97159138-c34b-416e-b079-5c952383a243");
    let disenchant = card_index("a7e97fa9-4b72-4548-b854-5be5f18a6f1a");
    let mut engine = Duel::new(1026, forest())
        .battlefield(0, &[flare, plains(), forest()])
        .hand(0, &[disenchant])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let source = on_battlefield(&engine, p0, plains()).unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source })
        .unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    let target = on_battlefield(&engine, p0, flare).unwrap();
    cast_with_floating(&mut engine, p0, disenchant);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, flare).is_none());
    let source = on_battlefield(&engine, p0, forest()).unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source })
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
}
