use super::{
    AuxTargets4, CoverageMatrix4, HeadKind4, Line, MAX_SEATS, Pending, PendingKind4, PlayerId,
    RECORD_VERSION, ReplaySummary, RolloutStepSidecar4, SeatKnowledge4, Session, Source,
    SourceKind4, Step4, StepProvenance4, Targets, TeacherTarget4, encode_candidates4,
    encode_state4, encode_teacher_action4,
};

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
    mut emit: impl FnMut(Step4, i32) -> Result<(), String>,
) -> Result<ReplaySummary, String> {
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
    let mut mana_pass_count: u64 = 0;
    let mut input_idx: usize = 0;

    for line in lines {
        match line? {
            Line::Header { .. } => return Err("second header in record".to_owned()),
            Line::Chair { .. } | Line::DeclaredMind { .. } => {}
            Line::Input {
                seat: seat_u8,
                by,
                action,
                ..
            } => {
                let seat = PlayerId::new(seat_u8);
                if action.is_automation_setting() {
                    let _ = session.act(seat, action);
                    continue;
                }
                let cur_sidecar = if matches!(by, Source::Seat) {
                    let entry = sidecar.and_then(|sc| sc.get(input_idx));
                    input_idx += 1;
                    entry
                } else {
                    None
                };

                let (pending, view) = session
                    .view_for(seat)
                    .ok_or_else(|| format!("seat {seat:?} was not owed a question"))?;

                // Diagnostic decision-only conversion is not a recurrent training dataset.
                if !include_passive_obs {
                    knowledge[seat_u8 as usize].observe(&view);
                }

                let p_kind = PendingKind4::of(&pending);
                let mut step_loss_weight = 1.0_f32;
                if matches!(&pending, Pending::Priority { .. })
                    && matches!(action, baylee_engine::choice::PlayerAction::PassPriority)
                {
                    // Never drop PassPriority steps from the trajectory sequence (P0-4);
                    // downweight 5/6 of them via `loss_weight = 0.2`.
                    let full_weight = mana_pass_count.is_multiple_of(6);
                    mana_pass_count += 1;
                    if !full_weight {
                        step_loss_weight = 0.2;
                    }
                }

                // Every seat observes the pre-action state, including the acting seat.
                // Decisions consume this memory; they never update it again.
                if include_passive_obs {
                    for other_idx in 0..seat_count {
                        let other_pid = PlayerId::new(other_idx as u8);
                        let other_v = session.observe_view(other_pid);
                        if knowledge[other_idx].observe(&other_v) {
                            let st =
                                encode_state4(&other_v, None, Some(&knowledge[other_idx]), &teams);
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
                                },
                                cur_life,
                            )?;
                        }
                    }
                }

                let state = encode_state4(
                    &view,
                    Some(&pending),
                    Some(&knowledge[seat_u8 as usize]),
                    &teams,
                );
                let cands = encode_candidates4(&state, &view, &pending);

                let is_forced = match cands.head {
                    HeadKind4::SingleChoice => cands.candidates.len() <= 1,
                    HeadKind4::SubsetPick => cands.candidates.is_empty(),
                    HeadKind4::PairPick => cands.pairs.is_empty(),
                    HeadKind4::ScalarValue => cands.max_choices <= cands.min_choices,
                };

                if is_forced {
                    coverage.record(p_kind, SourceKind4::Forced);
                } else if let Some(mut target) = encode_teacher_action4(&cands, &pending, &action) {
                    let default_src = match by {
                        Source::Seat => SourceKind4::Net,
                        _ => SourceKind4::House,
                    };
                    let prov = if let Some(sc_entry) = cur_sidecar {
                        if !sc_entry.free_mask.is_empty()
                            && sc_entry.free_mask.len() == target.multi_mask.len()
                        {
                            target.free_mask.clone_from(&sc_entry.free_mask);
                        }
                        if !sc_entry.pick_sequence.is_empty() {
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
                            loss_weight: step_loss_weight,
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
                        },
                        cur_life,
                    )?;
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

    let Some(winners) = winners else {
        return Err("record has no End line".to_owned());
    };
    if !matches!(session.pending(), Pending::GameOver(_)) {
        return Err("record ended before Pending::GameOver".to_owned());
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

    Ok(ReplaySummary {
        coverage,
        unencoded_decisions,
        targets: Targets::new(
            seat_count,
            &teams,
            &surviving_seats,
            &winners,
            final_turn,
            final_lives,
            session.snapshot_hash(),
        ),
    })
}
