#[allow(clippy::wildcard_imports)]
use super::*;

fn thran_turbine() -> CardIndex {
    card_index("6c31f06d-bf02-48a7-9c17-be2a226b8eae")
}

#[test]
fn thran_turbine_adds_two_colorless_mana_on_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[thran_turbine()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::Upkeep && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2
    );
    let _ = p0;
}
