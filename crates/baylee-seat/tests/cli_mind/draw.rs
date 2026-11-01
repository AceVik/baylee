//! CLI answers to draw offers take precedence over queued card decisions.

use super::*;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::YesNoPrompt;

#[tokio::test]
async fn draw_offers_interrupt_planned_yes_and_no_for_a_fresh_cli_answer() {
    let base = a_priority().await;
    for (planned, pick, accepted) in [("yes", "n", false), ("no", "y", true)] {
        let mut first = pass(1, "");
        first["answer"]["plan"] = json!([planned]);
        let mut second = pass(2, "");
        second["answer"]["pick"] = json!([pick]);
        let rig = Rig::new(
            &format!("draw-{planned}"),
            json!({"provider": "cli", "model": "agy", "command": fake()}),
            &json!({"steps": [first, second]}),
        );
        let mind = rig.mind(Limits::default());
        mind.decide(ask(&base, 1, base.view.turn, 20))
            .await
            .unwrap();
        let mut offered = ask(&base, 2, base.view.turn, 20);
        offered.pending = Pending::YesNo {
            player: PlayerId::new(0),
            prompt: YesNoPrompt::DrawOffer {
                proposer: PlayerId::new(1),
            },
            source: None,
        };
        let answer = mind.decide(offered).await.unwrap();
        assert_eq!(answer.action, PlayerAction::YesNo(accepted));
        let messages = rig.messages();
        assert_eq!(messages.len(), 2, "the CLI must choose anew");
        assert!(messages[1].1.contains("offers a draw. Accept?"));
        assert!(messages[1].1.contains("the rest of the plan is dropped"));
    }
}
