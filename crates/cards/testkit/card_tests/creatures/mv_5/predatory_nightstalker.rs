//! `cards/creatures/mv_5/predatory_nightstalker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn predatory_nightstalker() -> CardIndex {
    card_index("9d2ad65f-fba0-45e9-a5e6-bec63a9db979")
}

/// On entering, targets an opponent; accepting "you may" forces that opponent
/// to choose one of their own creatures to sacrifice.
#[test]
fn predatory_nightstalker_makes_targeted_opponent_sacrifice_a_creature_of_their_choice() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), craw_wurm(), forest()])
        .hand(0, &[predatory_nightstalker()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let own_elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let opp_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let opp_wurm = on_battlefield(&engine, p1, craw_wurm()).unwrap();
    let opp_land = on_battlefield(&engine, p1, forest()).unwrap();

    let swamps = all_on_battlefield(&engine, p0, swamp());
    tap_mana_where(&mut engine, p0, |id| swamps.contains(&id));
    let spell = in_hand(&engine, p0, predatory_nightstalker()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();

    // ETB trigger asks p0 to target an opponent.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("stopped on ChooseTargets")
    };
    assert_eq!(player, p0);
    assert!(options.is_empty());
    assert_eq!(player_options, vec![p1], "only opponents are legal targets");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();

    // Stop at "you may", answer yes, then verify p1 chooses which of p1's creatures dies.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected p1 creature sacrifice choice, got {:?}",
            engine.pending()
        );
    };
    assert_eq!((player, min, max), (p1, 1, 1));
    assert!(options.contains(&opp_bears) && options.contains(&opp_wurm));
    assert!(!options.contains(&opp_land) && !options.contains(&own_elf));

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![opp_bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p1, craw_wurm()).is_some());
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
}

/// Declining "you may" leaves the targeted opponent's creatures untouched.
#[test]
fn predatory_nightstalker_may_decline_forcing_a_sacrifice() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 5])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[predatory_nightstalker()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, predatory_nightstalker());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());
}
