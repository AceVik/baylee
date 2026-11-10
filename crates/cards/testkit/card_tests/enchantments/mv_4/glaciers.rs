//! `cards/enchantments/mv_4/glaciers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn glaciers() -> CardIndex {
    card_index("a4a9c2bb-e6d0-4665-8b2f-e90c78784303")
}

/// "At the beginning of your upkeep, sacrifice this enchantment unless you
/// pay {W}{U}. All Mountains are Plains.": paid with a Plains and an Island,
/// Glaciers stays, and the Mountain now taps for {W}, not {R}.
#[test]
fn glaciers_paid_stays_and_turns_a_mountain_into_a_plains() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[glaciers(), plains(), island(), mountain()])
        .start();
    keep_mulligans(&mut engine);

    let (asked, cost) = walk_to_price(&mut engine);
    assert_eq!(asked, p0);
    assert_eq!(cost, baylee_core::mana::ManaCost::parse("{W}{U}"));
    let mountain_id = on_battlefield(&engine, p0, mountain()).expect("seated");
    pay_the_price_with(&mut engine, p0, |id| id != mountain_id);
    assert!(
        on_battlefield(&engine, p0, glaciers()).is_some(),
        "paid, so Glaciers stays"
    );

    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the Mountain taps for W"
    );
    assert_eq!(pool.available(ManaColor::Red), 0, "and no longer for R");
}

/// Unpaid, Glaciers is sacrificed and the Mountain is a Mountain again.
#[test]
fn glaciers_unpaid_is_sacrificed_and_the_mountain_is_red_again() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[glaciers(), plains(), island(), mountain()])
        .start();
    keep_mulligans(&mut engine);

    walk_to_price(&mut engine);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, glaciers()).is_some(),
        "unpaid: sacrificed"
    );

    reach_main_phase(&mut engine, p0);
    let mountain_id = on_battlefield(&engine, p0, mountain()).expect("seated");
    tap_mana_where(&mut engine, p0, |id| id == mountain_id);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "a Mountain taps for R");
    assert_eq!(pool.available(ManaColor::White), 0);
}
