//! `cards/sorceries/mv_1/goblin_war_strike.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn goblin_war_strike() -> CardIndex {
    card_index("5944d5c7-65ab-425f-a453-ed5871ca3d73")
}

fn mons_s_goblin_raiders() -> CardIndex {
    card_index("a37159df-f6d7-4db6-85de-0ea77f425993")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Goblin War Strike (5944d5c7-65ab-425f-a453-ed5871ca3d73) — {R} Sorcery:
/// "Goblin War Strike deals damage to target player or planeswalker equal to
/// the number of Goblins you control."
#[test]
fn goblin_war_strike_counts_only_your_goblins_and_excludes_creatures_from_targets() {
    let mut engine = lk::game(
        SEED,
        &[
            mountain(),
            mons_s_goblin_raiders(),
            festering_goblin(),
            grizzly_bears(),
        ],
        &[mons_s_goblin_raiders(), karn_the_great_creator()],
        &[goblin_war_strike()],
        &[],
    );
    let bears = lk::obj(&engine, P0, grizzly_bears());
    let their_goblin = lk::obj(&engine, P1, mons_s_goblin_raiders());
    let karn = lk::obj(&engine, P1, karn_the_great_creator());

    cast_from_hand(&mut engine, P0, goblin_war_strike());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(player_options.contains(&P0) && player_options.contains(&P1));
    assert!(options.contains(&karn), "planeswalker is a legal target");
    assert!(!options.contains(&bears) && !options.contains(&their_goblin));

    lk::aim(&mut engine, &[], &[P1]);
    lk::settle(&mut engine);

    assert_eq!(
        lk::life(&engine, P1),
        18,
        "2 controlled Goblins -> 2 damage"
    );
    assert_eq!(lk::life(&engine, P0), 20);
}

/// Goblin War Strike — damages a targeted planeswalker's loyalty, and deals 0
/// when the caster controls no Goblins.
#[test]
fn goblin_war_strike_damages_planeswalker_and_deals_zero_without_goblins() {
    let mut engine = lk::game(
        SEED,
        &[mountain(), mons_s_goblin_raiders(), festering_goblin()],
        &[karn_the_great_creator()],
        &[goblin_war_strike()],
        &[],
    );
    let karn = lk::obj(&engine, P1, karn_the_great_creator());
    let loyalty_before = counters_on(&engine, karn, CounterKind::Loyalty);

    tap_all_mana(&mut engine, P0);
    lk::cast_at(&mut engine, P0, goblin_war_strike(), &[karn], &[]);
    assert_eq!(
        counters_on(&engine, karn, CounterKind::Loyalty),
        loyalty_before - 2
    );

    let mut empty = lk::game(
        SEED,
        &[mountain(), grizzly_bears()],
        &[mons_s_goblin_raiders()],
        &[goblin_war_strike()],
        &[],
    );
    tap_all_mana(&mut empty, P0);
    lk::cast_at(&mut empty, P0, goblin_war_strike(), &[], &[P1]);
    assert_eq!(lk::life(&empty, P1), 20);
}
