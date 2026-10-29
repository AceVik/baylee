//! Separate, opt-in training termination. No recovery action enters the prefix.
use crate::{convert4::RolloutStepSidecar4, features4::SourceKind4};
use anyhow::{Context as _, ensure};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::event::{CastCancelReason, GameEvent};
use baylee_gamehost::{
    Session,
    record::{Line, Source},
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// Immutable native research target descriptor.
pub fn contract() -> Value {
    json!({"name":"failure_terminal_research_v1","predicate":"repeated_view_two_replayed_announcement_rollbacks_v1",
        "utility_mapping":"natural_1_0_half_learner_loop_0_unknown_null_v1",
        "bootstrap":false,"fallback_recovery_actions_executed":0,
        "admission":"unadmitted_native_collection_only"})
}

/// Validate the supported authorized-view projections.
/// # Errors
/// Unsupported encoder versions cannot claim this research contract.
pub fn validate_encoder(encoder: u32) -> anyhow::Result<()> {
    ensure!(matches!(encoder, 7..=9), "unsupported research encoder");
    Ok(())
}

fn observed(
    session: &Session,
    seat: PlayerId,
    encoder: u32,
) -> anyhow::Result<(Pending, baylee_view::PlayerView, u64)> {
    let view = match encoder {
        9 => session.view_for_with_announcement_context(seat),
        8 => session.view_for_with_payment_context(seat),
        _ => session.view_for_with_context(seat),
    }
    .context("research boundary has no acting seat view")?;
    let hash = match encoder {
        9 => crate::features9::view_only_state_hash9(&view.1, &view.0),
        8 => crate::features8::view_only_state_hash8(&view.1, &view.0),
        _ => crate::features4::view_only_state_hash(&view.1, &view.0),
    };
    Ok((view.0, view.1, hash))
}

/// Hash the acting seat's authorized boundary projection.
/// # Errors
/// The seat must have an acting question at this boundary.
pub fn boundary_view_hash(session: &Session, seat: PlayerId, encoder: u32) -> anyhow::Result<u64> {
    Ok(observed(session, seat, encoder)?.2)
}

struct Attempt {
    source: ObjectId,
    start_input: u64,
    start_snapshot_hash: u64,
    views: BTreeSet<u64>,
}

/// Bind one original policy sidecar to its pre-action engine/view/question.
/// # Errors
/// Fails if the seat has no acting question or serialization fails.
pub fn sample_binding(
    session: &Session,
    seat: PlayerId,
    encoder: u32,
    action: &PlayerAction,
    step: &RolloutStepSidecar4,
) -> anyhow::Result<Value> {
    let (pending, _, view_hash) = observed(session, seat, encoder)?;
    Ok(
        json!({"seat":seat.get(),"source":step.provenance.source,"decision_seq":session.decision_seq(),
        "pre_snapshot_hash":format!("{:016x}",session.snapshot_hash()),
        "view_hash":format!("{view_hash:016x}"),"pending":pending,"action":action,
        "sidecar_sha256":sha(&serde_json::to_vec(step)?)}),
    )
}

/// Reconstruct only canonical inputs, verifying every engine hash and the sidecar
/// denominator. The journal distinguishes an unpaid automatic rewind from a
/// deliberate cancellation. Diagnostic branches never enter this replay.
/// # Errors
/// Refuses divergent inputs, unsupported origins and changed action bindings.
#[allow(
    clippy::too_many_lines,
    reason = "Keep canonical replay, proof capture and hash checks ordered"
)]
pub fn replay(
    record: &[u8],
    sidecar: &[RolloutStepSidecar4],
    bindings: Option<&[Value]>,
    encoder: u32,
    boundary_hash: u64,
) -> anyhow::Result<(Session, Vec<Value>, u64, Vec<Value>)> {
    validate_encoder(encoder)?;
    ensure!(record.ends_with(b"\n"), "truncated research record");
    let lines = record
        .split(|&byte| byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(serde_json::from_slice)
        .collect::<Result<Vec<Line>, _>>()?;
    let Some(Line::Header {
        preset,
        hash,
        record: version,
        ..
    }) = lines.first()
    else {
        anyhow::bail!("research prefix has no header");
    };
    ensure!(
        *version == baylee_gamehost::record::RECORD_VERSION,
        "unsupported engine record"
    );
    let mut session = Session::new(preset).context("research replay session")?;
    ensure!(
        format!("{:016x}", session.snapshot_hash()) == *hash,
        "research header hash mismatch"
    );
    for seat in 0..preset.seats.len() {
        session.take_over(PlayerId::new(u8::try_from(seat)?));
    }
    let mut attempts: Vec<Option<Attempt>> = (0..preset.seats.len()).map(|_| None).collect();
    let mut rollbacks = Vec::new();
    let mut samples = 0;
    let mut last_n = 0;
    let mut reconstructed_bindings = Vec::new();
    for line in &lines[1..] {
        match line {
            Line::Input {
                n,
                at,
                seat,
                by,
                action,
                hash,
                ..
            } => {
                ensure!(
                    *n > last_n && *by == Source::Seat,
                    "research requires ordered controlled-seat inputs"
                );
                last_n = *n;
                let provenance = &sidecar
                    .get(samples)
                    .context("missing research sidecar")?
                    .provenance;
                ensure!(
                    matches!(provenance.source, SourceKind4::Net | SourceKind4::Forced),
                    "recovery source in research prefix"
                );
                ensure!(
                    match provenance.source {
                        SourceKind4::Forced =>
                            !provenance.has_logprob && provenance.behavior_logprob == 0.0,
                        SourceKind4::Net =>
                            (provenance.has_logprob
                                || (matches!(session.pending(), Pending::ChooseNumber { .. })
                                    && matches!(action, PlayerAction::ChooseNumber(_))
                                    && provenance.behavior_logprob == 0.0))
                                && provenance.behavior_logprob.is_finite()
                                && provenance.behavior_logprob <= 0.0
                                && provenance.thinking_steps == 2
                                && (provenance.temperature - 0.7).abs() < 1e-6,
                        _ => false,
                    },
                    "invalid research behavior provenance at input {n}: {provenance:?}"
                );
                samples += 1;
                let player = PlayerId::new(*seat);
                let index = usize::from(*seat);
                ensure!(index < attempts.len(), "research seat outside roster");
                let mut binding =
                    sample_binding(&session, player, encoder, action, &sidecar[samples - 1])?;
                let (_, view, view_hash) = observed(&session, player, encoder)?;
                if let PlayerAction::CastSpell { card } = action {
                    attempts[index] = Some(Attempt {
                        source: *card,
                        start_input: *n,
                        start_snapshot_hash: session.snapshot_hash(),
                        views: BTreeSet::from([view_hash]),
                    });
                }
                if let Some(attempt) = &mut attempts[index] {
                    attempt.views.insert(view_hash);
                }
                let announced = session
                    .observe_view_with_payment_context(player)
                    .announced_cast;
                let previous_cancel = session
                    .state()
                    .journal
                    .entries()
                    .iter()
                    .rev()
                    .find_map(|entry| {
                        matches!(entry.event, GameEvent::CastCancelled {player:owner, card, ..}
                        if owner == player && Some(card) == announced)
                        .then_some(entry.seq)
                    })
                    .unwrap_or(0);
                session.tell_time(*at);
                session
                    .act_slice(player, action.clone(), 0)
                    .map_err(|error| anyhow::anyhow!("research input {n} refused: {error}"))?;
                ensure!(
                    format!("{:016x}", session.snapshot_hash()) == *hash,
                    "research input {n} hash mismatch"
                );
                binding["post_snapshot_hash"] = json!(hash);
                if let Some(bindings) = bindings {
                    ensure!(
                        bindings.get(samples - 1) == Some(&binding),
                        "research sample-to-executed binding mismatch at input {n}"
                    );
                }
                reconstructed_bindings.push(binding);
                if let Some(source) = announced
                    && session.observe_view_with_payment_context(player).announced_cast.is_none()
                    && let Some(attempt) = attempts[index].take()
                    && attempt.source == source
                    && !matches!(action, PlayerAction::CancelCast)
                    && (session.snapshot_hash() == attempt.start_snapshot_hash
                        || session.state().journal.entries().iter().rev().any(|entry|entry.seq > previous_cancel && matches!(entry.event,
                            GameEvent::CastCancelled {player:owner,card,reason:CastCancelReason::Unpaid} if owner==player && card==source)))
                {
                    rollbacks.push(json!({"seat":seat,"source":source,"start_input_n":attempt.start_input,
                        "rollback_input_n":n,"snapshot_hash":hash,
                        "pre_announcement_snapshot_hash":format!("{:016x}",attempt.start_snapshot_hash),
                        "proof":if session.snapshot_hash() == attempt.start_snapshot_hash {"exact_pre_announcement_snapshot_restored"}else{"native_unpaid_cancellation"},
                        "includes_guard_view":attempt.views.contains(&boundary_hash)
                            || observed(&session,player,encoder).is_ok_and(|(_,_,hash)|hash==boundary_hash),"action":action,
                        "pre_action_view_hash":format!("{view_hash:016x}"),"pre_action_casting":view.casting}));
                }
            }
            Line::Header { .. } | Line::End { .. } => {
                anyhow::bail!("research prefix contains an extra header or MTG terminal")
            }
            _ => {}
        }
    }
    ensure!(samples == sidecar.len(), "unused research sidecars");
    ensure!(
        bindings.is_none_or(|rows| rows.len() == samples),
        "unused research sample bindings"
    );
    Ok((session, rollbacks, last_n, reconstructed_bindings))
}

#[allow(clippy::too_many_arguments, reason = "Explicit stop-boundary evidence")]
/// Verify and describe a research stop without applying its fallback proposal.
/// # Errors
/// Refuses divergent canonical records or unbound boundary evidence.
pub fn stop(
    record: &[u8],
    sidecar: &[RolloutStepSidecar4],
    bindings: &[Value],
    encoder: u32,
    learner: usize,
    seat: PlayerId,
    guard: bool,
    reason: &str,
    view_hash: u64,
    proposal: Option<&PlayerAction>,
    expected_snapshot: u64,
) -> anyhow::Result<Value> {
    let (session, rollbacks, last_n, _) =
        replay(record, sidecar, Some(bindings), encoder, view_hash)?;
    ensure!(
        session.snapshot_hash() == expected_snapshot,
        "research boundary hash mismatch"
    );
    let (pending, view, hash) = observed(&session, seat, encoder)?;
    ensure!(hash == view_hash, "research boundary view mismatch");
    let matching: Vec<_> = rollbacks
        .iter()
        .filter(|event| event["seat"] == seat.get() && event["includes_guard_view"] == true)
        .collect();
    let repeated = matching.iter().any(|first| {
        matching
            .iter()
            .filter(|event| event["source"] == first["source"])
            .count()
            >= 2
    });
    let prior_visits = bindings
        .iter()
        .filter(|row| {
            row["seat"] == seat.get()
                && row["view_hash"] == format!("{view_hash:016x}")
                && (reason == "outer_repeated_view" || row["source"] == "Net")
        })
        .count();
    let guard_verified = guard
        && match reason {
            "outer_repeated_view" => prior_visits >= 4,
            "internal_repeated_view" => prior_visits >= 3,
            _ => false,
        };
    let certified = guard_verified && usize::from(seat.get()) == learner && repeated;
    let unknown_reason = if usize::from(seat.get()) != learner && reason != "answer_cap" {
        "opponent_failure"
    } else {
        reason
    };
    Ok(
        json!({"kind":"research_termination","target_contract":contract(),"executed_sample_bindings":bindings,
        "termination_kind":if certified {"learner_announcement_loop"}else{"unknown"},
        "mtg_outcome":null,"training_utility":if certified {Some(0.0)}else{None},"bootstrap":false,
        "unknown_reason":if certified {None}else{Some(unknown_reason)},
        "boundary":{"seat":seat.get(),"last_included_input_n":last_n,"record_byte_length":record.len(),
            "record_sha256":sha(record),"snapshot_hash":format!("{expected_snapshot:016x}"),
            "view_hash":format!("{view_hash:016x}"),"pending":pending,"authorized_view":view,
            "guard_reason":reason,"guard_verified_from_prior_visits":guard_verified,
            "prior_guard_view_visits":prior_visits,
            "unexecuted_fallback_proposal":proposal,"fallback_executed":false,
            "all_canonical_snapshot_hashes_match":true,"rollback_evidence":rollbacks}}),
    )
}

/// Preserve the actual natural outcome and separately named research utility.
pub fn natural(outcome: &Value, utility: f32, bindings: &[Value]) -> Value {
    json!({"kind":"research_termination","target_contract":contract(),"executed_sample_bindings":bindings,"termination_kind":"mtg_terminal",
        "mtg_outcome":outcome,"training_utility":utility,"bootstrap":false})
}

/// Existing lowercase SHA-256 content identity shared with native sidecars.
pub fn sha(bytes: &[u8]) -> String {
    use sha2::{Digest as _, Sha256};
    use std::fmt::Write as _;
    let mut result = String::new();
    for byte in Sha256::digest(bytes) {
        write!(result, "{byte:02x}").expect("write to String");
    }
    result
}
