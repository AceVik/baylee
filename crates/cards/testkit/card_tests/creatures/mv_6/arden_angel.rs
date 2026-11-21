//! `cards/creatures/mv_6/arden_angel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn arden_angel() -> CardIndex {
    card_index("82d51c8a-d42b-48b3-a726-3a8d28d057e5")
}

/// Arden Angel (82d51c8a-d42b-48b3-a726-3a8d28d057e5) — `{4}{W}{W}` 4/4 Creature — Angel:
/// - Flying
/// - At the beginning of your upkeep, if Arden Angel is in your graveyard, roll a
///   four-sided die. If the result is 1, return Arden Angel from your graveyard to
///   the battlefield.
#[test]
fn arden_angel_casts_as_four_four_flyer_and_returns_from_graveyard_only_on_d4_roll_of_one() {
    let lands = vec![plains(); 6];
    let mut cast_game = Duel::new(SEED, plains())
        .battlefield(0, &lands)
        .hand(0, &[arden_angel()])
        .start();
    keep_mulligans(&mut cast_game);
    reach_main_phase(&mut cast_game, P0);
    cast_from_hand(&mut cast_game, P0, arden_angel());
    pass_until(&mut cast_game, stack_is_empty);
    let angel = on_battlefield(&cast_game, P0, arden_angel()).expect("Arden Angel entered");
    assert_eq!(pt(&cast_game, angel), (4, 4));
    assert!(keywords(&cast_game, angel).contains(KeywordSet::FLYING));

    // Deterministic sweep across seeds to verify BOTH non-1 (stays in graveyard) and 1 (returns).
    let mut saw_miss = false;
    let mut saw_hit = false;
    for seed in 1_u64..40 {
        let mut engine = Duel::new(seed, plains()).hand(0, &[arden_angel()]).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, P0);
        let _ = hand_to_graveyard(&mut engine, P0, arden_angel());

        // Opponent's upkeep does not trigger Arden Angel.
        reach_their_main_phase(&mut engine, P1);
        assert!(in_graveyard(&engine, P0, arden_angel()).is_some());

        // P0's upkeep rolls 1d4.
        let before_len = engine.state().journal.entries().len();
        reach_their_main_phase(&mut engine, P0);
        let roll = engine.state().journal.entries()[before_len..]
            .iter()
            .find_map(|entry| match entry.event {
                GameEvent::DiceRolled {
                    player,
                    sides: 4,
                    result,
                } if player == P0 => Some(result),
                _ => None,
            })
            .expect("Arden Angel rolled a 4-sided die on P0's upkeep");

        if roll == 1 {
            assert!(on_battlefield(&engine, P0, arden_angel()).is_some());
            saw_hit = true;
        } else {
            assert!(in_graveyard(&engine, P0, arden_angel()).is_some());
            saw_miss = true;
        }
        if saw_hit && saw_miss {
            break;
        }
    }
    assert!(saw_hit && saw_miss, "verified both roll == 1 and roll != 1");
}
