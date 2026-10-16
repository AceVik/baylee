//! `cards/artifacts/mv_4/relic_of_sauron.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn relic_of_sauron() -> CardIndex {
    card_index("b3a81bb1-cbd5-41a0-8dd4-faea06593f84")
}

/// "{T}: Add two mana in any combination of {U}, {B}, and/or {R}." Whatever
/// the combination, it is two mana and none of it is white, green or colorless.
#[test]
fn tapping_adds_two_mana_from_blue_black_and_red_only() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[relic_of_sauron()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let relic = on_battlefield(&engine, p0, relic_of_sauron()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == relic);
    assert!(is_tapped(&engine, relic));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 2, "two mana");
    let outside: u64 = [ManaColor::White, ManaColor::Green, ManaColor::Colorless]
        .into_iter()
        .map(|c| pool.available(c))
        .sum();
    assert_eq!(outside, 0, "only U, B and R");
}

/// "{3}, {T}: Draw two cards, then discard a card."
#[test]
fn the_relic_loots_for_three() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[relic_of_sauron(), forest(), forest(), forest()])
        .hand(0, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let relic = on_battlefield(&engine, p0, relic_of_sauron()).unwrap();
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let lib = library_size(&engine, p0);
    tap_mana_except(&mut engine, p0, relic);
    activate(&mut engine, p0, relic_of_sauron(), 1);
    assert!(is_tapped(&engine, relic));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), hand + 2, "discard from the enlarged hand");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a listed card");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, p0), lib - 2);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 1
    );
}
