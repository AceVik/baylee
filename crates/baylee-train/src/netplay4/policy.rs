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

    /// `logprob` is the answer's log-probability under the sampler, `None`
    /// where the answer is no plain draw from it. Only a single choice keeps
    /// one, and only where its whole menu has a one-to-one action mapping
    /// (`features4::supports_behavior_probability4`, which the decision
    /// checks next).
    pub(super) fn provenance(
        self,
        temperature: f32,
        thinking_steps: u8,
        head: HeadKind4,
        logprob: Option<f32>,
    ) -> StepProvenance4 {
        let neural = self == Self::Raw;
        let behavior_logprob = logprob.filter(|_| {
            neural
                && temperature > 0.35
                && valid_raw_temperature(temperature)
                && head == HeadKind4::SingleChoice
        });
        StepProvenance4 {
            source: if neural {
                SourceKind4::Net
            } else {
                SourceKind4::House
            },
            temperature,
            thinking_steps,
            behavior_logprob: behavior_logprob.unwrap_or(0.0),
            has_logprob: behavior_logprob.is_some(),
        }
    }
}

/// A sampled answer: the action, its value, its behavior log-probability
/// (`None` where the answer is no plain draw from the sampler) and its
/// target.
pub(super) type Sampled4 = (PlayerAction, f32, Option<f32>, TeacherTarget4);

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
    /// Runs the decision step and samples from it: the answer, its value,
    /// its behavior log-probability and target, and the raw candidate
    /// logits the sample was drawn from (which a lookahead ranks).
    pub(super) fn infer_and_sample(
        &mut self,
        state: &crate::features4::EncodedState4,
        cands: &crate::features4::EncodedCandidates4,
        view: &PlayerView,
        pending: &Pending,
        h_in: &[f32],
    ) -> anyhow::Result<(Sampled4, Vec<f32>)> {
        self.last_grouping = None;
        let k_val = self.thinking_steps;
        let mut outputs = self.run_onnx_step(state, cands, h_in, k_val)?;
        self.last_depth = k_val;
        // A decision does not change the memory, so a second pass reads the
        // same state; only a real choice is worth one.
        if let Some(adaptive) = self.adaptive
            && cands.candidates.len() >= 2
        {
            let confidence = outputs
                .confidence
                .context("the graph stated no confidence in its answer")?;
            let again = adaptive.escalates(confidence);
            self.thinking_use.record(confidence, again);
            if again {
                outputs = self.run_onnx_step(state, cands, h_in, adaptive.deep)?;
                self.last_depth = adaptive.deep;
            }
        }
        let super::StepOutputs {
            mut cand_logits,
            pair_logits,
            scalar_pred,
            value,
            embeddings,
            ..
        } = outputs;
        if let Some(menu) = &mut self.payable_menu {
            menu.weigh(&mut cand_logits, cands);
        }
        let grouping = self.group_priority(cands, view, pending, &mut cand_logits);
        self.last_grouping = grouping;
        if self.last_menu.is_some() {
            let mut trace = menu_trace(cands, &cand_logits, &pair_logits);
            if let Some(grouping) = grouping {
                trace["grouped_decode"] = serde_json::json!(grouping);
            }
            self.last_menu = Some(trace);
        }
        let sampled = PolicySampler4 {
            mode: self.policy_mode,
            temperature: self.temperature,
            rng: &mut self.rng,
            active_source: self.last_announced_source.get(&view.seat.get()).copied(),
            guard: &mut self.choice_guard,
            rules: &mut self.tactic_rules,
            tactician: self.tactician.as_mut(),
            distill: self.distill.as_mut(),
            explorer: self.explorer.as_mut(),
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
                embeddings: embeddings.as_ref(),
            },
        )?;
        Ok((sampled, cand_logits))
    }
}

