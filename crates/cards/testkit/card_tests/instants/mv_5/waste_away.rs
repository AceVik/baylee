//! `cards/instants/mv_5/waste_away.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

// oracle_id = "d7f25f7f-3fa2-47ba-be33-ba0b08f26c94"
fn waste_away() -> CardIndex {
    card_index("d7f25f7f-3fa2-47ba-be33-ba0b08f26c94")
}

/// Waste Away ({4}{B}): cannot be cast with no other card in hand to discard.
#[test]
fn waste_away_cannot_be_cast_as_sole_card_in_hand() {
    let mut engine = table(
        SEED,
        &[waste_away()],
        &[swamp(), swamp(), swamp(), swamp(), swamp()],
        &[],
        &[index::SERRA_ANGEL],
    );
    tap_all_mana(&mut engine, me());
    assert!(
        !lk::castable(&engine, me(), waste_away()),
        "requires another card in hand to discard"
    );
}

/// Discards a chosen card as an additional cost and gives target creature
/// -5/-5 until end of turn (killing a 4/4, shrinking a 0/8 to -5/3 until EOT).
#[test]
fn waste_away_discards_a_card_and_gives_minus_five_minus_five_until_eot() {
    let mut kill_game = table(
        SEED,
        &[waste_away(), plains()],
        &[swamp(), swamp(), swamp(), swamp(), swamp()],
        &[],
        &[index::SERRA_ANGEL],
    );
    let angel = on_battlefield(&kill_game, them(), index::SERRA_ANGEL).unwrap();
    let discard_card = in_hand(&kill_game, me(), plains()).unwrap();

    tap_all_mana(&mut kill_game, me());
    cast_with_floating(&mut kill_game, me(), waste_away());
    for _ in 0..6 {
        match kill_game.pending().clone() {
            Pending::ChooseTargets { .. } => {
                aim_at(&mut kill_game, me(), angel);
            }
            Pending::ChooseCards { .. } => {
                kill_game
                    .apply(
                        me(),
                        PlayerAction::ChooseObjects {
                            objects: vec![discard_card],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    settle(&mut kill_game);
    assert!(in_graveyard(&kill_game, me(), plains()).is_some());
    assert!(in_graveyard(&kill_game, them(), index::SERRA_ANGEL).is_some());

    // Verify -5/-5 wears off at end of turn on a creature that survives.
    let mut eot_game = table(
        SEED,
        &[waste_away(), plains()],
        &[swamp(), swamp(), swamp(), swamp(), swamp()],
        &[],
        &[wall_of_stone()],
    );
    let wall = on_battlefield(&eot_game, them(), wall_of_stone()).unwrap();
    let disc = in_hand(&eot_game, me(), plains()).unwrap();
    tap_all_mana(&mut eot_game, me());
    cast_with_floating(&mut eot_game, me(), waste_away());
    for _ in 0..6 {
        match eot_game.pending().clone() {
            Pending::ChooseTargets { .. } => {
                aim_at(&mut eot_game, me(), wall);
            }
            Pending::ChooseCards { .. } => {
                eot_game
                    .apply(
                        me(),
                        PlayerAction::ChooseObjects {
                            objects: vec![disc],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    settle(&mut eot_game);
    assert_eq!(pt(&eot_game, wall), (-5, 3), "0/8 Wall gets -5/-5");
    reach_their_main_phase(&mut eot_game, them());
    assert_eq!(pt(&eot_game, wall), (0, 8), "-5/-5 wears off at EOT");
}
