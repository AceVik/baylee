//! Policy-head dispatch, separated from ONNX session and memory handling.
use super::*;

impl NetPlayer4 {
    #[allow(
        clippy::too_many_lines,
        reason = "Dispatches ONNX outputs across SingleChoice, SubsetPick, PairPick, and ScalarValue heads"
    )]
    pub(super) fn infer_and_sample(
        &mut self,
        state: &crate::features4::EncodedState4,
        cands: &crate::features4::EncodedCandidates4,
        view: &PlayerView,
        pending: &Pending,
        h_in: &[f32],
    ) -> anyhow::Result<(PlayerAction, f32, f32, TeacherTarget4)> {
        let k_val = self.thinking_steps;
        let (_, cand_logits, pair_logits, scalar_pred, value) =
            self.run_onnx_step(state, cands, h_in, k_val)?;
        let mut target = TeacherTarget4::default();

        let logprob: f32 = match cands.head {
            HeadKind4::SingleChoice => {
                let active_source = self.last_announced_source.get(&view.seat.get()).copied();
                let (chosen_idx, lp) = tactics::select_single_choice(
                    cands,
                    &cand_logits,
                    view,
                    pending,
                    active_source,
                    self.temperature,
                    &mut self.rng,
                );
                target.single_idx = chosen_idx as u16;
                lp
            }
            HeadKind4::SubsetPick => {
                let n = cands.candidates.len();
                let raw_logits = &cand_logits[..n];
                let active_source = self.last_announced_source.get(&view.seat.get()).copied();
                let adj_logits =
                    choices::adjust_subset_logits(cands, raw_logits, view, pending, active_source);
                let (mask, free_mask, seq, _) = sample_subset_constrained(
                    &adj_logits,
                    cands,
                    pending,
                    self.temperature,
                    &mut self.rng,
                );
                let lp = if self.temperature <= 1e-4 {
                    0.0
                } else {
                    let scale = 1.0 / self.temperature;
                    let mut acc = 0.0_f32;
                    for (i, &raw_l) in raw_logits.iter().enumerate().take(n) {
                        if free_mask.get(i).copied().unwrap_or(0) == 1 {
                            let z = raw_l * scale;
                            let picked = mask.get(i).copied().unwrap_or(0) == 1;
                            let step_lp = if picked {
                                if z >= 0.0 {
                                    -((-z).exp().ln_1p())
                                } else {
                                    z - z.exp().ln_1p()
                                }
                            } else if z >= 0.0 {
                                -z - (-z).exp().ln_1p()
                            } else {
                                -z.exp().ln_1p()
                            };
                            acc += step_lp;
                        }
                    }
                    acc
                };
                target.multi_mask = mask;
                target.free_mask = free_mask;
                target.pick_sequence = seq;
                lp
            }
            HeadKind4::PairPick => {
                let n = cands.pairs.len();
                let raw_logits = &pair_logits[..n];
                let adj_logits = choices::adjust_pair_logits(cands, raw_logits, view, pending);
                let (mask, free_mask, seq, _) =
                    sample_pairs_constrained(&adj_logits, cands, self.temperature, &mut self.rng);
                let lp = if self.temperature <= 1e-4 {
                    0.0
                } else {
                    let scale = 1.0 / self.temperature;
                    let mut acc = 0.0_f32;
                    for (i, &raw_l) in raw_logits.iter().enumerate().take(n) {
                        if free_mask.get(i).copied().unwrap_or(0) == 1 {
                            let z = raw_l * scale;
                            let picked = mask.get(i).copied().unwrap_or(0) == 1;
                            let step_lp = if picked {
                                if z >= 0.0 {
                                    -((-z).exp().ln_1p())
                                } else {
                                    z - z.exp().ln_1p()
                                }
                            } else if z >= 0.0 {
                                -z - (-z).exp().ln_1p()
                            } else {
                                -z.exp().ln_1p()
                            };
                            acc += step_lp;
                        }
                    }
                    acc
                };
                target.multi_mask = mask;
                target.free_mask = free_mask;
                target.pick_sequence = seq;
                lp
            }
            HeadKind4::ScalarValue => {
                let pred = match pending {
                    Pending::ChooseNumber {
                        reason:
                            baylee_engine::choice::NumberPrompt::X
                            | baylee_engine::choice::NumberPrompt::ManaPayment { .. },
                        ..
                    } => 1.0,
                    Pending::ChooseNumber {
                        min,
                        max,
                        reason:
                            baylee_engine::choice::NumberPrompt::CombatDamage {
                                recipient,
                                index,
                                of,
                                left,
                                ..
                            },
                        ..
                    } if *index + 1 < *of && *max > *min => {
                        let rem_t = view.object(*recipient).map_or(*max, |o| {
                            u32::try_from(
                                (i32::from(o.toughness.unwrap_or(1)) - i32::from(o.damage)).max(1),
                            )
                            .unwrap_or(1)
                        });
                        let assign = if rem_t <= *left {
                            rem_t.clamp(*min, *max)
                        } else {
                            *min
                        };
                        normalize_scalar_choice(assign, *min, *max)
                    }
                    _ => scalar_pred.clamp(0.0, 1.0),
                };
                let chosen_int =
                    denormalize_scalar_choice(pred, cands.min_choices, cands.max_choices);
                let norm =
                    normalize_scalar_choice(chosen_int, cands.min_choices, cands.max_choices);
                target.scalar_norm = norm;
                // Deterministic regression head: supervised training only.
                0.0
            }
        };

        let mut action = decode_action4(cands, view, pending, &target)
            .context("decode_action4 returned None")?;
        if view.owed.is_some()
            && cands.head == HeadKind4::SingleChoice
            && let Some(tap_src) = tactics::active_source_requires_tap_self(
                view,
                self.last_announced_source.get(&view.seat.get()).copied(),
            )
            && let Some(cand) = cands.candidates.get(target.single_idx as usize)
            && let Some(&alt_id) = cand.group_object_ids.iter().find(|&&oid| oid != tap_src)
        {
            match &mut action {
                PlayerAction::ActivateManaAbility { source }
                | PlayerAction::ActivateAbility { source, .. }
                    if *source == tap_src =>
                {
                    *source = alt_id;
                }
                _ => {}
            }
        }
        if !(matches!(action, PlayerAction::CancelCast) && view.casting.is_some())
            && let Some(fault) = pending.answer_fault(&action)
        {
            anyhow::bail!("decoded action failed answer_fault: {fault:?}");
        }
        Ok((action, value.clamp(0.0, 1.0), logprob, target))
    }
}

pub(super) fn sample_categorical(logits: &[f32], tau: f32, rng: &mut Rng) -> (usize, f32) {
    if logits.is_empty() {
        return (0, 0.0);
    }
    let scale = if tau <= 1e-4 { 1.0 } else { 1.0 / tau };
    let max_l = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut probs = Vec::with_capacity(logits.len());
    let mut sum = 0.0_f32;
    for &l in logits {
        let p = ((l - max_l) * scale).exp();
        probs.push(p);
        sum += p;
    }
    if sum <= 0.0 || !sum.is_finite() {
        return (0, 0.0);
    }
    for p in &mut probs {
        *p /= sum;
    }
    let chosen = if tau <= 1e-4 {
        logits
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map_or(0, |(i, _)| i)
    } else {
        let u = rng.unit() as f32;
        let mut acc = 0.0_f32;
        let mut idx = probs.len() - 1;
        for (i, &p) in probs.iter().enumerate() {
            acc += p;
            if u <= acc {
                idx = i;
                break;
            }
        }
        idx
    };
    let lp = if tau <= 1e-4 {
        0.0
    } else {
        probs[chosen].max(1e-12).ln()
    };
    (chosen, lp)
}
