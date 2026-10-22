/// Computes the binary `free_mask` (`1` = free Bernoulli decision, `0` = forced
/// or forbidden by constraints, `P0-3`) when visiting `cands.candidates` in
/// `visit_order` (completed with any remaining indices in `0..N`).
///
/// # Panics
/// If the internal attacker-count map loses an entry inserted in the same call.
#[must_use]
pub fn compute_subset_free_mask(
    cands: &EncodedCandidates4,
    pending: &Pending,
    multi_mask: &[u8],
    visit_order: &[u16],
) -> Vec<u8> {
    let n = cands.candidates.len();
    let mut free_mask = vec![0_u8; n];
    if n == 0 {
        return free_mask;
    }
    let mut order: Vec<usize> = Vec::with_capacity(n);
    let mut in_order = vec![false; n];
    for &idx in visit_order {
        let i = idx as usize;
        if i < n && !in_order[i] {
            in_order[i] = true;
            order.push(i);
        }
    }
    for (i, &visited) in in_order.iter().enumerate() {
        if !visited {
            order.push(i);
        }
    }

    if let Pending::ChooseAttackers { required, .. } = pending {
        let mut rem_for_creature: BTreeMap<u64, usize> = BTreeMap::new();
        for cand in &cands.candidates {
            if let Some(oid) = cand.object_id {
                *rem_for_creature.entry(pack_object_id(oid)).or_insert(0) += 1;
            }
        }
        let mut chosen_creatures: Vec<u64> = Vec::new();
        for i in order {
            let Some(oid) = cands.candidates[i].object_id else {
                continue;
            };
            let key = pack_object_id(oid);
            let rem_c = rem_for_creature.get_mut(&key).expect("counted above");
            let is_last_for_c = *rem_c <= 1;
            *rem_c = rem_c.saturating_sub(1);

            if chosen_creatures.contains(&key) {
                free_mask[i] = 0;
            } else if required.contains(&oid) && is_last_for_c {
                free_mask[i] = 0;
                chosen_creatures.push(key);
            } else {
                free_mask[i] = 1;
                if multi_mask.get(i).copied().unwrap_or(0) != 0 {
                    chosen_creatures.push(key);
                }
            }
        }
        return free_mask;
    }

    let min_k = (cands.min_choices as usize).min(n);
    let max_k = (cands.max_choices as usize).min(n);
    let mut picked = 0_usize;
    for (pos, i) in order.into_iter().enumerate() {
        let rem = n - pos;
        if picked >= max_k {
            free_mask[i] = 0;
        } else if picked + rem <= min_k {
            free_mask[i] = 0;
            picked += 1;
        } else {
            free_mask[i] = 1;
            if multi_mask.get(i).copied().unwrap_or(0) != 0 {
                picked += 1;
            }
        }
    }
    free_mask
}

/// Computes the binary `free_mask` (`1` = free Bernoulli decision, `0` =
/// forbidden because blocker is already assigned, `P0-3`) for `PairPick` when
/// visiting `cands.pair_objects` in `visit_order`.
#[must_use]
pub fn compute_pair_free_mask(
    cands: &EncodedCandidates4,
    multi_mask: &[u8],
    visit_order: &[u16],
) -> Vec<u8> {
    let n = cands.pair_objects.len();
    let mut free_mask = vec![0_u8; n];
    if n == 0 {
        return free_mask;
    }
    let mut order: Vec<usize> = Vec::with_capacity(n);
    let mut in_order = vec![false; n];
    for &idx in visit_order {
        let i = idx as usize;
        if i < n && !in_order[i] {
            in_order[i] = true;
            order.push(i);
        }
    }
    for (i, &visited) in in_order.iter().enumerate() {
        if !visited {
            order.push(i);
        }
    }

    let mut used_blockers: Vec<ObjectId> = Vec::new();
    for i in order {
        let (blk, _atk) = cands.pair_objects[i];
        if used_blockers.contains(&blk) {
            free_mask[i] = 0;
        } else {
            free_mask[i] = 1;
            if multi_mask.get(i).copied().unwrap_or(0) != 0 {
                used_blockers.push(blk);
            }
        }
    }
    free_mask
}

