use super::{
    AuxTargets4, CoverageMatrix4, Line, MAX_SEATS, Pending, PlayerId, RolloutStepSidecar4, Step4,
    side_utility,
};

#[derive(Debug, PartialEq)]
pub(super) struct Targets {
    outcomes: [f32; MAX_SEATS],
    min_other_life: [i32; MAX_SEATS],
    pub final_lives: [i32; MAX_SEATS],
    final_turn: u32,
    hash: u64,
}

impl Targets {
    pub fn new(
        seats: usize,
        teams: &[Option<u8>],
        survivors: &[u8],
        winners: &[u8],
        turn: u32,
        lives: [i32; MAX_SEATS],
        hash: u64,
    ) -> Self {
        let mut outcomes = [0.0; MAX_SEATS];
        let mut min_other_life = [0; MAX_SEATS];
        for s in 0..seats {
            outcomes[s] = side_utility(s as u8, teams, survivors, winners);
            // Preserve existing supervision exactly, including the historical
            // auxiliary's "other seats" treatment in team games.
            min_other_life[s] = (0..seats)
                .filter(|&o| o != s)
                .map(|o| lives[o])
                .min()
                .unwrap_or(0);
        }
        Self {
            outcomes,
            min_other_life,
            final_lives: lives,
            final_turn: turn,
            hash,
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "Existing normalized auxiliary targets"
    )]
    pub fn apply(&self, step: &mut Step4, current: i32, next: i32, terminal: bool) {
        let seat = step.seat as usize;
        step.outcome = self.outcomes[seat];
        step.is_terminal = terminal;
        step.aux = AuxTargets4 {
            turns_remaining_norm: (self.final_turn.saturating_sub(u32::from(step.turn)) as f32
                / 15.0)
                .clamp(0.0, 1.0),
            my_final_life_norm: (self.final_lives[seat] as f32 / 40.0).clamp(-1.0, 1.0),
            opp_min_final_life_norm: (self.min_other_life[seat] as f32 / 40.0).clamp(-1.0, 1.0),
            next_life_delta_norm: ((next - current) as f32 / 10.0).clamp(-1.0, 1.0),
        };
    }
}

pub(super) struct ReplaySummary {
    pub coverage: CoverageMatrix4,
    pub unencoded_decisions: u64,
    pub targets: Targets,
}

pub(super) struct FeatureReplaySummary {
    pub coverage: CoverageMatrix4,
    pub unencoded_decisions: u64,
    pub targets: Option<Targets>,
    pub snapshot_hash: u64,
}

/// Counters from a conversion retaining at most one encoded frame per seat.
#[derive(Debug)]
pub struct StreamedGame4 {
    /// Complete emitted observations and decisions.
    pub steps: u64,
    /// Peak retained frames, independent of game length.
    pub peak_buffered_frames: usize,
    /// Coverage of the recorded decisions.
    pub coverage: CoverageMatrix4,
    /// Decisions for which no exact supervised target was available.
    pub unencoded_decisions: u64,
    /// How the record was shown to be the game this build replayed.
    pub replay_check: ReplayCheck,
}

/// How a converted record was shown to be the recorded game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayCheck {
    /// Every engine hash matched: the build that played it.
    Exact,
    /// Another build: every input's public-table digest (`seen`) matched.
    SeenAcrossBuilds,
    /// Live exports: every recorded action was accepted, nothing compared.
    ActionsOnly,
}

impl ReplayCheck {
    /// The name written into dataset metadata.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::SeenAcrossBuilds => "seen_across_builds",
            Self::ActionsOnly => "actions_only",
        }
    }
}