impl NetPlayer4 {
    /// `--grouped-decode`: at a priority question with pass and act on the
    /// menu, the group with more of the policy's probability wins and the
    /// other leaves `cand_logits` where that decides the answer
    /// ([`super::grouped`]). Every other question is left as it is.
    fn group_priority(
        &mut self,
        cands: &crate::features4::EncodedCandidates4,
        view: &PlayerView,
        pending: &Pending,
        cand_logits: &mut [f32],
    ) -> Option<super::Grouping4> {
        if !self.grouped_decode
            || !matches!(pending, Pending::Priority { .. })
            || cands.head != HeadKind4::SingleChoice
        {
            return None;
        }
        let n = cands.candidates.len();
        let logits = cand_logits.get_mut(..n)?;
        if logits.iter().any(|logit| !logit.is_finite()) {
            // Invalid outputs are refused where they always were, unchanged.
            return None;
        }
        let classes = crate::coarse_class::coarse_classes_of(cands, view);
        let grouping = super::grouped::group(logits, &classes, self.temperature)?;
        super::grouped::apply(logits, &classes, &grouping, self.temperature);
        self.grouped_use.record(&grouping);
        Some(grouping)
    }
}

/// The options as the net weighed them, for a journal of the game.
fn menu_trace(
    cands: &EncodedCandidates4,
    cand_logits: &[f32],
    pair_logits: &[f32],
) -> serde_json::Value {
    let options: Vec<_> = cands
        .candidates
        .iter()
        .zip(cand_logits)
        .map(|(cand, logit)| {
            serde_json::json!({
                "action": cand.action, "object": cand.object_id, "player": cand.player_id,
                "index": cand.index_id, "logit": logit,
            })
        })
        .collect();
    let pairs: Vec<_> = cands
        .pair_objects
        .iter()
        .zip(pair_logits)
        .map(|(pair, logit)| serde_json::json!({"pair": pair, "logit": logit}))
        .collect();
    serde_json::json!({"head": cands.head as u8, "options": options, "pairs": pairs})
}

#[derive(Clone, Copy)]
struct PolicyOutputs4<'a> {
    cand_logits: &'a [f32],
    pair_logits: &'a [f32],
    scalar_pred: f32,
    value: f32,
    /// What a graph exported with `--embeddings` adds; the tactician reads it.
    embeddings: Option<&'a super::StepEmbeddings4>,
}

