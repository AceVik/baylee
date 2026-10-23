//! `cards/creatures/mv_4/mtenda_griffin.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "273f0353-4936-491b-82da-6d4d7f3259a6"
fn mtenda_griffin() -> CardIndex {
    card_index("273f0353-4936-491b-82da-6d4d7f3259a6")
}

#[test]
fn mtenda_griffin_activates_during_your_upkeep_to_bounce_self_and_return_griffin_from_graveyard() {
    let mut engine = lk::game(
        SEED,
        &[mtenda_griffin(), mtenda_griffin(), plains()],
        &[mountain()],
        &[],
        &[index::LIGHTNING_BOLT],
    );
    let griffin_victim = lk::obj(&engine, P0, mtenda_griffin());

    // Move one Mtenda Griffin to P0's graveyard by P1 bolting it.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    tap_all_mana(&mut engine, P1);
    let bolt = in_hand(&engine, P1, index::LIGHTNING_BOLT).unwrap();
    engine
        .apply(P1, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    lk::aim(&mut engine, &[griffin_victim], &[]);
    pass_until(&mut engine, stack_is_empty);

    // Advance to P1's main phase, then to P0's upkeep.
    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });

    let griffin = lk::obj(&engine, P0, mtenda_griffin());
    let griffin_gy = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(P0))
        .iter()
        .copied()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == mtenda_griffin())
        })
        .unwrap();

    tap_all_mana(&mut engine, P0);
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: griffin,
                ability_index: 0,
            },
        )
        .unwrap();
    lk::aim(&mut engine, &[griffin_gy], &[]);
    pass_until(&mut engine, stack_is_empty);

    let hand_griffins = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(P0))
        .iter()
        .filter(|&&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == mtenda_griffin())
        })
        .count();
    assert_eq!(hand_griffins, 2);
}

/// "Activate only during your upkeep": with a Griffin card in the graveyard to
/// target and a Plains untapped, the ability is still not offered in the main
/// phase.
#[test]
fn mtenda_griffin_is_not_offered_in_the_main_phase() {
    let mut engine = lk::game(
        SEED,
        &[mtenda_griffin(), mtenda_griffin(), plains()],
        &[],
        &[],
        &[],
    );
    let victim = lk::obj(&engine, P0, mtenda_griffin());
    bury(&mut engine, &[victim]);
    assert!(in_graveyard(&engine, P0, mtenda_griffin()).is_some());
    assert!(on_battlefield(&engine, P0, mtenda_griffin()).is_some());

    lk::float(&mut engine, P0, &[(ManaColor::White, 1)]);
    assert!(
        !lk::offered(&engine, mtenda_griffin(), P0, 0),
        "not offered in the main phase, though a target and the mana are there"
    );
}
