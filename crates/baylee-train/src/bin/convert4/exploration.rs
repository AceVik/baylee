//! Rule proposals as recorded exploration (`eval4 --explore-rules`,
//! `baylee_train_ops::exploration`). The converter checks each game's
//! mixture against its seeded stream and the run's ε, carries `ln μ` into the
//! frames, and keeps the records out of every supervised dataset: the layer's
//! answers are actions taken, never labels, and outcomes are the records'
//! only labels.
use anyhow::{Context as _, ensure};
use baylee_train::convert4::{RolloutStepSidecar4, Step4};
use baylee_train::deckgen::Rng;
use baylee_train::features4::SourceKind4;
use baylee_train_ops::exploration::{
    self, CONTRACT, GameExploration, NET, RULES, RunExploration, frame,
};
use serde_json::{Value, json};

/// A run's exploration contract, if it explores.
///
/// # Errors
/// A malformed contract.
pub(super) fn run_contract(run: &Value) -> anyhow::Result<Option<RunExploration>> {
    let Some(contract) = run.get("exploration").filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let contract: RunExploration =
        serde_json::from_value(contract.clone()).context("run.json exploration")?;
    contract.validate()?;
    Ok(Some(contract))
}

/// What a conversion of exploration records may be: an on-policy dataset
/// for the learner, never a supervised one.
#[allow(
    clippy::struct_excessive_bools,
    reason = "the converter's independent mode switches"
)]
pub(super) struct Modes {
    pub on_policy: bool,
    pub shadow_bc: bool,
    pub llm_bc: bool,
    pub live_bc: bool,
    pub self_imitation: bool,
    pub search_targets: bool,
    /// `--bundle-distill`: the layer's answers as policy targets.
    pub bundle_distill: bool,
}

/// Refuses every supervised conversion of exploration records.
///
/// # Errors
/// A BC mode, or a run that is not an on-policy collection.
pub(super) fn refuse_supervised(modes: &Modes) -> anyhow::Result<()> {
    ensure!(
        !(modes.shadow_bc
            || modes.llm_bc
            || modes.live_bc
            || modes.self_imitation
            || modes.search_targets
            || modes.bundle_distill),
        "exploration records are no supervised data: a rule proposal is an action taken, never \
         a label; convert them as an on-policy collection only"
    );
    ensure!(
        modes.on_policy,
        "exploration records come from an on-policy collection only"
    );
    Ok(())
}

/// Checks one game's exploration against its stream and the run's ε: the
/// coins are the seed's in order, each step checks against its π, and the
/// totals are the game's.
///
/// # Errors
/// Any disagreement, or a record on a step the net did not answer.
pub(super) fn verify_game(
    sidecar: &[RolloutStepSidecar4],
    game: &GameExploration,
    run: &RunExploration,
) -> anyhow::Result<()> {
    let mut stream = Rng::new(game.seed);
    let (mut draws, mut played) = (0_u32, 0_u32);
    for entry in sidecar {
        let Some(step) = &entry.exploration else {
            continue;
        };
        ensure!(
            entry.provenance.source == SourceKind4::Net,
            "an exploration record on a {:?} step",
            entry.provenance.source
        );
        ensure!(
            step.draw == draws,
            "exploration draw {} out of order",
            step.draw
        );
        ensure!(
            step.coin.to_bits() == stream.unit().to_bits(),
            "exploration coin {} is not the game's seeded stream",
            step.draw
        );
        exploration::check_step(
            step,
            run.epsilon,
            entry.provenance.has_logprob,
            entry.provenance.behavior_logprob,
            &run.proposers,
        )?;
        draws += 1;
        played += u32::from(step.played);
    }
    ensure!(
        draws == game.draws && played == game.played,
        "the game's exploration totals ({}, {}) differ from its records ({draws}, {played})",
        game.draws,
        game.played
    );
    Ok(())
}

/// No exploration in a run that does not declare it.
///
/// # Errors
/// A record or a game stream without the contract.
pub(super) fn refuse_undeclared(
    game: &Value,
    sidecar: Option<&[RolloutStepSidecar4]>,
) -> anyhow::Result<()> {
    ensure!(
        game.get("exploration").is_none_or(Value::is_null)
            && sidecar.is_none_or(|steps| steps.iter().all(|step| step.exploration.is_none())),
        "exploration records in a run that declares no exploration contract"
    );
    Ok(())
}

/// The dataset's proposer table: `net`, `rules`, then every run's groups in
/// the order the runs name them.
pub(super) fn proposer_table<'a>(
    runs: impl IntoIterator<Item = &'a RunExploration>,
) -> Vec<String> {
    let mut table = vec![NET.to_owned(), RULES.to_owned()];
    for run in runs {
        for name in &run.proposers {
            if !table.contains(name) {
                table.push(name.clone());
            }
        }
    }
    table
}

/// What `meta.json` says of a dataset holding exploration records.
pub(super) fn metadata(proposers: &[String]) -> Value {
    json!({
        "version": 1,
        "contract": CONTRACT,
        "records": "exploration",
        "labels": "outcomes_only",
        "supervised_policy_weight": 0.0,
        "column": "bst4_reserved_word_f32_ln_mu",
        "flags": {"mixture": frame::MIXTURE, "rule_same": frame::RULE_SAME,
            "rule_differs": frame::RULE_DIFFERS, "rule_played": frame::RULE_PLAYED,
            "proposer_shift": frame::PROPOSER_SHIFT},
        "proposers": proposers,
    })
}

/// An exploring game's decisions weigh nothing in any supervised loss: its
/// outcomes are the only labels, whatever a reader does with the frames.
pub(super) fn apply(step: &mut Step4) {
    if step.is_decision {
        step.loss_weight = 0.0;
    }
}

#[cfg(test)]
mod tests;
