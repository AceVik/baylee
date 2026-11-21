//! Innkeeper's Talent, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn innkeeper_s_talent_wards_permanents_with_counters_and_then_doubles_counters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let talent = card_index("915b75d1-b5e3-4aca-a54c-beaccef0a2e4");
    let disenchant = card_index("a7e97fa9-4b72-4548-b854-5be5f18a6f1a");
    let mut engine = Duel::new(901, forest())
        .battlefield(
            0,
            &[
                talent,
                grizzly_bears(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), plains(), plains()])
        .hand(1, &[swords_to_plowshares(), disenchant])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let class = on_battlefield(&engine, p0, talent).expect("the Class is in play");
    let ours = on_battlefield(&engine, p0, grizzly_bears()).expect("our Bear is in play");
    let theirs = on_battlefield(&engine, p1, grizzly_bears()).expect("their Bear is in play");
    let level = |e: &Engine<RegistryLookup>| e.state().object(class).unwrap().class_level();
    assert_eq!(level(&engine), 1);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, talent, 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(level(&engine), 2);

    let play_combat_trigger = |engine: &mut Engine<RegistryLookup>| {
        pass_until(engine, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        let Pending::ChooseTargets {
            player, options, ..
        } = engine.pending()
        else {
            unreachable!()
        };
        assert_eq!(*player, p0);
        assert!(options.contains(&ours));
        assert!(!options.contains(&theirs));
        choose_target(engine, p0, ours);
        pass_until(engine, stack_is_empty);
    };
    play_combat_trigger(&mut engine);
    assert_eq!(counters_on(&engine, ours, CounterKind::P1P1), 1);

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    choose_target(&mut engine, p1, ours);
    pass_until(&mut engine, |e| {
        stack_is_empty(e) || matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::PayTax { mana: 1 },
            ..
        } if *player == p1
    ));
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());

    reach_their_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, talent, 3);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(level(&engine), 3);
    assert_eq!(counters_on(&engine, class, CounterKind::Level), 0);
    play_combat_trigger(&mut engine);
    assert_eq!(counters_on(&engine, ours, CounterKind::P1P1), 3);

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, disenchant);
    choose_target(&mut engine, p1, class);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, talent).is_none(),
        "the Class itself has no counters and does not gain ward"
    );
}

fn choose_target(engine: &mut Engine<RegistryLookup>, player: PlayerId, target: ObjectId) {
    engine
        .apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("the object is a legal target");
}
