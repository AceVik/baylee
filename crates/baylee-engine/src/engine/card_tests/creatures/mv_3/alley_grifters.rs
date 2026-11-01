//! `cards/creatures/mv_3/alley_grifters.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "399af8ea-7a6f-4f43-bd39-dc29a6e75e06"
fn alley_grifters() -> CardIndex {
    card_index("399af8ea-7a6f-4f43-bd39-dc29a6e75e06")
}

/// Alley Grifters ({1}{B}{B} 2/2 Creature — Human Mercenary):
/// "Whenever this creature becomes blocked, defending player discards a card."
#[test]
fn alley_grifters_forces_defending_player_to_discard_when_blocked() {
    let mut engine = lk::game(
        SEED,
        &[alley_grifters()],
        &[index::GIANT_TORTOISE],
        &[],
        &[index::FOREST],
    );
    let grifters = lk::obj(&engine, P0, alley_grifters());
    let tortoise = lk::obj(&engine, P1, index::GIANT_TORTOISE);
    let held = in_hand(&engine, P1, index::FOREST).expect("in hand");

    let offer = lk::attack(&mut engine, &[grifters]);
    assert!(lk::may_block(&offer, tortoise, grifters));
    lk::block(&mut engine, &[(tortoise, grifters)]);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == P1),
    );
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![held],
            },
        )
        .unwrap();
    lk::through_combat(&mut engine);

    assert!(
        in_graveyard(&engine, P1, index::FOREST).is_some(),
        "defending player discarded a card when Alley Grifters became blocked"
    );
}

#[test]
fn alley_grifters_does_not_trigger_when_unblocked() {
    let mut engine = lk::game(SEED, &[alley_grifters()], &[], &[], &[index::FOREST]);
    let grifters = lk::obj(&engine, P0, alley_grifters());

    lk::attack(&mut engine, &[grifters]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);

    assert_eq!(lk::life(&engine, P1), 18);
    assert!(
        in_hand(&engine, P1, index::FOREST).is_some(),
        "unblocked Alley Grifters does not make defending player discard"
    );
}
