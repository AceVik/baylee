//! Packed model inputs of one step and the call that runs one or many.
//!
//! A step is packed once into a [`StepRow`], whoever runs it. [`run_rows`]
//! stacks rows into one call: every row is padded to the call's longest
//! entity, edge, candidate, pair and rule-reference axes with the padding a
//! single step already carries (masked entities, edges, candidates and
//! pairs, candidate references `-1`, zero rule references). Rows never mix
//! in the graph. One row alone is packed exactly as a step always was.

use std::collections::BTreeMap;

use ort::session::Session;
use ort::value::Tensor;

use super::{RECURRENT_SLOTS, StepOutputs, abi, ort_error, structure};
use crate::carddsl::CARD_DSL_WIDTH;
use crate::features4::{
    CAND_FEAT_WIDTH, EncodedCandidates4, EncodedState4, MAX_SEATS, SEAT_FEAT_WIDTH,
};

/// One step's model inputs at its own extents.
pub(super) struct StepRow {
    /// Thinking steps `K`; `0` is an observation update.
    pub(super) k_val: u8,
    entities: usize,
    entity_width: usize,
    global_width: usize,
    edges: usize,
    candidates: usize,
    pairs: usize,
    /// Rule references per entity and per candidate; `None` for graphs
    /// without the card-structure inputs.
    rule_widths: Option<(usize, usize)>,
    entity_feat: Vec<f32>,
    entity_dsl: Vec<f32>,
    entity_mask: Vec<bool>,
    seat_feat: Vec<f32>,
    global_feat: Vec<f32>,
    edge_index: Vec<i64>,
    edge_type: Vec<i64>,
    edge_mask: Vec<bool>,
    cand_feat: Vec<f32>,
    cand_dsl: Vec<f32>,
    cand_entity: Vec<i64>,
    cand_seat: Vec<i64>,
    cand_mask: Vec<bool>,
    pair_index: Vec<i64>,
    pair_mask: Vec<bool>,
    entity_rules: Vec<i64>,
    cand_rules: Vec<i64>,
    h_in: Vec<f32>,
}

