use super::*;
use baylee_client_core::test_support::{ViewBuilder, target_snapshot, token};
use baylee_core::ids::{DamageSourceRef, PlayerId};
use baylee_engine::choice::{Pending, TargetPrompt};

#[test]
fn historical_target_label_and_confirm_never_use_the_returned_incarnation() {
    let original = token(9, 0, "Original Bears", 2, 2);
    let mut historical = target_snapshot(&original, baylee_view::LogZone::Battlefield);
    historical.is_current = false;
    let source = historical.source;
    let mut view = ViewBuilder::new(2)
        .with_battlefield(0, vec![token(9, 0, "Returned card", 4, 4)])
        .build();
    view.target_objects = vec![historical];
    let label = target_label(
        Lang::De,
        baylee_view::TargetRef::Object(source),
        FaceNames {
            view: Some(&view),
            texts: None,
        },
        None,
    );
    assert!(label.contains("Original Bears") && label.contains("vor dem Zonenwechsel"));
    assert!(!label.contains("Returned"));
    let question = |version| Pending::ChooseTargets {
        player: PlayerId::new(0),
        options: vec![source.object],
        player_options: vec![],
        min: 0,
        max: 1,
        reason: TargetPrompt::Retarget {
            current: baylee_view::TargetRef::Object(DamageSourceRef { version, ..source }),
            index: 0,
            of: 1,
        },
    };
    let old = Interaction::new(question(1), view.seat);
    let fresh = Interaction::new(question(3), view.seat);
    let button = crate::hud::PromptButton {
        action: baylee_client_core::ledge::PromptAction::Confirm,
        decision_id: old.decision_id(),
    };
    assert!(!button.matches_decision_id(fresh.decision_id()));
    view.target_objects.clear();
    assert_eq!(
        target_label(
            Lang::De,
            baylee_view::TargetRef::Object(source),
            FaceNames {
                view: Some(&view),
                texts: None
            },
            None
        ),
        Phrase::PreviousTarget.text(Lang::De)
    );
}
