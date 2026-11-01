//! Opt-in BC labels on neural-visited views. Never an executed action or PPO target.
use std::path::Path;

use anyhow::{Context as _, ensure};
use baylee_ai::HeuristicAgent;
use baylee_core::preset::AIProfile;
use baylee_engine::choice::{Pending, PlayerAction};
pub(super) use baylee_train::failure_terminal::sha;
use baylee_train::features4::{
    EncodedCandidates4, EncodedState4, SourceKind4, decode_action4, encode_teacher_action4,
    view_only_state_hash,
};
use baylee_train::netplay4::Answer4;
use baylee_view::PlayerView;
use serde::Serialize;
use serde_json::{Value, json};

mod writer;
use writer::LabelWriter;

#[derive(Default, Serialize)]
struct Counts {
    queries: u64,
    labels: u64,
    mapping_failures: u64,
    teacher_refits: u64,
    rejected_executions: u64,
    non_neural_exclusions: u64,
}

/// A separate per-game stream; `.jsonl` exists only after record binding succeeds.
pub(super) struct ShadowGame {
    writer: LabelWriter,
    profile: AIProfile,
    seed: u64,
    encoder_version: u32,
    counts: Counts,
}

pub(super) fn metadata(profile: &str) -> Value {
    json!({"schema": 1, "teacher": profile, "label_use": "bc_only",
        "history_mode": "stateless_view_only", "scouting": false,
        "teacher_has_behavior_probability": false, "ppo_certification": false,
        "completion_required": true, "path": "shadow/*.jsonl"})
}

impl ShadowGame {
    pub(super) fn exclude_non_neural(&mut self) {
        self.counts.non_neural_exclusions += 1;
    }
    pub(super) fn new(
        dir: &Path,
        game: &str,
        deal: u64,
        profile: &str,
        seed: u64,
    ) -> anyhow::Result<Self> {
        let profile_value = match profile {
            "sharp" => AIProfile::SHARP,
            "expert" => AIProfile::EXPERT,
            _ => anyhow::bail!("Unsupported shadow teacher"),
        };
        // Binding to run.json also binds behavior checkpoint, encoder, catalog,
        // evaluator and policy contract. No personal/game-account identities.
        let run = std::fs::read(dir.join("run.json"))?;
        let run_value: Value = serde_json::from_slice(&run)?;
        ensure!(
            run_value["policy_mode"] == "raw",
            "Shadow requires Raw behavior"
        );
        ensure!(
            run_value["policy_provenance_version"] == 2
                && run_value["sampling_version"] == 2
                && run_value["policy_contract"] == "raw_neural_sampler2_v1",
            "Shadow requires current certified Raw behavior; historical runs are not upgraded"
        );
        ensure!(
            matches!(
                run_value["model_identity"]["encoder_version"].as_u64(),
                Some(7 | 8)
            ),
            "Shadow requires encoder 7"
        );
        ensure!(
            run_value["shadow_structure_sha256"]
                .as_str()
                .is_some_and(|v| v.len() == 64),
            "Shadow requires catalog identity"
        );
        ensure!(
            run_value["model_identity"]["source_checkpoint_sha256"]
                .as_str()
                .is_some_and(|v| v.len() == 64),
            "Shadow requires behavior checkpoint identity"
        );
        let header = json!({"kind": "header", "schema": 1, "game": game, "deal": deal,
            "run_sha256": sha(&run), "behavior_model_identity": run_value["model_identity"],
            "structure_sha256": run_value["shadow_structure_sha256"],
            "teacher_seed": seed, "contract": metadata(profile)});
        Ok(Self {
            writer: LabelWriter::new(dir, game, &header)?,
            profile: profile_value,
            seed,
            encoder_version: u32::try_from(
                run_value["model_identity"]["encoder_version"]
                    .as_u64()
                    .context("Missing encoder")?,
            )?,
            counts: Counts::default(),
        })
    }

    /// Query AFTER NN decision inference, BEFORE engine action. The supplied menu
    /// is captured read-only from the actual policy's current knowledge state.
    #[allow(
        clippy::too_many_arguments,
        reason = "Explicit immutable decision binding"
    )]
    pub(super) fn propose(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        state: &EncodedState4,
        menu: &EncodedCandidates4,
        answer: &Answer4,
        teams: &[Option<u8>],
        decision_seq: u64,
        record_byte_offset: usize,
    ) -> anyhow::Result<Option<Value>> {
        if answer.provenance.source != SourceKind4::Net {
            self.exclude_non_neural();
            return Ok(None);
        }
        self.counts.queries += 1;
        // No teacher history is advanced with its counterfactual action. A fresh
        // teacher sees only this authorized view and public team assignments.
        let teacher = HeuristicAgent::new(self.profile)
            .with_teams(teams.to_vec())
            .with_seed(self.seed ^ decision_seq.rotate_left(17) ^ u64::from(view.seat.get()));
        let action = teacher.act(view, pending);
        if teacher.fallbacks() != 0 {
            self.counts.teacher_refits += 1;
            return Ok(None);
        }
        let Some(target) = checked_target(view, pending, menu, &action) else {
            self.counts.mapping_failures += 1;
            return Ok(None);
        };
        Ok(Some(json!({"kind": "label", "seat": view.seat.get(),
            "decision_seq": decision_seq, "record_byte_offset": record_byte_offset,
            "view_hash_fnv1a64": format!("{:016x}", if self.encoder_version == 8 { baylee_train::features8::view_only_state_hash8(view, pending) } else { view_only_state_hash(view, pending) }),
            "pending_sha256": sha(&serde_json::to_vec(pending)?),
            "encoded_state_sha256": sha(&serde_json::to_vec(state)?),
            "candidates_sha256": sha(&serde_json::to_vec(menu)?),
            "teacher_action": action, "teacher_target": target,
            "executed_action": answer.action, "executed_provenance": answer.provenance,
            "label_use": "bc_only", "teacher_source": "house", "teacher_has_logprob": false})))
    }

    pub(super) fn executed(
        &mut self,
        proposal: Option<Value>,
        accepted: bool,
    ) -> anyhow::Result<()> {
        if let Some(proposal) = proposal {
            if accepted {
                self.writer.write(&proposal)?;
                self.counts.labels += 1;
            } else {
                self.counts.rejected_executions += 1;
            }
        }
        Ok(())
    }

    pub(super) fn finish(self, record: &[u8], capped: bool) -> anyhow::Result<()> {
        self.writer.finish(record, capped, &self.counts)
    }
}

fn checked_target(
    view: &PlayerView,
    pending: &Pending,
    menu: &EncodedCandidates4,
    action: &PlayerAction,
) -> Option<baylee_train::features4::TeacherTarget4> {
    if pending.answer_fault(action).is_some() {
        return None;
    }
    let target = encode_teacher_action4(menu, pending, action)?;
    let decoded = decode_action4(menu, view, pending, &target)?;
    if pending.answer_fault(&decoded).is_some()
        || (menu.head != baylee_train::features4::HeadKind4::SingleChoice && decoded != *action)
    {
        return None;
    }
    let encoded = encode_teacher_action4(menu, pending, &decoded)?;
    // Grouped single choices may decode to a different equivalent member;
    // subset/target repairs may NOT silently change the teacher's label.
    (target == encoded).then_some(target)
}

#[cfg(test)]
mod tests;
