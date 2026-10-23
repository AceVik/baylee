#[allow(clippy::wildcard_imports)]
use super::*;

fn multani_maro_sorcerer() -> CardIndex {
    card_index("bce5b327-7da1-4136-b76f-069e2fef5d2a")
}

#[test]
fn multani_maro_sorcerer_resolves_as_zero_zero_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[multani_maro_sorcerer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, multani_maro_sorcerer());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, multani_maro_sorcerer()).is_some());
}
