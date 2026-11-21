//! `cards/instants/mv_4/shriveling_rot.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "9ce8691a-d9e2-45b2-bda0-2c3891b55307"
fn shriveling_rot() -> CardIndex {
    card_index("9ce8691a-d9e2-45b2-bda0-2c3891b55307")
}

/// Mode 0 alone: until EOT, whenever a creature is dealt damage, destroy it
/// (without life loss on death), and expires next turn.
#[test]
fn shriveling_rot_mode_one_destroys_damaged_creature_without_life_loss() {
    let mut engine = lk::game(
        SEED,
        &[index::PRODIGAL_SORCERER],
        &[index::SERRA_ANGEL],
        &[shriveling_rot()],
        &[],
    );
    let tim = lk::obj(&engine, P0, index::PRODIGAL_SORCERER);
    let angel = lk::obj(&engine, P1, index::SERRA_ANGEL);

    lk::float(&mut engine, P0, &[(ManaColor::Black, 4)]);
    cast_with_floating(&mut engine, P0, shriveling_rot());
    let m0 = choose_cast_kind(&engine, CastModeKind::Mode(0));
    engine.apply(P0, PlayerAction::ChooseMode(m0)).unwrap();
    lk::settle(&mut engine);

    // 1 damage from Tim destroys the 4/4 Angel; Mode 1 was not chosen -> 0 life loss.
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: tim,
                ability_index: 0,
            },
        )
        .unwrap();
    lk::aim(&mut engine, &[angel], &[]);
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P1, index::SERRA_ANGEL).is_some());
    assert_eq!(lk::life(&engine, P1), 20, "Mode 0 does not cause life loss");
}

/// Entwine ({4}{B}{B}{B}): dealing 1 damage to a 4/4 Serra Angel destroys it
/// (Mode 0) AND makes its controller lose 4 life equal to its toughness
/// (Mode 1); both triggers expire at end of turn.
#[test]
fn shriveling_rot_entwined_destroys_and_drains_toughness_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[index::PRODIGAL_SORCERER],
        &[index::SERRA_ANGEL, index::WALL_OF_STONE],
        &[shriveling_rot()],
        &[],
    );
    let tim = lk::obj(&engine, P0, index::PRODIGAL_SORCERER);
    let angel = lk::obj(&engine, P1, index::SERRA_ANGEL);
    let wall = lk::obj(&engine, P1, index::WALL_OF_STONE);

    lk::float(&mut engine, P0, &[(ManaColor::Black, 7)]);
    cast_with_floating(&mut engine, P0, shriveling_rot());
    let both = choose_cast_kind(&engine, CastModeKind::Modes(3));
    engine.apply(P0, PlayerAction::ChooseMode(both)).unwrap();
    lk::settle(&mut engine);
    assert_eq!(
        lk::pool_total(&engine, P0),
        0,
        "all {{4}}{{B}}{{B}}{{B}} spent"
    );

    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: tim,
                ability_index: 0,
            },
        )
        .unwrap();
    lk::aim(&mut engine, &[angel], &[]);
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P1, index::SERRA_ANGEL).is_some());
    assert_eq!(
        lk::life(&engine, P1),
        16,
        "controller lost Angel's toughness (4)"
    );

    // Next turn both delayed triggers are gone: 1 damage to Wall of Stone does not destroy it.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: tim,
                ability_index: 0,
            },
        )
        .unwrap();
    lk::aim(&mut engine, &[wall], &[]);
    lk::settle(&mut engine);
    assert_eq!(
        on_battlefield(&engine, P1, index::WALL_OF_STONE),
        Some(wall)
    );
}