impl StepRow {
    /// Packs one step. Every axis keeps at least one (masked) slot.
    ///
    /// # Errors
    /// When a rule reference is not in the exported card structure.
    pub(super) fn pack(
        state: &EncodedState4,
        cands: &EncodedCandidates4,
        h_in: &[f32],
        k_val: u8,
        encoder_version: u32,
        rule_roots: Option<&BTreeMap<String, u32>>,
    ) -> anyhow::Result<Self> {
        let entities = state.entity_feat.len().max(1);
        let (default_entity_width, default_global_width) = abi::widths(encoder_version);
        let entity_width = state
            .entity_feat
            .first()
            .map_or(default_entity_width, Vec::len);
        let mut entity_feat = vec![0.0_f32; entities * entity_width];
        let mut entity_dsl = vec![0.0_f32; entities * CARD_DSL_WIDTH];
        let mut entity_mask = vec![false; entities];
        for i in 0..state.entity_feat.len() {
            entity_feat[i * entity_width..(i + 1) * entity_width]
                .copy_from_slice(&state.entity_feat[i]);
            let dsl = state.entity_dsl_summary(i);
            entity_dsl[i * CARD_DSL_WIDTH..(i + 1) * CARD_DSL_WIDTH].copy_from_slice(&dsl);
            entity_mask[i] = true;
        }

        let mut seat_feat = vec![0.0_f32; MAX_SEATS * SEAT_FEAT_WIDTH];
        for s in 0..MAX_SEATS {
            seat_feat[s * SEAT_FEAT_WIDTH..(s + 1) * SEAT_FEAT_WIDTH]
                .copy_from_slice(&state.seat_feat[s]);
        }
        let global_width = if state.global_feat.is_empty() {
            default_global_width
        } else {
            state.global_feat.len()
        };

        let edges = state.edges.len().max(1);
        let mut edge_index = vec![0_i64; edges * 2];
        let mut edge_type = vec![0_i64; edges];
        let mut edge_mask = vec![false; edges];
        for (i, &[src, dst, rel]) in state.edges.iter().enumerate() {
            edge_index[i * 2] = i64::from(src);
            edge_index[i * 2 + 1] = i64::from(dst);
            edge_type[i] = i64::from(rel);
            edge_mask[i] = true;
        }

        let candidates = cands.candidates.len().max(1);
        let mut cand_feat = vec![0.0_f32; candidates * CAND_FEAT_WIDTH];
        let mut cand_dsl = vec![0.0_f32; candidates * CARD_DSL_WIDTH];
        let mut cand_entity = vec![-1_i64; candidates];
        let mut cand_seat = vec![-1_i64; candidates];
        let mut cand_mask = vec![false; candidates];
        for (i, c) in cands.candidates.iter().enumerate() {
            cand_feat[i * CAND_FEAT_WIDTH..(i + 1) * CAND_FEAT_WIDTH].copy_from_slice(&c.feat);
            cand_dsl[i * CARD_DSL_WIDTH..(i + 1) * CARD_DSL_WIDTH]
                .copy_from_slice(&c.dsl_summary(state));
            cand_entity[i] = i64::from(c.entity_idx);
            cand_seat[i] = i64::from(c.rel_seat_idx);
            cand_mask[i] = true;
        }

        let pairs = cands.pairs.len().max(1);
        let mut pair_index = vec![0_i64; pairs * 2];
        let mut pair_mask = vec![false; pairs];
        for (i, &[b_idx, a_idx]) in cands.pairs.iter().enumerate() {
            pair_index[i * 2] = i64::from(b_idx.max(0));
            pair_index[i * 2 + 1] = i64::from(a_idx.max(0));
            pair_mask[i] = b_idx >= 0 && a_idx >= 0;
        }

        let (rule_widths, entity_rules, cand_rules) = match rule_roots {
            Some(roots) => {
                let (er, entity_rules) = structure::pack(&state.entity_rule_refs, entities, roots)?;
                let refs = cands
                    .candidates
                    .iter()
                    .map(|c| crate::cardstructure::refs::candidate_refs(c, state))
                    .collect::<Vec<_>>();
                let (cr, cand_rules) = structure::pack(&refs, candidates, roots)?;
                (Some((er, cr)), entity_rules, cand_rules)
            }
            None => (None, Vec::new(), Vec::new()),
        };

        Ok(Self {
            k_val,
            entities,
            entity_width,
            global_width,
            edges,
            candidates,
            pairs,
            rule_widths,
            entity_feat,
            entity_dsl,
            entity_mask,
            seat_feat,
            global_feat: state.global_feat.clone(),
            edge_index,
            edge_type,
            edge_mask,
            cand_feat,
            cand_dsl,
            cand_entity,
            cand_seat,
            cand_mask,
            pair_index,
            pair_mask,
            entity_rules,
            cand_rules,
            h_in: h_in.to_vec(),
        })
    }

    /// Attention tokens this row costs at its own size.
    pub(super) const fn tokens(&self) -> usize {
        self.entities + self.candidates
    }
}

/// Appends `src`, `rows` rows of `width`, as `target_rows` rows of
/// `target_width`, filling the new cells with `fill`.
fn extend_padded<T: Copy>(
    out: &mut Vec<T>,
    src: &[T],
    (rows, width): (usize, usize),
    (target_rows, target_width): (usize, usize),
    fill: T,
) {
    for row in 0..rows {
        out.extend_from_slice(&src[row * width..(row + 1) * width]);
        out.extend(std::iter::repeat_n(fill, target_width - width));
    }
    out.extend(std::iter::repeat_n(
        fill,
        (target_rows - rows) * target_width,
    ));
}

