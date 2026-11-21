//! `cards/instants/mv_1/explosive_growth.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "96c65faf-859c-4436-a551-5538409a9792"
fn explosive_growth() -> CardIndex {
    card_index("96c65faf-859c-4436-a551-5538409a9792")
}

/// Explosive Growth ({G}, Kicker {5}): unkicked gives target creature +2/+2
/// until end of turn, leaves other creatures alone, and wears off next turn.
#[test]
fn explosive_growth_unkicked_gives_plus_two_plus_two_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::SAVANNAH_LIONS],
        &[],
        &[explosive_growth()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);

    lk::float(&mut engine, P0, &[(ManaColor::Green, 6)]);
    cast_with_floating(&mut engine, P0, explosive_growth());
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::Kicker,
            ..
        }
    ));
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);

    assert_eq!(lk::pool_total(&engine, P0), 5, "only {{G}} was spent");
    assert_eq!(pt(&engine, bears), (4, 4), "+2/+2 until end of turn");
    assert_eq!(pt(&engine, lions), (2, 1), "bystander untouched");

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, bears), (2, 2), "bonus wears off at cleanup");
}

/// When kicked for {5}, Explosive Growth gives +5/+5 instead of +2/+2 until
/// end of turn, spending all six mana.
#[test]
fn explosive_growth_kicked_gives_plus_five_plus_five_instead() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[explosive_growth()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Green, 6)]);
    cast_with_floating(&mut engine, P0, explosive_growth());
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::Kicker,
            ..
        }
    ));
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);

    assert_eq!(lk::pool_total(&engine, P0), 0, "all {{5}}{{G}} was spent");
    assert_eq!(
        pt(&engine, bears),
        (7, 7),
        "kicked gives +5/+5 instead of +2/+2"
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, bears), (2, 2), "kicked bonus wears off at EOT");
}
