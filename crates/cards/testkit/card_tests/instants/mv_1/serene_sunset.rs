//! `cards/instants/mv_1/serene_sunset.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "e2ef027d-0ef7-4c8b-a594-3918290b02d9"
fn serene_sunset() -> CardIndex {
    card_index("e2ef027d-0ef7-4c8b-a594-3918290b02d9")
}

/// Serene Sunset ({X}{G}): prevents all combat damage X target creatures would
/// deal this turn (leaving untargeted attackers and noncombat damage intact),
/// and allows X = 0.
#[test]
fn serene_sunset_prevents_combat_damage_only_from_x_chosen_creatures() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[serene_sunset()])
        .battlefield(
            1,
            &[
                index::HILL_GIANT,
                index::GRIZZLY_BEARS,
                index::SAVANNAH_LIONS,
                index::PRODIGAL_SORCERER,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    reach_their_main_phase(&mut engine, P1);

    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let lions = lk::obj(&engine, P1, index::SAVANNAH_LIONS);
    let tim = lk::obj(&engine, P1, index::PRODIGAL_SORCERER);

    let _ = lk::attack(&mut engine, &[giant, lions]);
    lk::block(&mut engine, &[]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );

    // X = 2 targeting Giant and Prodigal Sorcerer; Lions is untargeted.
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, serene_sunset());
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![giant, tim],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    lk::through_combat(&mut engine);
    assert_eq!(
        lk::life(&engine, P0),
        18,
        "Giant's 3 combat damage prevented; untargeted Lions still hit for 2"
    );

    // Noncombat damage from targeted Prodigal Sorcerer is NOT prevented.
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: tim,
                ability_index: 0,
            },
        )
        .unwrap();
    lk::aim(&mut engine, &[], &[P0]);
    lk::settle(&mut engine);
    assert_eq!(
        lk::life(&engine, P0),
        17,
        "Serene Sunset prevents only combat damage, not Tim's ping"
    );
}

/// Serene Sunset can be cast for X = 0 (paying just {G}) with zero targets.
#[test]
fn serene_sunset_casts_for_x_zero() {
    let mut engine = lk::game(SEED, &[], &[], &[serene_sunset()], &[]);
    lk::float(&mut engine, P0, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut engine, P0, serene_sunset());
    if matches!(engine.pending(), Pending::ChooseNumber { .. }) {
        engine.apply(P0, PlayerAction::ChooseNumber(0)).unwrap();
    }
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P0, serene_sunset()).is_some());
}