/// Replays a record from another build against its `seen` digests, the
/// build-stable public table written after every input (`docs/protocol.md`
/// §"The game record"). Every input must carry one and match it.
fn replay_seen_across_builds(
    record: &[u8],
) -> Result<baylee_engine::engine::Engine<baylee_gamehost::RegistryLookup>, String> {
    use baylee_gamehost::record::seen;
    let digest =
        |engine: &baylee_engine::engine::Engine<_>| format!("{:016x}", seen(engine.state()));
    let mut lines = record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_slice::<Line>(l).map_err(|e| e.to_string()));
    let Some(Line::Header {
        record: version,
        preset,
        seen: opening,
        ..
    }) = lines.next().transpose()?
    else {
        return Err("no header".into());
    };
    if version != super::RECORD_VERSION {
        return Err(format!("record version {version} is not this build's"));
    }
    let mut engine = baylee_engine::engine::Engine::new(&preset, baylee_gamehost::RegistryLookup)
        .map_err(|_| "preset failed to build an engine".to_owned())?;
    if opening.as_deref() != Some(digest(&engine).as_str()) {
        return Err("seen replay audit: opening table differs".into());
    }
    let mut ended = false;
    for line in lines {
        match line? {
            Line::Header { .. } => return Err("second header in record".into()),
            Line::Chair { .. } | Line::DeclaredMind { .. } => {}
            Line::Input {
                n,
                seat,
                action,
                seen,
                ..
            } => {
                engine
                    .apply(PlayerId::new(seat), action)
                    .map_err(|_| format!("seen replay audit: input {n} refused"))?;
                if seen.as_deref() != Some(digest(&engine).as_str()) {
                    return Err(format!("seen replay audit: table differs after input {n}"));
                }
            }
            Line::End { n, .. } => {
                if !matches!(engine.pending(), Pending::GameOver(_)) {
                    return Err(format!("seen replay audit: game not over at end line {n}"));
                }
                ended = true;
            }
        }
    }
    if !ended {
        return Err("record has no verified terminal state".into());
    }
    Ok(engine)
}

pub(super) fn terminal_targets(
    record: &[u8],
    across_builds: bool,
) -> Result<(Targets, ReplayCheck), String> {
    // Replay once without expensive feature tensors. This verifies every
    // recorded state hash before any frame can be published and obtains only
    // training labels. Future information never reaches encode_state4.
    // A record from another build fails at its opening hash; only callers
    // whose labels carry no behaviour probabilities may check it by `seen`.
    let (engine, check) = match baylee_gamehost::record::replay_exact(record) {
        Ok(checked) if checked.ended => (checked.engine, ReplayCheck::Exact),
        Ok(_) => return Err("record has no verified terminal state".into()),
        Err(baylee_gamehost::record::ReplayError::Diverged { n: None, .. }) if across_builds => (
            replay_seen_across_builds(record)?,
            ReplayCheck::SeenAcrossBuilds,
        ),
        Err(e) => return Err(format!("replay audit: {e:?}")),
    };
    let Some(header) = record.split(|&b| b == b'\n').find(|l| !l.is_empty()) else {
        return Err("no header".into());
    };
    let Line::Header { preset, .. } =
        serde_json::from_slice::<Line>(header).map_err(|e| e.to_string())?
    else {
        return Err("no header".into());
    };
    let teams = preset.seats.iter().map(|s| s.team).collect::<Vec<_>>();
    let state = engine.state();
    let Pending::GameOver(result) = engine.pending() else {
        return Err("no GameOver".into());
    };
    let winners = (0..preset.seats.len())
        .filter(|&s| {
            result
                .winner
                .is_some_and(|winner| winner.includes(PlayerId::new(s as u8), teams[s]))
        })
        .map(|s| s as u8)
        .collect::<Vec<_>>();
    let mut lives = [0; MAX_SEATS];
    let mut survivors = Vec::new();
    for (s, life) in lives.iter_mut().enumerate().take(preset.seats.len()) {
        *life = state.players[s].life;
        if !state.has_left(PlayerId::new(s as u8)) {
            survivors.push(s as u8);
        }
    }
    Ok((
        Targets::new(
            preset.seats.len(),
            &teams,
            &survivors,
            &winners,
            state.turn.number,
            lives,
            engine.snapshot_hash(),
        ),
        check,
    ))
}

