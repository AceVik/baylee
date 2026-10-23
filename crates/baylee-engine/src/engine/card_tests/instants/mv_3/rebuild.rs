#[allow(clippy::wildcard_imports)]
use super::*;

fn rebuild() -> CardIndex {
    card_index("496999a7-a82a-45ea-9c4e-1b6aabc781cd")
}

fn thran_lens() -> CardIndex {
    card_index("0ac723bf-ce86-4028-a693-0ef498e4c82d")
}

#[test]
fn rebuild_bounces_all_artifacts() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), thran_lens()])
        .battlefield(1, &[thran_lens()])
        .hand(0, &[rebuild()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, rebuild());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, thran_lens()).is_some());
    assert!(in_hand(&engine, p1, thran_lens()).is_some());
}
