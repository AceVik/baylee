//! `cards/lands/utility/stardew_valley.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Stardew Valley` prints `{{T}}: Add {{C}}.`, `{{2}}, {{T}}, Tap an untapped creature you control: Create a Food token.`, and `{{3}}, {{T}}: Choose target permanent you control. Draw a card, then another player of your choice may gain control of that permanent. Activate only as a sorcery.`
///
/// Ability 1 requires floating `{{2}}`, tapping `Stardew Valley`, and tapping an untapped creature you control via `ChoicePrompt::CostTap`, which rejects opponent-controlled creatures.
/// Resolving ability 1 creates one Food token and leaves both `Stardew Valley` and the chosen creature tapped.
#[test]
fn stardew_valley_taps_controlled_creature_to_create_food_token() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[stardew_valley(), forest(), forest(), young_wolf()])
        .battlefield(1, &[young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let valley = on_battlefield(&engine, p0, stardew_valley()).expect("valley on battlefield");
    let my_wolf = on_battlefield(&engine, p0, young_wolf()).expect("my wolf on battlefield");
    let their_wolf = on_battlefield(&engine, p1, young_wolf()).expect("their wolf on battlefield");

    assert!(!is_tapped(&engine, my_wolf));

    // Float {{2}} from Forests while keeping Stardew Valley untapped.
    tap_mana_except(&mut engine, p0, valley);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(!is_tapped(&engine, valley));

    activate(&mut engine, p0, stardew_valley(), 1);

    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected CostTap prompt, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ChoicePrompt::CostTap);
    assert_eq!((min, max), (1, 1));
    assert!(
        options.contains(&my_wolf),
        "controlled creature is offered as cost"
    );
    assert!(
        !options.contains(&their_wolf),
        "opponent creature cannot be tapped for your cost"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_wolf],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, valley));
    assert!(is_tapped(&engine, my_wolf));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "created one Food token");
    let food = engine.state().object(tokens[0]).unwrap();
    assert!(food.characteristics().types.contains(TypeSet::ARTIFACT));
    assert!(
        food.characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::artifact::FOOD)
    );
}

/// Drawing happens before the other player's offer; that player alone accepts
/// or refuses the permanent, including when the permanent is the Valley itself.
#[test]
fn stardew_valley_draws_then_other_player_decides_whether_to_take_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for accepts in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[stardew_valley()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let valley = on_battlefield(&e, p0, stardew_valley()).unwrap();
        let hand_before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, 3);
        e.refresh_offer();
        activate(&mut e, p0, stardew_valley(), 2);
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![valley],
                players: vec![],
            },
        )
        .unwrap();
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        assert_eq!(e.pending().asked(), Some(p1));
        assert_eq!(
            e.state().zones.list(ZoneLocation::Hand(p0)).len(),
            hand_before + 1
        );
        let before = e.snapshot_hash();
        assert!(e.apply(p0, PlayerAction::YesNo(accepts)).is_err());
        assert_eq!(e.snapshot_hash(), before);
        e.apply(p1, PlayerAction::YesNo(accepts)).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(valley).unwrap().controller,
            if accepts { p1 } else { p0 }
        );
        assert!(is_tapped(&e, valley));
    }
}

/// A target that left prevents the entire ability from resolving, including
/// the draw that otherwise precedes the offer to another player.
#[test]
fn stardew_valley_does_not_draw_when_its_target_left() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[stardew_valley(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let target = on_battlefield(&e, p0, forest()).unwrap();
    let hand_before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();
    activate(&mut e, p0, stardew_valley(), 2);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![target],
            players: vec![],
        },
    )
    .unwrap();
    crate::sba::destroy(e.dev_state_mut(p0).unwrap(), target);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}

#[test]
fn stardew_valley_mana_is_immediate_and_taps_the_land() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[stardew_valley()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let valley = on_battlefield(&e, p0, stardew_valley()).unwrap();
    activate(&mut e, p0, stardew_valley(), 0);
    assert!(stack_is_empty(&e));
    assert_eq!(
        e.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&e, valley));
}

