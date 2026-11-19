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
        valid_behavior(menu.head, provenance),
        "invalid research behavior provenance: {provenance:?}"
    );
    Ok(())
}

/// The research behaviour contract for an answer at a question of `head`.
fn valid_behavior(head: HeadKind4, provenance: &StepProvenance4) -> bool {
    match provenance.source {
        SourceKind4::Forced => !provenance.has_logprob && provenance.behavior_logprob == 0.0,
        SourceKind4::Net => {
            let scalar = head == HeadKind4::ScalarValue;
            // A blocker draw the attackers' counts changed carries no
            // probability (`sample_pairs_constrained`); every other choice
            // carries its own.
            let undrawn = head == HeadKind4::PairPick && !provenance.has_logprob;
            (provenance.has_logprob != scalar || undrawn)
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

    /// Only a blocker draw the attackers' counts changed may come without a
    /// behaviour probability, and then with a zero logprob.
    #[test]
    fn only_a_pair_answer_may_lack_its_behavior_probability() {
        let net = |has_logprob, behavior_logprob| StepProvenance4 {
            source: SourceKind4::Net,
            temperature: 0.7,
            thinking_steps: 2,
            behavior_logprob,
            has_logprob,
        };
        for head in [
            HeadKind4::SingleChoice,
            HeadKind4::SubsetPick,
            HeadKind4::PairPick,
        ] {
            assert!(valid_behavior(head, &net(true, -0.7)), "{head:?}");
            assert_eq!(
                valid_behavior(head, &net(false, 0.0)),
                head == HeadKind4::PairPick,
                "{head:?}"
            );
            assert!(!valid_behavior(head, &net(false, -0.7)), "{head:?}");
        }
        assert!(valid_behavior(HeadKind4::ScalarValue, &net(false, 0.0)));
        assert!(!valid_behavior(HeadKind4::ScalarValue, &net(true, -0.7)));
    }
}
