//! A draw offer needs a fresh decision, even when a card's yes/no is queued.

use super::*;
use crate::narrator::tests::ME;
use baylee_engine::choice::YesNoPrompt;

#[tokio::test]
async fn draw_offers_interrupt_planned_yes_and_no_for_a_fresh_answer() {
    for (planned, pick, accepted) in [("yes", "n", false), ("no", "y", true)] {
        let (base, provider) = stand_in().await;
        provider.script(Scripted::ok(claude(&[(
            "toolu_1",
            "decide",
            json!({"ask": "q12", "pick": ["p"], "plan": [planned]}),
        )])));
        provider.script(Scripted::ok(claude(&[(
            "toolu_2",
            "decide",
            json!({"ask": "q13", "pick": [pick]}),
        )])));
        let mind = mind(&base, Provider::Anthropic, |_| {});
        let mut offered = a_priority();
        mind.decide(offered.clone()).await.unwrap();
        offered.question = 13;
        offered.pending = Pending::YesNo {
            player: ME,
            prompt: YesNoPrompt::DrawOffer { proposer: THEM },
            source: None,
        };
        offered.log = empty_log();
        let answer = mind.decide(offered).await.unwrap();
        assert_eq!(answer.action, PlayerAction::YesNo(accepted));
        assert_eq!(provider.seen().len(), 2, "the model must choose anew");
        let text = last_user(&provider.seen()[1])
            .iter()
            .filter_map(|block| block["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("offers a draw. Accept?"), "{text}");
        assert!(text.contains("accept the draw"), "{text}");
        assert!(text.contains("play on"), "{text}");
        assert!(text.contains("the rest of the plan is dropped"), "{text}");
    }
}
