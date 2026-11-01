//! Live-only, explicitly heuristic draw-offer decisions over neural side utility.
use crate::features4::{SourceKind4, side_utility};
use baylee_engine::choice::{Pending, YesNoPrompt};
use baylee_view::PlayerView;
use serde::Serialize;

const MARGIN: f32 = 0.10;

/// Audit record for a rule decision, distinct from a learned policy answer.
#[derive(Clone, Debug, Serialize)]
pub struct DrawDecision4 {
    /// Versioned live rule; frozen evaluation/training policies are unchanged.
    pub rule: &'static str,
    /// Whether a successful, finite neural decision value was available.
    pub evaluation_valid: bool,
    /// Expected continuation side utility, not a calibrated pure win probability.
    pub continuation_utility: Option<f32>,
    /// Agreed-draw terminal utility under the existing surviving-side target.
    pub draw_utility: f32,
    /// Decision distance; explicitly not a statistical confidence interval.
    pub margin: f32,
    /// The rule's yes/no answer.
    pub accepted: bool,
    /// Actual origin of the rule override, not learned Net or Search.
    pub decision_source: SourceKind4,
    /// Interpretation and calibration limitation.
    pub note: &'static str,
}

pub(super) fn decide(
    pending: &Pending,
    view: &PlayerView,
    teams: &[Option<u8>],
    source: SourceKind4,
    value: f32,
) -> Option<DrawDecision4> {
    if !matches!(
        pending,
        Pending::YesNo {
            prompt: YesNoPrompt::DrawOffer { .. },
            ..
        }
    ) {
        return None;
    }
    let alive: Vec<_> = view
        .seats
        .iter()
        .filter(|seat| !seat.has_lost())
        .map(|seat| seat.player.get())
        .collect();
    let draw_utility = side_utility(view.seat.get(), teams, &alive, &[]);
    let valid = source == SourceKind4::Net && value.is_finite() && (0.0..=1.0).contains(&value);
    Some(DrawDecision4 {
        rule: "critic_draw_rule_v1",
        evaluation_valid: valid,
        continuation_utility: valid.then_some(value),
        draw_utility,
        margin: MARGIN,
        accepted: valid && value < draw_utility - MARGIN,
        decision_source: SourceKind4::House,
        note: "Live rule over neural expected side utility; margin is a heuristic decision distance, not statistical confidence or proof of calibrated win probability. Ties/unavailable inference decline.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::ViewBuilder;
    use baylee_core::ids::PlayerId;
    fn offer() -> Pending {
        Pending::YesNo {
            player: PlayerId::new(0),
            prompt: YesNoPrompt::DrawOffer {
                proposer: PlayerId::new(1),
            },
            source: None,
        }
    }
    #[test]
    fn draw_accepts_bad_continuation_declines_good_tie_and_invalid_values() {
        let view = ViewBuilder::new(2).build();
        for (value, accepted) in [(0.1, true), (0.8, false), (0.4, false)] {
            let d = decide(&offer(), &view, &[None, None], SourceKind4::Net, value).unwrap();
            assert_eq!(d.accepted, accepted);
            assert_eq!(d.draw_utility, 0.5);
            assert!(d.evaluation_valid);
            assert_eq!(d.decision_source, SourceKind4::House);
        }
        for value in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
            let d = decide(&offer(), &view, &[None, None], SourceKind4::Net, value).unwrap();
            assert!(!d.accepted);
            assert!(!d.evaluation_valid);
            assert_eq!(d.continuation_utility, None);
            assert!(serde_json::to_string(&d).is_ok());
        }
        let d = decide(&offer(), &view, &[None, None], SourceKind4::House, 0.0).unwrap();
        assert!(!d.accepted);
        assert!(!d.evaluation_valid);
    }
    #[test]
    fn draw_utility_counts_surviving_sides_not_players() {
        let view = ViewBuilder::new(4).build();
        assert_eq!(
            decide(&offer(), &view, &[None; 4], SourceKind4::Net, 0.0)
                .unwrap()
                .draw_utility,
            0.25
        );
        assert_eq!(
            decide(
                &offer(),
                &view,
                &[Some(1), Some(1), Some(1), Some(2)],
                SourceKind4::Net,
                0.0
            )
            .unwrap()
            .draw_utility,
            0.5
        );
    }
    #[test]
    fn unrelated_yes_no_prompt_is_not_overridden() {
        let pending = Pending::YesNo {
            player: PlayerId::new(0),
            prompt: YesNoPrompt::MayDo,
            source: None,
        };
        assert!(
            decide(
                &pending,
                &ViewBuilder::new(2).build(),
                &[None, None],
                SourceKind4::Net,
                0.0
            )
            .is_none()
        );
    }
}
