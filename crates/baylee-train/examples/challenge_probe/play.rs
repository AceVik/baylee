//! Full fresh executions and exact replay checks; no hidden-state search.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};
use baylee_ai::{HeuristicAgent, intelligence::DeckIntel, pending_player};
use baylee_core::{
    ids::PlayerId,
    preset::{AIProfile, GamePreset, SeatController},
};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::Session;

pub(super) struct Run {
    pub record: Vec<u8>,
    pub won: bool,
    pub stop: String,
    pub inputs: u64,
    pub turn: u32,
    pub fallbacks: usize,
    pub exact_replay: bool,
    pub player_trace: String,
    pub elapsed_ms: u128,
    actions: Vec<(u8, PlayerAction)>,
    state_hash: u64,
}

impl Run {
    pub fn same_play(&self, other: &Self) -> bool {
        self.won == other.won
            && self.stop == other.stop
            && self.actions == other.actions
            && self.state_hash == other.state_hash
            && self.fallbacks == other.fallbacks
    }
}

pub(super) fn run(
    preset: &GamePreset,
    profile: AIProfile,
    player_seed: u64,
    attempt: &str,
    args: &super::Args,
) -> Result<Run> {
    let mut preset = preset.clone();
    // Human controllers keep Session from silently advancing either seat.
    for (spec, user_id) in preset.seats.iter_mut().zip([1, 2]) {
        spec.controller = SeatController::Human { user_id };
    }
    let mut session =
        Session::new_recorded(&preset, baylee_build::short()).context("build session")?;
    session.describe(
        attempt.to_owned(),
        vec!["probe-player".into(), "probe-opponent".into()],
    );
    session.set_fair_scouting(true);
    session.discard_frames();
    let agents = [
        HeuristicAgent::new(profile).with_seed(player_seed),
        HeuristicAgent::new(AIProfile::STEADY).with_seed(9001),
    ];
    let decks: Vec<_> = preset
        .seats
        .iter()
        .map(|seat| DeckIntel::new(seat.deck.iter().map(|c| c.card).collect(), Vec::new()))
        .collect();
    let started = Instant::now();
    let mut actions = Vec::new();
    let mut won = false;
    let mut ended = false;
    let stop = loop {
        if let Pending::GameOver(result) = session.pending() {
            won = session.winning_seats(*result) == [PlayerId::new(0)];
            ended = true;
            break format!("{result:?}");
        }
        if actions.len() as u64 >= args.max_answers {
            break "answer_cap".into();
        }
        if started.elapsed() >= Duration::from_secs(args.max_seconds) {
            break "time_cap".into();
        }
        let seat = pending_player(session.pending()).context("no deciding seat")?;
        let (pending, view) = session.view_for(seat).context("missing authorized view")?;
        let at = usize::from(seat.get());
        let note = session.question_note(seat);
        let action = baylee_ai::noted::answer_as_seated(
            &agents[at],
            view,
            &pending,
            note.as_ref(),
            Some(&decks[at]),
        );
        if let Err(error) = session.act_slice(seat, action.clone(), 0) {
            // Preserve the record and stop; a fallback must not hide an invalid candidate.
            break format!("refused: {error}; action: {action:?}");
        }
        actions.push((seat.get(), action));
    };
    let elapsed_ms = started.elapsed().as_millis();
    let turn = session.state().turn.number;
    let record = session.take_record();
    let replay = baylee_gamehost::record::replay_exact(&record)
        .map_err(|error| anyhow::anyhow!("exact replay failed: {error:?}"))?;
    ensure!(replay.ended == ended, "replayed terminal state differs");
    let player_actions: Vec<_> = actions.iter().filter(|(seat, _)| *seat == 0).collect();
    Ok(Run {
        record,
        won,
        stop,
        turn,
        inputs: actions.len() as u64,
        fallbacks: agents.iter().map(HeuristicAgent::fallbacks).sum(),
        exact_replay: true,
        player_trace: super::digest(&serde_json::to_vec(&player_actions)?),
        elapsed_ms,
        actions,
        state_hash: replay.engine.snapshot_hash(),
    })
}
