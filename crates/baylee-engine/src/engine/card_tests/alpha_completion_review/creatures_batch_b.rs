//! Independent Demonic Hordes upkeep scenarios; not registered yet.

#[allow(clippy::wildcard_imports)] // Shared behavioral card-test vocabulary.
use super::*;

fn hordes() -> CardIndex {
    card_index("2847c8a0-f6aa-4e4a-a7b8-fc116436a264")
}

fn hordes_payment(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
}

#[test]
fn demonic_hordes_independent_paying_three_black_preserves_hordes_and_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[hordes(), swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    let demon = on_battlefield(&engine, p0, hordes()).unwrap();
    hordes_payment(&mut engine);
    assert!(
        matches!(engine.pending(), Pending::YesNo { player, prompt: YesNoPrompt::PayMana { cost }, .. }
        if *player == p0 && *cost == baylee_core::mana!("{B}{B}{B}"))
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(tap_all_mana(&mut engine, p0), 4);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(!is_tapped(&engine, demon));
    assert_eq!(all_on_battlefield(&engine, p0, swamp()).len(), 4);
}

#[test]
fn demonic_hordes_independent_opponent_selects_only_the_unpaying_players_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[hordes(), swamp(), forest()])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    let demon = on_battlefield(&engine, p0, hordes()).unwrap();
    let victim = on_battlefield(&engine, p0, forest()).unwrap();
    let swamp_id = on_battlefield(&engine, p0, swamp()).unwrap();
    let their_land = on_battlefield(&engine, p1, mountain()).unwrap();
    hordes_payment(&mut engine);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "opponent must choose the sacrificed land: {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p1);
    assert_eq!((min, max), (1, 1));
    assert!(options.contains(&victim) && options.contains(&swamp_id));
    assert!(!options.contains(&demon) && !options.contains(&their_land));
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, demon));
    assert!(in_graveyard(&engine, p0, forest()).is_some());
    assert!(on_battlefield(&engine, p0, swamp()).is_some());
    assert!(on_battlefield(&engine, p1, mountain()).is_some());
}

#[test]
fn demonic_hordes_independent_no_land_still_taps_without_forcing_a_choice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp()).battlefield(0, &[hordes()]).start();
    keep_mulligans(&mut engine);
    let demon = on_battlefield(&engine, p0, hordes()).unwrap();
    hordes_payment(&mut engine);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, demon));
    assert!(on_battlefield(&engine, p0, hordes()).is_some());
}

#[test]
fn demonic_hordes_independent_controller_chooses_opponent_at_multiplayer_table() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, swamp(), 3)
        .battlefield(0, &[hordes(), swamp(), forest()])
        .start();
    keep_mulligans(&mut engine);
    let victim = on_battlefield(&engine, p0, forest()).unwrap();
    hordes_payment(&mut engine);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!("controller chooses an opponent: {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert!(options.contains(&p1) && options.contains(&p2));
    assert!(!options.contains(&p0));
    engine.apply(p0, PlayerAction::ChoosePlayer(p2)).unwrap();
    assert!(matches!(engine.pending(), Pending::ChooseCards { player, .. } if *player == p2));
    engine
        .apply(
            p2,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, forest()).is_some());
}

fn hordes_trigger_on_stack(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| !e.state().zones.stack_is_empty());
}

#[test]
fn demonic_hordes_independent_already_tapped_still_sacrifices_a_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[hordes(), swamp()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    let demon = on_battlefield(&engine, p0, hordes()).unwrap();
    let target = on_battlefield(&engine, p1, forest()).unwrap();
    let sacrifice = on_battlefield(&engine, p0, swamp()).unwrap();
    hordes_trigger_on_stack(&mut engine);
    activate(&mut engine, p0, hordes(), 0);
    aim(&mut engine, vec![target], vec![]);
    hordes_payment(&mut engine);
    assert!(is_tapped(&engine, demon));
    assert!(in_graveyard(&engine, p1, forest()).is_some());
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![sacrifice],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, swamp()).is_some(),
        "an impossible second tap does not stop sacrifice"
    );
    assert!(is_tapped(&engine, demon));
}

