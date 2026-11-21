//! `cards/instants/mv_1/orim_s_touch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "cc22c210-efd0-494c-8560-448b038b3c5f"
fn orim_s_touch() -> CardIndex {
    card_index("cc22c210-efd0-494c-8560-448b038b3c5f")
}

/// Orim's Touch ({W}, Kicker {1}): unkicked prevents the next 2 damage to
/// any target this turn, letting excess damage through and exhausting the shield.
#[test]
fn orims_touch_unkicked_prevents_next_two_damage_to_a_player() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[orim_s_touch(), lightning_bolt(), lightning_bolt()],
        &[],
    );

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::White, 2), (ManaColor::Red, 2)],
    );
    cast_with_floating(&mut engine, P0, orim_s_touch());
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::Kicker,
            ..
        }
    ));
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    lk::aim(&mut engine, &[], &[P0]);
    lk::settle(&mut engine);

    lk::cast_at(&mut engine, P0, lightning_bolt(), &[], &[P0]);
    assert_eq!(lk::life(&engine, P0), 19, "2 of 3 damage prevented");

    lk::cast_at(&mut engine, P0, lightning_bolt(), &[], &[P0]);
    assert_eq!(lk::life(&engine, P0), 16, "shield exhausted; full 3 hits");
}

/// When kicked for {1}, Orim's Touch prevents the next 4 damage to a creature
/// across multiple hits this turn, and expires at end of turn.
#[test]
fn orims_touch_kicked_prevents_next_four_damage_to_a_creature() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[orim_s_touch(), lightning_bolt(), shock()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::White, 2), (ManaColor::Red, 2)],
    );
    cast_with_floating(&mut engine, P0, orim_s_touch());
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);

    lk::cast_at(&mut engine, P0, lightning_bolt(), &[bears], &[]);
    assert_eq!(
        lk::damage_on(&engine, bears),
        0,
        "3 of 4 shield absorbs Bolt"
    );

    lk::cast_at(&mut engine, P0, shock(), &[bears], &[]);
    assert_eq!(
        lk::damage_on(&engine, bears),
        1,
        "final 1 of 4 shield absorbs half of Shock; Bears survives with 1 damage"
    );
    assert_eq!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS),
        Some(bears)
    );
}

/// Unused prevention from Orim's Touch wears off at end of turn.
#[test]
fn orims_touch_shield_expires_at_end_of_turn() {
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[orim_s_touch(), lightning_bolt()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::White, 1)]);
    lk::cast_at(&mut engine, P0, orim_s_touch(), &[bears], &[]);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, P0);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P0, lightning_bolt(), &[bears], &[]);

    assert!(
        in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "next turn the prevention shield has worn off"
    );
}
