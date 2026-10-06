//! `cards/lands/utility/ice_floe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ice Floe prints `You may choose not to untap this land during your untap step.` and `{{T}}: Tap target creature without flying that's attacking you. It doesn't untap during its controller's untap step for as long as this land remains tapped.`
///
/// Under `Coverage::Partial`, the untap-step choice (`Modifier::MayChooseNotToUntap`) and the activated tap ability are implemented, while the continuous lock keeping the creature tapped is omitted.
/// During an opponent's combat, when attacked by a non-flying creature (`llanowar_elves()`), ability 1 taps the attacker and taps Ice Floe.
/// On the controller's subsequent untap step, the engine offers the `crate::choice::ChoicePrompt::LeaveTapped` question, and electing to keep Ice Floe tapped leaves it tapped into the main phase.
#[test]
fn ice_floe_taps_attacking_nonflyer_and_may_remain_tapped() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ice_floe()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let floe = on_battlefield(&engine, p0, ice_floe()).expect("ice floe on battlefield");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("elf on battlefield");

    // Advance to p1's declare attackers step.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );

    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        panic!("expected choose attackers, got {:?}", engine.pending());
    };
    let defender = defenders[0];
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, defender)],
            },
        )
        .unwrap();

    // In declare attackers priority, pass p1's priority if held so p0 can respond.
    if let Pending::Priority { player, .. } = engine.pending().clone()
        && player == p1
    {
        engine.apply(p1, PlayerAction::PassPriority).unwrap();
    }

    activate(&mut engine, p0, ice_floe(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elf), "attacking elf is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, floe), "ice floe tapped to activate");

    // Progress through the rest of the turn to p0's untap step.
    let mut asked_leave_tapped = false;
    for _ in 0..100 {
        if let Pending::ChooseCards {
            player,
            options,
            prompt: crate::choice::ChoicePrompt::LeaveTapped,
            ..
        } = engine.pending().clone()
        {
            assert_eq!(player, p0);
            assert!(
                options.contains(&floe),
                "ice floe is offered to leave tapped"
            );
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseObjects {
                        objects: vec![floe],
                    },
                )
                .unwrap();
            asked_leave_tapped = true;
            break;
        }
        let (player, action) = answer_one(&engine).expect("walk to untap step");
        engine.apply(player, action).unwrap();
    }
    assert!(
        asked_leave_tapped,
        "untap step asked whether to leave Ice Floe tapped"
    );

    reach_main_phase(&mut engine, p0);
    assert!(
        is_tapped(&engine, floe),
        "ice floe remained tapped into main phase"
    );
}
