//! `cards/artifacts/mv_2/thaumatic_compass.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The front fetches the seventh land and transforms at the end step. The back
/// untaps an opposing attacker and removes it from combat on the following turn.
#[test]
fn thaumatic_compass_searches_for_land_and_transforms_at_end_step_with_seven_lands() {
    use crate::engine::card_tests::legends_kit as kit;
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(249, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                thaumatic_compass(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let compass = on_battlefield(&engine, p0, thaumatic_compass()).expect("compass on battlefield");
    let compass_was = identity(&engine, compass);
    assert_eq!(engine.state().object(compass).unwrap().face_index, 0);

    // Tap 3 Forests to activate the {3}, {T} search ability.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, thaumatic_compass(), 0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("expected search prompt, got {:?}", engine.pending())
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "search library prompt"
    );
    assert!(
        !options.is_empty(),
        "library contains basic lands to search"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let fetched_land = in_hand(&engine, p0, forest()).expect("searched land is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: fetched_land })
        .unwrap();

    assert_eq!(lands_of(&engine, p0).len(), 7);

    // Advance to the end step: with 7 lands, the transform trigger fires and resolves.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, thaumatic_compass())
            .is_some_and(|id| e.state().object(id).map(|o| o.face_index) == Some(1))
    });

    let spires = on_battlefield(&engine, p0, thaumatic_compass()).expect("spires on battlefield");
    assert_eq!(
        identity(&engine, spires),
        compass_was,
        "the same object, turned over: a transform changes no zone (CR 712.18)"
    );
    assert!(
        is_tapped(&engine, spires),
        "still tapped from the search: turning over is not entering"
    );
    let t = types(&engine, spires);
    assert!(t.contains(TypeSet::LAND), "transformed permanent is a land");
    assert!(
        !t.contains(TypeSet::ARTIFACT),
        "transformed permanent is no longer an artifact"
    );
    reach_their_main_phase(&mut engine, p0);
    let attacker = kit::obj(&engine, kit::P1, llanowar_elves());
    pass_until(&mut engine, |e| {
        e.state().turn.active == kit::P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            kit::P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    kit::use_ability(&mut engine, p0, thaumatic_compass(), 1, &[attacker], &[]);
    assert!(!is_tapped(&engine, attacker));
    assert!(engine.state().combat.defender_of(attacker).is_none());
}

/// "At the beginning of your end step, **if you control seven or more lands**":
/// with six, the end step puts no trigger on the stack and the Compass is still
/// the artifact it was.
#[test]
fn thaumatic_compass_does_not_transform_with_only_six_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(249, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                thaumatic_compass(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let compass = on_battlefield(&engine, p0, thaumatic_compass()).expect("compass on battlefield");
    assert_eq!(lands_of(&engine, p0).len(), 6, "exactly one short of seven");

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        stack_is_empty(&engine),
        "six lands: the intervening \"if\" is false, so no transform trigger was put on the stack (CR 603.4)"
    );
    assert_eq!(
        engine.state().object(compass).unwrap().face_index,
        0,
        "still Thaumatic Compass in its own end step"
    );

    // And it stays the artifact through the rest of the turn and into the next.
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.number > turn);
    assert_eq!(
        engine.state().object(compass).unwrap().face_index,
        0,
        "six lands never turn the Compass over"
    );
    assert!(
        types(&engine, compass).contains(TypeSet::ARTIFACT),
        "it is still an artifact"
    );
}

/// Spires of Orazca: "Untap target **attacking creature an opponent controls**".
/// On the opponent's turn the menu is the creature that attacks, not the one
/// that stayed home and not my own.
#[test]
fn spires_of_orazca_offers_only_the_attacking_creature_of_an_opponent() {
    use crate::engine::card_tests::legends_kit as kit;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(249, forest())
        .battlefield(0, &[thaumatic_compass(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spires = on_battlefield(&engine, p0, thaumatic_compass()).expect("compass on battlefield");
    let def = baylee_cards::by_index(thaumatic_compass()).expect("the card is in the pool");
    engine
        .dev_state_mut(p0)
        .unwrap()
        .switch_face(spires, def, 1);
    engine.sync_static_effects();
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves");
    let theirs = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(theirs.len(), 2, "two Elves across the table");
    let (attacker, stay_home) = (theirs[0], theirs[1]);

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    assert!(
        kit::offered(&engine, thaumatic_compass(), p0, 1),
        "Spires' untap ability is offered with an opposing attacker out"
    );
    activate(&mut engine, p0, thaumatic_compass(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the ability targets, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![attacker],
        "only the opposing creature that is attacking: not the one that stayed home ({stay_home:?}) \
         and not my own ({mine:?})"
    );
}

/// The other half of "attacking creature **an opponent controls**": when the only
/// attacker is mine, Spires has nothing to aim at and the ability is not offered.
#[test]
fn spires_of_orazca_cannot_aim_at_my_own_attacker() {
    use crate::engine::card_tests::legends_kit as kit;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(249, forest())
        .battlefield(0, &[thaumatic_compass(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let spires = on_battlefield(&engine, p0, thaumatic_compass()).expect("compass on battlefield");
    let def = baylee_cards::by_index(thaumatic_compass()).expect("the card is in the pool");
    engine
        .dev_state_mut(p0)
        .unwrap()
        .switch_face(spires, def, 1);
    engine.sync_static_effects();
    engine.refresh_offer();
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves");
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "their Elves stay home"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(mine, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    assert!(
        engine.state().combat.defender_of(mine).is_some(),
        "my Elves are attacking"
    );
    assert!(
        !kit::offered(&engine, thaumatic_compass(), p0, 1),
        "an attacking creature I control is not \"an opponent's\": Spires has no target"
    );
}
