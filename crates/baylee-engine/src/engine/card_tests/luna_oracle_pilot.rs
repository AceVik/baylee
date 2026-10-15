//! Independent Oracle-text pilot for Destroy Evil and Nature's Claim.
//!
//! Oracle text was read from `data/scryfall-cache/destroy_evil.json` and
//! `data/scryfall-cache/nature_s_claim.json`:
//! - Destroy Evil: "Choose one — Destroy target creature with toughness 4 or
//!   greater. Destroy target enchantment."
//! - Nature's Claim: "Destroy target artifact or enchantment. Its controller
//!   gains 4 life."
//!
//! The cached records have no ruling fields. These tests exercise the cast,
//! target menu, resolution, and the outcomes the Oracle text names.

#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary and harness.
use super::*;

fn destroy_evil() -> CardIndex {
    card_index("b75e667a-1ceb-445a-acf4-ae1fd2dae9e7")
}

fn natures_claim() -> CardIndex {
    card_index("6d4e558e-9109-4918-a082-fdcbaffd516b")
}

fn choose_de_mode(engine: &mut Engine<RegistryLookup>, seat: PlayerId, mode: usize) {
    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected Destroy Evil's mode choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, seat);
    let index = options
        .iter()
        .position(|option| option.kind == CastModeKind::Mode(mode))
        .expect("the selected Oracle mode is offered");
    engine
        .apply(seat, PlayerAction::ChooseMode(index))
        .expect("the offered mode is legal");
}

fn choose_one_target(engine: &mut Engine<RegistryLookup>, seat: PlayerId, target: ObjectId) {
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected one target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, seat);
    assert_eq!((min, max), (1, 1));
    assert!(
        options.contains(&target),
        "the target must be offered: {options:?}"
    );
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("the chosen target was offered");
}

fn put_target_in_graveyard(engine: &mut Engine<RegistryLookup>, owner: PlayerId, target: ObjectId) {
    engine
        .dev_state_mut(owner)
        .expect("the test harness may set up a response state")
        .move_object(
            target,
            ZoneLocation::Graveyard(owner),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("the target moves to its owner's graveyard");
    engine.refresh_offer();
}

#[test]
fn destroy_evil_creature_mode_accepts_toughness_four_and_rejects_three() {
    let caster = PlayerId::new(0);
    let opponent = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[destroy_evil()])
        .battlefield(1, &[fangren_hunter(), wild_elephant(), their_enchantment()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, caster));

    let four_toughness = on_battlefield(&game, opponent, fangren_hunter()).unwrap();
    let three_toughness = on_battlefield(&game, opponent, wild_elephant()).unwrap();
    let enchantment = on_battlefield(&game, opponent, their_enchantment()).unwrap();
    cast_from_hand(&mut game, caster, destroy_evil());
    choose_de_mode(&mut game, caster, 0);

    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!(
            "creature mode must ask for a target, got {:?}",
            game.pending()
        )
    };
    assert!(options.contains(&four_toughness));
    assert!(!options.contains(&three_toughness));
    assert!(!options.contains(&enchantment));
    choose_one_target(&mut game, caster, four_toughness);
    pass_until(&mut game, stack_is_empty);

    assert!(on_battlefield(&game, opponent, fangren_hunter()).is_none());
    assert!(in_graveyard(&game, opponent, fangren_hunter()).is_some());
    assert!(on_battlefield(&game, opponent, wild_elephant()).is_some());
    assert!(on_battlefield(&game, opponent, their_enchantment()).is_some());
    assert!(in_graveyard(&game, caster, destroy_evil()).is_some());
}

#[test]
fn destroy_evil_enchantment_mode_destroys_only_the_named_enchantment() {
    let caster = PlayerId::new(0);
    let opponent = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[destroy_evil()])
        .battlefield(
            1,
            &[
                quiet_artifact(),
                their_enchantment(),
                llanowar_elves(),
                fangren_hunter(),
            ],
        )
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, caster));

    let artifact = on_battlefield(&game, opponent, quiet_artifact()).unwrap();
    let enchantment = on_battlefield(&game, opponent, their_enchantment()).unwrap();
    let creature = on_battlefield(&game, opponent, llanowar_elves()).unwrap();
    cast_from_hand(&mut game, caster, destroy_evil());
    choose_de_mode(&mut game, caster, 1);

    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!(
            "enchantment mode must ask for a target, got {:?}",
            game.pending()
        )
    };
    assert_eq!(options, vec![enchantment]);
    assert!(!options.contains(&artifact));
    assert!(!options.contains(&creature));
    choose_one_target(&mut game, caster, enchantment);
    pass_until(&mut game, stack_is_empty);

    assert!(on_battlefield(&game, opponent, their_enchantment()).is_none());
    assert!(in_graveyard(&game, opponent, their_enchantment()).is_some());
    assert!(on_battlefield(&game, opponent, quiet_artifact()).is_some());
    assert!(on_battlefield(&game, opponent, llanowar_elves()).is_some());
    assert!(on_battlefield(&game, opponent, fangren_hunter()).is_some());
}

