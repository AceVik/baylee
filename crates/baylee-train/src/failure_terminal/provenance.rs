//! Structural provenance checks against the reconstructed authorized menu.
use crate::{
    features4::{HeadKind4, SeatKnowledge4, SourceKind4, StepProvenance4},
    features7::encode_candidates7_grouped,
};
use anyhow::ensure;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::Pending;
use baylee_gamehost::Session;

pub(super) fn observe_all(session: &Session, encoder: u32, knowledge: &mut [SeatKnowledge4]) {
    for (index, memory) in knowledge.iter_mut().enumerate() {
        let seat = PlayerId::new(u8::try_from(index).expect("validated seat roster"));
        match encoder {
            9 => memory.observe_with_announcement_context(
                &session.observe_view_with_announcement_context(seat),
            ),
            8 => memory
                .observe_with_payment_context(&session.observe_view_with_payment_context(seat)),
            _ => memory.observe(&session.observe_view_with_context(seat)),
        };
    }
}

pub(super) fn validate(
    encoder: u32,
    view: &baylee_view::PlayerView,
    pending: &Pending,
    knowledge: &SeatKnowledge4,
    teams: &[Option<u8>],
    provenance: &StepProvenance4,
) -> anyhow::Result<()> {
    let state = match encoder {
        9 => crate::features9::encode_state9(view, Some(pending), Some(knowledge), teams),
        8 => crate::features8::encode_state8(view, Some(pending), Some(knowledge), teams),
        _ => crate::features7::encode_state7(view, Some(pending), Some(knowledge), teams),
    };
    let menu = encode_candidates7_grouped(&state, view, pending);
    let forced = match menu.head {
        HeadKind4::SingleChoice => menu.candidates.len() <= 1,
        HeadKind4::SubsetPick => menu.candidates.is_empty(),
        HeadKind4::PairPick => menu.pairs.is_empty(),
        HeadKind4::ScalarValue => menu.max_choices <= menu.min_choices,
    };
    ensure!(
        (provenance.source == SourceKind4::Forced) == forced,
        "research source forcedness differs from reconstructed menu"
    );
    ensure!(
        valid_behavior(&menu, view, pending, provenance),
        "invalid research behavior provenance: {provenance:?}"
    );
    Ok(())
}

/// The research behaviour contract for an answer at `menu`.
fn valid_behavior(
    menu: &CandidateMenu4,
    view: &PlayerView,
    pending: &Pending,
    provenance: &StepProvenance4,
) -> bool {
    match provenance.source {
        SourceKind4::Forced => !provenance.has_logprob && provenance.behavior_logprob == 0.0,
        SourceKind4::Net => {
            let scalar = menu.head == HeadKind4::ScalarValue;
            let supported = crate::features4::supports_behavior_probability4(menu, view, pending);
            // Historical original sidecars may claim an LP for an unsafe menu.
            // Feature replay discards it before publishing any actor target.
            (!supported || provenance.has_logprob)
                && (!scalar || !provenance.has_logprob)
                && (provenance.has_logprob || provenance.behavior_logprob == 0.0)
                && provenance.behavior_logprob.is_finite()
                && provenance.behavior_logprob <= 0.0
                && provenance.thinking_steps == 2
                && (provenance.temperature - 0.7).abs() < 1e-6
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features4::encode_candidates7_grouped;
    use baylee_core::card::CardIndex;
    use baylee_core::ids::PlayerId;
    use baylee_engine::actions::Pending;
    use baylee_engine::rules::state::{PlayerView, SeatState, ZoneSummary};

    fn test_menu(head: HeadKind4) -> (PlayerView, Pending, CandidateMenu4) {
        let seat = PlayerId::new(0);
        let view = PlayerView {
            seat,
            active_player: seat,
            priority_holder: Some(seat),
            seats: vec![SeatState::default(); 2],
            zones: ZoneSummary::default(),
            ..PlayerView::default()
        };
        let pending = Pending::Mulligan {
            player: seat,
            taken: 0,
            next_is_free: false,
            can_take: true,
        };
        let state = crate::features7::encode_state7(&view, Some(&pending), None, &[]);
        let mut menu = encode_candidates7_grouped(&state, &view, &pending);
        menu.head = head;
        if head == HeadKind4::SingleChoice {
            assert!(menu.candidates.len() > 1);
            assert!(
                menu.candidates
                    .iter()
                    .all(|cand| matches!(cand.action, baylee_core::actions::PlayerAction::MulliganKeep | baylee_core::actions::PlayerAction::MulliganTake))
            );
        } else {
            let _ = CardIndex::new(0);
        }
        (view, pending, menu)
    }

    /// Under `injective_categorical_menu_v1`, only supported categorical
    /// menus require a behavior probability; other heads/unsafe menus may
    /// carry none (with zero logprob), while scalar values never claim one.
    #[test]
    fn only_supported_categorical_menus_require_behavior_probability() {
        let net = |has_logprob, behavior_logprob| StepProvenance4 {
            source: SourceKind4::Net,
            temperature: 0.7,
            thinking_steps: 2,
            behavior_logprob,
            has_logprob,
        };
        let (view, pending, single) = test_menu(HeadKind4::SingleChoice);
        assert!(valid_behavior(&single, &view, &pending, &net(true, -0.7)));
        assert!(!valid_behavior(&single, &view, &pending, &net(false, 0.0)));
        assert!(!valid_behavior(&single, &view, &pending, &net(false, -0.7)));
        for head in [HeadKind4::SubsetPick, HeadKind4::PairPick] {
            let (view, pending, menu) = test_menu(head);
            assert!(valid_behavior(&menu, &view, &pending, &net(true, -0.7)), "{head:?}");
            assert!(valid_behavior(&menu, &view, &pending, &net(false, 0.0)), "{head:?}");
            assert!(!valid_behavior(&menu, &view, &pending, &net(false, -0.7)), "{head:?}");
        }
        let (view, pending, scalar) = test_menu(HeadKind4::ScalarValue);
        assert!(valid_behavior(&scalar, &view, &pending, &net(false, 0.0)));
        assert!(!valid_behavior(&scalar, &view, &pending, &net(true, -0.7)));
    }
}
