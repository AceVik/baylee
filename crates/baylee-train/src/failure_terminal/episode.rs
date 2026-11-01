//! Revalidate collector episodes before any separate research feature conversion.
use super::{contract, natural, replay, sha, stop, validate_encoder};
use crate::{convert4::RolloutStepSidecar4, features4::side_utility};
use anyhow::{Context as _, ensure};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::Pending;
use baylee_gamehost::record::Line;
use serde_json::{Value, json};

pub use baylee_train_ops::research_contract::ResearchReturn;

/// Canonical collector proof. Only native verification constructs this token.
#[derive(Clone, Debug)]
pub struct VerifiedEpisode {
    pub(crate) record_sha256: String,
    pub(crate) sidecar_sha256: String,
    pub(crate) learner: u8,
    pub(crate) encoder: u32,
    pub(crate) natural: bool,
    pub(crate) snapshot_hash: u64,
    /// Verified separately named learner return, never assigned to Step4 outcome.
    pub(crate) returns: ResearchReturn,
    /// Last accepted original input; no fallback is added to reach termination.
    pub(crate) last_input_n: u64,
}

impl VerifiedEpisode {
    /// Immutable separately named return verified against canonical execution.
    #[must_use]
    pub fn returns(&self) -> &ResearchReturn {
        &self.returns
    }
    /// Last original accepted input covered by the proof.
    #[must_use]
    pub fn last_input_n(&self) -> u64 {
        self.last_input_n
    }
}

fn hash(value: &Value) -> anyhow::Result<u64> {
    let text = value
        .as_str()
        .context("missing research snapshot/view hash")?;
    ensure!(
        text.len() == 16
            && text
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
        "invalid research snapshot/view hash"
    );
    Ok(u64::from_str_radix(text, 16)?)
}

fn natural_proof(
    record: &[u8],
    sidecar: &[RolloutStepSidecar4],
    bindings: &[Value],
    encoder: u32,
    learner: u8,
) -> anyhow::Result<(Value, u64, u64)> {
    let mut end_offset = None;
    let mut offset = 0;
    for raw in record.split_inclusive(|&byte| byte == b'\n') {
        let line: Line = serde_json::from_slice(raw)?;
        if matches!(line, Line::End { .. }) {
            ensure!(
                end_offset.is_none() && offset + raw.len() == record.len(),
                "research natural End must be final"
            );
            end_offset = Some(offset);
        }
        offset += raw.len();
    }
    let offset = end_offset.context("natural research episode lacks an engine End")?;
    // The established replay checks End/winners as well as every engine hash,
    // whichever build the header names: the sidecar's behaviour labels are
    // only this record's if the whole state is.
    let terminal = baylee_gamehost::record::replay_exact(record)
        .map_err(|error| anyhow::anyhow!("research natural terminal replay: {error:?}"))?;
    ensure!(
        terminal.ended,
        "natural research episode is not engine-terminal"
    );
    // The same controlled-input replay used by stopped episodes binds every
    // original sidecar and accepted action; stripping only the real End marker
    // here does not turn a nonterminal game into a natural one.
    let (session, _, last_n, _) = replay(&record[..offset], sidecar, Some(bindings), encoder, 0)?;
    let Pending::GameOver(result) = session.pending() else {
        anyhow::bail!("natural research episode lacks GameOver");
    };
    let winners: Vec<_> = session
        .winning_seats(*result)
        .into_iter()
        .map(PlayerId::get)
        .collect();
    let outcome = match winners.as_slice() {
        [] => json!({"kind":"draw"}),
        [seat] => json!({"kind":"won","seat":seat}),
        seats => json!({"kind":"team_won","seats":seats}),
    };
    let survivors = if winners.is_empty() {
        vec![0, 1]
    } else {
        winners.clone()
    };
    let utility = side_utility(learner, &[None, None], &survivors, &winners);
    Ok((
        natural(&outcome, utility, bindings),
        last_n,
        session.snapshot_hash(),
    ))
}

/// Revalidate one originally collected research episode with its unchanged inputs.
///
/// # Errors
/// Rejects unsupported targets, absent/mutated bindings, false natural outcomes,
/// incorrect failure ownership, divergent boundary evidence, and unknown utilities.
pub fn verify_episode(
    record: &[u8],
    sidecar: &[RolloutStepSidecar4],
    episode: &Value,
    encoder: u32,
    learner: u8,
) -> anyhow::Result<VerifiedEpisode> {
    validate_encoder(encoder)?;
    ensure!(
        learner < 2 && record.ends_with(b"\n"),
        "invalid research learner or truncated record"
    );
    let first = record
        .split(|&byte| byte == b'\n')
        .find(|line| !line.is_empty())
        .context("research header")?;
    let Line::Header { preset, .. } = serde_json::from_slice(first)? else {
        anyhow::bail!("research record lacks header");
    };
    ensure!(
        preset.seats.len() == 2 && preset.seats.iter().all(|seat| seat.team.is_none()),
        "research records require unteamed 1v1"
    );
    ensure!(
        episode["kind"] == "research_termination"
            && episode["target_contract"] == contract()
            && episode["bootstrap"] == false,
        "unsupported or mixed research target contract"
    );
    let bindings: Vec<Value> = serde_json::from_value(episode["executed_sample_bindings"].clone())
        .context("research episode lacks original sample bindings")?;
    let is_natural = episode["termination_kind"] == "mtg_terminal";
    let (expected, last_input_n, snapshot_hash) = if is_natural {
        natural_proof(record, sidecar, &bindings, encoder, learner)?
    } else {
        ensure!(
            matches!(
                episode["termination_kind"].as_str(),
                Some("learner_announcement_loop" | "unknown")
            ),
            "unsupported research termination"
        );
        let boundary = &episode["boundary"];
        let seat = PlayerId::new(u8::try_from(
            boundary["seat"]
                .as_u64()
                .context("research boundary seat")?,
        )?);
        ensure!(seat.get() < 2, "research boundary seat outside roster");
        let reason = boundary["guard_reason"]
            .as_str()
            .context("research boundary reason")?;
        let proposal = boundary
            .get("unexecuted_fallback_proposal")
            .filter(|value| !value.is_null())
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()?;
        let snapshot = hash(&boundary["snapshot_hash"])?;
        let expected = stop(
            record,
            sidecar,
            &bindings,
            encoder,
            usize::from(learner),
            seat,
            matches!(reason, "outer_repeated_view" | "internal_repeated_view"),
            reason,
            hash(&boundary["view_hash"])?,
            proposal.as_ref(),
            snapshot,
        )?;
        let last_n = expected["boundary"]["last_included_input_n"]
            .as_u64()
            .context("research last input")?;
        (expected, last_n, snapshot)
    };
    ensure!(
        expected == *episode,
        "research episode differs from canonical target/boundary/provenance proof"
    );
    let returns = ResearchReturn {
        termination_kind: episode["termination_kind"]
            .as_str()
            .context("research termination")?
            .into(),
        training_utility: serde_json::from_value(episode["training_utility"].clone())?,
        mtg_outcome: if episode["mtg_outcome"].is_null() {
            None
        } else {
            Some(episode["mtg_outcome"].clone())
        },
        bootstrap: false,
    };
    Ok(VerifiedEpisode {
        record_sha256: sha(record),
        sidecar_sha256: sha(&serde_json::to_vec(sidecar)?),
        learner,
        encoder,
        natural: is_natural,
        snapshot_hash,
        returns,
        last_input_n,
    })
}
