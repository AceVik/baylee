//! `cards/creatures/mv_6/yawgmoth_demon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Yawgmoth Demon — flying, first strike, and "At the beginning of your
/// upkeep, you may sacrifice an artifact. If you don't, tap this creature and
/// it deals 2 damage to you." The price is offered as a menu whose empty
/// answer is the refusal (CR 118.12a, CR 608.2d), and the official ruling is
/// explicit that "the sacrificing of an artifact is not mandatory": paying
/// keeps the 6/6 standing and its controller at 20, declining taps it and
/// burns the controller for 2.
#[test]
fn yawgmoth_demon_sacrifices_an_artifact_or_taps_and_burns_its_controller() {
    let p0 = PlayerId::new(0);
    for pay in [true, false] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[yawgmoth_demon(), darksteel_pendant()])
            .start();
        keep_mulligans(&mut engine);
        let demon = on_battlefield(&engine, p0, yawgmoth_demon()).expect("seated");
        let artifact = on_battlefield(&engine, p0, darksteel_pendant()).expect("seated");
        assert!(
            keywords(&engine, demon).contains(KeywordSet::FLYING),
            "the card's reminder text is carried by the projection"
        );
        assert!(
            keywords(&engine, demon).contains(KeywordSet::FIRST_STRIKE),
            "and first strike is the other half of the line"
        );

        pass_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::CostSacrifice,
                    ..
                }
            )
        });
        assert_eq!(
            (engine.state().turn.active, engine.state().turn.step),
            (p0, Step::Upkeep),
            "\"at the beginning of *your* upkeep\""
        );
        let Pending::ChooseCards {
            player,
            options,
            min,
            max,
            ..
        } = engine.pending().clone()
        else {
            unreachable!("the pass waited for exactly this")
        };
        assert_eq!(player, p0, "the Demon's controller is the one who may pay");
        assert_eq!(
            (min, max),
            (0, 1),
            "naming nothing is how the price is declined"
        );
        assert_eq!(
            options,
            vec![artifact],
            "an artifact, and nothing else, may pay"
        );

        let answer = if pay { vec![artifact] } else { vec![] };
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: answer })
            .expect("the question's own menu answers it");
        pass_until(&mut engine, stack_is_empty);

        assert!(
            on_battlefield(&engine, p0, yawgmoth_demon()).is_some(),
            "neither arm removes the Demon"
        );
        if pay {
            assert!(
                in_graveyard(&engine, p0, darksteel_pendant()).is_some(),
                "the artifact paid for the upkeep"
            );
            assert!(!is_tapped(&engine, demon), "and the Demon keeps its feet");
            assert_eq!(engine.state().players[0].life, 20, "no damage was dealt");
        } else {
            assert!(
                on_battlefield(&engine, p0, darksteel_pendant()).is_some(),
                "declining keeps the artifact"
            );
            assert!(is_tapped(&engine, demon), "and taps the Demon");
            assert_eq!(
                engine.state().players[0].life,
                18,
                "\"it deals 2 damage to you\""
            );
        }
    }
}
