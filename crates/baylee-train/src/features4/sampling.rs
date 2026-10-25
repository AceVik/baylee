/// Samples a valid subset mask for `SubsetPick` (`ChooseAttackers`, `ChooseCards`,
/// `ChooseTargets`, `MulliganBottom`, `DiscardChoice`) while tracking exact
/// conditional `free_mask` and `logprob` (`P0-3`).
///
/// # Panics
/// If the internal attacker-count map loses an entry inserted in the same call.
pub fn sample_subset_constrained(
    logits: &[f32],
    cands: &EncodedCandidates4,
    pending: &Pending,
    tau: f32,
    rng: &mut crate::deckgen::Rng,
) -> (Vec<u8>, Vec<u8>, Vec<u16>, f32) {
    let n = logits.len().min(cands.candidates.len());
    let mut mask = vec![0_u8; n];
    let mut free_mask = vec![0_u8; n];
    let mut total_lp = 0.0_f32;
    if n == 0 {
        return (mask, free_mask, Vec::new(), 0.0);
    }
    let scale = if tau <= 1e-4 { 1.0 } else { 1.0 / tau };

    let order: Vec<(usize, f32)> = logits[..n]
        .iter()
        .enumerate()
        .map(|(i, &l)| {
            let p = 1.0 / (1.0 + (-l * scale).exp());
            (i, p)
        })
        .collect();
    // Canonical candidate order is independent of model weights. Otherwise
    // recorded conditional masks are invalid after the first PPO update.
    let visit_seq: Vec<u16> = order.iter().map(|&(i, _)| i as u16).collect();

    if let Pending::ChooseAttackers {
        required,
        alone,
        minimum_required,
        ..
    } = pending
    {
        let mut subset = AttackSubset::new(
            cands.candidates[..n].iter().filter_map(|c| c.object_id),
            required,
            alone,
            *minimum_required,
        );
        for &(i, p) in &order {
            let Some(id) = cands.candidates[i].object_id else {
                continue;
            };
            let (can_take, can_skip) = subset.options(id);
            let free = can_take && can_skip;
            let take = can_take
                && (!can_skip
                    || if tau <= 1e-4 {
                        logits[i] > 0.0
                    } else {
                        (rng.unit() as f32) < p
                    });
            free_mask[i] = u8::from(free);
            mask[i] = u8::from(take);
            if take {
                subset.take(id);
            }
            if free {
                total_lp += if take {
                    p.max(1e-7).ln()
                } else {
                    (1.0 - p).max(1e-7).ln()
                };
            }
        }
        return (mask, free_mask, visit_seq, total_lp);
    }

    let min_k = (cands.min_choices as usize).min(n);
    let max_k = (cands.max_choices as usize).min(n);
    let mut picked = 0_usize;
    for (pos, &(i, p)) in order.iter().enumerate() {
        let rem = n - pos;
        if picked >= max_k {
            mask[i] = 0;
            free_mask[i] = 0;
        } else if picked + rem <= min_k {
            mask[i] = 1;
            free_mask[i] = 0;
            picked += 1;
        } else {
            free_mask[i] = 1;
            let take = if tau <= 1e-4 {
                logits[i] > 0.0
            } else {
                (rng.unit() as f32) < p
            };
            if take {
                mask[i] = 1;
                picked += 1;
                total_lp += p.max(1e-7).ln();
            } else {
                mask[i] = 0;
                total_lp += (1.0 - p).max(1e-7).ln();
            }
        }
    }
    (mask, free_mask, visit_seq, total_lp)
}

/// Samples a valid blocker assignment mask for `PairPick` (`ChooseBlockers`)
/// while enforcing at-most-one-attacker-per-blocker and tracking exact
/// conditional `free_mask` and `logprob` (`P0-3`).
pub fn sample_pairs_constrained(
    logits: &[f32],
    cands: &EncodedCandidates4,
    tau: f32,
    rng: &mut crate::deckgen::Rng,
) -> (Vec<u8>, Vec<u8>, Vec<u16>, f32) {
    let n = logits.len().min(cands.pair_objects.len());
    let mut mask = vec![0_u8; n];
    let mut free_mask = vec![0_u8; n];
    let mut total_lp = 0.0_f32;
    if n == 0 {
        return (mask, free_mask, Vec::new(), 0.0);
    }
    let scale = if tau <= 1e-4 { 1.0 } else { 1.0 / tau };

    let order: Vec<(usize, f32)> = logits[..n]
        .iter()
        .enumerate()
        .map(|(i, &l)| {
            let p = 1.0 / (1.0 + (-l * scale).exp());
            (i, p)
        })
        .collect();
    // Canonical candidate order is independent of model weights. Otherwise
    // recorded conditional masks are invalid after the first PPO update.
    let visit_seq: Vec<u16> = order.iter().map(|&(i, _)| i as u16).collect();

    let mut used_blockers = Vec::new();
    for &(i, p) in &order {
        let [b_idx, a_idx] = cands.pairs[i];
        if b_idx < 0 || a_idx < 0 {
            mask[i] = 0;
            free_mask[i] = 0;
            continue;
        }
        let (blk, _atk) = cands.pair_objects[i];
        if used_blockers.contains(&blk) {
            mask[i] = 0;
            free_mask[i] = 0;
        } else {
            free_mask[i] = 1;
            let take = if tau <= 1e-4 {
                logits[i] > 0.0
            } else {
                (rng.unit() as f32) < p
            };
            if take {
                mask[i] = 1;
                used_blockers.push(blk);
                total_lp += p.max(1e-7).ln();
            } else {
                mask[i] = 0;
                total_lp += (1.0 - p).max(1e-7).ln();
            }
        }
    }
    (mask, free_mask, visit_seq, total_lp)
}