fn terminal_targets_live(record: &[u8]) -> Result<Targets, String> {
    let mut lines = record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_slice::<Line>(l).map_err(|e| e.to_string()));
    let Some(first) = lines.next() else {
        return Err("empty record".into());
    };
    let Line::Header {
        record: version,
        preset,
        ..
    } = first?
    else {
        return Err("first line is not a header".into());
    };
    if version != super::RECORD_VERSION {
        return Err(format!(
            "record version {version} != expected {}",
            super::RECORD_VERSION
        ));
    }
    let seat_count = preset.seats.len().clamp(1, MAX_SEATS);
    let teams: Vec<Option<u8>> = preset.seats.iter().map(|s| s.team).collect();
    let mut session =
        super::Session::new(&preset).ok_or_else(|| "preset failed to build session".to_owned())?;
    for s in 0..seat_count {
        session.take_over(PlayerId::new(s as u8));
    }
    let mut rec_winners: Option<Vec<u8>> = None;
    let mut final_turn: u32 = 1;
    for line in lines {
        match line? {
            Line::Header { .. } => return Err("second header in record".into()),
            Line::Chair { .. } | Line::DeclaredMind { .. } => {}
            Line::Input {
                seat: seat_u8,
                action,
                ..
            } => {
                let seat = PlayerId::new(seat_u8);
                if action.is_automation_setting() {
                    let _ = session.act(seat, action);
                    continue;
                }
                session
                    .act(seat, action)
                    .map_err(|e| format!("engine rejected recorded action for {seat:?}: {e}"))?;
                final_turn = session.state().turn.number;
            }
            Line::End {
                winners: w, turn, ..
            } => {
                rec_winners = Some(w);
                final_turn = turn.unwrap_or(final_turn);
            }
        }
    }
    let Some(rec_winners) = rec_winners else {
        return Err("record has no End line".into());
    };
    let Pending::GameOver(result) = session.pending() else {
        return Err("no GameOver".into());
    };
    let winners = (0..seat_count)
        .filter(|&s| {
            result
                .winner
                .is_some_and(|winner| winner.includes(PlayerId::new(s as u8), teams[s]))
        })
        .map(|s| s as u8)
        .collect::<Vec<_>>();
    if winners != rec_winners {
        return Err(format!(
            "replayed winners {winners:?} != recorded winners {rec_winners:?}"
        ));
    }
    let state = session.state();
    let mut lives = [0; MAX_SEATS];
    let mut survivors = Vec::new();
    for (s, life) in lives.iter_mut().enumerate().take(seat_count) {
        *life = state.players[s].life;
        if !state.has_left(PlayerId::new(s as u8)) {
            survivors.push(s as u8);
        }
    }
    Ok(Targets::new(
        seat_count,
        &teams,
        &survivors,
        &winners,
        final_turn,
        lives,
        session.snapshot_hash(),
    ))
}

/// Convert a complete record without retaining the whole game's observations.
///
/// A first replay checks hashes and determines final labels. The feature replay
/// buffers one frame per seat until its successor supplies the next-life label.
/// Per-seat order, every passive frame, targets and terminal markers are kept;
/// interleaving between seats may differ. Callers must publish output atomically
/// only after success, because an output error can leave a partial stream.
///
/// # Errors
/// Invalid/divergent records, inconsistent terminal labels, or callback errors.
pub fn stream_game4(
    record: &[u8],
    game_id: u32,
    include_passive_obs: bool,
    sidecar: Option<&[RolloutStepSidecar4]>,
    emit: impl FnMut(Step4) -> Result<(), String>,
) -> Result<StreamedGame4, String> {
    stream_game4_with_encoder(
        record,
        game_id,
        include_passive_obs,
        sidecar,
        None,
        None,
        false,
        7,
        emit,
    )
}

