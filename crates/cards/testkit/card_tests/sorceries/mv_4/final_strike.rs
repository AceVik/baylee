//! `cards/sorceries/mv_4/final_strike.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn final_strike() -> CardIndex {
    card_index("4d98aea2-b4ff-4903-ba28-a53fbfaad6b1")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Final Strike (4d98aea2-b4ff-4903-ba28-a53fbfaad6b1) — {2}{B}{B} Sorcery:
/// "As an additional cost to cast this spell, sacrifice a creature. Final
/// Strike deals damage to target opponent or planeswalker equal to the
/// sacrificed creature's power."
#[test]
fn final_strike_sacrifices_a_creature_and_deals_its_power_to_opponent() {
    let mut engine = lk::game(
        SEED,
        &[
            swamp(),
            swamp(),
            swamp(),
            swamp(),
            fangren_hunter(),
            sol_ring(),
        ],
        &[grizzly_bears(), karn_the_great_creator()],
        &[final_strike()],
        &[],
    );
    let hunter = lk::obj(&engine, P0, fangren_hunter());
    let ring = lk::obj(&engine, P0, sol_ring());
    let their_bears = lk::obj(&engine, P1, grizzly_bears());
    let karn = lk::obj(&engine, P1, karn_the_great_creator());

    tap_mana_except(&mut engine, P0, ring);
    cast_with_floating(&mut engine, P0, final_strike());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!(player_options, vec![P1], "caster is not an opponent");
    assert_eq!(options, vec![karn], "planeswalker offered, creatures not");
    assert!(!options.contains(&their_bears));
    lk::aim(&mut engine, &[], &[P1]);

    let Pending::ChooseCards {
        player,
        options: sac_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, P0);
    assert_eq!(sac_options, vec![hunter]);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![hunter],
            },
        )
        .unwrap();
    assert_eq!(lk::zone_of(&engine, hunter), Zone::Graveyard);
    assert_eq!(lk::life(&engine, P1), 20, "damage waits for resolution");

    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P1), 16, "4 power -> 4 damage");
}

/// Final Strike — hits planeswalkers for the sacrificed creature's power, and
/// cannot be cast without a controlled creature to sacrifice.
#[test]
fn final_strike_damages_planeswalker_and_requires_controlled_creature() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), swamp(), swamp(), swamp(), aurochs()],
        &[karn_the_great_creator()],
        &[final_strike()],
        &[],
    );
    let beast = lk::obj(&engine, P0, aurochs());
    let karn = lk::obj(&engine, P1, karn_the_great_creator());
    let loyalty_before = counters_on(&engine, karn, CounterKind::Loyalty);

    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, final_strike(), 0, 0, &[karn], &[]);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![beast],
            },
        )
        .unwrap();
    lk::settle(&mut engine);
    assert_eq!(
        counters_on(&engine, karn, CounterKind::Loyalty),
        loyalty_before - 2
    );

    let mut no_creature = lk::game(
        SEED,
        &[swamp(), swamp(), swamp(), swamp()],
        &[grizzly_bears()],
        &[final_strike()],
        &[],
    );
    tap_all_mana(&mut no_creature, P0);
    assert!(!lk::castable(&no_creature, P0, final_strike()));
}
