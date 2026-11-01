//! Explicit collection purpose and per-seat policy identities; evaluation stays separate.
use super::{Args, ModelIdentity, inspect_model_identity, is_net_seat};
use anyhow::{Context as _, ensure};
use serde_json::{Value, json};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Purpose {
    Evaluation,
    OnPolicyTraining,
    /// Separate research MDP; stopped prefixes are never ordinary MTG outcomes.
    #[value(name = "failure_terminal_research_v1")]
    FailureTerminalResearchV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum OpponentRole {
    Historical,
    SelfPlay,
}

#[derive(Clone)]
pub(super) struct Contract {
    pub purpose: Purpose,
    pub opponent_role: OpponentRole,
    pub opponent_path: Option<PathBuf>,
    pub opponent_identity: Option<ModelIdentity>,
    pub learner_identity: Option<ModelIdentity>,
    pub depth: u8,
    pub temperature: f32,
}

impl Contract {
    pub fn new(args: &Args, learner: Option<&ModelIdentity>) -> anyhow::Result<Self> {
        let opponent_path = match (args.opponent_role, &args.opponent_model) {
            (OpponentRole::SelfPlay, None) => Some(args.model.clone()),
            (_, path) => path.clone(),
        };
        let opponent_identity = opponent_path
            .as_deref()
            .map(inspect_model_identity)
            .transpose()?;
        let mixed = opponent_path.is_some() || args.collection_purpose != Purpose::Evaluation;
        if mixed {
            ensure!(
                !args.legacy_v3 && args.seats == 2 && args.teams.is_empty(),
                "initial per-seat collector requires 1v1 neural models without teams"
            );
            ensure!(
                args.shadow_teacher.is_none(),
                "mixed policy shadow labels are not supported"
            );
        }
        if let Some(opponent) = &opponent_identity {
            let learner = learner.context("missing learner identity")?;
            ensure!(
                opponent.encoder_version == learner.encoder_version,
                "opponent encoder differs"
            );
            if args.opponent_role == OpponentRole::SelfPlay {
                ensure!(
                    opponent == learner,
                    "self-play requires the exact frozen learner model identity"
                );
            }
        }
        if args.collection_purpose != Purpose::Evaluation {
            ensure!(
                args.record_out.is_some()
                    && args.deals > 0
                    && args.raw_policy
                    && args.thinking_steps == 2
                    && (args.temperature - 0.7).abs() < 1e-6
                    && !args.held_out_cards
                    && args.holdout_tier.is_empty()
                    && (args.decks_manifest.is_none() || args.deck_split == "train"),
                "on-policy collection requires raw FP32 K2 temperature0.7 recording; never held-out evaluation"
            );
            checkpoint(learner.context("missing learner identity")?)?;
            if let Some(opponent) = &opponent_identity {
                checkpoint(opponent)?;
            }
        }
        if args.collection_purpose == Purpose::FailureTerminalResearchV1 {
            super::failure_terminal::validate_encoder(
                learner
                    .context("missing research learner identity")?
                    .encoder_version,
            )?;
            ensure!(
                opponent_identity.is_some() && args.no_baseline,
                "failure-terminal research currently requires a frozen neural opponent and --no-baseline"
            );
        }
        Ok(Self {
            purpose: args.collection_purpose,
            opponent_role: args.opponent_role,
            opponent_path,
            opponent_identity,
            learner_identity: learner.cloned(),
            depth: args.thinking_steps,
            temperature: args.temperature,
        })
    }

    pub fn uses_seat_pool(&self) -> bool {
        self.opponent_path.is_some() || self.purpose != Purpose::Evaluation
    }

    pub fn metadata(&self) -> Value {
        json!({"version":if self.purpose == Purpose::FailureTerminalResearchV1 {2}else{1},
            "purpose":self.purpose,
            // PPO admission accepts FP32 rollouts only; an FP16 graph says so.
            "precision":self.learner_identity.as_ref().and_then(|i| i.precision.as_deref()).unwrap_or("float32"),
            "research_target_contract":if self.purpose == Purpose::FailureTerminalResearchV1 {
                Some(super::failure_terminal::contract())
            } else {None},
            "learner_model_identity":self.learner_identity,"opponent_model_identity":self.opponent_identity,
            "opponent_role":if self.opponent_identity.is_some(){Some(self.opponent_role)}else{None},
            "seat_rng_rule":"independent_policy_seed_xor_physical_seat_v1"})
    }

    pub fn seats(
        &self,
        count: usize,
        focal: usize,
        teams: &[Option<u8>],
    ) -> anyhow::Result<Vec<Value>> {
        (0..count)
            .map(|seat| {
                let learner = is_net_seat(seat, focal, teams);
                let identity = if learner {
                    self.learner_identity.as_ref()
                } else {
                    self.opponent_identity.as_ref()
                };
                let role = if learner {
                    "learner"
                } else if identity.is_some() {
                    match self.opponent_role {
                        OpponentRole::Historical => "historical",
                        OpponentRole::SelfPlay => "self_play",
                    }
                } else {
                    "house"
                };
                Ok(if let Some(identity) = identity {
                    json!({"seat":seat,"role":role,"checkpoint_sha256":checkpoint(identity)?,
                    "thinking_steps":self.depth,"temperature":self.temperature})
                } else {
                    json!({"seat":seat,"role":role})
                })
            })
            .collect()
    }
}

fn checkpoint(identity: &ModelIdentity) -> anyhow::Result<&str> {
    let sha = identity
        .source_checkpoint_sha256
        .as_deref()
        .context("model lacks parent checkpoint identity")?;
    ensure!(
        sha.len() == 64
            && sha
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
        "invalid parent checkpoint digest"
    );
    Ok(sha)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser as _;
    use std::collections::BTreeMap;
    fn identity() -> ModelIdentity {
        ModelIdentity {
            model_family: "v5",
            model_version: 5,
            encoder_version: 7,
            global_feat_width: None,
            critic_semantics: None,
            architecture: Some("baylee_ai_v5_relational_v2".into()),
            onnx_sha256: "a".repeat(64),
            depth_graph_sha256: BTreeMap::from([(0, "b".repeat(64)), (2, "c".repeat(64))]),
            metadata_sha256: "d".repeat(64),
            source_checkpoint_sha256: Some("e".repeat(64)),
            precision: None,
        }
    }
    #[test]
    fn explicit_training_purpose_and_role_bindings_are_required() {
        let base = [
            "eval4",
            "--model",
            "fixture",
            "--out",
            "result",
            "--record-out",
            "records",
            "--collection-purpose",
            "on-policy-training",
            "--raw-policy",
            "--temperature",
            "0.7",
        ];
        let args = Args::try_parse_from(base).unwrap();
        let contract = Contract::new(&args, Some(&identity())).unwrap();
        assert!(contract.uses_seat_pool());
        let roles = contract.seats(2, 1, &[None, None]).unwrap();
        assert_eq!(roles[0]["role"], "house");
        assert_eq!(roles[1]["role"], "learner");
        assert_eq!(roles[1]["checkpoint_sha256"], "e".repeat(64));
        for extra in [
            vec!["--held-out-cards"],
            vec!["--seats", "4"],
            vec!["--teams", "1,2"],
            vec!["--deals", "0"],
            vec!["--holdout-tier", "eval_benchmark"],
            vec!["--decks-manifest", "test-decks.json"],
        ] {
            let mut command = base.to_vec();
            command.extend(extra);
            let args = Args::try_parse_from(command).unwrap();
            assert!(Contract::new(&args, Some(&identity())).is_err());
        }
        let mut args = Args::try_parse_from(base).unwrap();
        args.temperature = 0.0;
        assert!(Contract::new(&args, Some(&identity())).is_err());
        args.temperature = 0.7;
        args.raw_policy = false;
        assert!(Contract::new(&args, Some(&identity())).is_err());
        args.collection_purpose = Purpose::Evaluation;
        assert!(
            !Contract::new(&args, Some(&identity()))
                .unwrap()
                .uses_seat_pool()
        );
    }
}