struct PolicySampler4<'a> {
    mode: PolicyMode4,
    temperature: f32,
    rng: &'a mut Rng,
    active_source: Option<(ObjectId, Option<u32>)>,
    /// `eval4 --choice-guard`: the rules a run holds this answer to.
    guard: &'a mut super::ChoiceGuard4,
    /// The rule groups over the tactical layer.
    rules: &'a mut super::TacticRules4,
    /// `eval4 --tactician`: answers its windows before any other path.
    tactician: Option<&'a mut super::tactician::Tactician4>,
    /// `eval4 --distill-labels`: labels every answer with `N` and `R`.
    distill: Option<&'a mut super::distill::Distill4>,
    /// `eval4 --explore-rules`: may play the layer's answer in place of the
    /// raw sample, and records the behaviour mixture.
    explorer: Option<&'a mut super::explore::RuleExplorer4>,
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
    ) -> anyhow::Result<Sampled4> {
        anyhow::ensure!(
            self.mode != PolicyMode4::Raw
                || self.guard.free_temperature
                || valid_raw_temperature(self.temperature),
            "raw policy requires temperature 0 or >0.35 under sampler version 2"
        );
        let PolicyOutputs4 {
            cand_logits,
            pair_logits,
            scalar_pred,
            value,
            embeddings,
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
        // The layer's answer an explorer reads is a function of the decision
        // and the sampling stream as it stands before the net's draw.
        let rng_at_decision = self.explorer.is_some().then(|| self.rng.clone());
        let tactician = match self.tactician.as_deref_mut() {
            Some(tactician) => {
                anyhow::ensure!(
                    self.temperature <= 0.0,
                    "the tactician answers at temperature 0"
                );
                tactician.answer(&super::tactician::TacticianStep {
                    cands,
                    view,
                    pending,
                    cand_logits,
                    pair_logits,
                    value,
                    embeddings,
                    active_source: self.active_source,
                    rng: self.rng,
                })?
            }
            None => None,
        };

        // What the rule groups change at this question, for an imitation record.
        let rules_before = *self.rules;
        let raw = self.mode == PolicyMode4::Raw;
        let parts = self.guard.tactical;
        let at_priority = matches!(pending, Pending::Priority { .. });
        let tactician_answered = tactician.is_some().then_some(());
        let logprob: Option<f32> = if let Some(played) = tactician {
            target = played;
            None
        } else {
            match cands.head {
                HeadKind4::SingleChoice => {
                    let active_source = self.active_source;
                    let (chosen_idx, lp) = if raw
                        && !(if at_priority {
                            parts.priority
                        } else {
                            parts.single
                        }) {
                        sample_categorical(
                            &cand_logits[..cands.candidates.len()],
                            self.temperature,
                            self.rng,
                        )
                    } else {
                        tactics::rules::single_choice(
                            self.rules,
                            cands,
                            cand_logits,
                            view,
                            pending,
                            active_source,
                            self.temperature,
                            self.rng,
                        )
                    };
                    let chosen_idx = tactics::guard::review(
                        self.guard,
                        cands,
                        &cand_logits[..cands.candidates.len()],
                        view,
                        pending,
                        chosen_idx,
                    );
                    target.single_idx = chosen_idx as u16;
                    Some(lp)
                }
                HeadKind4::SubsetPick => {
                    let n = cands.candidates.len();
                    let raw_logits = &cand_logits[..n];
                    let active_source = self.active_source;
                    let adj_logits = if raw && !parts.subset {
                        raw_logits.to_vec()
                    } else {
                        tactical_subset_logits(
                            self.rules,
                            cands,
                            raw_logits,
                            view,
                            pending,
                            active_source,
                        )
                    };
                    let (mask, free_mask, seq, _) = sample_subset_constrained(
                        &adj_logits,
                        cands,
                        pending,
                        self.temperature,
                        self.rng,
                    );
                    target.multi_mask = mask;
                    target.free_mask = free_mask;
                    target.pick_sequence = seq;
                    // SubsetPick is excluded under `injective_categorical_menu_v1`.
                    None
                }
                HeadKind4::PairPick => {
                    let n = cands.pairs.len();
                    let raw_logits = &pair_logits[..n];
                    let adj_logits = if raw && !parts.pair {
                        raw_logits.to_vec()
                    } else {
                        tactical_pair_logits(self.rules, cands, raw_logits, view, pending)
                    };
                    let (mask, free_mask, seq, _) = sample_pairs_constrained(
                        &adj_logits,
                        cands,
                        pending,
                        self.temperature,
                        self.rng,
                    );
                    target.multi_mask = mask;
                    target.free_mask = free_mask;
                    target.pick_sequence = seq;
                    // PairPick is excluded under `injective_categorical_menu_v1`.
                    None
                }
                HeadKind4::ScalarValue => {
                    let chosen_int = if raw && !parts.scalar {
                        net_number(cands, scalar_pred)
                    } else {
                        layer_number(cands, view, pending, scalar_pred)
                    };
                    let norm =
                        normalize_scalar_choice(chosen_int, cands.min_choices, cands.max_choices);
                    target.scalar_norm = norm;
                    // Deterministic regression head: supervised training only.
                    None
                }
            }
        };
        // `eval4 --explore-rules`: the layer's answer may be played in place
        // of the raw sample, read on the stream as it stood at the decision.
        let logprob = match (self.explorer.as_deref_mut(), rng_at_decision) {
            (Some(explorer), Some(rng)) if tactician_answered.is_none() => explorer.explore(
                &super::explore::ExploreStep {
                    cands,
                    view,
                    pending,
                    cand_logits,
                    pair_logits,
                    scalar_pred,
                    active_source: self.active_source,
                    rng: &rng,
                    temperature: self.temperature,
                    raw,
                },
                &mut target,
                logprob,
            )?,
            _ => logprob,
        };

        // `eval4 --tactician-imitate`: the layer has answered; at a window the
        // tactician keeps its answer as the label and may explore away from it.
        if let Some(tactician) = self.tactician.as_deref_mut()
            && tactician.imitates()
            && let Some(played) = tactician.imitate(
                &super::tactician::TacticianStep {
                    cands,
                    view,
                    pending,
                    cand_logits,
                    pair_logits,
                    value,
                    embeddings,
                    active_source: self.active_source,
                    rng: self.rng,
                },
                &target,
                &self.rules.fired_since(&rules_before),
            )?
        {
            target = played;
        }
        // `eval4 --distill-labels`: `N` and the layer's `R` at this state, read
        // on copies, whoever answered it.
        // `--distill-teacher house:<level>`: `N` and the house's `H`, given by
        // the host before the net answered.
        if let Some(distill) = self.distill.as_deref_mut() {
            let question = super::distill::Question {
                cands,
                view,
                pending,
                cand_logits,
                pair_logits,
                scalar_pred,
                active_source: self.active_source,
                rng: self.rng,
            };
            match distill.teacher() {
                super::distill::DistillTeacher4::Rules(rules) => {
                    distill.keep(super::distill::label(&question, rules, &target));
                }
                super::distill::DistillTeacher4::House => {
                    if let Some(house) = distill.take_given() {
                        match super::distill::house_label(&question, &house, &target) {
                            Some(label) => distill.keep(label),
                            None => distill.count_unencodable(),
                        }
                    }
                }
            }
        }
        let action = decode_target(cands, view, pending, &target, self.active_source, raw)?;
        Ok((action, value.clamp(0.0, 1.0), logprob, target))
    }
}

