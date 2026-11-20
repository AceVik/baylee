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
/// while enforcing at-most-one-attacker-per-blocker and the counts each
/// attacker states (`ChooseBlockers::bounds`, [`meet_attacker_bounds`]), and
/// tracking `free_mask` and `logprob` for the draw before the counts are
/// repaired. The executed action can receive mass from several draws, even
/// when the sampled mask already met the counts; returns `None` if repaired.
///
/// # Panics
/// If the internal blocker-count map loses an entry inserted in the same call.
pub fn sample_pairs_constrained(
    logits: &[f32],
    cands: &EncodedCandidates4,
    pending: &Pending,
    tau: f32,
    rng: &mut crate::deckgen::Rng,
) -> (Vec<u8>, Vec<u8>, Vec<u16>, Option<f32>) {
    let (mut mask, free_mask, visit_seq, total_lp) = sample_pairs_free(logits, cands, tau, rng);
    let changed = meet_attacker_bounds(&mut mask, logits, cands, pending);
    (mask, free_mask, visit_seq, (!changed).then_some(total_lp))
}

/// Whether the counts the attackers state (`ChooseBlockers::bounds`) can move
/// some per-pair draw at this question ([`meet_attacker_bounds`]): an
/// attacker that needs two or more blockers and that a creature may block
/// (a lone blocker on it is moved), or one that takes fewer blockers than
/// the creatures that may block it. Where they can, an answer of
/// [`sample_pairs_constrained`] may also be reached from other draws, so its
/// probability is more than its own draw's; that sum is not computed.
#[must_use]
pub fn attacker_counts_may_move(cands: &EncodedCandidates4, pending: &Pending) -> bool {
    let Pending::ChooseBlockers { bounds, .. } = pending else {
        return false;
    };
    let n = cands.pairs.len().min(cands.pair_objects.len());
    bounds.iter().any(|bound| {
        let mut may_block: Vec<ObjectId> = Vec::new();
        for i in 0..n {
            let [b_idx, a_idx] = cands.pairs[i];
            let (blocker, attacker) = cands.pair_objects[i];
            if b_idx >= 0
                && a_idx >= 0
                && attacker == bound.attacker
                && !may_block.contains(&blocker)
            {
                may_block.push(blocker);
            }
        }
        let min = usize::try_from(bound.min_blockers).unwrap_or(usize::MAX);
        let max = usize::try_from(bound.max_blockers).unwrap_or(usize::MAX);
        (min > 1 && !may_block.is_empty()) || max < may_block.len()
    })
}

/// Whether `mask` meets every count the attackers state: each attacker it
/// blocks has at least its least and at most its most blockers, so
/// [`meet_attacker_bounds`] leaves it as it stands.
fn meets_attacker_bounds(mask: &[u8], cands: &EncodedCandidates4, pending: &Pending) -> bool {
    let Pending::ChooseBlockers { bounds, .. } = pending else {
        return true;
    };
    bounds.iter().all(|bound| {
        let on = (0..mask.len())
            .filter(|&i| mask[i] == 1 && cands.pair_objects[i].1 == bound.attacker)
            .count();
        let min = usize::try_from(bound.min_blockers).unwrap_or(usize::MAX);
        let max = usize::try_from(bound.max_blockers).unwrap_or(usize::MAX);
        on == 0 || (min..=max).contains(&on)
    })
}

/// An attacker blocked by fewer creatures than it states (menace: "can't be
/// blocked except by two or more", CR 702.111b) or by more is no answer the
/// engine takes (`AnswerFault::TooFewBlockers`, `TooManyBlockers`). Where a
/// sample leaves one so, the creatures still free that may block it join it,
/// best logit first, up to its least count; where too few are free it is left
/// unblocked, and blockers past its most go, worst logit first. Says whether
/// it changed the sample.
fn meet_attacker_bounds(
    mask: &mut [u8],
    logits: &[f32],
    cands: &EncodedCandidates4,
    pending: &Pending,
) -> bool {
    let Pending::ChooseBlockers { bounds, .. } = pending else {
        return false;
    };
    let drawn = mask.to_vec();
    let n = mask.len();
    let blocker_of = |i: usize| cands.pair_objects[i].0;
    let attacker_of = |i: usize| cands.pair_objects[i].1;
    let by_logit = |picks: &mut Vec<usize>| {
        picks.sort_by(|a, b| logits[*b].total_cmp(&logits[*a]).then(a.cmp(b)));
    };
    for bound in bounds {
        let min = usize::try_from(bound.min_blockers).unwrap_or(usize::MAX);
        let max = usize::try_from(bound.max_blockers).unwrap_or(usize::MAX);
        let mut on: Vec<usize> = (0..n)
            .filter(|&i| mask[i] == 1 && attacker_of(i) == bound.attacker)
            .collect();
        if on.is_empty() {
            continue;
        }
        if on.len() < min {
            let used: Vec<ObjectId> = (0..n).filter(|&i| mask[i] == 1).map(blocker_of).collect();
            let mut free: Vec<usize> = (0..n)
                .filter(|&i| {
                    cands.pairs[i][0] >= 0
                        && cands.pairs[i][1] >= 0
                        && attacker_of(i) == bound.attacker
                        && !used.contains(&blocker_of(i))
                })
                .collect();
            by_logit(&mut free);
            let mut joined: Vec<ObjectId> = Vec::new();
            for i in free {
                if on.len() >= min {
                    break;
                }
                if !joined.contains(&blocker_of(i)) {
                    joined.push(blocker_of(i));
                    on.push(i);
                }
            }
            if on.len() < min {
                for &i in &on {
                    mask[i] = 0;
                }
                continue;
            }
            for &i in &on {
                mask[i] = 1;
            }
        }
        if on.len() > max {
            by_logit(&mut on);
            for &i in &on[max..] {
                mask[i] = 0;
            }
        }
    }
    mask[..] != drawn[..]
}

