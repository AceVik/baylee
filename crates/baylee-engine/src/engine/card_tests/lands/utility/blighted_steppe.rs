//! `cards/lands/utility/blighted_steppe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn blighted_steppe_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(116, forest())
        .battlefield(0, &[blighted_steppe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let steppe = on_battlefield(&engine, p0, blighted_steppe()).expect("Steppe deployed");
    activate(&mut engine, p0, blighted_steppe(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, steppe));
}

#[test]
fn blighted_steppe_gains_twice_the_creature_count_on_resolution() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                blighted_steppe(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    tap_all_mana_but(&mut e, p0, Some(blighted_steppe()));
    activate(&mut e, p0, blighted_steppe(), 1);
    assert!(in_graveyard(&e, p0, blighted_steppe()).is_some());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 24);
    assert_eq!(e.state().players[1].life, 20);
}
