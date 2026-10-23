//! `cards/creatures/mv_4/wild_wurm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn wild_wurm() -> CardIndex {
    card_index("ba14d185-9432-4a67-8bfb-544c2cebe71d")
}

/// `Wild Wurm` ({3}{R}, 5/4 Creature — Wurm, `Coverage::Implemented`):
/// `When this creature enters, flip a coin. If you lose the flip, return this
/// creature to its owner's hand.`
#[test]
fn wild_wurm_flips_a_coin_on_etb_and_either_stays_or_returns_to_hand() {
    let mut seen = [false; 2];
    for seed in 0..32 {
        let p0 = PlayerId::new(0);
        let mut engine = Duel::new(seed, mountain())
            .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
            .hand(0, &[wild_wurm()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let journal_from = engine.state().journal.len();
        cast_from_hand(&mut engine, p0, wild_wurm());
        pass_until(&mut engine, stack_is_empty);

        let won_flip = engine.state().journal.entries()[journal_from..]
            .iter()
            .find_map(|e| match e.event {
                GameEvent::CoinFlipped { won, .. } => Some(won),
                _ => None,
            })
            .expect("Wild Wurm's ETB trigger flipped a coin");

        seen[usize::from(won_flip)] = true;
        if won_flip {
            assert!(on_battlefield(&engine, p0, wild_wurm()).is_some());
        } else {
            assert!(on_battlefield(&engine, p0, wild_wurm()).is_none());
            assert!(in_hand(&engine, p0, wild_wurm()).is_some());
        }
    }
    assert!(
        seen.into_iter().all(|result| result),
        "both printed coin branches were exercised"
    );
}