/// Normalizes an integer `v` in `[min, max]` into `[0.0, 1.0]`, using linear
/// scaling when `max - min <= 20` and log-scaling when `max - min > 20` (`P1-8`).
#[must_use]
pub fn normalize_scalar_choice(v: u32, min: u32, max: u32) -> f32 {
    let lo = min;
    let hi = max.max(lo);
    let span = hi.saturating_sub(lo);
    if span == 0 {
        return 0.0;
    }
    let offset = v.clamp(lo, hi).saturating_sub(lo);
    if span <= 20 {
        (offset as f32 / span as f32).clamp(0.0, 1.0)
    } else {
        ((f64::from(offset).ln_1p() / f64::from(span).ln_1p()) as f32).clamp(0.0, 1.0)
    }
}

/// Inverts [`normalize_scalar_choice`] back to an integer in `[min, max]`.
#[must_use]
pub fn denormalize_scalar_choice(scalar_norm: f32, min: u32, max: u32) -> u32 {
    let lo = min;
    let hi = max.max(lo);
    let span = hi.saturating_sub(lo);
    if span == 0 {
        return lo;
    }
    let norm = scalar_norm.clamp(0.0, 1.0);
    let offset = if span <= 20 {
        (norm * span as f32).round() as u32
    } else {
        ((f64::from(norm) * f64::from(span).ln_1p()).exp_m1().round() as u32).min(span)
    };
    lo.saturating_add(offset).clamp(lo, hi)
}

/// Encodes a ground-truth teacher or rollout [`PlayerAction`] into a
/// [`TeacherTarget4`] against `cands`.
///
/// Never falls back to `single_idx = 0` when an action is missing from `cands`:
/// returns `None` so `convert4` increments `unencoded_decisions` honestly (`P0-2`).
#[must_use]
#[allow(clippy::too_many_lines, reason = "Explicit action-to-label mapping for each Pending variant")]
pub fn encode_teacher_action4(
    cands: &EncodedCandidates4,
    pending: &Pending,
    action: &PlayerAction,
) -> Option<TeacherTarget4> {
    let mut target = TeacherTarget4::default();
    match cands.head {
        HeadKind4::SingleChoice => {
            if let Some(idx) = cands
                .candidates
                .iter()
                .position(|c| c.action.as_ref() == Some(action))
            {
                target.single_idx = idx as u16;
                return Some(target);
            }
            // Match ChooseObjects / ChooseTargets for single-target questions
            if let (
                Pending::ChooseTargets { .. },
                PlayerAction::ChooseObjects { objects }
                | PlayerAction::ChooseTargets { objects, .. },
            ) = (pending, action)
            {
                if objects.is_empty()
                    && let Some(idx) = cands.candidates.iter().position(|c| {
                        matches!(
                            &c.action,
                            Some(PlayerAction::ChooseTargets { objects: o, players: p })
                                if o.is_empty() && p.is_empty()
                        )
                    })
                {
                    target.single_idx = idx as u16;
                    return Some(target);
                }
                if let Some(&first_oid) = objects.first()
                    && let Some(idx) = cands.candidates.iter().position(|c| {
                        matches!(
                            &c.action,
                            Some(PlayerAction::ChooseTargets { objects: o, .. })
                                if o.first() == Some(&first_oid)
                        )
                    })
                {
                    target.single_idx = idx as u16;
                    return Some(target);
                }
            }
            if let (Pending::Arrange { .. }, PlayerAction::Arrange { piles: act_piles }) =
                (pending, action)
                && let Some(idx) = cands.candidates.iter().position(|c| {
                    let Some(PlayerAction::Arrange { piles: cand_piles }) = &c.action else {
                        return false;
                    };
                    cand_piles.len() == act_piles.len()
                        && cand_piles.iter().zip(act_piles.iter()).all(|(cp, ap)| {
                            cp.len() == ap.len() && ap.iter().all(|oid| cp.contains(oid))
                        })
                })
            {
                target.single_idx = idx as u16;
                return Some(target);
            }
            None
        }

        HeadKind4::SubsetPick => {
            let n = cands.candidates.len();
            if n == 0 {
                return None;
            }
            target.multi_mask = vec![0; n];
            match action {
                PlayerAction::DeclareAttackers { attackers } => {
                    for &(oid, def) in attackers {
                        let maybe_idx = cands
                            .candidates
                            .iter()
                            .position(|c| c.object_id == Some(oid) && c.defender == Some(def))
                            .or_else(|| {
                                cands
                                    .candidates
                                    .iter()
                                    .position(|c| c.object_id == Some(oid))
                            });
                        if let Some(idx) = maybe_idx {
                            target.multi_mask[idx] = 1;
                            target.pick_sequence.push(idx as u16);
                        } else if cands.candidates.len() < MAX_CANDIDATES {
                            return None;
                        }
                    }
                    if !attackers.is_empty() && target.pick_sequence.is_empty() {
                        return None;
                    }
                    target.free_mask = compute_subset_free_mask(
                        cands,
                        pending,
                        &target.multi_mask,
                        &target.pick_sequence,
                    );
                    Some(target)
                }
                PlayerAction::ChooseObjects { objects } => {
                    for oid in objects {
                        let idx = cands
                            .candidates
                            .iter()
                            .position(|c| c.object_id == Some(*oid))?;
                        target.multi_mask[idx] = 1;
                        target.pick_sequence.push(idx as u16);
                    }
                    target.free_mask = compute_subset_free_mask(
                        cands,
                        pending,
                        &target.multi_mask,
                        &target.pick_sequence,
                    );
                    Some(target)
                }
                PlayerAction::ChooseTargets { objects, players } => {
                    for oid in objects {
                        let idx = cands
                            .candidates
                            .iter()
                            .position(|c| c.object_id == Some(*oid))?;
                        target.multi_mask[idx] = 1;
                        target.pick_sequence.push(idx as u16);
                    }
                    for pid in players {
                        let idx = cands
                            .candidates
                            .iter()
                            .position(|c| c.player_id == Some(*pid))?;
                        target.multi_mask[idx] = 1;
                        target.pick_sequence.push(idx as u16);
                    }
                    target.free_mask = compute_subset_free_mask(
                        cands,
                        pending,
                        &target.multi_mask,
                        &target.pick_sequence,
                    );
                    Some(target)
                }
                _ => None,
            }
        }

        HeadKind4::PairPick => {
            let n = cands.pair_objects.len();
            target.multi_mask = vec![0; n];
            if let PlayerAction::DeclareBlockers { blockers } = action {
                for &(blk, atk) in blockers {
                    if let Some(idx) = cands
                        .pair_objects
                        .iter()
                        .position(|&(b, a)| b == blk && a == atk)
                    {
                        target.multi_mask[idx] = 1;
                        target.pick_sequence.push(idx as u16);
                    } else if cands.pair_objects.len() < MAX_PAIRS {
                        return None;
                    }
                }
                if !blockers.is_empty() && target.pick_sequence.is_empty() {
                    return None;
                }
                target.free_mask =
                    compute_pair_free_mask(cands, &target.multi_mask, &target.pick_sequence);
                return Some(target);
            }
            None
        }

        HeadKind4::ScalarValue => {
            if let PlayerAction::ChooseNumber(v) = action {
                if *v < cands.min_choices || *v > cands.max_choices {
                    return None;
                }
                target.scalar_norm =
                    normalize_scalar_choice(*v, cands.min_choices, cands.max_choices);
                return Some(target);
            }
            None
        }
    }
}

