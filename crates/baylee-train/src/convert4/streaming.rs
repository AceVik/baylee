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
}

fn terminal_targets(record: &[u8]) -> Result<Targets, String> {
    // Replay once without expensive feature tensors. This verifies every
    // recorded state hash before any frame can be published and obtains only
    // training labels. Future information never reaches encode_state4.
    let checked =
        baylee_gamehost::record::replay(record).map_err(|e| format!("replay audit: {e:?}"))?;
    if !checked.ended {
        return Err("record has no verified terminal state".into());
    }
    let Some(header) = record.split(|&b| b == b'\n').find(|l| !l.is_empty()) else {
        return Err("no header".into());
    };
    let Line::Header { preset, .. } =
        serde_json::from_slice::<Line>(header).map_err(|e| e.to_string())?
    else {
        return Err("no header".into());
    };
    let teams = preset.seats.iter().map(|s| s.team).collect::<Vec<_>>();
    let state = checked.engine.state();
    let Pending::GameOver(result) = checked.engine.pending() else {
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
    Ok(Targets::new(
        preset.seats.len(),
        &teams,
        &survivors,
        &winners,
        state.turn.number,
        lives,
        checked.engine.snapshot_hash(),
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
    stream_inner(
        record,
        game_id,
        include_passive_obs,
        sidecar,
        None,
        None,
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
    stream_inner(
        record,
        game_id,
        true,
        Some(sidecar),
        Some(labels),
        None,
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
    stream_inner(record, game_id, true, None, None, Some(labels), emit)
}

fn stream_inner(
    record: &[u8],
    game_id: u32,
    include_passive_obs: bool,
    sidecar: Option<&[RolloutStepSidecar4]>,
    shadow: Option<&mut super::shadow_bc::ShadowBc4>,
    llm: Option<&mut super::llm_bc::LlmBc4>,
    mut emit: impl FnMut(Step4) -> Result<(), String>,
) -> Result<StreamedGame4, String> {
    let targets = terminal_targets(record)?;
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
    })
}