#[test]
fn demonic_hordes_independent_source_returned_to_hand_still_demands_its_controllers_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[hordes(), swamp()])
        .battlefield(1, &[island()])
        .hand(1, &[unsummon()])
        .start();
    keep_mulligans(&mut engine);
    let demon = on_battlefield(&engine, p0, hordes()).unwrap();
    let sacrifice = on_battlefield(&engine, p0, swamp()).unwrap();
    hordes_trigger_on_stack(&mut engine);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, unsummon());
    aim(&mut engine, vec![demon], vec![]);
    hordes_payment(&mut engine);
    assert!(in_hand(&engine, p0, hordes()).is_some());
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    assert!(matches!(engine.pending(), Pending::ChooseCards { player, .. } if *player == p1));
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![sacrifice],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, swamp()).is_some());
    assert!(in_hand(&engine, p0, hordes()).is_some());
}

#[test]
fn demonic_hordes_independent_control_magic_moves_upkeep_obligation_to_new_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let control = card_index("cd0d7141-46d2-4aa3-bc77-6b3b4513803e");
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[hordes(), swamp()])
        .battlefield(1, &[island(); 4])
        .hand(1, &[control])
        .start();
    keep_mulligans(&mut engine);
    let demon = on_battlefield(&engine, p0, hordes()).unwrap();
    let first_land = on_battlefield(&engine, p0, swamp()).unwrap();
    hordes_payment(&mut engine);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![first_land],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    cast_from_hand(&mut engine, p1, control);
    aim(&mut engine, vec![demon], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(demon).unwrap().controller, p1);
    hordes_payment(&mut engine);
    assert_eq!(
        engine.state().turn.active,
        p1,
        "p0's intervening upkeep must not trigger Hordes"
    );
    assert!(matches!(engine.pending(), Pending::YesNo { player, .. } if *player == p1));
    let victim = on_battlefield(&engine, p1, island()).unwrap();
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    assert!(
        matches!(engine.pending(), Pending::ChooseCards { player, options, .. }
        if *player == p0 && options.contains(&victim))
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, island()).is_some());
    assert!(is_tapped(&engine, demon));
}

#[test]
fn demonic_hordes_independent_control_change_in_response_keeps_original_payment_and_land_owner() {
    use super::super::super::synthetic::{self, SyntheticLookup};
    use baylee_cards_dsl::{
        AbilityDef, Cost, Duration, Effect, Filter, Modifier, TargetSpec, activated,
    };
    const THIEF: u32 = 4_000_032;
    static ABILITIES: &[AbilityDef] = &[activated!(
        Cost::FREE,
        &[Effect::continuous(
            &Filter::This,
            Modifier::GainControl,
            Duration::Indefinitely
        )],
        target = Some(TargetSpec::Object(&Filter::CREATURE))
    )];
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let preset = synthetic::preset_both(SEED, &[hordes().get(), swamp().get()], &[THIEF]);
    let mut engine = Engine::new(
        &preset,
        SyntheticLookup::new(vec![synthetic::land(
            THIEF,
            "Control transfer fixture",
            ABILITIES,
        )]),
    )
    .unwrap();
    synthetic::keep_mulligans(&mut engine);
    let demon = synthetic::permanents(&engine, hordes().get())[0];
    let swamp_id = synthetic::permanents(&engine, swamp().get())[0];
    for _ in 0..20 {
        if !engine.state().zones.stack_is_empty() {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let thief = synthetic::permanents(&engine, THIEF)[0];
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: thief,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![demon],
                players: vec![],
            },
        )
        .unwrap();
    for _ in 0..20 {
        if matches!(engine.pending(), Pending::YesNo { .. }) {
            break;
        }
        let pending = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &pending));
    }
    assert_eq!(engine.state().object(demon).unwrap().controller, p1);
    assert!(matches!(engine.pending(), Pending::YesNo { player, .. } if *player == p0));
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    assert!(
        matches!(engine.pending(), Pending::ChooseCards { player, options, .. }
        if *player == p1 && options.contains(&swamp_id))
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![swamp_id],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(swamp_id).unwrap().zone,
        Zone::Graveyard
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&swamp_id)
    );
    assert!(
        synthetic::tapped(&engine, demon),
        "the source is tapped even under its new controller"
    );
}
