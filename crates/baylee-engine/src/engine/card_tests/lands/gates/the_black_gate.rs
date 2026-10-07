//! `cards/lands/gates/the_black_gate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Black Gate is `Coverage::Partial`: the third ability is not
/// implemented.  The two implemented halves are the shockland-style entry —
/// pay 3 life or enter tapped — and `{T}: Add {B}`.
///
/// Two games prove the two arms of the entry question.  In the first, the
/// controller declines the payment: the Gate enters tapped and no life is
/// lost.  In the second, the controller accepts: the Gate enters untapped and
/// life falls by exactly 3.  Two games because one land drop per turn is one
/// arm per game.
#[test]
fn the_black_gate_enters_untapped_for_three_life_or_tapped_for_free() {
    let p0 = PlayerId::new(0);

    // ── Arm 1: decline → enters tapped, no life lost.
    {
        let mut engine = Duel::new(43, forest()).hand(0, &[the_black_gate()]).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let life_before = engine.state().players[0].life;
        let land = in_hand(&engine, p0, the_black_gate()).expect("the Gate is in hand");
        engine
            .apply(p0, PlayerAction::PlayLand { card: land })
            .unwrap();

        let Pending::YesNo {
            player,
            prompt: crate::choice::YesNoPrompt::PayLifeOrEnterTapped { amount },
            ..
        } = engine.pending().clone()
        else {
            panic!("expected PayLifeOrEnterTapped, got {:?}", engine.pending())
        };
        assert_eq!(player, p0, "the controller decides");
        assert_eq!(amount, 3, "the Gate asks for 3 life");

        engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
        pass_until(&mut engine, stack_is_empty);

        let gate =
            on_battlefield(&engine, p0, the_black_gate()).expect("the Gate is on the battlefield");
        assert!(
            is_tapped(&engine, gate),
            "declining the payment leaves the Gate tapped"
        );
        assert_eq!(
            engine.state().players[0].life,
            life_before,
            "declining costs no life"
        );
    }

    // ── Arm 2: accept → enters untapped, costs 3 life.
    {
        let mut engine = Duel::new(44, forest()).hand(0, &[the_black_gate()]).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let life_before = engine.state().players[0].life;
        let land = in_hand(&engine, p0, the_black_gate()).expect("the Gate is in hand");
        engine
            .apply(p0, PlayerAction::PlayLand { card: land })
            .unwrap();

        let Pending::YesNo {
            player,
            prompt: crate::choice::YesNoPrompt::PayLifeOrEnterTapped { amount },
            ..
        } = engine.pending().clone()
        else {
            panic!("expected PayLifeOrEnterTapped, got {:?}", engine.pending())
        };
        assert_eq!(player, p0);
        assert_eq!(amount, 3);

        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
        pass_until(&mut engine, stack_is_empty);

        let gate =
            on_battlefield(&engine, p0, the_black_gate()).expect("the Gate is on the battlefield");
        assert!(
            !is_tapped(&engine, gate),
            "paying 3 life leaves the Gate untapped"
        );
        assert_eq!(
            engine.state().players[0].life,
            life_before - 3,
            "the payment costs exactly 3 life"
        );
    }
}
