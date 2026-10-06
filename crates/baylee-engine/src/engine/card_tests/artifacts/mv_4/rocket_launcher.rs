//! `cards/artifacts/mv_4/rocket_launcher.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rocket Launcher: "{2}: This artifact deals 1 damage to any target.
/// Destroy this artifact at the beginning of the next end step. Activate
/// only if you've controlled this artifact continuously since the beginning
/// of your most recent turn."
///
/// Placed before the first turn, it has been controlled since that turn
/// began, so the ability is offered. The delayed destroy is a trigger that
/// uses the stack (CR 603.7) and destroys only as it resolves.
#[test]
fn rocket_launcher_deals_1_and_destroys_itself_at_the_next_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rocket_launcher(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, rocket_launcher(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p1), 19, "1 damage, the printed amount");

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
    });
    assert!(
        !stack_is_empty(&engine),
        "the delayed destroy uses the stack (CR 603.7)"
    );
    assert!(
        on_battlefield(&engine, p0, rocket_launcher()).is_some(),
        "and nothing is destroyed before it resolves"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, rocket_launcher()).is_some(),
        "destroyed at the beginning of the end step"
    );
}

/// A returned-and-recast Rocket Launcher is a new object, so the delayed
/// destroy about the old one destroys nothing (CR 603.7c, 400.7) — the 2004
/// ruling's "destroyed at the end of any turn in which you use it" is about
/// the object that was used.
#[test]
fn a_rocket_launcher_that_left_and_came_back_is_not_destroyed() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                rocket_launcher(),
                island(),
                island(),
                island(),
                island(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[boomerang()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let first = on_battlefield(&engine, p0, rocket_launcher()).expect("the Launcher is out");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, rocket_launcher(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, p0, boomerang());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Boomerang asks for its target, got {:?}", engine.pending())
    };
    assert!(options.contains(&first));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![first],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, rocket_launcher()).is_some());

    cast_with_floating(&mut engine, p0, rocket_launcher());
    pass_until(&mut engine, stack_is_empty);
    let rebuilt = on_battlefield(&engine, p0, rocket_launcher()).expect("it came back");

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(rebuilt).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the new Launcher is not the object the delayed trigger was about"
    );
}

/// "Activate only if you've controlled this artifact continuously since the
/// beginning of your most recent turn." Cast this turn it is not offered;
/// once a turn has passed under its controller it is.
#[test]
fn rocket_launcher_is_withheld_on_the_turn_it_arrives() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[rocket_launcher()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Six Forests pay the {4} and leave {G}{G} floating, so the ability is
    // affordable on the turn it arrives: absent from the offer means the
    // condition withheld it, not the mana.
    let spare = all_on_battlefield(&engine, p0, forest())[0];
    tap_mana_except(&mut engine, p0, spare);
    cast_with_floating(&mut engine, p0, rocket_launcher());
    pass_until(&mut engine, stack_is_empty);
    let launcher = on_battlefield(&engine, p0, rocket_launcher()).expect("it resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{2}} activation would be affordable"
    );
    assert!(
        !priority_offer(&engine).abilities.contains(&(launcher, 0)),
        "it arrived this turn, so it has not been controlled since the turn began"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
    });
    tap_all_mana(&mut engine, p0);
    assert!(
        priority_offer(&engine).abilities.contains(&(launcher, 0)),
        "on the controller's next turn the condition is met"
    );
}
