//! Separate feature/return framing for verified research episodes; no MTG labels.
use super::{RolloutStepSidecar4, Step4};
use crate::failure_terminal::{ResearchReturn, VerifiedEpisode, sha};
use serde::{Serialize, Serializer, ser::Error as _};

/// Structural framing/preprocessing identity. Old BST4 caches cannot consume it.
pub const FRAME_CONTRACT: &str = "failure_terminal_research_observations_v1";

/// Existing feature tensors and unchanged executed-action provenance without
/// legacy outcome, terminal markers or final-state auxiliary labels.
#[derive(Debug)]
pub struct ResearchFeatures4(Step4);

impl Serialize for ResearchFeatures4 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut value = serde_json::to_value(&self.0).map_err(S::Error::custom)?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| S::Error::custom("feature object"))?;
        for label in ["outcome", "aux", "is_terminal"] {
            object.remove(label);
        }
        value.serialize(serializer)
    }
}

/// A learner history frame in the separate research format.
#[derive(Debug, Serialize)]
pub struct ResearchFrame4 {
    /// Explicit row format, distinct from ordinary Step4/BST4 data.
    pub schema: &'static str,
    /// Native feature tensors, action target and original provenance.
    pub features: ResearchFeatures4,
    /// Separately named research return; never substituted for MTG outcome.
    pub returns: ResearchReturn,
    /// The learner history ended under the declared research MDP.
    pub training_terminal: bool,
    /// Only originally stochastic Net decisions are potential actor samples.
    pub actor_eligible: bool,
}

/// Research conversion counts; unknown attempts explicitly emit no rows.
#[derive(Debug, Serialize)]
pub struct StreamedResearch4 {
    /// Learner observations and decisions emitted in original order.
    pub frames: u64,
    /// Original Net decisions carrying an actor likelihood.
    pub actor_decisions: u64,
    /// Unknown outcomes remain in the attempt ledger, without training targets.
    pub excluded_unknown: bool,
}

fn row(step: Step4, proof: &VerifiedEpisode, terminal: bool) -> ResearchFrame4 {
    let actor = step.is_decision
        && step.provenance.source == crate::features4::SourceKind4::Net
        && step.provenance.has_logprob;
    ResearchFrame4 {
        schema: FRAME_CONTRACT,
        features: ResearchFeatures4(step),
        returns: proof.returns.clone(),
        training_terminal: terminal,
        actor_eligible: actor,
    }
}

/// Stream a separately verified learner history without ordinary terminal targets.
///
/// # Errors
/// Refuses altered proof inputs, unencoded decisions, changed replay boundaries,
/// unsupported end semantics or emission errors. Callers publish only on success.
pub fn stream_research4(
    record: &[u8],
    game_id: u32,
    sidecar: &[RolloutStepSidecar4],
    proof: &VerifiedEpisode,
    mut emit: impl FnMut(ResearchFrame4) -> Result<(), String>,
) -> Result<StreamedResearch4, String> {
    if proof.record_sha256 != sha(record)
        || proof.sidecar_sha256
            != sha(&serde_json::to_vec(sidecar).map_err(|error| error.to_string())?)
    {
        return Err("research proof inputs changed before feature replay".into());
    }
    if proof.returns.training_utility.is_none() {
        return Ok(StreamedResearch4 {
            frames: 0,
            actor_decisions: 0,
            excluded_unknown: true,
        });
    }
    let mut previous = None;
    let mut frames = 0;
    let mut actor_decisions = 0;
    let replay = super::replay::replay_research_frames(
        record,
        game_id,
        sidecar,
        proof.encoder,
        proof.natural,
        |step, _| {
            if step.seat != proof.learner {
                return Ok(());
            }
            if let Some(previous) = previous.replace(step) {
                let frame = row(previous, proof, false);
                actor_decisions += u64::from(frame.actor_eligible);
                emit(frame)?;
                frames += 1;
            }
            Ok(())
        },
    )?;
    if replay.snapshot_hash != proof.snapshot_hash || replay.targets.is_some() != proof.natural {
        return Err("research feature replay differs from verified ending".into());
    }
    if replay.unencoded_decisions != 0 {
        return Err(
            "research source has unencoded decisions; refusing actor-tail exclusion".into(),
        );
    }
    let last = previous.ok_or("known research episode has no learner history")?;
    let frame = row(last, proof, true);
    actor_decisions += u64::from(frame.actor_eligible);
    emit(frame)?;
    frames += 1;
    Ok(StreamedResearch4 {
        frames,
        actor_decisions,
        excluded_unknown: false,
    })
}

#[cfg(test)]
mod tests;
