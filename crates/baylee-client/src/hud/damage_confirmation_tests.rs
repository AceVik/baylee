use super::*;
use baylee_engine::choice::DamageChoiceId;
#[test]
fn stale_confirmation_cannot_answer_a_new_damage_question() {
    let old =
        baylee_client_core::interaction::DecisionId::Damage(DamageChoiceId { batch: 9, step: 1 });
    let new =
        baylee_client_core::interaction::DecisionId::Damage(DamageChoiceId { batch: 9, step: 2 });
    let button = PromptButton {
        action: PromptAction::Confirm,
        decision_id: Some(old),
    };
    assert!(button.matches_decision_id(Some(old)));
    assert!(!button.matches_decision_id(Some(new)));
    assert!(!button.matches_decision_id(None));
    let ordinary = PromptButton {
        action: PromptAction::Confirm,
        decision_id: None,
    };
    assert!(!ordinary.matches_decision_id(Some(new)));
    let source = baylee_client_core::interaction::DecisionId::Source(
        baylee_core::ids::SourceChoiceId::new(9),
    );
    assert!(!button.matches_decision_id(Some(source)));
    let source_button = PromptButton {
        action: PromptAction::Confirm,
        decision_id: Some(source),
    };
    assert!(source_button.matches_decision_id(Some(source)));
    assert!(!source_button.matches_decision_id(Some(new)));
    assert!(!source_button.matches_decision_id(Some(
        baylee_client_core::interaction::DecisionId::Source(baylee_core::ids::SourceChoiceId::new(
            10
        ))
    )));
}
