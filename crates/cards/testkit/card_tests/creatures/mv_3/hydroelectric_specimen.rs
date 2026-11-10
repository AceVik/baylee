//! `cards/creatures/mv_3/hydroelectric_specimen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn a_flashed_in_weird_takes_the_swords_that_was_aimed_at_the_elves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), llanowar_elves()])
        .hand(0, &[hydroelectric_specimen()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    let my_elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the elves are out");
    let life_before = engine.state().players[0].life;

    // p0's main phase, and p0 hands priority straight over: the opponent's
    // removal is cast at p0's only creature.
    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the swords' target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![my_elves], "the only creature on the table");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![my_elves],
            },
        )
        .unwrap();
    let swords_spell = engine.state().zones.list(ZoneLocation::Stack)[0];

    let offered = flash_the_specimen_in(&mut engine, p0, swords_spell);
    assert_eq!(
        offered,
        vec![swords_spell],
        "\"target instant or sorcery spell\": the one instant still on the \
         stack is the only thing the trigger may point at"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![swords_spell],
            },
        )
        .unwrap();
    let weird = on_battlefield(&engine, p0, hydroelectric_specimen()).expect("the Weird landed");
    the_weird_takes_the_aim(&mut engine, p0, weird);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature the removal named is still standing: the aim moved"
    );
    assert_eq!(
        engine.state().object(weird).map(|o| o.zone),
        Some(Zone::Exile),
        "the Weird took the Swords, which exiles rather than destroys"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "\"its controller gains life equal to its power\", and after the \
         redirect that controller is the Weird's own, for its own 1 power"
    );
    assert!(
        in_graveyard(&engine, p1, swords_to_plowshares()).is_some(),
        "one spell was cast and it resolved"
    );
}

#[test]
fn the_weird_is_the_only_creature_the_aim_moves_to() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), llanowar_elves()])
        .hand(0, &[hydroelectric_specimen()])
        .battlefield(1, &[plains(), llanowar_elves()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    let swords = engine.state().zones.list(ZoneLocation::Stack)[0];
    flash_the_specimen_in(&mut engine, p0, swords);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![swords],
            },
        )
        .unwrap();
    let weird = on_battlefield(&engine, p0, hydroelectric_specimen()).expect("the Weird landed");
    // Asserts the Weird is the whole of the offer, with p1's Elves legal
    // for the Swords beside it.
    the_weird_takes_the_aim(&mut engine, p0, weird);
}

#[test]
fn the_weird_leaves_the_aim_of_a_spell_it_is_no_target_for() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[hydroelectric_specimen()])
        .battlefield(1, &[island()])
        .hand(1, &[the_specimens_player_spell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, the_specimens_player_spell());
    engine
        .apply(p1, PlayerAction::ChoosePlayer(p1))
        .expect("\"target player\": its own caster");
    let recall = engine.state().zones.list(ZoneLocation::Stack)[0];
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    let offered = flash_the_specimen_in(&mut engine, p0, recall);
    assert_eq!(offered, vec![recall], "an instant is on the stack");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![recall],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert!(
        !matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "the Weird is no legal target for \"target player\", so there is \
         nothing to change the target to: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().object(recall).map(|o| o.chosen_player),
        Some(Some(p1)),
        "Ancestral Recall still names its caster"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand_before + 3,
        "and its caster draws three"
    );
    assert!(
        on_battlefield(&engine, p0, hydroelectric_specimen()).is_some(),
        "the Weird stands"
    );
}

#[test]
fn the_weirds_trigger_excludes_a_spell_holding_two_targets() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let elf = llanowar_elves();
    let mine = [island(), island(), island(), elf, elf];
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &mine)
        .hand(0, &[hydroelectric_specimen()])
        .battlefield(1, &[island(), island(), island(), island()])
        .hand(1, &[the_specimens_two_target_spell()])
        .start();
    keep_mulligans(&mut engine);
    let my_elves = all_on_battlefield(&engine, p0, elf);
    assert_eq!(my_elves.len(), 2, "two creatures for the curse to name");

    // p1's own main phase, because a sorcery has no other window (CR 307.1).
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, the_specimens_two_target_spell());
    let Pending::ChooseNumber { .. } = engine.pending().clone() else {
        panic!("expected the X choice, got {:?}", engine.pending())
    };
    engine.apply(p1, PlayerAction::ChooseNumber(2)).unwrap();
    let Pending::ChooseTargets { min, max, .. } = engine.pending().clone() else {
        panic!("expected the curse's aim, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (2, 2), "\"Exile X target creatures\", X = 2");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: my_elves.clone(),
            },
        )
        .expect("two creatures is what X = 2 asks for");
    let curse = engine.state().zones.list(ZoneLocation::Stack)[0];
    let aimed: Vec<ObjectId> = engine
        .state()
        .object(curse)
        .expect("the curse is on the stack")
        .targets
        .to_vec();
    assert_eq!(aimed.len(), 2, "the premise: two targets on the stack");
    assert!(
        my_elves.iter().all(|named| aimed.contains(named)),
        "and they are the two creatures that were named: {aimed:?}"
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    cast_from_hand(&mut engine, p0, hydroelectric_specimen());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, hydroelectric_specimen()).is_some()
    });
    let weird = on_battlefield(&engine, p0, hydroelectric_specimen()).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "no trigger target is offered"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).as_slice(),
        &[curse]
    );
    let kept: Vec<ObjectId> = engine
        .state()
        .object(curse)
        .expect("the curse is still on the stack")
        .targets
        .to_vec();
    assert_eq!(
        kept, aimed,
        "the Curse still names the two creatures it was cast at"
    );

    // And what that is worth on the board.
    pass_until(&mut engine, stack_is_empty);
    assert!(
        all_on_battlefield(&engine, p0, elf).is_empty(),
        "\"Exile X target creatures\": both Elves it named are exiled"
    );
    assert_eq!(
        engine.state().object(weird).map(|o| o.zone),
        Some(Zone::Battlefield),
        "and the Weird, which never became a target, stays"
    );
    assert_eq!(
        tokens_of(&engine, p0).len(),
        2,
        "\"for each creature exiled this way, its controller creates a 2/2 \
         green Boar\": two exiles, two Boars"
    );
}
