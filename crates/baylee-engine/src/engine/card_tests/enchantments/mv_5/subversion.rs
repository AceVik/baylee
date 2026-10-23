//! `cards/enchantments/mv_5/subversion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn subversion() -> CardIndex {
    card_index("0772c5d2-9189-47e2-b0ee-13babd94ea93")
}

#[test]
fn subversion_drains_on_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[subversion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, subversion());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "unsupported gain is omitted"
    );
}
