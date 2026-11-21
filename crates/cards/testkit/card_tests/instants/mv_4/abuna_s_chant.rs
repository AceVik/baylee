//! `cards/instants/mv_4/abuna_s_chant.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "2d4721f1-1a49-45aa-99d5-ab8c6f3f3ab6"
fn abuna_s_chant() -> CardIndex {
    card_index("2d4721f1-1a49-45aa-99d5-ab8c6f3f3ab6")
}

/// Abuna's Chant ({3}{W}, Entwine {2}): Mode 0 gains 5 life; Mode 1 prevents
/// the next 5 damage to target creature; Entwine ({5}{W}) does both.
#[test]
fn abunas_chant_plays_lifegain_prevention_and_entwine() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[
            abuna_s_chant(),
            abuna_s_chant(),
            lightning_bolt(),
            lightning_bolt(),
        ],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    // 1. Mode 0: gain 5 life (20 -> 25).
    lk::float(&mut engine, P0, &[(ManaColor::White, 4)]);
    cast_with_floating(&mut engine, P0, abuna_s_chant());
    let m0 = choose_cast_kind(&engine, CastModeKind::Mode(0));
    engine.apply(P0, PlayerAction::ChooseMode(m0)).unwrap();
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P0), 25, "Mode 0 gained 5 life");

    // 2. Entwine ({5}{W}): gain 5 more life (25 -> 30) AND prevent next 5 damage
    //    to Bears (absorbs 3 from Bolt #1 + 2 of 3 from Bolt #2 -> 1 damage).
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::White, 6), (ManaColor::Red, 2)],
    );
    cast_with_floating(&mut engine, P0, abuna_s_chant());
    let both = choose_cast_kind(&engine, CastModeKind::Modes(3));
    engine.apply(P0, PlayerAction::ChooseMode(both)).unwrap();
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P0), 30, "Entwine gained 5 more life");

    lk::cast_at(&mut engine, P0, lightning_bolt(), &[bears], &[]);
    assert_eq!(
        lk::damage_on(&engine, bears),
        0,
        "3 of 5 shield absorbed Bolt #1"
    );
    lk::cast_at(&mut engine, P0, lightning_bolt(), &[bears], &[]);
    assert_eq!(
        lk::damage_on(&engine, bears),
        1,
        "remaining 2 of 5 shield absorbed part of Bolt #2; 1 damage leaked"
    );
    assert_eq!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS),
        Some(bears)
    );
}
