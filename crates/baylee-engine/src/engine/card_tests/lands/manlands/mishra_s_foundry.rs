//! `cards/lands/manlands/mishra_s_foundry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mishra's Foundry prints `{{T}}: Add {{C}}`, `{2}: This land becomes a 2/2
/// Assembly-Worker artifact creature until end of turn. It's still a land.`,
/// and `{1}, {{T}}: Target attacking Assembly-Worker gets +2/+2 until end
/// of turn.`
///
/// Under `Coverage::Implemented`, all printed characteristics are fully
/// realized. This test activates the `{2}` ability off two basic lands,
/// verifying that Mishra's Foundry becomes a 2/2 Assembly-Worker artifact
/// creature while continuing to be a land.
#[test]
fn mishra_s_foundry_animates_into_an_assembly_worker() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mishra_s_foundry(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let foundry = on_battlefield(&engine, p0, mishra_s_foundry()).expect("foundry on battlefield");
    assert!(
        !engine
            .state()
            .object(foundry)
            .expect("foundry exists")
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "a land is not a creature before activation"
    );

    tap_all_mana_but(&mut engine, p0, Some(mishra_s_foundry()));
    activate(&mut engine, p0, mishra_s_foundry(), 1);
    pass_until(&mut engine, stack_is_empty);

    let chars = engine
        .state()
        .object(foundry)
        .expect("foundry exists")
        .characteristics();
    assert!(chars.types.contains(TypeSet::LAND), "it's still a land");
    assert!(chars.types.contains(TypeSet::ARTIFACT));
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::ASSEMBLY_WORKER)
    );
    assert_eq!(pt(&engine, foundry), (2, 2));
}

/// Mishra's Foundry: "{1}, {T}: Target attacking Assembly-Worker gets +2/+2
/// until end of turn." Mishra's Factory is the Assembly-Worker; it is not a
/// target until it attacks.
#[test]
fn mishra_s_foundry_pumps_an_attacking_assembly_worker_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let foundry = card_index("b43e9772-6ad4-49c7-9557-b18ee1e4587d");
    let factory = card_index("5963e0ef-e0bc-4611-ad4f-813a4c0eacfb");
    let mut engine = Duel::new(2105, forest())
        .battlefield(0, &[foundry, factory, forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let fy = on_battlefield(&engine, p0, foundry).expect("foundry");
    let fc = on_battlefield(&engine, p0, factory).expect("factory");
    let forests: Vec<_> = lands_of(&engine, p0)
        .into_iter()
        .filter(|id| *id != fy && *id != fc)
        .collect();

    tap_mana_where(&mut engine, p0, |id| id == forests[0]);
    activate(&mut engine, p0, factory, 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, fc), (2, 2));
    assert!(
        !priority_offer(&engine).abilities.contains(&(fy, 2)),
        "a Factory that is not attacking is no target"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers");
    };
    let defender = defenders.into_iter().next().expect("a defender");
    let _ = p1;
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(fc, defender)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && e.state().turn.step == crate::turn::Step::DeclareAttackers
    });
    tap_mana_where(&mut engine, p0, |id| id == forests[1]);
    activate(&mut engine, p0, foundry, 2);
    unf_aim_and_pay(&mut engine, p0, Some(fc), None);
    assert_eq!(pt(&engine, fc), (4, 4));
}