/// Stacks `rows` into one call of `session` and splits its outputs back per
/// row, each at its own candidate and pair count.
///
/// # Errors
/// When the rows disagree on widths, depth or rule inputs, or the call fails.
#[allow(
    clippy::too_many_lines,
    reason = "Explicit tensor packing and extraction for the model ABI"
)]
#[allow(
    clippy::many_single_char_names,
    reason = "b, e, m, c and p are the graph's own axis names"
)]
pub(super) fn run_rows(
    session: &mut Session,
    rows: &[&StepRow],
    d_model: usize,
) -> anyhow::Result<Vec<StepOutputs>> {
    let first = rows
        .first()
        .ok_or_else(|| anyhow::anyhow!("an empty call"))?;
    for row in rows {
        anyhow::ensure!(
            row.k_val == first.k_val
                && row.entity_width == first.entity_width
                && row.global_width == first.global_width
                && row.rule_widths.is_some() == first.rule_widths.is_some()
                && row.h_in.len() == RECURRENT_SLOTS * d_model,
            "rows of one call differ in depth, widths or rule inputs"
        );
    }
    let b = rows.len();
    let e = rows.iter().map(|r| r.entities).max().unwrap_or(1);
    let m = rows.iter().map(|r| r.edges).max().unwrap_or(1);
    let c = rows.iter().map(|r| r.candidates).max().unwrap_or(1);
    let p = rows.iter().map(|r| r.pairs).max().unwrap_or(1);
    let ew = first.entity_width;

    let mut entity_feat = Vec::with_capacity(b * e * ew);
    let mut entity_dsl = Vec::with_capacity(b * e * CARD_DSL_WIDTH);
    let mut entity_mask = Vec::with_capacity(b * e);
    let mut seat_feat = Vec::with_capacity(b * MAX_SEATS * SEAT_FEAT_WIDTH);
    let mut global_feat = Vec::with_capacity(b * first.global_width);
    let mut edge_index = Vec::with_capacity(b * m * 2);
    let mut edge_type = Vec::with_capacity(b * m);
    let mut edge_mask = Vec::with_capacity(b * m);
    let mut cand_feat = Vec::with_capacity(b * c * CAND_FEAT_WIDTH);
    let mut cand_dsl = Vec::with_capacity(b * c * CARD_DSL_WIDTH);
    let mut cand_entity = Vec::with_capacity(b * c);
    let mut cand_seat = Vec::with_capacity(b * c);
    let mut cand_mask = Vec::with_capacity(b * c);
    let mut pair_index = Vec::with_capacity(b * p * 2);
    let mut pair_mask = Vec::with_capacity(b * p);
    let mut h_in = Vec::with_capacity(b * RECURRENT_SLOTS * d_model);
    for r in rows {
        extend_padded(
            &mut entity_feat,
            &r.entity_feat,
            (r.entities, ew),
            (e, ew),
            0.0,
        );
        extend_padded(
            &mut entity_dsl,
            &r.entity_dsl,
            (r.entities, CARD_DSL_WIDTH),
            (e, CARD_DSL_WIDTH),
            0.0,
        );
        extend_padded(
            &mut entity_mask,
            &r.entity_mask,
            (r.entities, 1),
            (e, 1),
            false,
        );
        seat_feat.extend_from_slice(&r.seat_feat);
        global_feat.extend_from_slice(&r.global_feat);
        extend_padded(&mut edge_index, &r.edge_index, (r.edges, 2), (m, 2), 0);
        extend_padded(&mut edge_type, &r.edge_type, (r.edges, 1), (m, 1), 0);
        extend_padded(&mut edge_mask, &r.edge_mask, (r.edges, 1), (m, 1), false);
        extend_padded(
            &mut cand_feat,
            &r.cand_feat,
            (r.candidates, CAND_FEAT_WIDTH),
            (c, CAND_FEAT_WIDTH),
            0.0,
        );
        extend_padded(
            &mut cand_dsl,
            &r.cand_dsl,
            (r.candidates, CARD_DSL_WIDTH),
            (c, CARD_DSL_WIDTH),
            0.0,
        );
        extend_padded(
            &mut cand_entity,
            &r.cand_entity,
            (r.candidates, 1),
            (c, 1),
            -1,
        );
        extend_padded(&mut cand_seat, &r.cand_seat, (r.candidates, 1), (c, 1), -1);
        extend_padded(
            &mut cand_mask,
            &r.cand_mask,
            (r.candidates, 1),
            (c, 1),
            false,
        );
        extend_padded(&mut pair_index, &r.pair_index, (r.pairs, 2), (p, 2), 0);
        extend_padded(&mut pair_mask, &r.pair_mask, (r.pairs, 1), (p, 1), false);
        h_in.extend_from_slice(&r.h_in);
    }

    let mut inputs = ort::inputs![
        "entity_feat" => Tensor::from_array(([b, e, ew], entity_feat)).map_err(ort_error)?,
        "entity_dsl" => Tensor::from_array(([b, e, CARD_DSL_WIDTH], entity_dsl)).map_err(ort_error)?,
        "entity_mask" => Tensor::from_array(([b, e], entity_mask)).map_err(ort_error)?,
        "seat_feat" => Tensor::from_array(([b, MAX_SEATS, SEAT_FEAT_WIDTH], seat_feat)).map_err(ort_error)?,
        "global_feat" => Tensor::from_array(([b, first.global_width], global_feat)).map_err(ort_error)?,
        "edge_index" => Tensor::from_array(([b, m, 2], edge_index)).map_err(ort_error)?,
        "edge_type" => Tensor::from_array(([b, m], edge_type)).map_err(ort_error)?,
        "edge_mask" => Tensor::from_array(([b, m], edge_mask)).map_err(ort_error)?,
        "cand_feat" => Tensor::from_array(([b, c, CAND_FEAT_WIDTH], cand_feat)).map_err(ort_error)?,
        "cand_entity_idx" => Tensor::from_array(([b, c], cand_entity)).map_err(ort_error)?,
        "cand_seat_idx" => Tensor::from_array(([b, c], cand_seat)).map_err(ort_error)?,
        "cand_mask" => Tensor::from_array(([b, c], cand_mask)).map_err(ort_error)?,
        "pair_index" => Tensor::from_array(([b, p, 2], pair_index)).map_err(ort_error)?,
        "pair_mask" => Tensor::from_array(([b, p], pair_mask)).map_err(ort_error)?,
        "cand_dsl" => Tensor::from_array(([b, c, CARD_DSL_WIDTH], cand_dsl)).map_err(ort_error)?,
        "h_in" => Tensor::from_array(([b, RECURRENT_SLOTS, d_model], h_in)).map_err(ort_error)?,
        "thinking_steps" => Tensor::from_array(([b], vec![i64::from(first.k_val); b])).map_err(ort_error)?,
    ];

    if first.rule_widths.is_some() {
        let er = rows
            .iter()
            .filter_map(|r| r.rule_widths.map(|w| w.0))
            .max()
            .unwrap_or(1);
        let cr = rows
            .iter()
            .filter_map(|r| r.rule_widths.map(|w| w.1))
            .max()
            .unwrap_or(1);
        let mut entity_rules = Vec::with_capacity(b * e * er * 2);
        let mut cand_rules = Vec::with_capacity(b * c * cr * 2);
        for r in rows {
            let (row_er, row_cr) = r.rule_widths.unwrap_or((er, cr));
            // A rule reference is a (node, role) pair; role 0 is padding.
            extend_padded(
                &mut entity_rules,
                &r.entity_rules,
                (r.entities, row_er * 2),
                (e, er * 2),
                0,
            );
            extend_padded(
                &mut cand_rules,
                &r.cand_rules,
                (r.candidates, row_cr * 2),
                (c, cr * 2),
                0,
            );
        }
        inputs.extend(ort::inputs![
            "entity_rules" => Tensor::from_array(([b, e, er, 2], entity_rules)).map_err(ort_error)?,
            "cand_rules" => Tensor::from_array(([b, c, cr, 2], cand_rules)).map_err(ort_error)?,
        ]);
    }

    let outputs = session.run(inputs).map_err(ort_error)?;
    let (_, h_out) = outputs["h_out"]
        .try_extract_tensor::<f32>()
        .map_err(ort_error)?;
    let (_, cand_logits) = outputs["cand_logits"]
        .try_extract_tensor::<f32>()
        .map_err(ort_error)?;
    let (_, pair_logits) = outputs["pair_logits"]
        .try_extract_tensor::<f32>()
        .map_err(ort_error)?;
    let (_, scalar_pred) = outputs["scalar_pred"]
        .try_extract_tensor::<f32>()
        .map_err(ort_error)?;
    let (_, value) = outputs["value"]
        .try_extract_tensor::<f32>()
        .map_err(ort_error)?;
    let slots = RECURRENT_SLOTS * d_model;
    anyhow::ensure!(
        h_out.len() == b * slots && cand_logits.len() == b * c && pair_logits.len() == b * p,
        "the graph answered a call of {b} rows in other shapes"
    );
    Ok(rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            (
                h_out[i * slots..(i + 1) * slots].to_vec(),
                cand_logits[i * c..i * c + r.candidates].to_vec(),
                pair_logits[i * p..i * p + r.pairs].to_vec(),
                scalar_pred.get(i).copied().unwrap_or(0.0),
                value.get(i).copied().unwrap_or(0.5),
            )
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::extend_padded;

    #[test]
    fn padding_keeps_rows_in_place_and_fills_the_rest() {
        let mut out = Vec::new();
        extend_padded(&mut out, &[1, 2, 3, 4], (2, 2), (3, 3), 0);
        assert_eq!(out, [1, 2, 0, 3, 4, 0, 0, 0, 0]);
        let mut same = Vec::new();
        extend_padded(&mut same, &[5, 6], (1, 2), (1, 2), -1);
        assert_eq!(same, [5, 6]);
    }
}
