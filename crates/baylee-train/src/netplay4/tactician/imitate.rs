//! The tactician's bootstrap (stage S1): imitation of the tactical layer at
//! its windows, for the tactician alone.
//!
//! The seat plays as `--choice-guard tactical` with its rule groups plays it.
//! At an attack, block or single-target question the layer's answer `R` is
//! the label. With probability `1 − ε` the seat plays `R`; with `ε/3` the
//! net's own answer `N` when it differs; the rest of `ε` spreads evenly over
//! the neighbors of `R` (one change away), so the states one mistake off the
//! layer's line are seen too. The played answer's probability under the whole
//! mixture is exact. With ε = 0 the games are the layer's own, decision for
//! decision.
//!
//! The rows never carry `R` (window-code columns 5 and 11 stay zero): the
//! tactician that reads them must build its answer where no layer runs.
//!
//! **Imitation labels teach the tactician and nothing else.** A run that keeps
//! them says so in its `run.json` ([`IMITATION_CONTRACT`]), and every other
//! consumer of a run refuses it with [`refuse_imitation`]: `convert4`, the
//! research and self-imitation readers, every trainer.

pub use baylee_train_ops::tactician_labels::{IMITATION_CONTRACT, refuse_imitation};

use super::{
    Drawn4, Question, TacticianRecord4, TacticianStep, Window4, elapsed_micros, explore, look,
    neighbors, net_answer, one_hot, to_target, unit_groups, unit_keys,
};
use crate::features4::TeacherTarget4;

/// The layer's answer as a mask over the question's units.
///
/// # Errors
/// A mask of another size than the question's units.
fn layer_mask(window: Window4, units: usize, layer: &TeacherTarget4) -> anyhow::Result<Vec<u8>> {
    match window {
        Window4::Target | Window4::Priority => {
            let index = usize::from(layer.single_idx);
            anyhow::ensure!(
                index < units,
                "the layer's target lies outside the question"
            );
            Ok(one_hot(units, index))
        }
        Window4::Attack | Window4::Block => {
            anyhow::ensure!(
                layer.multi_mask.len() == units,
                "the layer's answer covers {} units of {units}",
                layer.multi_mask.len()
            );
            Ok(layer
                .multi_mask
                .iter()
                .map(|&bit| u8::from(bit != 0))
                .collect())
        }
    }
}

/// One imitation decision: the answer to play, and its record.
///
/// `draw` is uniform in `[0, 1)` from the tactician's own stream. When the
/// layer's answer is played, `layer` itself is returned unchanged, so the
/// game is the layer's own.
///
/// # Errors
/// A layer's answer of another size, or an explored answer that does not
/// decode (never for `N` or a neighbor, each checked or sampled under the
/// question's constraints).
pub(super) fn answer(
    step: &TacticianStep<'_>,
    question: &Question<'_>,
    layer: &TeacherTarget4,
    epsilon: f64,
    draw: f64,
    fired: &[&'static str],
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
    let rules = layer_mask(window, units, layer)?;
    let net = net_answer(window, step, logits);
    let near = neighbors(window, step, &rules);
    // The layer's answer is the base, the net's the one alternative.
    let (played, probability, drawn) = explore(epsilon, draw, &rules, &net, &near);
    let drawn = match drawn {
        Drawn4::Net => Drawn4::Rules,
        Drawn4::Rules => Drawn4::Net,
        other => other,
    };
    let target = if played == rules {
        layer.clone()
    } else {
        to_target(window, step, &played)?
    };
    let rows = look::rows(window, step, &net, None, logits, prior);
    let record = TacticianRecord4 {
        seat: step.view.seat.get(),
        seq: step.view.seq,
        turn: step.view.turn,
        window,
        value: step.value,
        keys: unit_keys(window, step.cands),
        logits: logits.to_vec(),
        neighbors: near.len(),
        offered: near,
        net,
        rules,
        played,
        probability,
        drawn,
        micros: elapsed_micros(started),
        scores: None,
        groups: unit_groups(window, step.cands),
        label: Some("rules".to_owned()),
        rows_read_rules: Some(false),
        built_logits: None,
        built: None,
        confidence: None,
        net_micros: None,
        fired: fired.iter().map(|&name| name.to_owned()).collect(),
        state: state.to_vec(),
        units: unit_embeddings.to_vec(),
        rows,
    };
    Ok((target, record))
}
