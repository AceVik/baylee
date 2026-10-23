#[allow(clippy::wildcard_imports)]
use super::*;

fn viashino_outrider() -> CardIndex {
    card_index("3e81c22e-52ec-4142-9a98-88681d04ce60")
}

#[test]
fn viashino_outrider_sacrifices_on_upkeep_if_echo_unpaid() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[viashino_outrider()])
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, viashino_outrider());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, viashino_outrider()).is_some());

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, viashino_outrider()).is_none());
    assert!(in_graveyard(&engine, p0, viashino_outrider()).is_some());
}
