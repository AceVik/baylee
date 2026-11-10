//! `cards/creatures/mv_4/daraja_griffin.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn daraja_griffin() -> CardIndex {
    card_index("daa0d720-cac1-487a-93e2-33e18f91f19b")
}

fn scathe_zombies() -> CardIndex {
    baylee_core::generated::index::SCATHE_ZOMBIES
}

#[test]
fn daraja_griffin_sacrifices_to_destroy_target_black_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .battlefield(1, &[scathe_zombies(), grizzly_bears()])
        .hand(0, &[daraja_griffin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_zombies = on_battlefield(&engine, p1, scathe_zombies()).unwrap();
    let opp_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, daraja_griffin());
    pass_until(&mut engine, stack_is_empty);

    let griffin = on_battlefield(&engine, p0, daraja_griffin()).unwrap();
    assert_eq!(pt(&engine, griffin), (2, 2));
    assert!(keywords(&engine, griffin).contains(KeywordSet::FLYING));

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: griffin,
                ability_index: 0,
            },
        )
        .unwrap();
    let menu = aim_at(&mut engine, p0, opp_zombies);
    assert!(
        !menu.contains(&opp_bear),
        "\"target black creature\": a green Bears is no target"
    );
    assert!(!menu.contains(&griffin), "the white Griffin is no target");
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, scathe_zombies()).is_some());
    assert!(
        in_graveyard(&engine, p0, daraja_griffin()).is_some(),
        "the Griffin is sacrificed as the cost"
    );
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_some(),
        "the Bears is untouched"
    );
}