/// The action `target` names: decoded, an equivalence group resolved, and
/// under the raw policy checked to re-encode to exactly `target`.
///
/// # Errors
/// A target that does not decode, decodes to another answer, or answers
/// what the question does not allow.
pub(super) fn decode_target(
    cands: &EncodedCandidates4,
    view: &PlayerView,
    pending: &Pending,
    target: &TeacherTarget4,
    active_source: Option<(ObjectId, Option<u32>)>,
    raw: bool,
) -> anyhow::Result<PlayerAction> {
    let mut action =
        decode_action4(cands, view, pending, target).context("decode_action4 returned None")?;
    // The policy selects an equivalence group, not a particular copy.
    // A still-unpaid TapSelf activation reserves one member even though
    // its mana actions remain grouped. Resolve that group deterministically
    // using our own announced intent in BOTH modes; the index/LP is unchanged.
    if view.owed.is_some()
        && cands.head == HeadKind4::SingleChoice
        && let Some(tap_src) = tactics::active_source_requires_tap_self(view, active_source)
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
    if raw {
        anyhow::ensure!(
            decoded_matches_sample(cands, pending, &action, target),
            "raw decoded action differs from sampled target (decoder repair)"
        );
    }
    if !(matches!(action, PlayerAction::CancelCast) && view.casting.is_some())
        && let Some(fault) = pending.answer_fault(&action)
    {
        anyhow::bail!("decoded action failed answer_fault: {fault:?}");
    }
    Ok(action)
}

/// The subset logits the tactical layer samples from: its own adjustments,
/// then the rule groups.
pub(super) fn tactical_subset_logits(
    rules: &mut super::TacticRules4,
    cands: &EncodedCandidates4,
    raw_logits: &[f32],
    view: &PlayerView,
    pending: &Pending,
    active_source: Option<(ObjectId, Option<u32>)>,
) -> Vec<f32> {
    let mut adjusted =
        choices::adjust_subset_logits(cands, raw_logits, view, pending, active_source);
    tactics::rules::subset(rules, cands, &mut adjusted, view, pending, active_source);
    adjusted
}

