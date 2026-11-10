//! `cards/sorceries/mv_1/mind_twist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mind Twist chooses random discards, not a discard menu (CR 701.9b).
/// Zero, a partial hand, and more than the hand all target only one player.
#[test]
fn alpha_eval_mind_twist_discards_randomly_up_to_x() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let twist = card_index("78f9c223-9982-4282-a496-a6f892f0a5bf");
    for x in [0, 2, 9] {
        let mut engine = Duel::new(1003, forest())
            .battlefield(0, &[swamp(); 10])
            .hand(0, &[twist, forest()])
            .hand(1, &[forest(), island(), mountain(), plains()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        cast_from_hand(&mut engine, p0, twist);
        engine.apply(p0, PlayerAction::ChooseNumber(x)).unwrap();
        engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
        let mine = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
        let theirs = engine.state().zones.list(ZoneLocation::Hand(p1)).clone();
        let rng = engine.state().rng.calls();
        // pass_until refuses a discard choice: the victim never chooses.
        pass_until(&mut engine, stack_is_empty);
        let remaining = engine.state().zones.list(ZoneLocation::Hand(p1));
        assert_eq!(remaining.len(), theirs.len().saturating_sub(x as usize));
        assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)), &mine);
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
            theirs.len() - remaining.len()
        );
        if x == 0 {
            assert_eq!(engine.state().rng.calls(), rng);
        } else if x == 2 {
            assert!(
                engine.state().rng.calls() > rng,
                "randomness selected the discarded cards"
            );
        }
        assert!(in_graveyard(&engine, p0, twist).is_some());
    }
}