/// The per-pair sample, one attacker per blocker, before the attackers'
/// counts are met.
fn sample_pairs_free(
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

/// The free bits under which [`sample_subset_constrained`] draws exactly
/// `mask` above sampler 2's boundary, or `None` when it never draws it (a
/// bit its constraints force otherwise). With the canonical visit order
/// `0..n` the sampler records, these are the conditional bits under which the
/// probability of an answer the sampler did not draw is read, such as the
/// tactical layer's answer an explorer plays in its place (`netplay4::explore`).
#[must_use]
pub fn subset_free_mask_of(
    cands: &EncodedCandidates4,
    pending: &Pending,
    mask: &[u8],
) -> Option<Vec<u8>> {
    let n = mask.len();
    if n > cands.candidates.len() || mask.iter().any(|&bit| bit > 1) {
        return None;
    }
    let mut free_mask = vec![0_u8; n];
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
        for (i, candidate) in cands.candidates[..n].iter().enumerate() {
            let Some(id) = candidate.object_id else {
                if mask[i] != 0 {
                    return None;
                }
                continue;
            };
            let (can_take, can_skip) = subset.options(id);
            let free = can_take && can_skip;
            let take = if free {
                mask[i] == 1
            } else {
                can_take && !can_skip
            };
            if u8::from(take) != mask[i] {
                return None;
            }
            free_mask[i] = u8::from(free);
            if take {
                subset.take(id);
            }
        }
        return Some(free_mask);
    }
    let min_k = (cands.min_choices as usize).min(n);
    let max_k = (cands.max_choices as usize).min(n);
    let mut picked = 0_usize;
    for (i, &bit) in mask.iter().enumerate() {
        let rem = n - i;
        if picked >= max_k {
            if bit != 0 {
                return None;
            }
        } else if picked + rem <= min_k {
            if bit != 1 {
                return None;
            }
            picked += 1;
        } else {
            free_mask[i] = 1;
            picked += usize::from(bit);
        }
    }
    Some(free_mask)
}

/// As [`subset_free_mask_of`], for [`sample_pairs_constrained`]: a pair
/// whose blocker is already assigned, or that names an unencoded object, is
/// never free and never taken; and a mask that misses a count an attacker
/// states is never drawn as it stands, since [`meet_attacker_bounds`] moves
/// it. These are the bits of the draw that stands as `mask`; where the counts
/// can move other draws into it ([`attacker_counts_may_move`]), its
/// probability under the sampler is more than that draw's.
#[must_use]
pub fn pair_free_mask_of(
    cands: &EncodedCandidates4,
    pending: &Pending,
    mask: &[u8],
) -> Option<Vec<u8>> {
    let n = mask.len();
    if n > cands.pair_objects.len()
        || n > cands.pairs.len()
        || mask.iter().any(|&bit| bit > 1)
        || !meets_attacker_bounds(mask, cands, pending)
    {
        return None;
    }
    let mut free_mask = vec![0_u8; n];
    let mut used_blockers = Vec::new();
    for (i, &bit) in mask.iter().enumerate() {
        let [b_idx, a_idx] = cands.pairs[i];
        let (blocker, _) = cands.pair_objects[i];
        if b_idx < 0 || a_idx < 0 || used_blockers.contains(&blocker) {
            if bit != 0 {
                return None;
            }
            continue;
        }
        free_mask[i] = 1;
        if bit == 1 {
            used_blockers.push(blocker);
        }
    }
    Some(free_mask)
}