#[test]
fn stardew_valley_draw_target_is_controlled_and_not_restricted_to_lands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[stardew_valley(), young_wolf(), sol_ring()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let wolf = on_battlefield(&e, p0, young_wolf()).unwrap();
    let ring = on_battlefield(&e, p0, sol_ring()).unwrap();
    let enemy = on_battlefield(&e, p1, gray_ogre()).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();
    activate(&mut e, p0, stardew_valley(), 2);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("controlled permanent target")
    };
    assert!(options.contains(&wolf));
    assert!(options.contains(&ring));
    assert!(!options.contains(&enemy));
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![enemy],
                players: vec![]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![ring],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(ring).unwrap().controller, p1);
    assert_eq!(e.state().object(ring).unwrap().owner, p0);
    assert_eq!(e.state().object(wolf).unwrap().controller, p0);
    pass_until(&mut e, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(e.state().object(ring).unwrap().controller, p1);
}

#[test]
fn stardew_valley_food_requires_mana_untapped_land_and_untapped_controlled_creature() {
    use crate::object::Status;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for missing in 0..3 {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[stardew_valley(), young_wolf()])
            .battlefield(1, &[gray_ogre()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let valley = on_battlefield(&e, p0, stardew_valley()).unwrap();
        let wolf = on_battlefield(&e, p0, young_wolf()).unwrap();
        let state = e.dev_state_mut(p0).unwrap();
        state.players[0]
            .mana_pool
            .add(ManaColor::Colorless, if missing == 0 { 1 } else { 2 });
        if missing != 0 {
            state
                .object_mut(if missing == 1 { valley } else { wolf })
                .unwrap()
                .status
                .insert(Status::TAPPED);
        }
        e.refresh_offer();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(
            !legal.abilities.contains(&(valley, 1)),
            "missing cost case {missing}"
        );
        let before = e.snapshot_hash();
        assert!(
            e.apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: valley,
                    ability_index: 1
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), before);
        assert!(tokens_of(&e, p0).is_empty());
        assert!(!is_tapped(&e, on_battlefield(&e, p1, gray_ogre()).unwrap()));
    }
}

#[test]
fn stardew_valley_draw_is_unavailable_outside_sorcery_timing() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[stardew_valley()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::Combat);
    let valley = on_battlefield(&e, p0, stardew_valley()).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("combat priority")
    };
    assert!(!legal.abilities.contains(&(valley, 2)));
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: valley,
                ability_index: 2
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
}

#[test]
fn stardew_valley_controller_chooses_another_player_after_drawing() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut e = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[stardew_valley(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let target = on_battlefield(&e, p0, forest()).unwrap();
    let hand_before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();
    activate(&mut e, p0, stardew_valley(), 2);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![target],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    let Pending::ChoosePlayer { player, options } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert_eq!(options, vec![p1, p2]);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
    let before = e.snapshot_hash();
    assert!(e.apply(p0, PlayerAction::ChoosePlayer(p0)).is_err());
    assert_eq!(e.snapshot_hash(), before);
    e.apply(p0, PlayerAction::ChoosePlayer(p2)).unwrap();
    assert_eq!(e.pending().asked(), Some(p2));
    let before = e.snapshot_hash();
    assert!(e.apply(p1, PlayerAction::YesNo(true)).is_err());
    assert_eq!(e.snapshot_hash(), before);
    e.apply(p2, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(target).unwrap().controller, p2);
    assert_eq!(e.state().object(target).unwrap().owner, p0);
}

#[test]
fn stardew_valley_draw_costs_three_and_an_untapped_land() {
    use crate::object::Status;
    let p0 = PlayerId::new(0);
    for tapped in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[stardew_valley(), forest()])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let valley = on_battlefield(&e, p0, stardew_valley()).unwrap();
        let state = e.dev_state_mut(p0).unwrap();
        state.players[0]
            .mana_pool
            .add(ManaColor::Colorless, if tapped { 3 } else { 2 });
        if tapped {
            state
                .object_mut(valley)
                .unwrap()
                .status
                .insert(Status::TAPPED);
        }
        e.refresh_offer();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(!legal.abilities.contains(&(valley, 2)));
        let before = e.snapshot_hash();
        assert!(
            e.apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: valley,
                    ability_index: 2
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), before);
    }
}

#[test]
fn stardew_valley_target_losing_control_prevents_the_draw_and_transfer() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[stardew_valley(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let target = on_battlefield(&e, p0, forest()).unwrap();
    let hand_before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();
    activate(&mut e, p0, stardew_valley(), 2);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![target],
            players: vec![],
        },
    )
    .unwrap();
    crate::resolve::gain_control(e.dev_state_mut(p0).unwrap(), &[(target, p1)]);
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    assert_eq!(e.state().object(target).unwrap().controller, p1);
}
