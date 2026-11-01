fn is_net_seat(seat: usize, focal_seat: usize, teams: &[Option<u8>]) -> bool {
    if seat == focal_seat {
        return true;
    }
    if let Some(Some(tf)) = teams.get(focal_seat)
        && let Some(Some(ts)) = teams.get(seat)
    {
        return tf == ts;
    }
    false
}

type TableOutcome = (
    f32,
    bool,
    u64,
    u64,
    u64,
    u64,
    Vec<f64>,
    Vec<u8>,
    Value,
    Vec<RolloutStepSidecar4>,
    Vec<Value>,
);

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "Ordered observation, action and recording loop"
)]
fn play_one_table(
    game_id: &str,
    deal_idx: u64,
    focal_seat: usize,
    seed: u64,
    policy_seed: u64,
    decks: &[&HouseDeck],
    profiles: &[AIProfile],
    teams_u8: &[u8],
    teams: &[Option<u8>],
    mut net: Option<&mut dyn EvaluationPlayer>,
    fair_scouting: bool,
    max_decisions: u64,
    record_steps: bool,
    mut shadow: Option<&mut shadow::ShadowGame>,
    research_encoder: Option<u32>,
    mut search: Option<&mut lookahead::GameSearch>,
) -> anyhow::Result<TableOutcome> {
    let preset = table_for(seed, decks, profiles, teams_u8);
    let n_seats = decks.len();
    if let Some(n) = net.as_deref_mut() {
        n.configure_seats(focal_seat, teams)?;
        // Independent policy RNG seed: never depends on deal shuffle `seed` (P0-7)
        let indep_policy_seed =
            policy_seed ^ deal_idx.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ ((n_seats as u64) << 48);
        n.reseed(indep_policy_seed);
        n.init_game(&preset);
        if let Some(search) = search.as_deref_mut() {
            search.begin(indep_policy_seed);
        }
    }

    let mut session = if record_steps {
        Session::new_recorded(&preset, baylee_build::short()).context("new_recorded")?
    } else {
        Session::new(&preset).context("Session::new")?
    };
    session.describe(
        game_id.to_owned(),
        (0..n_seats).map(|s| s.to_string()).collect(),
    );
    session.set_fair_scouting(fair_scouting);
    if let Some(n) = net.as_deref() {
        for s in 0..n_seats {
            if n.controls_seat(s, focal_seat, teams) {
                session.take_over(PlayerId::new(s as u8));
            }
        }
    }

    let mut fallback_events = Vec::new();
    let mut net_answers = 0_u64;
    let mut fallbacks = 0_u64;
    let mut refused = 0_u64;
    let mut stalls = 0_u64;
    let mut latencies = Vec::new();
    let mut rollout_sidecar: Vec<RolloutStepSidecar4> = Vec::new();
    let mut research_samples = Vec::new();
    let mut visit_counts = vec![std::collections::BTreeMap::<u64, u8>::new(); n_seats];

    macro_rules! research_stop {
        ($seat:expr, $guard:expr, $reason:expr, $hash:expr, $proposal:expr) => {{
            let snapshot = session.snapshot_hash();
            let record = session.take_record();
            let outcome = failure_terminal::stop(
                &record,
                &rollout_sidecar,
                &research_samples,
                research_encoder.context("research encoder")?,
                focal_seat,
                $seat,
                $guard,
                $reason,
                $hash,
                $proposal,
                snapshot,
            )?;
            return Ok((
                0.0,
                $reason == "answer_cap",
                net_answers,
                fallbacks,
                refused,
                stalls,
                latencies,
                record,
                outcome,
                rollout_sidecar,
                fallback_events,
            ));
        }};
    }

    loop {
        if let Pending::GameOver(result) = session.pending() {
            let winners: Vec<u8> = session
                .winning_seats(*result)
                .into_iter()
                .map(PlayerId::get)
                .collect();
            // One scale with the search's terminal leaves.
            let util = terminal_utility(focal_seat as u8, teams, &winners, n_seats);
            let mut outcome_json = match winners.as_slice() {
                [] => json!({"kind": "draw"}),
                [w] => json!({"kind": "won", "seat": w}),
                ws => json!({"kind": "team_won", "seats": ws}),
            };
            if research_encoder.is_some() {
                outcome_json = failure_terminal::natural(&outcome_json, util, &research_samples);
            }
            return Ok((
                util,
                false,
                net_answers,
                fallbacks,
                refused,
                stalls,
                latencies,
                session.take_record(),
                outcome_json,
                rollout_sidecar,
                fallback_events,
            ));
        }
        if session.decision_seq() >= max_decisions {
            if let Some(encoder) = research_encoder {
                let pid = session
                    .pending()
                    .asked()
                    .context("cap boundary has no acting seat")?;
                let hash = failure_terminal::boundary_view_hash(&session, pid, encoder)?;
                research_stop!(pid, false, "answer_cap", hash, None);
            }
            return Ok((
                0.5,
                true,
                net_answers,
                fallbacks,
                refused,
                stalls,
                latencies,
                session.take_record(),
                json!({"kind": "answer_cap"}),
                rollout_sidecar,
                fallback_events,
            ));
        }

        // Passively observe the table for all neural seats after opponent progression (P0-4)
        let payment_ctx = net
            .as_deref()
            .is_some_and(EvaluationPlayer::uses_payment_context);
        let announcement_ctx = net
            .as_deref()
            .is_some_and(EvaluationPlayer::uses_announcement_context);
        let uses_ctx = net.as_deref().is_some_and(EvaluationPlayer::uses_context);
        if let Some(n) = net.as_deref_mut() {
            for s in 0..n_seats {
                if n.controls_seat(s, focal_seat, teams) {
                    let pid = PlayerId::new(s as u8);
                    let obs_v = if announcement_ctx {
                        session.observe_view_with_announcement_context(pid)
                    } else if payment_ctx {
                        session.observe_view_with_payment_context(pid)
                    } else if uses_ctx {
                        session.observe_view_with_context(pid)
                    } else {
                        session.observe_view(pid)
                    };
                    let observation = n.observe(&obs_v);
                    if research_encoder.is_some() {
                        observation.context(attempts::OBSERVATION_ERROR)?;
                    } else {
                        observation?;
                    }
                }
            }
        }

        // Observe every pre-action root. A multi-answer pump would skip states
        // that the record converter subsequently replays, breaking recurrent PPO.
        if session.needs_pump() {
            session.pump_at_most(1);
            continue;
        }

        for (s, seat_visits) in visit_counts.iter_mut().enumerate() {
            if !net
                .as_deref()
                .is_some_and(|n| n.controls_seat(s, focal_seat, teams))
            {
                continue;
            }
            let pid = PlayerId::new(s as u8);
            let pending_view = if announcement_ctx {
                session.view_for_with_announcement_context(pid)
            } else if payment_ctx {
                session.view_for_with_payment_context(pid)
            } else if uses_ctx {
                session.view_for_with_context(pid)
            } else {
                session.view_for(pid)
            };
            let Some((pending, view)) = pending_view else {
                continue;
            };
            let vh = if announcement_ctx {
                baylee_train::features9::view_only_state_hash9(&view, &pending)
            } else if payment_ctx {
                baylee_train::features8::view_only_state_hash8(&view, &pending)
            } else {
                view_only_state_hash(&view, &pending)
            };
            let vc = seat_visits.entry(vh).or_insert(0);
            *vc = vc.saturating_add(1);
            if *vc > 4 {
                stalls += 1;
                if research_encoder.is_some() {
                    let proposal = session.house_action(pid);
                    research_stop!(pid, true, "outer_repeated_view", vh, proposal.as_ref());
                }
                fallbacks += 1;
                let act = session.house_action(pid).context("house fallback")?;
                let sequence = session.decision_seq();
                let record_offset = session.record_pending();
                let result = session.act_slice(pid, act.clone(), 0);
                fallback_events.push(json!({"decision_seq":sequence,"record_byte_offset":record_offset,"seat":pid.get(),
                    "reason":"outer_repeated_view","view_hash":format!("{vh:016x}"),
                    "visits":*vc,"action":act,"accepted":result.is_ok()}));
                if result.is_ok() && record_steps {
                    rollout_sidecar.push(RolloutStepSidecar4 {
                        provenance: StepProvenance4 {
                            source: SourceKind4::House,
                            has_logprob: false,
                            ..StepProvenance4::default()
                        },
                        free_mask: Vec::new(),
                        pick_sequence: Vec::new(),
                    });
                }
                break;
            }

            if let Some(n) = net.as_deref_mut() {
                let t0 = Instant::now();
                let searched = match search.as_deref_mut() {
                    Some(search) => search.answer(n, &session, &view, &pending, teams)?,
                    None => None,
                };
                let answer = match searched {
                    Some(answer) => Ok(Some(answer)),
                    None => n.answer(&view, &pending, teams),
                };
                if research_encoder.is_some() && (answer.is_err() || matches!(answer, Ok(None))) {
                    research_stop!(pid, false, "unsupported_or_inference_error", vh, None);
                }
                let ans = match answer? {
                    Some(answer) => answer,
                    None => baylee_train::netplay4::Answer4 {
                        action: session
                            .house_action(pid)
                            .context("legacy unsupported choice")?,
                        value: 0.5,
                        provenance: StepProvenance4 {
                            source: SourceKind4::House,
                            has_logprob: false,
                            ..StepProvenance4::default()
                        },
                        target: baylee_train::features4::TeacherTarget4::default(),
                    },
                };
                if research_encoder.is_some() && ans.provenance.source == SourceKind4::House {
                    let repeated = n.fallback_diagnostic().is_some_and(|diagnostic| {
                        diagnostic.reason == baylee_train::netplay4::FallbackReason4::RepeatedView
                    });
                    research_stop!(
                        pid,
                        repeated,
                        if repeated {
                            "internal_repeated_view"
                        } else {
                            "policy_fallback"
                        },
                        vh,
                        Some(&ans.action)
                    );
                }
                let dt_ms = t0.elapsed().as_secs_f64() * 1000.0;
                if ans.provenance.source == SourceKind4::Net {
                    latencies.push(dt_ms);
                    net_answers += 1;
                } else if ans.provenance.source == SourceKind4::House {
                    fallbacks += 1;
                }
                let proposal = if let Some(shadow) = shadow.as_deref_mut() {
                    if ans.provenance.source == SourceKind4::Net {
                        let (state, menu) = n
                            .policy_encoding(&view, &pending, teams)
                            .context("Shadow requires exact runtime encoding")?;
                        shadow.propose(
                            &view,
                            &pending,
                            &state,
                            &menu,
                            &ans,
                            teams,
                            session.decision_seq(),
                            session.record_pending(),
                        )?
                    } else {
                        shadow.exclude_non_neural();
                        None
                    }
                } else {
                    None
                };
                let recovery = if ans.provenance.source == SourceKind4::House {
                    Some(n.fallback_diagnostic().map_or_else(
                        || json!({"reason":"house_policy_answer"}),
                        |diagnostic| {
                            serde_json::to_value(diagnostic).expect("serializable diagnostic")
                        },
                    ))
                } else {
                    None
                };
                let sequence = session.decision_seq();
                let record_offset = session.record_pending();
                let attempted_action = ans.action.clone();
                let research_binding = research_encoder
                    .map(|encoder| {
                        failure_terminal::sample_binding(
                            &session,
                            pid,
                            encoder,
                            &attempted_action,
                            &RolloutStepSidecar4 {
                                provenance: ans.provenance,
                                free_mask: ans.target.free_mask.clone(),
                                pick_sequence: ans.target.pick_sequence.clone(),
                            },
                        )
                    })
                    .transpose()?;
                let result = session.act_slice(pid, ans.action, 0);
                if let Some(diagnostic) = recovery {
                    fallback_events.push(json!({"decision_seq":sequence,"record_byte_offset":record_offset,"seat":pid.get(),
                        "diagnostic":diagnostic,"action":attempted_action,"accepted":result.is_ok()}));
                }
                if let Some(shadow) = shadow.as_deref_mut() {
                    shadow.executed(proposal, result.is_ok())?;
                }
                if result.is_ok() {
                    if let Some(mut binding) = research_binding {
                        binding["post_snapshot_hash"] =
                            json!(format!("{:016x}", session.snapshot_hash()));
                        research_samples.push(binding);
                    }
                    if record_steps {
                        rollout_sidecar.push(RolloutStepSidecar4 {
                            provenance: ans.provenance,
                            free_mask: ans.target.free_mask,
                            pick_sequence: ans.target.pick_sequence,
                        });
                    }
                } else if let Err(error) = result {
                    refused += 1;
                    fallback_events.push(json!({"decision_seq":sequence,"record_byte_offset":record_offset,"seat":pid.get(),
                        "reason":"engine_refusal","action":attempted_action,"accepted":false,
                        "error":error}));
                    eprintln!(
                        "[eval4] refused deal={deal_idx} seed={seed} seat={} action={attempted_action:?} pending={pending:?}: {error}",
                        pid.get(),
                    );
                    if research_encoder.is_some() {
                        research_stop!(pid, false, "engine_refusal", vh, None);
                    }
                    let fb = session
                        .house_action(pid)
                        .context("house fallback after refusal")?;
                    if session.act_slice(pid, fb, 0).is_ok() && record_steps {
                        rollout_sidecar.push(RolloutStepSidecar4 {
                            provenance: StepProvenance4 {
                                source: SourceKind4::House,
                                has_logprob: false,
                                ..StepProvenance4::default()
                            },
                            free_mask: Vec::new(),
                            pick_sequence: Vec::new(),
                        });
                    }
                }
            }
            break;
        }
    }
}
