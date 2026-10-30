use std::borrow::Cow;

use baylee_engine::choice::PlayerAction;
use baylee_gamehost::record::MindKind;

use super::{
    AuxTargets4, CoverageMatrix4, HeadKind4, Line, MAX_SEATS, Pending, PendingKind4, PlayerId,
    RECORD_VERSION, ReplaySummary, RolloutStepSidecar4, SeatKnowledge4, Session, Source,
    SourceKind4, Step4, StepProvenance4, Targets, TeacherTarget4, encode_candidates7_grouped,
    encode_state7, encode_teacher_action4,
};

const fn is_client_only_live_action(action: &PlayerAction) -> bool {
    matches!(
        action,
        PlayerAction::PayWindow { .. } | PlayerAction::Concede | PlayerAction::OfferDraw
    )
}

fn is_payable_cast(pending: &Pending, action: &PlayerAction) -> bool {
    matches!(
        (pending, action),
        (Pending::Priority { legal, .. }, PlayerAction::CastSpell { card })
            if legal.payable.contains(card)
    )
}

#[allow(
    clippy::too_many_lines,
    reason = "Keep audited replay and observation ordering contiguous"
)]
#[allow(
    clippy::cast_precision_loss,
    reason = "Outcome counts are normalized into f32 model targets"
)]
pub(super) fn replay_frames(
    record: &[u8],
    game_id: u32,
    include_passive_obs: bool,
    sidecar: Option<&[RolloutStepSidecar4]>,
    emit: impl FnMut(Step4, i32) -> Result<(), String>,
) -> Result<ReplaySummary, String> {
    replay_frames_shadow(record, game_id, include_passive_obs, sidecar, None, emit)
}

#[allow(
    clippy::too_many_lines,
    reason = "Audited chronological observation and decision replay"
)]
#[allow(
    clippy::cast_precision_loss,
    reason = "Existing normalized auxiliary values"
)]
pub(super) fn replay_frames_shadow(
    record: &[u8],
    game_id: u32,
    include_passive_obs: bool,
    sidecar: Option<&[RolloutStepSidecar4]>,
    shadow: Option<&mut super::shadow_bc::ShadowBc4>,
    emit: impl FnMut(Step4, i32) -> Result<(), String>,
) -> Result<ReplaySummary, String> {
    replay_frames_modes(
        record,
        game_id,
        include_passive_obs,
        sidecar,
        shadow,
        None,
        false,
        7,
        emit,
    )
}

