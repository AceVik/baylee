//! Observability of native recovery paths, separate from policy provenance.
use crate::features4::PendingKind4;

/// Why the native policy returned a non-neural recovery answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackReason4 {
    /// The menu contains a constraint the sampler does not implement.
    UnsupportedChoice,
    /// The only candidate failed decoding or question/context validation.
    ForcedDecode,
    /// The same authorized view/menu reached the internal visit limit.
    RepeatedView,
    /// ONNX execution, sampling or decoded-action verification failed.
    InferenceOrDecode,
}

/// Diagnostic for the most recent decision, never a policy likelihood or label.
#[derive(Clone, Debug, serde::Serialize)]
pub struct FallbackDiagnostic4 {
    /// Structured recovery branch identifier.
    pub reason: FallbackReason4,
    /// Question family visible to the acting seat.
    pub pending_kind: PendingKind4,
    /// Authorized view/menu digest, when the internal guard computed one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_hash: Option<String>,
    /// Visit count for the digest, including the current decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visits: Option<u8>,
    /// Runtime failure detail; no privileged engine state is inspected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl FallbackDiagnostic4 {
    pub(super) const fn new(reason: FallbackReason4, pending_kind: PendingKind4) -> Self {
        Self {
            reason,
            pending_kind,
            view_hash: None,
            visits: None,
            detail: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features4::{CoverageMatrix4, SourceKind4, view_only_state_hash};
    use crate::netplay4::{ModelIdentity, NetPlayer4, PolicyMode4};
    use baylee_core::ids::PlayerId;
    use baylee_core::preset::AIProfile;
    use baylee_engine::choice::{ChoicePrompt, Pending};
    use std::collections::BTreeMap;

    #[test]
    fn repeated_view_reason_is_exact_and_cleared_by_the_next_decision() {
        let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
        let preset = crate::selfplay::table_for(42, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
        let mut session = baylee_gamehost::Session::new(&preset).unwrap();
        let player = PlayerId::new(0);
        session.take_over(player);
        let (pending, view) = session.view_for_with_context(player).unwrap();
        let hash = view_only_state_hash(&view, &pending);
        let mut net = NetPlayer4 {
            backend: crate::netplay4::Backend::Owned(BTreeMap::new()),
            model_dir: std::path::PathBuf::new(),
            rule_roots: None,
            encoder_version: 7,
            model_identity: ModelIdentity {
                model_family: "v4",
                model_version: 3,
                encoder_version: 7,
                global_feat_width: None,
                critic_semantics: None,
                architecture: None,
                onnx_sha256: String::new(),
                depth_graph_sha256: BTreeMap::new(),
                metadata_sha256: String::new(),
                source_checkpoint_sha256: None,
                precision: None,
            },
            d_model: 16,
            thinking_steps: 2,
            temperature: 0.7,
            policy_mode: PolicyMode4::Raw,
            rng: crate::deckgen::Rng::new(19),
            teams: vec![],
            h_states: BTreeMap::new(),
            knowledge: BTreeMap::new(),
            seen_hashes: BTreeMap::from([((0, hash), 3)]),
            last_announced_source: BTreeMap::new(),
            observed_values: BTreeMap::new(),
            coverage: CoverageMatrix4::default(),
            last_fallback: None,
        };
        let answer = net.act_detailed(&view, &pending, &[]);
        assert_eq!(answer.provenance.source, SourceKind4::House);
        assert!(!answer.provenance.has_logprob);
        let diagnostic = net.last_fallback().unwrap();
        assert_eq!(diagnostic.reason, FallbackReason4::RepeatedView);
        assert_eq!(diagnostic.visits, Some(4));
        assert_eq!(diagnostic.view_hash, Some(format!("{hash:016x}")));
        let band = Pending::ChooseCards {
            player,
            options: vec![],
            min: 0,
            max: 0,
            prompt: ChoicePrompt::Band {
                with: baylee_core::ids::ObjectId::new(0, 0),
            },
            total: None,
        };
        net.act_detailed(&view, &band, &[]);
        let diagnostic = net.last_fallback().unwrap();
        assert_eq!(diagnostic.reason, FallbackReason4::UnsupportedChoice);
        assert_eq!(diagnostic.visits, None);
        assert_eq!(diagnostic.view_hash, None);
        net.reset_game();
        assert!(net.last_fallback().is_none());
    }
}
