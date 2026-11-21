//! `cards/instants/mv_3/dismantling_blow.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "300cba5a-adbb-4852-be06-ac00d9d6fd37"
fn dismantling_blow() -> CardIndex {
    card_index("300cba5a-adbb-4852-be06-ac00d9d6fd37")
}

/// Dismantling Blow ({2}{W}, Kicker {2}{U}): targets only an artifact or
/// enchantment (excluding plain creatures), destroys it, and draws nothing
/// when unkicked.
#[test]
fn dismantling_blow_unkicked_destroys_artifact_without_drawing() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[sol_ring(), their_enchantment(), index::GRIZZLY_BEARS],
        &[dismantling_blow()],
        &[],
    );
    let ring = lk::obj(&engine, P1, sol_ring());
    let ench = lk::obj(&engine, P1, their_enchantment());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::White, 3), (ManaColor::Blue, 3)],
    );
    cast_with_floating(&mut engine, P0, dismantling_blow());
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::Kicker,
            ..
        }
    ));
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ring) && options.contains(&ench));
    assert!(
        !options.contains(&bears),
        "plain creature is not a legal target: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![ring],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(lk::pool_total(&engine, P0), 3, "only {{2}}{{W}} spent");
    assert!(in_graveyard(&engine, P1, sol_ring()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before - 1,
        "unkicked draws 0 cards"
    );
}

/// When kicked for {2}{U}, Dismantling Blow destroys target enchantment and
/// draws two cards.
#[test]
fn dismantling_blow_kicked_destroys_enchantment_and_draws_two() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[their_enchantment()],
        &[dismantling_blow()],
        &[],
    );
    let ench = lk::obj(&engine, P1, their_enchantment());
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::White, 3), (ManaColor::Blue, 3)],
    );
    cast_with_floating(&mut engine, P0, dismantling_blow());
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    lk::aim(&mut engine, &[ench], &[]);
    lk::settle(&mut engine);

    assert_eq!(lk::pool_total(&engine, P0), 0, "all {{4}}{{W}}{{U}} spent");
    assert!(in_graveyard(&engine, P1, their_enchantment()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before + 1,
        "cast 1 spell, drew 2 cards -> net +1"
    );
}