#[test]
fn destroy_evil_does_nothing_when_its_only_target_leaves_before_resolution() {
    let caster = PlayerId::new(0);
    let opponent = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[destroy_evil()])
        .hand(1, &[unsummon()])
        .battlefield(
            1,
            &[
                island(),
                fangren_hunter(),
                fangren_hunter(),
                their_enchantment(),
            ],
        )
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, caster));

    let mut large_creatures: Vec<_> = game
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            game.state().object(*id).is_some_and(|object| {
                object.controller == opponent
                    && object
                        .card
                        .is_some_and(|card| card.index == fangren_hunter())
            })
        })
        .collect();
    assert_eq!(large_creatures.len(), 2);
    let target = large_creatures.pop().unwrap();
    let bystander = large_creatures.pop().unwrap();
    cast_from_hand(&mut game, caster, destroy_evil());
    choose_de_mode(&mut game, caster, 0);
    choose_one_target(&mut game, caster, target);
    game.apply(caster, PlayerAction::PassPriority)
        .expect("the caster passes priority to the opponent");
    cast_from_hand(&mut game, opponent, unsummon());
    choose_one_target(&mut game, opponent, target);
    pass_until(&mut game, |e| {
        in_hand(e, opponent, fangren_hunter()).is_some()
    });
    pass_until(&mut game, stack_is_empty);

    assert!(in_hand(&game, opponent, fangren_hunter()).is_some());
    assert_eq!(
        on_battlefield(&game, opponent, fangren_hunter()),
        Some(bystander)
    );
    assert!(in_graveyard(&game, caster, destroy_evil()).is_some());
}

#[test]
fn destroy_evil_rechecks_the_toughness_threshold_when_it_resolves() {
    let caster = PlayerId::new(0);
    let opponent = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[destroy_evil()])
        .hand(1, &[afflict()])
        .battlefield(
            1,
            &[
                swamp(),
                swamp(),
                swamp(),
                fangren_hunter(),
                wild_elephant(),
                their_enchantment(),
            ],
        )
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, caster));

    let target = on_battlefield(&game, opponent, fangren_hunter()).unwrap();
    assert_eq!(pt(&game, target).1, 4);
    cast_from_hand(&mut game, caster, destroy_evil());
    choose_de_mode(&mut game, caster, 0);
    choose_one_target(&mut game, caster, target);
    game.apply(caster, PlayerAction::PassPriority)
        .expect("the caster passes priority to the opponent");
    cast_from_hand(&mut game, opponent, afflict());
    choose_one_target(&mut game, opponent, target);
    pass_until(&mut game, |e| pt(e, target).1 == 3);
    assert_eq!(pt(&game, target).1, 3);
    pass_until(&mut game, stack_is_empty);

    assert!(on_battlefield(&game, opponent, fangren_hunter()).is_some());
    assert!(in_graveyard(&game, opponent, fangren_hunter()).is_none());
}

#[test]
fn natures_claim_targets_artifact_or_enchantment_and_gives_four_life_to_its_controller() {
    let caster = PlayerId::new(0);
    let opponent = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .life(0, 17)
        .life(1, 21)
        .battlefield(0, &[forest(), quiet_artifact()])
        .hand(0, &[natures_claim()])
        .battlefield(
            1,
            &[quiet_artifact(), their_enchantment(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, caster));

    let own_artifact = on_battlefield(&game, caster, quiet_artifact()).unwrap();
    let other_artifact = on_battlefield(&game, opponent, quiet_artifact()).unwrap();
    let enchantment = on_battlefield(&game, opponent, their_enchantment()).unwrap();
    let creature = on_battlefield(&game, opponent, llanowar_elves()).unwrap();
    let land = on_battlefield(&game, caster, forest()).unwrap();
    cast_from_hand(&mut game, caster, natures_claim());

    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!(
            "Nature's Claim must ask for a target, got {:?}",
            game.pending()
        )
    };
    assert!(options.contains(&own_artifact));
    assert!(options.contains(&other_artifact));
    assert!(options.contains(&enchantment));
    assert!(!options.contains(&creature));
    assert!(!options.contains(&land));
    assert_eq!(options.len(), 3);
    choose_one_target(&mut game, caster, own_artifact);
    pass_until(&mut game, stack_is_empty);

    assert_eq!(game.state().players[0].life, 21);
    assert_eq!(game.state().players[1].life, 21);
    assert!(on_battlefield(&game, caster, quiet_artifact()).is_none());
    assert!(in_graveyard(&game, caster, quiet_artifact()).is_some());
    assert!(on_battlefield(&game, opponent, their_enchantment()).is_some());
    assert!(on_battlefield(&game, opponent, quiet_artifact()).is_some());
}

