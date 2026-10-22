//! Replays recorded games into Playernet v4 trajectory steps (`Step4`),
//! maintaining continuous per-seat [`SeatKnowledge4`] observation streams,
//! relational graph edges, lossless candidate/action targets, auxiliary
//! self-supervised targets, and [`CoverageMatrix4`] statistics.

use baylee_core::ids::PlayerId;
use baylee_engine::choice::Pending;
use baylee_gamehost::Session;
use baylee_gamehost::record::{Line, RECORD_VERSION, Source};
use serde::{Deserialize, Serialize};

use crate::features4::{
    CoverageMatrix4, EncodedState4, HeadKind4, MAX_SEATS, PendingKind4, SeatKnowledge4,
    SourceKind4, StepProvenance4, TeacherTarget4, encode_candidates4, encode_state4,
    encode_teacher_action4, side_utility,
};

/// Auxiliary self-supervised training targets attached to each [`Step4`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AuxTargets4 {
    /// Normalized remaining turns in the game (`((final_turn - turn) / 15.0).clamp(0.0, 1.0)`).
    pub turns_remaining_norm: f32,
    /// Seat's own final life total normalized (`(final_life / 40.0).clamp(-1.0, 1.0)`).
    pub my_final_life_norm: f32,
    /// Minimum final life total among opponents (`(min_opp_life / 40.0).clamp(-1.0, 1.0)`).
    pub opp_min_final_life_norm: f32,
    /// Next-decision life delta for the acting seat (`(next_life - cur_life) / 10.0`).
    pub next_life_delta_norm: f32,
}

fn default_loss_weight() -> f32 {
    1.0
}

/// Per-input-step rollout provenance & constraint sidecar written by `eval4` (`P0-2`, `P0-3`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RolloutStepSidecar4 {
    /// Action provenance (`Net | House | Forced | Search`, `temperature`, `K`, `behavior_logprob`, `has_logprob`).
    pub provenance: StepProvenance4,
    /// Exact rollout `free_mask` (`1` = free Bernoulli decision, `0` = forced/forbidden).
    #[serde(default)]
    pub free_mask: Vec<u8>,
    /// Exact rollout `pick_sequence` visit order.
    #[serde(default)]
    pub pick_sequence: Vec<u16>,
}

/// One v4 trajectory step (`Step4`) for supervised imitation, auxiliary heads,
/// or recurrent RL (`train4.py` / `train_rl4.py`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Step4 {
    /// Monotonic game identifier within a converted dataset.
    pub game_id: u32,
    /// Acting (or observing) seat (`0..8`).
    pub seat: u8,
    /// Monotonic step index for `(game_id, seat)`.
    pub step_cursor: u32,
    /// Turn number when this frame occurred.
    pub turn: u16,
    /// Whether this frame is an active decision (`true`) or a passive
    /// observation update (`false`).
    pub is_decision: bool,
    /// Per-step policy loss weight (`1.0` normal, `0.2` downweighted pass, `0.0` passive/forced, `P0-4`).
    #[serde(default = "default_loss_weight")]
    pub loss_weight: f32,
    /// Whether this is the terminal step of `(game_id, seat)` (`P0-5`).
    #[serde(default)]
    pub is_terminal: bool,
    /// Encoded v4 state (`entity_feat`, `seat_feat`, `global_feat`, `edges`).
    pub state: EncodedState4,
    /// Policy head kind (`0..4`).
    pub head: u8,
    /// `PendingKind4` (`0..28`).
    pub pending_kind: u8,
    /// Candidate numeric features (`num_candidates x 48`).
    pub cand_feat: Vec<Vec<f32>>,
    /// Candidate referenced entity index (`-1` if none).
    pub cand_entity_idx: Vec<i16>,
    /// Candidate referenced relative seat index (`-1` if none).
    pub cand_rel_seat_idx: Vec<i8>,
    /// Candidate referenced `CardIndex` (`0` if none).
    pub cand_card_id: Vec<u32>,
    /// Named face, or 255 to use the referenced entity's rules face.
    pub cand_face_id: Vec<u8>,
    /// Candidate referenced ability index (`-1` if none).
    pub cand_ability_idx: Vec<i8>,
    /// Legal `(blocker_entity_idx, attacker_entity_idx)` pairs for `PairPick`.
    pub pairs: Vec<[i16; 2]>,
    /// Ground-truth teacher or rollout action target.
    pub target: TeacherTarget4,
    /// Terminal side utility in `[0.0, 1.0]` (`side_utility`).
    pub outcome: f32,
    /// Auxiliary self-supervised targets.
    pub aux: AuxTargets4,
    /// Action provenance (`Net | House | Forced | Search`, `temperature`, `K`, `behavior_logprob`).
    pub provenance: StepProvenance4,
}