/// Decodes a [`TeacherTarget4`] (or model prediction) back into a legal
/// [`PlayerAction`] for `pending`.
#[must_use]
#[allow(clippy::too_many_lines, reason = "Inverse label-to-action mapping for each Pending variant")]
pub fn decode_action4(
    cands: &EncodedCandidates4,
    view: &PlayerView,
    pending: &Pending,
    choice: &TeacherTarget4,
) -> Option<PlayerAction> {
    match cands.head {
        HeadKind4::SingleChoice => cands
            .candidates
            .get(choice.single_idx as usize)
            .and_then(|c| c.action.clone()),

        HeadKind4::SubsetPick => match pending {
            Pending::ChooseAttackers {
                attackers,
                defenders,
                required,
                limits,
                ..
            } => {
                let default_def = choose_best_attack_target(view, defenders)?;
                let mut chosen: Vec<(ObjectId, Defender)> = Vec::new();
                let def_for = |cid: ObjectId| -> Defender {
                    if let Some(lim) = limits.iter().find(|l| l.creature == cid)
                        && !lim.defenders.contains(&default_def)
                        && let Some(&first_d) = lim.defenders.first()
                    {
                        return first_d;
                    }
                    default_def
                };
                for (idx, cand) in cands.candidates.iter().enumerate() {
                    if choice.multi_mask.get(idx).copied().unwrap_or(0) != 0
                        && let Some(oid) = cand.object_id
                        && !chosen.iter().any(|(c, _)| *c == oid)
                    {
                        let def = cand.defender.unwrap_or_else(|| def_for(oid));
                        chosen.push((oid, def));
                    }
                }
                for &req in required {
                    if attackers.contains(&req) && !chosen.iter().any(|(c, _)| *c == req) {
                        chosen.push((req, def_for(req)));
                    }
                }
                Some(PlayerAction::DeclareAttackers { attackers: chosen })
            }
            Pending::MulliganBottom { count, .. } | Pending::DiscardChoice { count, .. } => {
                let need = usize::from(*count);
                let mut objects: Vec<ObjectId> = Vec::new();
                for (idx, cand) in cands.candidates.iter().enumerate() {
                    if choice.multi_mask.get(idx).copied().unwrap_or(0) != 0
                        && let Some(oid) = cand.object_id
                        && objects.len() < need
                    {
                        objects.push(oid);
                    }
                }
                for cand in &cands.candidates {
                    if objects.len() >= need {
                        break;
                    }
                    if let Some(oid) = cand.object_id
                        && !objects.contains(&oid)
                    {
                        objects.push(oid);
                    }
                }
                Some(PlayerAction::ChooseObjects { objects })
            }
            Pending::ChooseCards { min, max, .. } => {
                let mut objects: Vec<ObjectId> = Vec::new();
                for (idx, cand) in cands.candidates.iter().enumerate() {
                    if choice.multi_mask.get(idx).copied().unwrap_or(0) != 0
                        && let Some(oid) = cand.object_id
                        && objects.len() < usize::from(*max)
                    {
                        objects.push(oid);
                    }
                }
                if objects.len() < usize::from(*min) {
                    for cand in &cands.candidates {
                        if objects.len() >= usize::from(*min) {
                            break;
                        }
                        if let Some(oid) = cand.object_id
                            && !objects.contains(&oid)
                        {
                            objects.push(oid);
                        }
                    }
                }
                Some(PlayerAction::ChooseObjects { objects })
            }
            Pending::ChooseTargets { min, max, .. } => {
                let mut objects: Vec<ObjectId> = Vec::new();
                let mut players: Vec<PlayerId> = Vec::new();
                let max_n = *max as usize;
                let min_n = *min as usize;
                for (idx, cand) in cands.candidates.iter().enumerate() {
                    if choice.multi_mask.get(idx).copied().unwrap_or(0) != 0
                        && (objects.len() + players.len()) < max_n
                    {
                        if let Some(oid) = cand.object_id {
                            objects.push(oid);
                        } else if let Some(pid) = cand.player_id {
                            players.push(pid);
                        }
                    }
                }
                if (objects.len() + players.len()) < min_n {
                    for cand in &cands.candidates {
                        if (objects.len() + players.len()) >= min_n {
                            break;
                        }
                        if let Some(oid) = cand.object_id {
                            if !objects.contains(&oid) {
                                objects.push(oid);
                            }
                        } else if let Some(pid) = cand.player_id
                            && !players.contains(&pid)
                        {
                            players.push(pid);
                        }
                    }
                }
                Some(PlayerAction::ChooseTargets { objects, players })
            }
            _ => None,
        },

        HeadKind4::PairPick => {
            if let Pending::ChooseBlockers { .. } = pending {
                let mut blockers: Vec<(ObjectId, ObjectId)> = Vec::new();
                for (idx, &(blk, atk)) in cands.pair_objects.iter().enumerate() {
                    if choice.multi_mask.get(idx).copied().unwrap_or(0) != 0
                        && !blockers.iter().any(|&(b, _)| b == blk)
                    {
                        blockers.push((blk, atk));
                    }
                }
                return Some(PlayerAction::DeclareBlockers { blockers });
            }
            None
        }

        HeadKind4::ScalarValue => match pending {
            Pending::ChooseNumber { min, max, .. } => {
                let v = denormalize_scalar_choice(choice.scalar_norm, *min, *max);
                Some(PlayerAction::ChooseNumber(v))
            }
            _ => None,
        },
    }
}

fn choose_best_attack_target(view: &PlayerView, targets: &[Defender]) -> Option<Defender> {
    // Prefer an opponent player who can be killed or has the lowest positive life
    let mut best: Option<(i32, Defender)> = None;
    for &def in targets {
        let score = match def {
            Defender::Player(pid) => view.seats.get(pid.get() as usize).map_or(20, |s| -s.life),
            Defender::Planeswalker(_) | Defender::Battle(_) => 0,
        };
        if best.is_none_or(|(b_score, _)| score > b_score) {
            best = Some((score, def));
        }
    }
    best.map(|(_, d)| d)
}