/// Convert a verified live-server record into a BC-only dataset (`--live-bc`).
///
/// # Errors
/// Any engine action refusal, missing/mismatched `GameOver`, unencoded engine
/// decision, or output failure.
pub fn stream_game4_live_bc(
    record: &[u8],
    game_id: u32,
    include_passive_obs: bool,
    emit: impl FnMut(Step4) -> Result<(), String>,
) -> Result<StreamedGame4, String> {
    stream_game4_with_encoder(
        record,
        game_id,
        include_passive_obs,
        None,
        None,
        None,
        true,
        7,
        emit,
    )
}

/// Convert a separate BC-only dataset with strictly verified shadow advice.
/// Original inputs and all passive observations remain unchanged.
///
/// # Errors
/// Any replay, label, identity or output failure; incomplete advice is never accepted.
pub fn stream_game4_shadow_bc(
    record: &[u8],
    game_id: u32,
    sidecar: &[RolloutStepSidecar4],
    labels: &mut super::shadow_bc::ShadowBc4,
    emit: impl FnMut(Step4) -> Result<(), String>,
) -> Result<StreamedGame4, String> {
    stream_game4_with_encoder(
        record,
        game_id,
        true,
        Some(sidecar),
        Some(labels),
        None,
        false,
        7,
        emit,
    )
}

/// Convert verified external LLM actions as BC-only targets, preserving every observation.
///
/// # Errors
/// Any record, origin, replay or output mismatch.
pub fn stream_game4_llm_bc(
    record: &[u8],
    game_id: u32,
    labels: &mut super::llm_bc::LlmBc4,
    emit: impl FnMut(Step4) -> Result<(), String>,
) -> Result<StreamedGame4, String> {
    stream_game4_with_encoder(
        record,
        game_id,
        true,
        None,
        None,
        Some(labels),
        false,
        7,
        emit,
    )
}

/// Producer encoder identity validated from the frozen run metadata.
/// Keeps neural behavior probabilities from crossing observation contracts.
#[derive(Clone, Debug)]
pub struct ProducerEncoder {
    encoder_version: u32,
}

impl ProducerEncoder {
    /// Validate explicit producer model/encoder metadata before replay admission.
    ///
    /// # Errors
    /// Missing, unknown or inconsistent producer contracts are rejected.
    pub fn from_run(run: &serde_json::Value) -> Result<Self, String> {
        let identity = &run["model_identity"];
        let encoder = identity["encoder_version"].as_u64();
        let model = identity["model_version"].as_u64();
        let architecture = identity["architecture"].as_str();
        let encoder_version = match (encoder, model, architecture) {
            (Some(7), Some(3), _)
            | (
                Some(7),
                Some(5),
                Some(
                    "baylee_ai_v5_relational_v1"
                    | "baylee_ai_v5_relational_v2"
                    | "baylee_ai_v5_relational_v2_candidate_v2"
                    | "baylee_ai_v5_relational_v2_candidate_v3",
                ),
            ) => 7,
            (
                Some(8),
                Some(5),
                Some(
                    "baylee_ai_v5_relational_v2"
                    | "baylee_ai_v5_relational_v2_candidate_v2"
                    | "baylee_ai_v5_relational_v2_candidate_v3",
                ),
            ) => 8,
            (
                Some(9),
                Some(5),
                Some("baylee_ai_v5_relational_v2" | "baylee_ai_v5_relational_v2_candidate_v2"),
            ) if identity["global_feat_width"] == 82 => 9,
            _ => return Err("Missing or unsupported producer encoder identity".into()),
        };
        Ok(Self { encoder_version })
    }
}

