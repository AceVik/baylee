//! `cards/instants/mv_2/devour_in_shadow.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "9086d8ff-e375-4149-996d-2a88a7596c1c"
fn devour_in_shadow() -> CardIndex {
    card_index("9086d8ff-e375-4149-996d-2a88a7596c1c")
}

/// Devour in Shadow ({B}{B}): destroys target creature (including black
/// creatures and through an active regeneration shield) and makes you lose
/// life equal to its toughness.
#[test]
fn devour_in_shadow_ignores_regeneration_and_loses_targets_toughness_life() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[devour_in_shadow()])
        .battlefield(1, &[forest(), index::RIVER_BOA])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let boa = lk::obj(&engine, P1, index::RIVER_BOA);
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, devour_in_shadow());
    lk::aim(&mut engine, &[boa], &[]);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    // P1 activates River Boa's regeneration shield in response.
    tap_all_mana(&mut engine, P1);
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: boa,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(boa)
            .is_some_and(|o| o.regeneration_shields == 1)
    });

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, P1, index::RIVER_BOA).is_some(),
        "River Boa (2/1) destroyed through regeneration"
    );
    assert_eq!(
        lk::life(&engine, P0),
        19,
        "lost life equal to River Boa's toughness (1), not its power (2)"
    );
}
