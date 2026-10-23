#[allow(clippy::wildcard_imports)]
use super::*;

fn king_crab() -> CardIndex {
    card_index("24982bfe-5276-44a8-99a5-43b497ebf51f")
}

#[test]
fn king_crab_puts_target_green_creature_into_owners_library() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), king_crab()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let crab = on_battlefield(&engine, p0, king_crab()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: crab,
                ability_index: 0,
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bear],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
}