/// The pair logits the tactical layer samples from: its own adjustments,
/// then the rule groups.
pub(super) fn tactical_pair_logits(
    rules: &mut super::TacticRules4,
    cands: &EncodedCandidates4,
    raw_logits: &[f32],
    view: &PlayerView,
    pending: &Pending,
) -> Vec<f32> {
    let mut adjusted = choices::adjust_pair_logits(cands, raw_logits, view, pending);
    tactics::rules::pairs(rules, cands, &mut adjusted, view, pending);
    adjusted
}

/// The log-probability of `mask` under the subset and pair heads: an
/// independent Bernoulli choice at each free bit on the raw logits at
/// `temperature`, in a numerically stable form. Zero at temperature zero.
pub(super) fn bernoulli_logprob(
    raw_logits: &[f32],
    mask: &[u8],
    free_mask: &[u8],
    temperature: f32,
) -> f32 {
    if temperature <= 1e-4 {
        return 0.0;
    }
    let scale = 1.0 / temperature;
    let mut acc = 0.0_f32;
    for (i, &raw_l) in raw_logits.iter().enumerate() {
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
}

/// The net's own number for a scalar question, within the menu's bounds.
pub(super) fn net_number(cands: &EncodedCandidates4, scalar_pred: f32) -> u32 {
    denormalize_scalar_choice(
        scalar_pred.clamp(0.0, 1.0),
        cands.min_choices,
        cands.max_choices,
    )
}

/// The tactical layer's number for a scalar question, within the menu's
/// bounds: an X sized by what it buys and never paid in life the seat cannot
/// spare, else the whole X ([`tactics::number`]); mana that prevents all the
/// damage it can, no more; a kill's worth of divided damage; lethal combat
/// damage to each blocker in turn; the net's own number elsewhere.
pub(super) fn layer_number(
    cands: &EncodedCandidates4,
    view: &PlayerView,
    pending: &Pending,
    scalar_pred: f32,
) -> u32 {
    let (min, max) = (cands.min_choices, cands.max_choices.max(cands.min_choices));
    match pending {
        Pending::ChooseNumber {
            reason: baylee_engine::choice::NumberPrompt::X,
            ..
        } => {
            tactics::number::x_value(view, min, max, net_number(cands, scalar_pred)).unwrap_or(max)
        }
        Pending::ChooseNumber {
            reason: baylee_engine::choice::NumberPrompt::ManaPayment { preventable_damage },
            ..
        } => tactics::number::prevention(*preventable_damage, min, max),
        _ => denormalize_scalar_choice(
            layer_scalar(view, pending, scalar_pred),
            cands.min_choices,
            cands.max_choices,
        ),
    }
}

/// The tactical layer's normalized number for a scalar question that is not
/// a price ([`layer_number`]): a kill's worth of divided damage, lethal
/// combat damage to each blocker in turn; the net's own prediction elsewhere.
fn layer_scalar(view: &PlayerView, pending: &Pending, scalar_pred: f32) -> f32 {
    match pending {
        Pending::ChooseNumber {
            min,
            max,
            reason: baylee_engine::choice::NumberPrompt::DivideDamage { target, left, .. },
            ..
        } if *max > *min => {
            let rem_t = match target {
                baylee_engine::choice::TargetRef::Object(oid) => {
                    view.object(oid.object).map_or(*max, |o| {
                        u32::try_from(
                            (i32::from(o.toughness.unwrap_or(1)) - i32::from(o.damage)).max(1),
                        )
                        .unwrap_or(1)
                    })
                }
                baylee_engine::choice::TargetRef::Player(pid) => view
                    .seat(*pid)
                    .map_or(*max, |s| u32::try_from(s.life.max(1)).unwrap_or(*max)),
            };
            let assign = rem_t.min(*left).clamp(*min, *max);
            normalize_scalar_choice(assign, *min, *max)
        }
        Pending::ChooseNumber {
            min,
            max,
            reason:
                baylee_engine::choice::NumberPrompt::DivideCounters {
                    kind, target, left, ..
                },
            ..
        } if *max > *min => {
            divide_counters_scalar(view, *kind, *target, *left, *min, *max, scalar_pred)
        }
        Pending::ChooseNumber {
            min,
            max,
            reason:
                baylee_engine::choice::NumberPrompt::CombatDamage {
                    source,
                    recipient,
                    index,
                    of,
                    left,
                    lethal,
                },
            ..
        } if (*index + 1 < *of || lethal.is_some()) && *max > *min => {
            // A trampler's question says what is lethal, and
            // its last blocker is asked too (CR 702.19b).
            let src_dt = view.object(*source).is_some_and(|o| {
                (o.keywords & baylee_cards_dsl::KeywordSet::DEATHTOUCH.bits()) != 0
            });
            let rem_t = lethal.unwrap_or_else(|| {
                if src_dt {
                    1
                } else {
                    view.object(*recipient).map_or(*max, |o| {
                        u32::try_from(
                            (i32::from(o.toughness.unwrap_or(1)) - i32::from(o.damage)).max(1),
                        )
                        .unwrap_or(1)
                    })
                }
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
}

fn divide_counters_scalar(
    view: &PlayerView,
    kind: baylee_cards_dsl::CounterKind,
    target: baylee_engine::choice::TargetRef,
    left: u32,
    min: u32,
    max: u32,
    scalar_pred: f32,
) -> f32 {
    use baylee_cards_dsl::CounterKind as K;
    let assign = match kind {
        K::Minus { toughness, .. } if toughness > 0 => {
            let unit = i32::from(toughness);
            let rem_t = match target {
                baylee_engine::choice::TargetRef::Object(oid) => {
                    view.object(oid.object).map_or(max, |o| {
                        let t = i32::from(o.toughness.unwrap_or(1)).max(1);
                        u32::try_from((t + unit - 1) / unit).unwrap_or(1)
                    })
                }
                baylee_engine::choice::TargetRef::Player(_) => min,
            };
            rem_t.min(left).clamp(min, max)
        }
        K::Stun
        | K::Finality
        | K::Lifelink
        | K::Deathtouch
        | K::Flying
        | K::Indestructible
        | K::Hone
        | K::Shadow
        | K::Shield => 1u32.clamp(min, max),
        K::Plus { .. } | K::Loyalty | K::Charge | K::Level | K::Energy => match target {
            baylee_engine::choice::TargetRef::Object(oid) => {
                if view
                    .object(oid.object)
                    .is_some_and(|o| o.controller == view.seat)
                {
                    max
                } else {
                    min
                }
            }
            baylee_engine::choice::TargetRef::Player(_) => min,
        },
        _ => return scalar_pred,
    };
    normalize_scalar_choice(assign, min, max)
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

/// The log-probability of `chosen` under [`sample_categorical`] at `tau`,
/// by the same arithmetic: the probability of an answer the sampler did not
/// draw, such as one an explorer played in its place.
pub(super) fn categorical_logprob(logits: &[f32], chosen: usize, tau: f32) -> f32 {
    if logits.is_empty() || tau <= 1e-4 {
        return 0.0;
    }
    let scale = 1.0 / tau;
    let max_l = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0_f32;
    for &l in logits {
        sum += ((l - max_l) * scale).exp();
    }
    ((logits[chosen] - max_l) * scale) - sum.ln()
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
mod number_tests;
#[cfg(test)]
mod payment_tests;
#[cfg(test)]
mod questions_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod grouped_attack_tests;
#[cfg(test)]
mod grouped_decode_tests;
#[cfg(test)]
mod tactician_tests;

#[cfg(test)]
mod distill_tests;

#[cfg(test)]
mod explore_tests;
