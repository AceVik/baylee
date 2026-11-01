#[allow(clippy::wildcard_imports)]
use super::*;

fn scoria_wurm() -> CardIndex {
    card_index("ee45c80c-699c-4e37-97e6-1859c6601c99")
}

#[test]
fn scoria_wurm_returns_exactly_when_its_own_upkeep_flip_is_lost() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut won_seen, mut lost_seen) = (false, false);
    for seed in 1..=16 {
        let mut e = Duel::new(seed, forest())
            .battlefield(0, &[scoria_wurm()])
            .start();
        keep_mulligans(&mut e);
        reach_their_main_phase(&mut e, p0);
        let flips: Vec<_> = e
            .state()
            .journal
            .entries()
            .iter()
            .filter_map(|entry| match entry.event {
                crate::event::GameEvent::CoinFlipped { player, won } => Some((player, won)),
                _ => None,
            })
            .collect();
        assert_eq!(flips.len(), 1);
        assert_eq!(flips[0].0, p0);
        let won = flips[0].1;
        assert_eq!(on_battlefield(&e, p0, scoria_wurm()).is_some(), won);
        assert_eq!(in_hand(&e, p0, scoria_wurm()).is_some(), !won);
        won_seen |= won;
        lost_seen |= !won;
        reach_their_main_phase(&mut e, p1);
        let count = e
            .state()
            .journal
            .entries()
            .iter()
            .filter(|entry| matches!(entry.event, crate::event::GameEvent::CoinFlipped { .. }))
            .count();
        assert_eq!(count, 1, "no flip in the opponent's upkeep");
    }
    assert!(won_seen && lost_seen);
}
