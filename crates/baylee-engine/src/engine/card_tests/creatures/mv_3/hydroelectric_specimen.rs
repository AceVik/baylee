//! `cards/creatures/mv_3/hydroelectric_specimen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hydroelectric Specimen ({2}{U}, 1/4 Weird): "Flash. When this creature
/// enters, you may change the target of target instant or sorcery spell with
/// a single target to this creature."
///
/// The whole card is one play, so it is played as one: the removal is already
/// on the stack pointing at something else, which is the only board on which
/// the trigger has a legal target at all — and the only board a creature
/// without flash could never be cast onto.
///
/// Swords to Plowshares is the sharpest reading available, because its two
/// clauses land on two different players once the target moves. The Elves it
/// was aimed at are untouched, the Weird that stole the aim is **exiled**
/// rather than destroyed, and the life goes to the Weird's controller — the
/// player who redirected it — for the Weird's own power.
///
/// This is also the working half of `Coverage::Partial`: one target is
/// exactly what the printed line asks for, so nothing here is an
/// approximation.
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

/// "To this creature": the Weird is the only new target, although another
/// creature is legal for the Swords (#247). The redirect is offered what
/// the Swords may target **and** what "this creature" names.
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

/// "Change the target … to this creature" when this creature is no legal
/// target for the spell (#247): the target stays (CR 115.7a), and nothing is
/// asked. Ancestral Recall names a player, and a Weird is not one. The
/// redirect used to offer the Weird anyway, which pointed Ancestral Recall at
/// a creature and fizzled it.
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

/// The `Coverage::Partial` half: "with a single target" is a clause no
/// `Filter` can ask about (CR 115.9a, #249).
///
/// Curse of the Swine is cast for X = 2 and stands on the stack naming two
/// creatures, which the printed line excludes from the trigger outright — and
/// the engine offers it anyway, because `TargetSpec::Spell` can only narrow a
/// spell by its *printed* characteristics and no `Filter` counts what an
/// object points at.
///
/// Taking it changes nothing, and asks nothing. The redirect used to write one
/// target where two stood, so the Curse exiled the Weird alone and made one
/// Boar (#247). "Change the target" is all of them or none (CR 115.7a), and a
/// spell the card should never have been offered is left alone: both Elves
/// are exiled, two Boars are made, and the Weird stays.
///
/// This is the test that turns red when the gap closes. A trigger with no
/// legal target is removed from the stack (CR 603.3d), so a `TargetSpec` that
/// could count targets would leave nothing to choose and
/// `flash_the_specimen_in` would never see the choice at all.
///
/// A **sorcery** is deliberate rather than convenient. The printed line reads
/// "instant or sorcery", and a sorcery only ever stands on the stack during
/// its own controller's turn — which is the other thing flash buys and the
/// board above cannot show, since it never leaves p0's own main phase.
#[test]
fn the_weirds_trigger_offers_a_spell_holding_two_targets_and_moves_neither() {
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

    let offered = flash_the_specimen_in(&mut engine, p0, curse);
    assert_eq!(
        offered,
        vec![curse],
        "NOT SUPPORTED, and this is its exact shape: the printed line reads \
         \"with a single target\", and a two-target spell is offered anyway"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![curse],
            },
        )
        .unwrap();
    let weird = on_battlefield(&engine, p0, hydroelectric_specimen()).expect("the Weird landed");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert!(
        !matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "no new target is asked for: the Curse holds two, and \"change the \
         target\" moves all or none (CR 115.7a), got {:?}",
        engine.pending()
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
