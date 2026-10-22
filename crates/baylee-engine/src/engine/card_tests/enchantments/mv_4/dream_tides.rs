//! `cards/enchantments/mv_4/dream_tides.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn dream_tides() -> CardIndex {
    card_index("d9583aab-3eff-4d88-84d5-7219ab982636")
}

#[test]
fn dream_tides_prevents_creatures_from_untapping() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[dream_tides()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    engine.state.set_tapped(opp_bear, true);

    cast_from_hand(&mut engine, p0, dream_tides());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    assert!(is_tapped(&engine, opp_bear));
}
