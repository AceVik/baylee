//! Draw offers reach the mind and both answers survive the seat's wire path.

use super::*;
use baylee_engine::choice::YesNoPrompt;

#[test]
fn draw_offers_wake_a_mind_that_is_passing_until_the_turn_ends() {
    let mut core = seated(BridgeConfig::default());
    let question = ask_mind(&mut core, 2, &colour());
    let mut decision = Answer::new(GREEN);
    decision.orders = Some(crate::wake::Orders {
        until: Some(crate::narrator::Until::EndOfTurn),
        ..crate::wake::Orders::default()
    });
    core.answered(question, Ok(decision));
    let offered = Pending::YesNo {
        player: ME,
        prompt: YesNoPrompt::DrawOffer {
            proposer: PlayerId::new(1),
        },
        source: None,
    };
    ask_mind(&mut core, 3, &offered);
}

#[test]
fn draw_offers_replace_in_flight_questions_and_accept_either_answer() {
    for accepted in [true, false] {
        let mut core = seated(BridgeConfig::default());
        let previous = ask_mind(&mut core, 2, &colour());
        let offered = Pending::YesNo {
            player: ME,
            prompt: YesNoPrompt::DrawOffer {
                proposer: PlayerId::new(1),
            },
            source: None,
        };
        core.hear(&view(3));
        let steps = core.hear(&asked(3, &offered));
        assert!(matches!(steps.first(), Some(Step::Cancel(q)) if *q == previous));
        let fresh = request(&steps).expect("a draw offer wakes the mind");
        let question = fresh.question;
        assert!(matches!(
            fresh.pending,
            Pending::YesNo {
                prompt: YesNoPrompt::DrawOffer { .. },
                ..
            }
        ));
        assert!(core.answered(previous, Ok(Answer::new(GREEN))).is_empty());
        let decision = PlayerAction::YesNo(accepted);
        let sent = core.answered(question, Ok(Answer::new(decision.clone())));
        assert_eq!(
            answer(&sent).map(|(action, by, _)| (action, by)),
            Some((&decision, By::Mind))
        );
        assert_eq!(core.stats().refused_by_referee, 0);
    }
}