/// Summary returned by [`extract_game4`].
#[derive(Clone, Debug, Default)]
pub struct ExtractedGame4 {
    /// Extracted trajectory steps across all seats.
    pub steps: Vec<Step4>,
    /// Per-`PendingKind4` coverage counts observed in this game.
    pub coverage: CoverageMatrix4,
    /// Decisions skipped because `encode_teacher_action4` returned `None`.
    pub unencoded_decisions: u64,
}

/// Replays a single JSONL game `record` and extracts v4 trajectory steps
/// (`Step4`) across all seats.
///
/// # Errors
/// When the record header is missing/invalid, the version does not match, or
/// an input in the record is rejected by the engine.
pub fn extract_game4(
    record: &[u8],
    game_id: u32,
    include_passive_obs: bool,
) -> Result<ExtractedGame4, String> {
    extract_game4_with_sidecar(record, game_id, include_passive_obs, None)
}

/// Replays a single JSONL game `record` and extracts v4 trajectory steps
/// (`Step4`) across all seats, optionally restoring lossless rollout provenance
/// and `free_mask` from `sidecar` (`P0-2`, `P0-3`, `P0-4`, `P0-5`).
///
/// # Errors
/// When the record header is missing/invalid, the version does not match, or
/// an input in the record is rejected by the engine.
#[allow(
    clippy::too_many_lines,
    reason = "Keep audited replay and observation ordering contiguous"
)]
#[allow(
    clippy::cast_precision_loss,
    reason = "Outcome counts are normalized into f32 model targets"
)]
pub fn extract_game4_with_sidecar(
    record: &[u8],
    game_id: u32,
    include_passive_obs: bool,
    sidecar: Option<&[RolloutStepSidecar4]>,
) -> Result<ExtractedGame4, String> {
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

    let mut raw_steps: Vec<(Step4, i32)> = Vec::new();
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
                            raw_steps.push((
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
                            ));
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
                    raw_steps.push((
                        Step4 {
                            game_id,
                            seat: seat_u8,
                            step_cursor: sc,
                            turn: view.turn.min(u32::from(u16::MAX)) as u16,
                            is_decision: true,
                            loss_weight: step_loss_weight,
                            is_terminal: false,
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
                                .map(super::features4::Candidate4::named_face)
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
                    ));
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

    // Find the last step index and last decision step index for each seat (`is_terminal`, P0-5)
    let mut last_any_idx = [None::<usize>; MAX_SEATS];
    for (idx, (st, _)) in raw_steps.iter().enumerate() {
        let s = st.seat as usize;
        if s < MAX_SEATS {
            last_any_idx[s] = Some(idx);
        }
    }

    // Populate outcome & auxiliary targets on every step
    let n_steps = raw_steps.len();
    let mut steps = Vec::with_capacity(n_steps);
    let mut next_lives = final_lives;
    // Move each large encoded frame instead of duplicating the whole game.
    // Walking backwards also supplies the next same-seat life in constant time.
    for (i, (mut step, cur_life)) in raw_steps.into_iter().enumerate().rev() {
        let s = step.seat as usize;
        step.outcome = side_utility(step.seat, &teams, &surviving_seats, &winners);
        step.is_terminal = last_any_idx[s] == Some(i);
        let next_life = next_lives[s];
        next_lives[s] = cur_life;

        let min_opp_life = (0..seat_count)
            .filter(|&o| o != s)
            .map(|o| final_lives[o])
            .min()
            .unwrap_or(0);

        step.aux = AuxTargets4 {
            turns_remaining_norm: (final_turn.saturating_sub(u32::from(step.turn)) as f32 / 15.0)
                .clamp(0.0, 1.0),
            my_final_life_norm: (final_lives[s] as f32 / 40.0).clamp(-1.0, 1.0),
            opp_min_final_life_norm: (min_opp_life as f32 / 40.0).clamp(-1.0, 1.0),
            next_life_delta_norm: ((next_life - cur_life) as f32 / 10.0).clamp(-1.0, 1.0),
        };
        steps.push(step);
    }
    steps.reverse();

    Ok(ExtractedGame4 {
        steps,
        coverage,
        unencoded_decisions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::housedeck::HouseDeck;
    use crate::selfplay::{Caps, play_with_options, table};
    use baylee_core::preset::AIProfile;
    use std::time::Duration;

    #[test]
    fn extract_game4_replays_fair_selfplay_with_zero_unencoded_decisions() {
        let a = HouseDeck::named("allytifact").unwrap();
        let b = HouseDeck::named("victory").unwrap();
        let preset = table(19, &a, &b, [AIProfile::SHARP, AIProfile::EXPERT]);
        let played = play_with_options(
            &preset,
            "t-convert4-19",
            Caps {
                answers: 10_000,
                wall: Duration::from_secs(60),
            },
            true,
        );
        assert!(played.outcome.finished(), "{:?}", played.outcome);
        let extracted = extract_game4(&played.record, 1, false).expect("extracts cleanly");
        assert!(!extracted.steps.is_empty());
        assert_eq!(
            extracted.unencoded_decisions, 0,
            "every non-forced decision must encode into a v4 target"
        );
        for st in &extracted.steps {
            assert!((0.0..=1.0).contains(&st.outcome));
            assert!(!st.state.entity_feat.is_empty());
        }
        // Verify PassPriority steps are retained in trajectory order (with 0.2 weight on 5/6)
        // and final step of each seat trajectory has is_terminal = true
        let mut last_per_seat = std::collections::BTreeMap::new();
        for (idx, st) in extracted.steps.iter().enumerate() {
            last_per_seat.insert(st.seat, idx);
        }
        for (&_seat, &last_idx) in &last_per_seat {
            assert!(
                extracted.steps[last_idx].is_terminal,
                "final step of seat trajectory must have is_terminal = true"
            );
        }
    }

    #[test]
    fn observe_view_hides_opponent_private_hand_and_library() {
        use crate::selfplay::table_for;
        use baylee_core::ids::PlayerId;
        use baylee_gamehost::Session;

        let d1 = HouseDeck::named("allytifact").unwrap();
        let d2 = HouseDeck::named("victory").unwrap();
        let preset = table_for(99, &[&d1, &d2], &[AIProfile::STEADY; 2], &[]);
        let session = Session::new(&preset).unwrap();
        let v0 = session.observe_view(PlayerId::new(0));
        let v1 = session.observe_view(PlayerId::new(1));
        assert_eq!(v0.decision_player, Some(PlayerId::new(0)));
        assert_eq!(v1.decision_player, Some(PlayerId::new(1)));
        assert!(!v0.hand.is_empty());
        assert!(!v1.hand.is_empty());
        // Seat 0's hand card ObjectIds must not appear in Seat 1's hand
        for card in &v0.hand {
            assert!(!v1.hand.iter().any(|c| c.id == card.id));
        }
    }
}
