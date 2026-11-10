//! The tactician builds the answer (stage S1, `tactician_head_v2`).
//!
//! A [`TacticianNet4`] reads the question's rows (`R` never among them) and
//! gives one logit per unit and a confidence. Its logits take the place of the
//! net's for that question's head and go through the same constrained decoder
//! the raw policy uses at temperature 0 (`sample_subset_constrained` at an
//! attack, `sample_pairs_constrained` at a block, the greedy single choice at
//! a target), so the answer is legal. Below the confidence threshold, or in
//! the unexpected case that the built answer does not decode exactly, the seat
//! plays `N`: the trunk's own answer, never a rule's.

use super::net::TacticianNet4;
use super::{
    Drawn4, Question, TacticianRecord4, TacticianStep, elapsed_micros, exact_action, look,
    net_answer, to_target, unit_groups, unit_keys,
};
use crate::features4::TeacherTarget4;

/// What the seat plays: the built answer when the net is sure enough and it
/// decodes, `N` otherwise.
pub(super) fn choose(
    built: &[u8],
    legal: bool,
    confidence: f32,
    threshold: f32,
    net: &[u8],
) -> (Vec<u8>, Drawn4) {
    if legal && confidence.is_finite() && confidence >= threshold {
        (built.to_vec(), Drawn4::Built)
    } else {
        (net.to_vec(), Drawn4::Net)
    }
}

/// One built decision: the answer to play and its record.
///
/// # Errors
/// The net's failure, or `N` failing to decode (never: it is sampled under the
/// question's constraints).
pub(super) fn answer(
    step: &TacticianStep<'_>,
    question: &Question<'_>,
    tactician: &TacticianNet4,
    threshold: f32,
    prior: &look::LookPrior4,
) -> anyhow::Result<(TeacherTarget4, TacticianRecord4)> {
    let &Question {
        window,
        units,
        logits,
        state,
        unit_embeddings,
        started,
    } = question;
    let net = net_answer(window, step, logits);
    let rows = look::rows(window, step, &net, None, logits, prior);
    let forward = std::time::Instant::now();
    let (built_logits, confidence) = tactician.run(state, unit_embeddings, &rows, units)?;
    let net_micros = elapsed_micros(forward);
    let built = net_answer(window, step, &built_logits);
    let legal = exact_action(window, step, &built).is_some();
    let (played, drawn) = choose(&built, legal, confidence, threshold, &net);
    let target = to_target(window, step, &played)?;
    let record = TacticianRecord4 {
        seat: step.view.seat.get(),
        seq: step.view.seq,
        turn: step.view.turn,
        window,
        value: step.value,
        keys: unit_keys(window, step.cands),
        logits: logits.to_vec(),
        neighbors: 0,
        offered: Vec::new(),
        // No layer runs here; `R` is not read.
        rules: net.clone(),
        net,
        played,
        probability: 1.0,
        drawn,
        micros: elapsed_micros(started),
        scores: None,
        groups: unit_groups(window, step.cands),
        label: None,
        rows_read_rules: Some(false),
        built_logits: Some(built_logits),
        built: Some(built),
        confidence: Some(confidence),
        net_micros: Some(net_micros),
        state: state.to_vec(),
        units: unit_embeddings.to_vec(),
        rows,
    };
    Ok((target, record))
}
