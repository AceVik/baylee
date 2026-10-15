//! `cards/enchantments/mv_4/vernal_bloom.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn vernal_bloom() -> CardIndex {
    card_index("238b270c-152a-48e5-a955-65f5cb80c174")
}

/// "Whenever a Forest is tapped for mana, its controller adds an additional
/// {G}": my Forest makes two green, the opponent's Forest makes two green
/// for the opponent, and a Mountain makes its one red.
#[test]
fn vernal_bloom_doubles_every_forest_for_its_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[vernal_bloom(), forest(), mountain()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mountain_id = on_battlefield(&engine, p0, mountain()).expect("seated");
    tap_mana_where(&mut engine, p0, |id| id == mountain_id);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "a Mountain: one mana"
    );

    let forest_id = on_battlefield(&engine, p0, forest()).expect("seated");
    tap_mana_where(&mut engine, p0, |id| id == forest_id);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "a Forest: {{G}} and the additional {{G}}"
    );

    reach_their_main_phase(&mut engine, p1);
    let theirs = on_battlefield(&engine, p1, forest()).expect("seated");
    tap_mana_where(&mut engine, p1, |id| id == theirs);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "its controller, the opponent, adds the additional {{G}}"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "not me");
}
