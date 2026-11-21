//! `cards/instants/mv_3/puppet_s_verdict.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_engine::event::GameEvent;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "e07a7c75-b7b7-4c2d-bd7d-5f056cde871d"
fn puppet_s_verdict() -> CardIndex {
    card_index("e07a7c75-b7b7-4c2d-bd7d-5f056cde871d")
}

/// Puppet's Verdict ({1}{R}{R}): flips a coin; win destroys all creatures with
/// power <= 2 across both boards (sparing >= 3), loss destroys all creatures
/// with power >= 3 across both boards (sparing <= 2).
#[test]
fn puppets_verdict_destroys_small_on_win_and_large_on_loss() {
    let mut saw_win = false;
    let mut saw_loss = false;

    for seed in 1..40 {
        let mut engine = lk::game(
            seed,
            &[index::GRIZZLY_BEARS, index::HILL_GIANT],
            &[index::SAVANNAH_LIONS, index::SERRA_ANGEL],
            &[puppet_s_verdict()],
            &[],
        );
        lk::float(&mut engine, P0, &[(ManaColor::Red, 3)]);
        cast_with_floating(&mut engine, P0, puppet_s_verdict());
        lk::settle(&mut engine);

        let won = engine
            .journal()
            .entries()
            .iter()
            .find_map(|entry| match entry.event {
                GameEvent::CoinFlipped { won, .. } => Some(won),
                _ => None,
            })
            .expect("Puppet's Verdict flipped a coin");

        if won {
            saw_win = true;
            assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some());
            assert!(in_graveyard(&engine, P1, index::SAVANNAH_LIONS).is_some());
            assert!(on_battlefield(&engine, P0, index::HILL_GIANT).is_some());
            assert!(on_battlefield(&engine, P1, index::SERRA_ANGEL).is_some());
        } else {
            saw_loss = true;
            assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
            assert!(on_battlefield(&engine, P1, index::SAVANNAH_LIONS).is_some());
            assert!(in_graveyard(&engine, P0, index::HILL_GIANT).is_some());
            assert!(in_graveyard(&engine, P1, index::SERRA_ANGEL).is_some());
        }

        if saw_win && saw_loss {
            break;
        }
    }

    assert!(
        saw_win && saw_loss,
        "both coin-flip branches were exercised"
    );
}
