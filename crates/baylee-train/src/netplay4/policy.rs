//! Policy-head dispatch, separated from ONNX session and memory handling.
use super::*;

/// Which decision policy produced a successful non-forced answer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyMode4 {
    /// Existing playtest heuristics; demonstrations, never neural PPO behavior.
    #[default]
    Tactical,
    /// Unmodified model outputs with the version-2 constrained sampler.
    Raw,
}

impl PolicyMode4 {
    /// Stable provenance label for evaluation/collection metadata.
    #[must_use]
    pub const fn contract(self) -> &'static str {
        match self {
            Self::Tactical => "tactical_house_demonstrations_v1",
            Self::Raw => "raw_neural_sampler2_v1",
        }
    }

    pub(super) fn provenance(
        self,
        temperature: f32,
        thinking_steps: u8,
        head: HeadKind4,
        logprob: f32,
    ) -> StepProvenance4 {
        let neural = self == Self::Raw;
        let has_logprob = neural
            && temperature > 0.35
            && valid_raw_temperature(temperature)
            && head != HeadKind4::ScalarValue;
        StepProvenance4 {
            source: if neural {
                SourceKind4::Net
            } else {
                SourceKind4::House
            },
            temperature,
            thinking_steps,
            behavior_logprob: if has_logprob { logprob } else { 0.0 },
            has_logprob,
        }
    }
}

/// Version 2 is stochastic for every policy head only above this boundary.
#[must_use]
pub fn valid_raw_temperature(temperature: f32) -> bool {
    temperature.is_finite() && (temperature == 0.0 || temperature > 0.35)
}

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
        PolicySampler4 {
            mode: self.policy_mode,
            temperature: self.temperature,
            rng: &mut self.rng,
            active_source: self.last_announced_source.get(&view.seat.get()).copied(),
        }
        .select(
            cands,
            view,
            pending,
            PolicyOutputs4 {
                cand_logits: &cand_logits,
                pair_logits: &pair_logits,
                scalar_pred,
                value,
            },
        )
    }
}

#[derive(Clone, Copy)]
struct PolicyOutputs4<'a> {
    cand_logits: &'a [f32],
    pair_logits: &'a [f32],
    scalar_pred: f32,
    value: f32,
}

struct PolicySampler4<'a> {
    mode: PolicyMode4,
    temperature: f32,
    rng: &'a mut Rng,
    active_source: Option<(ObjectId, Option<u32>)>,
}

impl PolicySampler4<'_> {
    #[allow(
        clippy::too_many_lines,
        reason = "Dispatches four existing policy heads without altering their candidate contract"
    )]
    fn select(
        &mut self,
        cands: &EncodedCandidates4,
        view: &PlayerView,
        pending: &Pending,
        outputs: PolicyOutputs4<'_>,
    ) -> anyhow::Result<(PlayerAction, f32, f32, TeacherTarget4)> {
        anyhow::ensure!(
            self.mode != PolicyMode4::Raw || valid_raw_temperature(self.temperature),
            "raw policy requires temperature 0 or >0.35 under sampler version 2"
        );
        let PolicyOutputs4 {
            cand_logits,
            pair_logits,
            scalar_pred,
            value,
        } = outputs;
        if self.mode == PolicyMode4::Raw {
            let logits = match cands.head {
                HeadKind4::SingleChoice | HeadKind4::SubsetPick => {
                    cand_logits.get(..cands.candidates.len())
                }
                HeadKind4::PairPick => pair_logits.get(..cands.pairs.len()),
                HeadKind4::ScalarValue => Some(&[][..]),
            };
            anyhow::ensure!(
                value.is_finite()
                    && logits.is_some_and(|ls| ls.iter().all(|l| l.is_finite()))
                    && (cands.head != HeadKind4::ScalarValue || scalar_pred.is_finite()),
                "raw policy received invalid model outputs"
            );
        }
        let mut target = TeacherTarget4::default();

        let logprob: f32 = match cands.head {
            HeadKind4::SingleChoice => {
                let active_source = self.active_source;
                let (chosen_idx, lp) = if self.mode == PolicyMode4::Raw {
                    sample_categorical(
                        &cand_logits[..cands.candidates.len()],
                        self.temperature,
                        self.rng,
                    )
                } else {
                    tactics::select_single_choice(
                        cands,
                        cand_logits,
                        view,
                        pending,
                        active_source,
                        self.temperature,
                        self.rng,
                    )
                };
                target.single_idx = chosen_idx as u16;
                lp
            }
            HeadKind4::SubsetPick => {
                let n = cands.candidates.len();
                let raw_logits = &cand_logits[..n];
                let active_source = self.active_source;
                let adj_logits = if self.mode == PolicyMode4::Raw {
                    raw_logits.to_vec()
                } else {
                    choices::adjust_subset_logits(cands, raw_logits, view, pending, active_source)
                };
                let (mask, free_mask, seq, _) = sample_subset_constrained(
                    &adj_logits,
                    cands,
                    pending,
                    self.temperature,
                    self.rng,
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
                let adj_logits = if self.mode == PolicyMode4::Raw {
                    raw_logits.to_vec()
                } else {
                    choices::adjust_pair_logits(cands, raw_logits, view, pending)
                };
                let (mask, free_mask, seq, _) =
                    sample_pairs_constrained(&adj_logits, cands, self.temperature, self.rng);
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
                let pred = if self.mode == PolicyMode4::Raw {
                    scalar_pred.clamp(0.0, 1.0)
                } else {
                    match pending {
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
                                    (i32::from(o.toughness.unwrap_or(1)) - i32::from(o.damage))
                                        .max(1),
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
                    }
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
        // The policy selects an equivalence group, not a particular copy.
        // A still-unpaid TapSelf activation reserves one member even though
        // its mana actions remain grouped. Resolve that group deterministically
        // using our own announced intent in BOTH modes; the index/LP is unchanged.
        if view.owed.is_some()
            && cands.head == HeadKind4::SingleChoice
            && let Some(tap_src) =
                tactics::active_source_requires_tap_self(view, self.active_source)
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
        if self.mode == PolicyMode4::Raw {
            anyhow::ensure!(
                decoded_matches_sample(cands, pending, &action, &target),
                "raw decoded action differs from sampled target (decoder repair)"
            );
        }
        if !(matches!(action, PlayerAction::CancelCast) && view.casting.is_some())
            && let Some(fault) = pending.answer_fault(&action)
        {
            anyhow::bail!("decoded action failed answer_fault: {fault:?}");
        }
        Ok((action, value.clamp(0.0, 1.0), logprob, target))
    }
}

fn decoded_matches_sample(
    cands: &EncodedCandidates4,
    pending: &Pending,
    action: &PlayerAction,
    target: &TeacherTarget4,
) -> bool {
    let Some(encoded) = crate::features4::encode_executed_action4(cands, pending, action) else {
        return false;
    };
    match cands.head {
        HeadKind4::SingleChoice => encoded.single_idx == target.single_idx,
        // Include forced bits: a decoder must not fill missing selections.
        // Re-encoding has a different visitation representation, so compare the
        // chosen action mask, retaining the sampler's original free/order/LP.
        HeadKind4::SubsetPick | HeadKind4::PairPick => encoded.multi_mask == target.multi_mask,
        HeadKind4::ScalarValue => encoded.scalar_norm.to_bits() == target.scalar_norm.to_bits(),
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
        ((logits[chosen] - max_l) * scale) - sum.ln()
    };
    (chosen, lp)
}

#[cfg(test)]
mod payment_tests;
#[cfg(test)]
mod questions_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod grouped_attack_tests;