/// Convert a complete record using an explicitly selected encoder and label mode.
/// Existing conversion entry points preserve Encoder 7.
///
/// # Errors
/// Unsupported encoders, producer mismatches, replay failures or output errors.
#[allow(clippy::too_many_arguments)]
pub fn stream_game4_with_encoder(
    record: &[u8],
    game_id: u32,
    include_passive_obs: bool,
    sidecar: Option<&[RolloutStepSidecar4]>,
    shadow: Option<&mut super::shadow_bc::ShadowBc4>,
    llm: Option<&mut super::llm_bc::LlmBc4>,
    live_bc: bool,
    encoder_version: u32,
    emit: impl FnMut(Step4) -> Result<(), String>,
) -> Result<StreamedGame4, String> {
    stream_game4_with_encoder_and_producer(
        record,
        game_id,
        include_passive_obs,
        sidecar,
        shadow,
        llm,
        live_bc,
        encoder_version,
        None,
        emit,
    )
}

/// Convert with an explicit verified neural producer identity.
///
/// # Errors
/// Encoder 8 sidecars require a matching producer; all ordinary replay checks apply.
#[allow(clippy::too_many_arguments)]
pub fn stream_game4_with_encoder_and_producer(
    record: &[u8],
    game_id: u32,
    include_passive_obs: bool,
    sidecar: Option<&[RolloutStepSidecar4]>,
    shadow: Option<&mut super::shadow_bc::ShadowBc4>,
    llm: Option<&mut super::llm_bc::LlmBc4>,
    live_bc: bool,
    encoder_version: u32,
    producer: Option<&ProducerEncoder>,
    mut emit: impl FnMut(Step4) -> Result<(), String>,
) -> Result<StreamedGame4, String> {
    if !matches!(encoder_version, 7..=9) {
        return Err("Unsupported encoder version".into());
    }
    if encoder_version == 9 && (shadow.is_some() || llm.is_some()) {
        return Err("Encoder9 shadow and LLM labels are not supported".into());
    }
    if let Some(labels) = shadow.as_ref()
        && labels.encoder_version != encoder_version
    {
        return Err("Shadow producer/converter encoder mismatch".into());
    }
    if let Some(producer) = producer
        && producer.encoder_version != encoder_version
    {
        return Err("Producer/converter encoder mismatch".into());
    }
    if matches!(encoder_version, 8 | 9)
        && sidecar.is_some()
        && shadow.is_none()
        && producer.is_none_or(|producer| producer.encoder_version != encoder_version)
    {
        return Err(format!(
            "Encoder {encoder_version} neural sidecars require verified producer encoder {encoder_version}"
        ));
    }
    // LLM demonstrations are scarce and outlive the build that played them;
    // their labels carry no behaviour probabilities, so features re-encoded
    // on this build remain valid imitation targets.
    let (targets, replay_check) = if live_bc {
        (terminal_targets_live(record)?, ReplayCheck::ActionsOnly)
    } else {
        terminal_targets(record, llm.is_some() && sidecar.is_none())?
    };
    let mut pending: [Option<(Step4, i32)>; MAX_SEATS] = std::array::from_fn(|_| None);
    let mut steps = 0;
    let mut peak_buffered_frames = 0;
    let replay = super::replay::replay_frames_modes(
        record,
        game_id,
        include_passive_obs,
        sidecar,
        shadow,
        llm,
        live_bc,
        encoder_version,
        |step, life| {
            let seat = step.seat as usize;
            if let Some((mut previous, previous_life)) = pending[seat].take() {
                targets.apply(&mut previous, previous_life, life, false);
                emit(previous)?;
                steps += 1;
            }
            pending[seat] = Some((step, life));
            peak_buffered_frames = peak_buffered_frames.max(pending.iter().flatten().count());
            Ok(())
        },
    )?;
    if replay.targets != targets {
        return Err("feature replay differs from audited terminal labels".into());
    }
    for (mut last, life) in pending.into_iter().flatten() {
        let next = targets.final_lives[last.seat as usize];
        targets.apply(&mut last, life, next, true);
        emit(last)?;
        steps += 1;
    }
    Ok(StreamedGame4 {
        steps,
        peak_buffered_frames,
        coverage: replay.coverage,
        unencoded_decisions: replay.unencoded_decisions,
        replay_check,
    })
}