#[allow(
    clippy::too_many_lines,
    reason = "Audited chronological observation and decision replay"
)]
#[allow(
    clippy::cast_precision_loss,
    clippy::too_many_arguments,
    reason = "Existing normalized auxiliary values and mode flags"
)]
pub(super) fn replay_frames_modes(
    record: &[u8],
    game_id: u32,
    include_passive_obs: bool,
    sidecar: Option<&[RolloutStepSidecar4]>,
    shadow: Option<&mut super::shadow_bc::ShadowBc4>,
    llm: Option<&mut super::llm_bc::LlmBc4>,
    live_bc: bool,
    encoder_version: u32,
    emit: impl FnMut(Step4, i32) -> Result<(), String>,
) -> Result<ReplaySummary, String> {
    let replay = replay_feature_frames(
        record,
        game_id,
        include_passive_obs,
        sidecar,
        shadow,
        llm,
        live_bc,
        encoder_version,
        FeaturePurpose::Ordinary,
        emit,
    )?;
    Ok(ReplaySummary {
        coverage: replay.coverage,
        unencoded_decisions: replay.unencoded_decisions,
        targets: replay
            .targets
            .ok_or("ordinary replay lacks a verified natural terminal")?,
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "Explicit feature replay mode retains existing observation ordering"
)]
pub(super) fn replay_research_frames(
    record: &[u8],
    game_id: u32,
    sidecar: &[RolloutStepSidecar4],
    encoder: u32,
    natural: bool,
    emit: impl FnMut(Step4, i32) -> Result<(), String>,
) -> Result<super::streaming::FeatureReplaySummary, String> {
    replay_feature_frames(
        record,
        game_id,
        true,
        Some(sidecar),
        None,
        None,
        false,
        encoder,
        FeaturePurpose::Research { natural },
        emit,
    )
}

#[derive(Clone, Copy)]
enum FeaturePurpose {
    Ordinary,
    Research { natural: bool },
}

#[allow(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::cast_precision_loss,
    reason = "Shared chronological feature replay; only explicit research prefix mode omits natural targets"
)]
fn replay_feature_frames(
    record: &[u8],
    game_id: u32,
    include_passive_obs: bool,
    sidecar: Option<&[RolloutStepSidecar4]>,
    mut shadow: Option<&mut super::shadow_bc::ShadowBc4>,
    mut llm: Option<&mut super::llm_bc::LlmBc4>,
    live_bc: bool,
    encoder_version: u32,
    purpose: FeaturePurpose,
    mut emit: impl FnMut(Step4, i32) -> Result<(), String>,
) -> Result<super::streaming::FeatureReplaySummary, String> {
    if !matches!(encoder_version, 7..=9) {
        return Err("Unsupported encoder version".into());
    }
    if encoder_version == 9 && (shadow.is_some() || llm.is_some()) {
        return Err("Encoder9 shadow and LLM labels are not supported".into());
    }
    let mut lines = record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_slice::<Line>(l).map_err(|e| e.to_string()));

    let Some(first) = lines.next() else {
        return Err("empty record".to_owned());
    };
    let Line::Header {
        record: version,
        preset,
        ..
    } = first?
    else {
        return Err("first line is not a header".to_owned());
    };
    if version != RECORD_VERSION {
        return Err(format!(
            "record version {version} != expected {RECORD_VERSION}"
        ));
    }

    let seat_count = preset.seats.len().clamp(1, MAX_SEATS);
    let teams: Vec<Option<u8>> = preset.seats.iter().map(|s| s.team).collect();

    let mut session =
        Session::new(&preset).ok_or_else(|| "preset failed to build session".to_owned())?;
    for s in 0..seat_count {
        session.take_over(PlayerId::new(s as u8));
    }

    let mut knowledge: Vec<SeatKnowledge4> = (0..seat_count)
        .map(|s| SeatKnowledge4::from_preset(&preset, PlayerId::new(s as u8)))
        .collect();
    let mut seat_cursors = [0_u32; MAX_SEATS];

    let mut coverage = CoverageMatrix4::default();
    let mut unencoded_decisions = 0_u64;
    let mut winners: Option<Vec<u8>> = None;
    let mut final_turn: u32 = 1;
    let mut input_idx: usize = 0;
    let mut seat_minds: [Option<MindKind>; MAX_SEATS] = [None; MAX_SEATS];

    for line in lines {
        match line? {
            Line::Header { .. } => return Err("second header in record".to_owned()),
            Line::Chair { .. } => {}
            Line::DeclaredMind { seat, mind, .. } => {
                if (seat as usize) < MAX_SEATS {
                    seat_minds[seat as usize] = Some(mind.kind);
                }
            }
            Line::Input {
                seat: seat_u8,
                by,
                action,
                n: input_n,
                ..
            } => {
                let seat = PlayerId::new(seat_u8);
                if action.is_automation_setting()
                    || (live_bc
                        && matches!(action, PlayerAction::Concede | PlayerAction::OfferDraw))
                {
                    let _ = session.act(seat, action);
                    final_turn = session.state().turn.number;
                    continue;
                }
                let cur_sidecar = if matches!(by, Source::Seat) {
                    let entry = sidecar.and_then(|sc| sc.get(input_idx));
                    if shadow.is_some() && entry.is_none() {
                        return Err("Missing original neural behavior provenance".into());
                    }
                    input_idx += 1;
                    entry
                } else {
                    None
                };

                let pending_view = if encoder_version == 9 {
                    session.view_for_with_announcement_context(seat)
                } else if encoder_version == 8 {
                    session.view_for_with_payment_context(seat)
                } else {
                    session.view_for_with_context(seat)
                };
                let (pending, view) =
                    pending_view.ok_or_else(|| format!("seat {seat:?} was not owed a question"))?;

                // Diagnostic decision-only conversion is not a recurrent training dataset.
                if !include_passive_obs {
                    if encoder_version == 9 {
                        knowledge[seat_u8 as usize].observe_with_announcement_context(&view);
                    } else if encoder_version == 8 {
                        knowledge[seat_u8 as usize].observe_with_payment_context(&view);
                    } else {
                        knowledge[seat_u8 as usize].observe(&view);
                    }
                }

                let p_kind = PendingKind4::of(&pending);
                let step_loss_weight = if live_bc {
                    if matches!(by, Source::Seat)
                        && matches!(
                            seat_minds[seat_u8 as usize],
                            Some(MindKind::Human | MindKind::LlmApi | MindKind::LlmCli)
                        )
                    {
                        super::idle_pass::live_weight(&pending, &view, &action)
                    } else {
                        0.0_f32
                    }
                } else {
                    1.0_f32
                };

                // Every seat observes the pre-action state, including the acting seat.
                // Decisions consume this memory; they never update it again.
                if include_passive_obs {
                    for other_idx in 0..seat_count {
                        let other_v = if other_idx == seat_u8 as usize {
                            Cow::Borrowed(&view)
                        } else {
                            let other_pid = PlayerId::new(other_idx as u8);
                            Cow::Owned(if encoder_version == 9 {
                                session.observe_view_with_announcement_context(other_pid)
                            } else if encoder_version == 8 {
                                session.observe_view_with_payment_context(other_pid)
                            } else {
                                session.observe_view_with_context(other_pid)
                            })
                        };
                        let changed = if encoder_version == 9 {
                            knowledge[other_idx].observe_with_announcement_context(&other_v)
                        } else if encoder_version == 8 {
                            knowledge[other_idx].observe_with_payment_context(&other_v)
                        } else {
                            knowledge[other_idx].observe(&other_v)
                        };
                        if changed {
                            let st = encode_selected_state(
                                encoder_version,
                                &other_v,
                                None,
                                Some(&knowledge[other_idx]),
                                &teams,
                            );
                            let cur_life = other_v.seats.get(other_idx).map_or(20, |s| s.life);
                            let sc = seat_cursors[other_idx];
                            seat_cursors[other_idx] += 1;
                            emit(
                                Step4 {
                                    game_id,
                                    seat: other_idx as u8,
                                    step_cursor: sc,
                                    turn: other_v.turn.min(u32::from(u16::MAX)) as u16,
                                    is_decision: false,
                                    loss_weight: 0.0,
                                    is_terminal: false,
                                    state: st,
                                    head: 0,
                                    pending_kind: PendingKind4::Priority as u8,
                                    cand_feat: Vec::new(),
                                    cand_entity_idx: Vec::new(),
                                    cand_rel_seat_idx: Vec::new(),
                                    cand_card_id: Vec::new(),
                                    cand_face_id: Vec::new(),
                                    cand_ability_idx: Vec::new(),
                                    cand_rule_refs: Vec::new(),
                                    pairs: Vec::new(),
                                    target: TeacherTarget4::default(),
                                    outcome: 0.0,
                                    aux: AuxTargets4::default(),
                                    provenance: StepProvenance4 {
                                        source: SourceKind4::Forced,
                                        ..StepProvenance4::default()
                                    },
                                    shadow_label: None,
                                    llm_label: None,
                                },
                                cur_life,
                            )?;
                        }
                    }
                }

                let state = encode_selected_state(
                    encoder_version,
                    &view,
                    Some(&pending),
                    Some(&knowledge[seat_u8 as usize]),
                    &teams,
                );
                let cands = encode_candidates7_grouped(&state, &view, &pending);

                let is_forced = match cands.head {
                    HeadKind4::SingleChoice => cands.candidates.len() <= 1,
                    HeadKind4::SubsetPick => cands.candidates.is_empty(),
                    HeadKind4::PairPick => cands.pairs.is_empty(),
                    HeadKind4::ScalarValue => cands.max_choices <= cands.min_choices,
                };

                let llm_label = if matches!(by, Source::Seat) {
                    llm.as_deref_mut()
                        .map(|labels| labels.input(input_n, seat_u8, &action, is_forced))
                        .transpose()?
                        .flatten()
                } else {
                    None
                };

                if is_forced
                    || (llm.is_some()
                        && matches!(action, baylee_engine::choice::PlayerAction::Concede))
                {
                    if shadow
                        .as_ref()
                        .is_some_and(|labels| labels.has_label(input_n))
                    {
                        return Err("Shadow label attached to a forced decision".into());
                    }
                    coverage.record(
                        p_kind,
                        if is_forced {
                            SourceKind4::Forced
                        } else {
                            SourceKind4::External
                        },
                    );
                } else if let Some(mut target) = if llm.is_some()
                    || cur_sidecar.is_some_and(|entry| entry.provenance.source == SourceKind4::Net)
                {
                    crate::features4::encode_executed_action4(&cands, &pending, &action)
                } else {
                    encode_teacher_action4(&cands, &pending, &action)
                } {
                    let default_src = match by {
                        Source::Seat => SourceKind4::Net,
                        _ => SourceKind4::House,
                    };
                    let mut prov = if let Some(sc_entry) = cur_sidecar {
                        if matches!(purpose, FeaturePurpose::Research { .. }) {
                            if sc_entry.free_mask.len() != target.multi_mask.len()
                                || sc_entry.free_mask.iter().any(|&bit| bit > 1)
                                || sc_entry
                                    .pick_sequence
                                    .iter()
                                    .any(|&pick| usize::from(pick) >= target.multi_mask.len())
                            {
                                return Err(
                                    "research original action mask/sequence is incompatible".into(),
                                );
                            }
                            target.free_mask.clone_from(&sc_entry.free_mask);
                            target.pick_sequence.clone_from(&sc_entry.pick_sequence);
                        } else if !sc_entry.free_mask.is_empty()
                            && sc_entry.free_mask.len() == target.multi_mask.len()
                        {
                            target.free_mask.clone_from(&sc_entry.free_mask);
                        }
                        if matches!(purpose, FeaturePurpose::Ordinary)
                            && !sc_entry.pick_sequence.is_empty()
                        {
                            target.pick_sequence.clone_from(&sc_entry.pick_sequence);
                        }
                        sc_entry.provenance
                    } else {
                        StepProvenance4 {
                            source: default_src,
                            temperature: 0.0,
                            thinking_steps: 1,
                            behavior_logprob: 0.0,
                            has_logprob: false,
                        }
                    };
                    let mut policy_weight = step_loss_weight;
                    let mut shadow_label = None;
                    if llm.is_some() {
                        policy_weight = if llm_label.is_some() {
                            super::idle_pass::llm_weight(&pending, &view, &action)
                        } else {
                            0.0_f32
                        };
                        prov.source = if matches!(by, Source::Seat) {
                            SourceKind4::External
                        } else {
                            SourceKind4::House
                        };
                        prov.temperature = 0.0;
                        prov.thinking_steps = 0;
                        prov.has_logprob = false;
                        prov.behavior_logprob = 0.0;
                    }
                    if let Some(labels) = shadow.as_deref_mut() {
                        let original_provenance = prov;
                        policy_weight = 0.0;
                        prov.has_logprob = false;
                        prov.behavior_logprob = 0.0;
                        if let Some((advice, identity)) = labels.target(
                            input_n,
                            session.decision_seq(),
                            seat_u8,
                            &action,
                            &original_provenance,
                            &view,
                            &pending,
                            &state,
                            &cands,
                            &teams,
                        )? {
                            target = advice;
                            policy_weight = 1.0;
                            prov.source = SourceKind4::House;
                            prov.temperature = 0.0;
                            shadow_label = Some(identity);
                        }
                    }
                    coverage.record(p_kind, prov.source);
                    let s_idx = seat_u8 as usize;
                    let sc = seat_cursors[s_idx];
                    seat_cursors[s_idx] += 1;
                    let cur_life = view.seats.get(s_idx).map_or(20, |s| s.life);
                    emit(
                        Step4 {
                            game_id,
                            seat: seat_u8,
                            step_cursor: sc,
                            turn: view.turn.min(u32::from(u16::MAX)) as u16,
                            is_decision: true,
                            loss_weight: policy_weight,
                            is_terminal: false,
                            cand_rule_refs: cands
                                .candidates
                                .iter()
                                .map(|c| crate::cardstructure::refs::candidate_refs(c, &state))
                                .collect(),
                            state,
                            head: cands.head as u8,
                            pending_kind: p_kind as u8,
                            cand_feat: cands.candidates.iter().map(|c| c.feat.clone()).collect(),
                            cand_entity_idx: cands
                                .candidates
                                .iter()
                                .map(|c| c.entity_idx)
                                .collect(),
                            cand_rel_seat_idx: cands
                                .candidates
                                .iter()
                                .map(|c| c.rel_seat_idx)
                                .collect(),
                            cand_card_id: cands.candidates.iter().map(|c| c.card_id).collect(),
                            cand_face_id: cands
                                .candidates
                                .iter()
                                .map(crate::features4::Candidate4::named_face)
                                .collect(),
                            cand_ability_idx: cands
                                .candidates
                                .iter()
                                .map(|c| c.ability_idx)
                                .collect(),
                            pairs: cands.pairs,
                            target,
                            outcome: 0.0,
                            aux: AuxTargets4::default(),
                            provenance: prov,
                            shadow_label,
                            llm_label,
                        },
                        cur_life,
                    )?;
                } else if live_bc
                    && (is_client_only_live_action(&action) || is_payable_cast(&pending, &action))
                {
                    coverage.record(p_kind, SourceKind4::Forced);
                } else {
                    eprintln!(
                        "[convert4] UNENCODED game_id={game_id} kind={p_kind:?} head={:?} cands={} pairs={} action={action:?} pending={pending:?}",
                        cands.head,
                        cands.candidates.len(),
                        cands.pairs.len(),
                    );
                    unencoded_decisions += 1;
                    coverage.record(p_kind, SourceKind4::House);
                }

                session
                    .act(seat, action)
                    .map_err(|e| format!("engine rejected recorded action for {seat:?}: {e}"))?;
                final_turn = session.state().turn.number;
            }
            Line::End {
                winners: w, turn, ..
            } => {
                winners = Some(w);
                final_turn = turn.unwrap_or(final_turn);
            }
        }
    }

    if let Some(labels) = shadow {
        if sidecar.is_none_or(|rows| rows.len() != input_idx) {
            return Err("Original behavior provenance length mismatch".into());
        }
        labels.finish()?;
    }

    if let Some(labels) = llm {
        labels.finish()?;
    }

    if winners.is_none() && !matches!(purpose, FeaturePurpose::Research { natural: false }) {
        return Err("record has no End line".to_owned());
    }
    if winners.is_some() != matches!(session.pending(), Pending::GameOver(_)) {
        return Err("record terminal marker differs from Pending::GameOver".to_owned());
    }

    // Compute final life totals and surviving seats for side_utility + AuxTargets4
    let final_state = session.state();
    let mut final_lives = [0_i32; MAX_SEATS];
    let mut surviving_seats = Vec::new();
    for (s, life) in final_lives.iter_mut().enumerate().take(seat_count) {
        let pid = PlayerId::new(s as u8);
        *life = final_state.players[s].life;
        if !final_state.has_left(pid) {
            surviving_seats.push(s as u8);
        }
    }

    Ok(super::streaming::FeatureReplaySummary {
        coverage,
        unencoded_decisions,
        targets: winners.map(|winners| {
            Targets::new(
                seat_count,
                &teams,
                &surviving_seats,
                &winners,
                final_turn,
                final_lives,
                session.snapshot_hash(),
            )
        }),
        snapshot_hash: session.snapshot_hash(),
    })
}

fn encode_selected_state(
    encoder: u32,
    view: &baylee_view::PlayerView,
    pending: Option<&Pending>,
    knowledge: Option<&SeatKnowledge4>,
    teams: &[Option<u8>],
) -> crate::features4::EncodedState4 {
    if encoder == 9 {
        crate::features9::encode_state9(view, pending, knowledge, teams)
    } else if encoder == 8 {
        crate::features8::encode_state8(view, pending, knowledge, teams)
    } else {
        encode_state7(view, pending, knowledge, teams)
    }
}
