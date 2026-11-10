//! `cards/artifacts/mv_4/hedron_archive.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hedron_archive() -> CardIndex {
    card_index("32263baa-d3f0-463f-92b3-4e9938476add")
}

/// "{T}: Add {C}{C}."
#[test]
fn tapping_the_archive_adds_two_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hedron_archive()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let archive = on_battlefield(&engine, p0, hedron_archive()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == archive);
    assert!(is_tapped(&engine, archive));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 2);
    assert_eq!(pool.total(), 2);
}

/// "{2}, {T}, Sacrifice this artifact: Draw two cards."
#[test]
fn sacrificing_the_archive_draws_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hedron_archive(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let archive = on_battlefield(&engine, p0, hedron_archive()).unwrap();
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_mana_except(&mut engine, p0, archive);
    activate(&mut engine, p0, hedron_archive(), 1);
    assert!(in_graveyard(&engine, p0, hedron_archive()).is_some());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 2
    );
}
