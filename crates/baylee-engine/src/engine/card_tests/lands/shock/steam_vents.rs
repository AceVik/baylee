//! `cards/lands/shock/steam_vents.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Steam Vents` enters untapped for 2 life or tapped for free under `Coverage::Implemented`.
/// In the first game, declining the payment leaves the land tapped with no life lost.
/// In the second game, paying 2 life allows it to enter untapped at the cost of 2 life.
#[test]
fn steam_vents_enters_untapped_for_two_life_or_tapped_for_free() {
    let p0 = PlayerId::new(0);

    // Arm 1: decline -> enters tapped, no life lost.
    {
        let mut engine = Duel::new(2318, forest()).hand(0, &[steam_vents()]).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let life_before = engine.state().players[0].life;
        let land = play_land(&mut engine, p0, steam_vents());

        let Pending::YesNo {
            player,
            prompt: YesNoPrompt::PayLifeOrEnterTapped { amount },
            ..
        } = engine.pending().clone()
        else {
            panic!("expected PayLifeOrEnterTapped, got {:?}", engine.pending());
        };
        assert_eq!(player, p0);
        assert_eq!(amount, 2);

        engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
        pass_until(&mut engine, stack_is_empty);

        assert!(is_tapped(&engine, land));
        assert_eq!(engine.state().players[0].life, life_before);
    }

    // Arm 2: accept -> enters untapped, costs 2 life.
    {
        let mut engine = Duel::new(2319, forest()).hand(0, &[steam_vents()]).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let life_before = engine.state().players[0].life;
        let land = play_land(&mut engine, p0, steam_vents());

        let Pending::YesNo {
            player,
            prompt: YesNoPrompt::PayLifeOrEnterTapped { amount },
            ..
        } = engine.pending().clone()
        else {
            panic!("expected PayLifeOrEnterTapped, got {:?}", engine.pending());
        };
        assert_eq!(player, p0);
        assert_eq!(amount, 2);

        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
        pass_until(&mut engine, stack_is_empty);

        assert!(!is_tapped(&engine, land));
        assert_eq!(engine.state().players[0].life, life_before - 2);
    }
}
