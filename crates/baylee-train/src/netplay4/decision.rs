//! Native decision pipeline, separated without changing its operations.
use super::ranked::{RootDecision4, rank_single_choice};
use super::{
    Answer4, FallbackDiagnostic4, FallbackReason4, HeadKind4, NetPlayer4, Pending, PendingKind4,
    PlayerAction, PlayerView, RECURRENT_SLOTS, SourceKind4, StepProvenance4, TeacherTarget4,
    decode_action4, fallback, fallback_action, tactics, view_only_state_hash,
};

impl NetPlayer4 {
    /// Chooses a legal [`PlayerAction`] for `view` and `pending`.
    pub fn act(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> PlayerAction {
        self.act_detailed(view, pending, teams).action
    }

    /// Chooses a legal [`PlayerAction`] and returns full [`Answer4`] metadata
    /// (value prediction, [`StepProvenance4`], and [`TeacherTarget4`]).
    pub fn act_detailed(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> Answer4 {
        let decision = self.decide(view, pending, teams, false);
        self.conclude(view, &decision, &decision.answer.action);
        decision.answer
    }

    /// Finishes a decision [`Self::decide`] took, with `action` as its
    /// answer: notes an announced spell or ability the way
    /// [`Self::act_detailed`] notes its own answer, for the target and
    /// number questions that follow it.
    pub(super) fn conclude(
        &mut self,
        view: &PlayerView,
        decision: &RootDecision4,
        action: &PlayerAction,
    ) {
        if !decision.notes_answer || view.owed.is_some() {
            return;
        }
        let seat_u8 = view.seat.get();
        match action {
            PlayerAction::CastSpell { card } => {
                self.last_announced_source.insert(seat_u8, (*card, None));
            }
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            } if !tactics::is_mana_ability_entry(view, *source, *ability_index) => {
                self.last_announced_source
                    .insert(seat_u8, (*source, Some(*ability_index)));
            }
            _ => {}
        }
    }

    /// [`Self::act_detailed`] up to noting its answer ([`Self::conclude`]),
    /// with the policy's ranking of the legal answers when `rank` asks for
    /// it and the answer is the policy's own single choice.
    #[allow(
        clippy::too_many_lines,
        reason = "One decision pipeline records provenance for forced, guarded, neural and fallback answers"
    )]
    pub(super) fn decide(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
        rank: bool,
    ) -> RootDecision4 {
        let only = |answer| RootDecision4 {
            answer,
            ranking: None,
            notes_answer: false,
        };
        self.last_fallback = None;
        let seat_u8 = view.seat.get();
        if matches!(pending, Pending::Priority { .. })
            && view.owed.is_none()
            && view.stack.is_empty()
        {
            self.last_announced_source.remove(&seat_u8);
        }
        let p_kind = PendingKind4::of(pending);

        if let Some(answer) = fallback::unsupported_choice_answer(pending, self.temperature) {
            self.last_fallback = Some(FallbackDiagnostic4::new(
                FallbackReason4::UnsupportedChoice,
                p_kind,
            ));
            self.coverage.record(p_kind, SourceKind4::House);
            return only(answer);
        }

        // The host calls observe() for every neural seat before each table action.
        // Decision inference is read-only with respect to persistent memory.
        let h_in = self
            .h_states
            .entry(seat_u8)
            .or_insert_with(|| vec![0.0; RECURRENT_SLOTS * self.d_model])
            .clone();

        let (state, cands) = self.policy_encoding(view, pending, teams);

        // 1. Check if forced (<= 1 legal choice)
        let is_forced = match cands.head {
            HeadKind4::SingleChoice => cands.candidates.len() <= 1,
            HeadKind4::SubsetPick => cands.candidates.is_empty(),
            HeadKind4::PairPick => cands.pairs.is_empty(),
            HeadKind4::ScalarValue => cands.max_choices <= cands.min_choices,
        };
        if is_forced {
            let forced_target = TeacherTarget4 {
                single_idx: 0,
                multi_mask: vec![0; cands.candidates.len().max(cands.pairs.len())],
                free_mask: vec![0; cands.candidates.len().max(cands.pairs.len())],
                pick_sequence: Vec::new(),
                scalar_norm: 0.0,
            };
            let (action, source) = decode_action4(&cands, view, pending, &forced_target)
                .filter(|a| {
                    (matches!(a, PlayerAction::CancelCast) && view.casting.is_some())
                        || pending.answer_fault(a).is_none()
                })
                .map_or_else(
                    || {
                        (
                            fallback_action(&cands, view, pending, teams),
                            SourceKind4::House,
                        )
                    },
                    |action| (action, SourceKind4::Forced),
                );
            if source == SourceKind4::House {
                self.last_fallback = Some(FallbackDiagnostic4::new(
                    FallbackReason4::ForcedDecode,
                    p_kind,
                ));
            }
            self.coverage.record(p_kind, source);
            return only(Answer4 {
                action,
                value: 0.5,
                provenance: StepProvenance4 {
                    source,
                    temperature: self.temperature,
                    thinking_steps: 0,
                    behavior_logprob: 0.0,
                    has_logprob: false,
                },
                target: forced_target,
            });
        }

        // 2. View-only stall/loop guard (never uses Session::snapshot_hash!)
        let v_hash = if self.uses_announcement_context() {
            crate::features9::view_only_state_hash9(view, pending)
        } else if self.uses_payment_context() {
            crate::features8::view_only_state_hash8(view, pending)
        } else {
            view_only_state_hash(view, pending)
        };
        let visits = self.seen_hashes.entry((seat_u8, v_hash)).or_insert(0);
        *visits = visits.saturating_add(1);
        if *visits >= 4 {
            self.last_fallback = Some(FallbackDiagnostic4 {
                reason: FallbackReason4::RepeatedView,
                pending_kind: p_kind,
                view_hash: Some(format!("{v_hash:016x}")),
                visits: Some(*visits),
                detail: None,
            });
            let action = fallback_action(&cands, view, pending, teams);
            self.coverage.record(p_kind, SourceKind4::House);
            return only(Answer4 {
                action,
                value: 0.5,
                provenance: StepProvenance4 {
                    source: SourceKind4::House,
                    temperature: self.temperature,
                    thinking_steps: 0,
                    behavior_logprob: 0.0,
                    has_logprob: false,
                },
                target: TeacherTarget4::default(),
            });
        }

        // 3. Run ONNX inference
        let inference = self.infer_and_sample(&state, &cands, view, pending, &h_in);
        let (answer, ranking) = match inference {
            Ok(((action, value, logprob, target), logits)) => {
                let provenance = self.policy_mode.provenance(
                    self.temperature,
                    self.thinking_steps,
                    cands.head,
                    logprob,
                );
                self.coverage.record(p_kind, provenance.source);
                let ranking = (rank && cands.head == HeadKind4::SingleChoice)
                    .then(|| rank_single_choice(&cands, &logits, view, pending));
                let answer = Answer4 {
                    action,
                    value,
                    provenance,
                    target,
                };
                (answer, ranking)
            }
            Err(error) => {
                self.last_fallback = Some(FallbackDiagnostic4 {
                    reason: FallbackReason4::InferenceOrDecode,
                    pending_kind: p_kind,
                    view_hash: Some(format!("{v_hash:016x}")),
                    visits: None,
                    detail: Some(error.to_string()),
                });
                let action = fallback_action(&cands, view, pending, teams);
                self.coverage.record(p_kind, SourceKind4::House);
                let answer = Answer4 {
                    action,
                    value: 0.5,
                    provenance: StepProvenance4 {
                        source: SourceKind4::House,
                        temperature: self.temperature,
                        thinking_steps: 0,
                        behavior_logprob: 0.0,
                        has_logprob: false,
                    },
                    target: TeacherTarget4::default(),
                };
                (answer, None)
            }
        };
        RootDecision4 {
            answer,
            ranking,
            notes_answer: true,
        }
    }
}
