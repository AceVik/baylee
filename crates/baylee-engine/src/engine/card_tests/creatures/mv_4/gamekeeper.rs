#[allow(clippy::wildcard_imports)]
use super::*;

fn gamekeeper() -> CardIndex {
    card_index("c6921cfe-48f9-4378-b145-d9b430c2aaf6")
}

#[test]
fn gamekeeper_exiles_itself_on_death_and_searches_library_for_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[gamekeeper()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let keeper = on_battlefield(&engine, p0, gamekeeper()).unwrap();
    let state = engine.dev_state_mut(p0).unwrap();
    crate::sba::destroy(state, keeper);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
}