#[test]
fn natures_claim_destroys_an_artifact_and_rewards_its_controller() {
    let caster = PlayerId::new(0);
    let opponent = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .life(0, 20)
        .life(1, 13)
        .battlefield(0, &[forest()])
        .hand(0, &[natures_claim()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, caster));

    let artifact = on_battlefield(&game, opponent, quiet_artifact()).unwrap();
    game.dev_state_mut(opponent)
        .expect("the test harness may establish control by another player")
        .object_mut(artifact)
        .expect("the artifact remains on the battlefield")
        .set_controller(caster);
    game.refresh_offer();
    assert_eq!(game.state().object(artifact).unwrap().controller, caster);
    assert_eq!(game.state().object(artifact).unwrap().owner, opponent);
    cast_from_hand(&mut game, caster, natures_claim());
    choose_one_target(&mut game, caster, artifact);
    pass_until(&mut game, stack_is_empty);

    assert_eq!(game.state().players[0].life, 24);
    assert_eq!(game.state().players[1].life, 13);
    assert!(on_battlefield(&game, opponent, quiet_artifact()).is_none());
    assert!(in_graveyard(&game, opponent, quiet_artifact()).is_some());
}

#[test]
fn natures_claim_destroys_an_enchantment_and_gives_four_life_to_its_controller() {
    let caster = PlayerId::new(0);
    let opponent = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .life(0, 18)
        .life(1, 12)
        .battlefield(0, &[forest()])
        .hand(0, &[natures_claim()])
        .battlefield(
            1,
            &[their_enchantment(), quiet_artifact(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, caster));

    let enchantment = on_battlefield(&game, opponent, their_enchantment()).unwrap();
    let artifact = on_battlefield(&game, opponent, quiet_artifact()).unwrap();
    let creature = on_battlefield(&game, opponent, llanowar_elves()).unwrap();
    cast_from_hand(&mut game, caster, natures_claim());
    choose_one_target(&mut game, caster, enchantment);
    pass_until(&mut game, stack_is_empty);

    assert_eq!(game.state().players[0].life, 18);
    assert_eq!(game.state().players[1].life, 16);
    assert!(on_battlefield(&game, opponent, their_enchantment()).is_none());
    assert!(in_graveyard(&game, opponent, their_enchantment()).is_some());
    assert!(on_battlefield(&game, opponent, quiet_artifact()).is_some());
    assert!(on_battlefield(&game, opponent, llanowar_elves()).is_some());
    assert!(game.state().object(artifact).is_some());
    assert!(game.state().object(creature).is_some());
}

#[test]
fn natures_claim_does_not_offer_an_enchantment_after_it_leaves_the_battlefield() {
    let caster = PlayerId::new(0);
    let opponent = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[natures_claim()])
        .battlefield(1, &[quiet_artifact(), their_enchantment()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, caster));

    let artifact = on_battlefield(&game, opponent, quiet_artifact()).unwrap();
    let enchantment = on_battlefield(&game, opponent, their_enchantment()).unwrap();
    put_target_in_graveyard(&mut game, opponent, enchantment);
    cast_from_hand(&mut game, caster, natures_claim());

    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!(
            "Nature's Claim must ask for a target, got {:?}",
            game.pending()
        )
    };
    assert!(options.contains(&artifact));
    assert!(!options.contains(&enchantment));
    assert_eq!(options, vec![artifact]);
    choose_one_target(&mut game, caster, artifact);
    pass_until(&mut game, stack_is_empty);
    assert!(on_battlefield(&game, opponent, quiet_artifact()).is_none());
    assert!(in_graveyard(&game, opponent, quiet_artifact()).is_some());
}

#[test]
fn natures_claim_gives_life_even_when_an_indestructible_artifact_survives() {
    let caster = PlayerId::new(0);
    let opponent = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .life(0, 16)
        .life(1, 19)
        .battlefield(0, &[forest()])
        .hand(0, &[natures_claim()])
        .battlefield(1, &[darksteel_pendant()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, caster));

    let pendant = on_battlefield(&game, opponent, darksteel_pendant()).unwrap();
    cast_from_hand(&mut game, caster, natures_claim());
    choose_one_target(&mut game, caster, pendant);
    pass_until(&mut game, stack_is_empty);

    assert_eq!(game.state().players[0].life, 16);
    assert_eq!(game.state().players[1].life, 23);
    assert!(on_battlefield(&game, opponent, darksteel_pendant()).is_some());
    assert!(in_graveyard(&game, opponent, darksteel_pendant()).is_none());
}

#[test]
fn natures_claim_gives_no_life_when_its_only_target_is_illegal_on_resolution() {
    let caster = PlayerId::new(0);
    let opponent = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .life(0, 17)
        .life(1, 21)
        .battlefield(0, &[forest()])
        .hand(0, &[natures_claim()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, caster));

    let artifact = on_battlefield(&game, opponent, quiet_artifact()).unwrap();
    cast_from_hand(&mut game, caster, natures_claim());
    choose_one_target(&mut game, caster, artifact);
    put_target_in_graveyard(&mut game, opponent, artifact);
    pass_until(&mut game, stack_is_empty);

    assert_eq!(game.state().players[0].life, 17);
    assert_eq!(game.state().players[1].life, 21);
    assert!(on_battlefield(&game, opponent, quiet_artifact()).is_none());
    assert!(in_graveyard(&game, opponent, quiet_artifact()).is_some());
}
