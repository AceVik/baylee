#[allow(clippy::wildcard_imports)]
use super::*;

fn molten_hydra() -> CardIndex {
    card_index("cd5bc95d-4915-4c5a-9b57-2730adf69797")
}

#[test]
fn molten_hydra_adds_counters_and_removes_them_to_deal_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                molten_hydra(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hydra = on_battlefield(&engine, p0, molten_hydra()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: hydra,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: hydra,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, hydra), (3, 3));

    assert!(!is_tapped(&engine, hydra));
    if let Pending::Priority { legal, .. } = engine.pending() {
        assert!(
            !legal.abilities.contains(&(hydra, 1)),
            "unsupported damage ability is unavailable"
        );
    }
    let _ = attack_and_collect_blocks(&mut engine, hydra, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 17);
    assert_eq!(pt(&engine, hydra), (3, 3));
}
