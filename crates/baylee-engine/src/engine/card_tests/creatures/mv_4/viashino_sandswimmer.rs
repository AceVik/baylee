#[allow(clippy::wildcard_imports)]
use super::*;

fn viashino_sandswimmer() -> CardIndex {
    card_index("1d82f01a-29be-4756-8745-e2c04bf77c61")
}

#[test]
fn viashino_sandswimmer_flips_coin_to_bounce_or_sacrifice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[viashino_sandswimmer(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, viashino_sandswimmer(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, viashino_sandswimmer()).is_none());
    assert!(
        in_hand(&engine, p0, viashino_sandswimmer()).is_some()
            || in_graveyard(&engine, p0, viashino_sandswimmer()).is_some()
    );
}
