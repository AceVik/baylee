//! `cards/artifacts/mv_6/rakalite.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rakalite: "{2}: Prevent the next 1 damage that would be dealt to any
/// target this turn. Return this artifact to its owner's hand at the
/// beginning of the next end step."
///
/// The shield is read by its number: a Bolt for 3 still takes 2 life, which
/// only holds if exactly one point was prevented. The return is the second
/// sentence's delayed trigger (CR 603.7): it uses the stack at the end step
/// and hands the artifact back.
#[test]
fn rakalite_prevents_one_damage_and_returns_at_the_next_end_step() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rakalite(), mountain(), mountain(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bolt_land = all_on_battlefield(&engine, p0, mountain())[0];
    tap_mana_except(&mut engine, p0, bolt_land);
    activate(&mut engine, p0, rakalite(), 0);
    let Pending::ChooseTargets { min, max, .. } = engine.pending().clone() else {
        panic!("any target is a question, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (1, 1));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().shields.len(),
        1,
        "one prevention shield, made by the ability"
    );

    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: bolt_land })
        .unwrap();
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, p0),
        18,
        "3 dealt, 1 prevented: the shield is exactly 1"
    );
    assert!(engine.state().shields.is_empty(), "and the shield is spent");

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
    });
    assert!(
        !stack_is_empty(&engine),
        "the delayed return uses the stack (CR 603.7)"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p0, rakalite()).is_some(),
        "and hands Rakalite back"
    );
    assert!(on_battlefield(&engine, p0, rakalite()).is_none());
}

/// The 2004 ruling: "Only returns to your hand if it is still on the
/// battlefield at the end of the turn. If it leaves the battlefield, it does
/// not return." Disenchanted before the end step, the delayed trigger finds
/// the object in a graveyard and returns nothing (CR 603.7c, 400.7).
#[test]
fn rakalite_that_left_the_battlefield_does_not_return() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rakalite(), plains(), plains(), plains(), plains()])
        .hand(0, &[disenchant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let artifact = on_battlefield(&engine, p0, rakalite()).expect("Rakalite is out");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, rakalite(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, disenchant());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Disenchant asks for its target, got {:?}", engine.pending())
    };
    assert!(options.contains(&artifact));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![artifact],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(artifact).map(|o| o.zone),
        Some(Zone::Graveyard),
        "destroyed before the end step"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(artifact).map(|o| o.zone),
        Some(Zone::Graveyard),
        "the delayed trigger does not return it from the graveyard"
    );
    assert!(in_hand(&engine, p0, rakalite()).is_none());
}
