//! Sliver Overlord — `{W}{U}{B}{R}{G}` 7/7 Legendary Creature — Sliver Mutant (SCG).
//! Oracle: "{3}: Search your library for a Sliver card, reveal that card, put it into your hand, then shuffle. {3}: Gain control of target Sliver."

use super::*;

fn sliver_overlord() -> CardIndex {
    card_index("762d864b-69bc-43ac-af2f-e18186010169")
}

fn blade_sliver() -> CardIndex {
    card_index("110e5695-a9df-4201-9230-99f6abe9978f")
}

#[test]
fn sliver_overlord_tutors_sliver_to_hand_and_steals_target_sliver() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let overlord = sliver_overlord();
    let blade_sliver = blade_sliver();

    let mut engine = Duel::new(SEED, blade_sliver)
        .battlefield(
            0,
            &[
                plains(),
                island(),
                swamp(),
                mountain(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[blade_sliver])
        .hand(0, &[overlord])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let w = on_battlefield(&engine, p0, plains()).unwrap();
    let u = on_battlefield(&engine, p0, island()).unwrap();
    let b = on_battlefield(&engine, p0, swamp()).unwrap();
    let r = on_battlefield(&engine, p0, mountain()).unwrap();
    let forests = all_on_battlefield(&engine, p0, forest());
    let their_sliver = on_battlefield(&engine, p1, blade_sliver).unwrap();

    // Cast Sliver Overlord for {W}{U}{B}{R}{G}.
    tap_mana_where(&mut engine, p0, |id| [w, u, b, r, forests[0]].contains(&id));
    cast_with_floating(&mut engine, p0, overlord);
    pass_until(&mut engine, stack_is_empty);

    let overlord_id = on_battlefield(&engine, p0, overlord).unwrap();

    // Ability 0 ({3}): search library for a Sliver card and put it into hand.
    tap_mana_where(&mut engine, p0, |id| forests[1..4].contains(&id));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: overlord_id,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected library search choice");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, blade_sliver).is_some());

    // Ability 1 ({3}): gain control of target Sliver indefinitely.
    tap_mana_where(&mut engine, p0, |id| forests[4..7].contains(&id));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: overlord_id,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![their_sliver],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(their_sliver).unwrap().controller, p0);
}
