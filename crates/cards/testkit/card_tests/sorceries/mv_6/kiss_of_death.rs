//! `cards/sorceries/mv_6/kiss_of_death.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn kiss_of_death() -> CardIndex {
    card_index("c2c25406-e84a-47b0-8a46-fe60f9b0cffa")
}

/// Kiss of Death (c2c25406-e84a-47b0-8a46-fe60f9b0cffa) — {4}{B}{B} Sorcery:
/// "Kiss of Death deals 4 damage to target opponent or planeswalker. You gain
/// 4 life."
#[test]
fn kiss_of_death_drains_target_opponent_for_four_and_excludes_self_and_creatures() {
    let mut engine = lk::game(
        SEED,
        &[
            swamp(),
            swamp(),
            swamp(),
            swamp(),
            swamp(),
            swamp(),
            aurochs(),
        ],
        &[aurochs(), karn_the_great_creator()],
        &[kiss_of_death()],
        &[],
    );
    let my_beast = lk::obj(&engine, P0, aurochs());
    let their_beast = lk::obj(&engine, P1, aurochs());
    let karn = lk::obj(&engine, P1, karn_the_great_creator());

    cast_from_hand(&mut engine, P0, kiss_of_death());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!(player_options, vec![P1], "only opponents may be targeted");
    assert_eq!(options, vec![karn], "planeswalker offered, creatures not");
    assert!(!options.contains(&my_beast) && !options.contains(&their_beast));

    lk::aim(&mut engine, &[], &[P1]);
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P1), 16);
    assert_eq!(lk::life(&engine, P0), 24);
}

/// Kiss of Death — targets a planeswalker for 4 loyalty damage while gaining 4
/// life.
#[test]
fn kiss_of_death_damages_planeswalker_and_gains_four_life() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()],
        &[karn_the_great_creator()],
        &[kiss_of_death()],
        &[],
    );
    let karn = lk::obj(&engine, P1, karn_the_great_creator());
    let loyalty_before = counters_on(&engine, karn, CounterKind::Loyalty);

    tap_all_mana(&mut engine, P0);
    lk::cast_at(&mut engine, P0, kiss_of_death(), &[karn], &[]);

    assert_eq!(
        counters_on(&engine, karn, CounterKind::Loyalty),
        loyalty_before - 4
    );
    assert_eq!(lk::life(&engine, P0), 24);
    assert_eq!(lk::life(&engine, P1), 20);
}
