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
    mut net: Option<&mut NetPlayer4>,
    fair_scouting: bool,
    max_decisions: u64,
    record_steps: bool,
) -> anyhow::Result<TableOutcome> {
    let preset = table_for(seed, decks, profiles, teams_u8);
    let n_seats = decks.len();
    if let Some(n) = net.as_deref_mut() {
        // Independent policy RNG seed: never depends on deal shuffle `seed` (P0-7)
        let indep_policy_seed =
            policy_seed ^ deal_idx.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ ((n_seats as u64) << 48);
        n.reseed(indep_policy_seed);
        n.init_game(&preset);
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
    if net.is_some() {
        for s in 0..n_seats {
            if is_net_seat(s, focal_seat, teams) {
                session.take_over(PlayerId::new(s as u8));
            }
        }
    }

    let mut net_answers = 0_u64;
    let mut fallbacks = 0_u64;
    let mut refused = 0_u64;
    let mut stalls = 0_u64;
    let mut latencies = Vec::new();
    let mut rollout_sidecar: Vec<RolloutStepSidecar4> = Vec::new();
    let mut visit_counts = vec![std::collections::BTreeMap::<u64, u8>::new(); n_seats];

    loop {
        if let Pending::GameOver(result) = session.pending() {
            let winners: Vec<u8> = session
                .winning_seats(*result)
                .into_iter()
                .map(PlayerId::get)
                .collect();
            let surviving: Vec<u8> = if winners.is_empty() {
                (0..n_seats as u8).collect()
            } else {
                winners.clone()
            };
            let util = side_utility(focal_seat as u8, teams, &surviving, &winners);
            let outcome_json = match winners.as_slice() {
                [] => json!({"kind": "draw"}),
                [w] => json!({"kind": "won", "seat": w}),
                ws => json!({"kind": "team_won", "seats": ws}),
            };
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
            ));
        }
        if session.decision_seq() >= max_decisions {
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
            ));
        }

        // Passively observe the table for all neural seats after opponent progression (P0-4)
        if let Some(n) = net.as_deref_mut() {
            for s in 0..n_seats {
                if is_net_seat(s, focal_seat, teams) {
                    let pid = PlayerId::new(s as u8);
                    let obs_v = session.observe_view(pid);
                    n.observe(&obs_v)?;
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
            if !is_net_seat(s, focal_seat, teams) {
                continue;
            }
            let pid = PlayerId::new(s as u8);
            let Some((pending, view)) = session.view_for(pid) else {
                continue;
            };
            let vh = view_only_state_hash(&view, &pending);
            let vc = seat_visits.entry(vh).or_insert(0);
            *vc = vc.saturating_add(1);
            if *vc > 4 {
                stalls += 1;
                fallbacks += 1;
                let act = session.house_action(pid).context("house fallback")?;
                if session.act_slice(pid, act, 0).is_ok() && record_steps {
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
                let ans = n.act_detailed(&view, &pending, teams);
                let dt_ms = t0.elapsed().as_secs_f64() * 1000.0;
                if ans.provenance.source == SourceKind4::Net {
                    latencies.push(dt_ms);
                    net_answers += 1;
                } else if ans.provenance.source == SourceKind4::House {
                    fallbacks += 1;
                }
                if session.act_slice(pid, ans.action, 0).is_ok() {
                    if record_steps {
                        rollout_sidecar.push(RolloutStepSidecar4 {
                            provenance: ans.provenance,
                            free_mask: ans.target.free_mask,
                            pick_sequence: ans.target.pick_sequence,
                        });
                    }
                } else {
                    refused += 1;
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

