//! `cards/sorceries/mv_3/vampiric_touch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn vampiric_touch() -> CardIndex {
    card_index("b701378f-681b-4840-bd11-1a97c2a675f2")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Vampiric Touch (b701378f-681b-4840-bd11-1a97c2a675f2) — {2}{B} Sorcery:
/// "Vampiric Touch deals 2 damage to target opponent or planeswalker and you
/// gain 2 life."
#[test]
fn vampiric_touch_drains_target_opponent_and_excludes_self_and_creatures() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), swamp(), swamp(), grizzly_bears()],
        &[grizzly_bears(), karn_the_great_creator()],
        &[vampiric_touch()],
        &[],
    );
    let my_bears = lk::obj(&engine, P0, grizzly_bears());
    let their_bears = lk::obj(&engine, P1, grizzly_bears());
    let karn = lk::obj(&engine, P1, karn_the_great_creator());

    cast_from_hand(&mut engine, P0, vampiric_touch());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!(player_options, vec![P1], "only opponents, not caster");
    assert_eq!(options, vec![karn], "planeswalker offered, creatures not");
    assert!(!options.contains(&my_bears) && !options.contains(&their_bears));

    lk::aim(&mut engine, &[], &[P1]);
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P1), 18);
    assert_eq!(lk::life(&engine, P0), 22);
}

/// Vampiric Touch — can target a planeswalker, dealing 2 loyalty damage while
/// the caster gains 2 life.
#[test]
fn vampiric_touch_damages_planeswalker_and_gains_two_life() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), swamp(), swamp()],
        &[karn_the_great_creator()],
        &[vampiric_touch()],
        &[],
    );
    let karn = lk::obj(&engine, P1, karn_the_great_creator());
    let loyalty_before = counters_on(&engine, karn, CounterKind::Loyalty);

    tap_all_mana(&mut engine, P0);
    lk::cast_at(&mut engine, P0, vampiric_touch(), &[karn], &[]);

    assert_eq!(
        counters_on(&engine, karn, CounterKind::Loyalty),
        loyalty_before - 2
    );
    assert_eq!(lk::life(&engine, P0), 22);
    assert_eq!(lk::life(&engine, P1), 20);
}
