/// Samples a valid subset mask for `SubsetPick` (`ChooseAttackers`, `ChooseCards`,
/// `ChooseTargets`, `MulliganBottom`, `DiscardChoice`) while tracking exact
/// conditional `free_mask` and `logprob` (`P0-3`).
///
/// # Panics
/// If the internal attacker-count map loses an entry inserted in the same call.
#[allow(
    clippy::too_many_lines,
    reason = "Handles both multi-defender ChooseAttackers and general cardinality-constrained SubsetPick"
)]
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
        let mut greedy_best: BTreeMap<ObjectId, (usize, f32, f32)> = BTreeMap::new();
        for &(i, p) in &order {
            if let Some(id) = cands.candidates[i].object_id {
                let noise = if tau > 1e-4 && tau <= 0.35 {
                    (rng.unit() as f32 - 0.5) * tau * 0.5
                } else {
                    0.0
                };
                let l_noisy = logits[i] + noise;
                let entry = greedy_best
                    .entry(id)
                    .or_insert((i, f32::NEG_INFINITY, 0.0));
                if l_noisy > entry.1 {
                    entry.0 = i;
                    entry.1 = l_noisy;
                }
                entry.2 += p;
            }
        }
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
                    || if tau <= 0.35 {
                        let &(best_i, best_l, p_sum) =
                            greedy_best.get(&id).expect("counted above");
                        i == best_i
                            && (required.contains(&id)
                                || best_l > 0.0
                                || (p_sum > 0.50 && best_l > -0.65))
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
    let greedy_selected: Vec<bool> = if tau <= 0.35 {
        let noisy_logits: Vec<f32> = (0..n)
            .map(|i| {
                if tau > 1e-4 {
                    logits[i] + (rng.unit() as f32 - 0.5) * tau * 0.5
                } else {
                    logits[i]
                }
            })
            .collect();
        let mut ranked: Vec<usize> = (0..n).collect();
        ranked.sort_by(|&a, &b| {
            noisy_logits[b]
                .total_cmp(&noisy_logits[a])
                .then_with(|| a.cmp(&b))
        });
        let mut sel = vec![false; n];
        let best_logit = ranked.first().map_or(0.0, |&idx| noisy_logits[idx]);
        let best_is_opp_player = ranked.first().is_some_and(|&idx| {
            cands.candidates[idx].player_id.is_some() && cands.candidates[idx].rel_seat_idx > 0
        });
        for (rank, &idx) in ranked.iter().enumerate().take(max_k) {
            if rank < min_k {
                sel[idx] = true;
            } else if noisy_logits[idx] > 0.0 && noisy_logits[idx] >= best_logit - 1.25 {
                let is_self_player = cands.candidates[idx].player_id.is_some()
                    && cands.candidates[idx].rel_seat_idx == 0;
                if !(best_is_opp_player && is_self_player) {
                    sel[idx] = true;
                }
            }
        }
        sel
    } else {
        Vec::new()
    };
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
            let take = if tau <= 0.35 {
                greedy_selected[i]
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
///
/// # Panics
/// If the internal blocker-count map loses an entry inserted in the same call.
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

    let mut greedy_blk_best: BTreeMap<u64, (usize, f32, f32)> = BTreeMap::new();
    if tau <= 0.35 {
        for &(i, p) in &order {
            let [b_idx, a_idx] = cands.pairs[i];
            if b_idx < 0 || a_idx < 0 {
                continue;
            }
            let (blk, _atk) = cands.pair_objects[i];
            let key = pack_object_id(blk);
            let noise = if tau > 1e-4 {
                (rng.unit() as f32 - 0.5) * tau * 0.5
            } else {
                0.0
            };
            let l_noisy = logits[i] + noise;
            let entry = greedy_blk_best
                .entry(key)
                .or_insert((i, f32::NEG_INFINITY, 0.0));
            if l_noisy > entry.1 {
                entry.0 = i;
                entry.1 = l_noisy;
            }
            entry.2 += p;
        }
    }

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
            let take = if tau <= 0.35 {
                let &(best_i, best_l, p_sum) = greedy_blk_best
                    .get(&pack_object_id(blk))
                    .expect("counted above");
                i == best_i && (best_l > 0.0 || (p_sum > 0.50 && best_l > -0.65))
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
