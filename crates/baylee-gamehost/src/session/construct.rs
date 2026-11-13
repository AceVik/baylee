#[cfg(any(test, feature = "foresight"))]
use super::foresight;
use super::{RegistryLookup, SeatKind, Session, challenge, own_prints, seat_byte, spectators};
use crate::log::GameLog;
use baylee_ai::HeuristicAgent;
use baylee_core::ids::SeatSet;
use baylee_core::preset::{AiHandSharePolicy, GamePreset, SeatController};
use baylee_engine::engine::Engine;
use baylee_view::{StaticSender, ViewSender};
use std::collections::BTreeMap;

/// The cosmetic tabletop seed: the same for every seat, spectator and replay
/// of the game, because it is derived from the preset and the opening public
/// table, which a replay has too.
///
/// `preset.seed` dealt the hands, so this must not give it away. A chain of
/// invertible mixers would: with the public table known, 24 bits of output
/// leave 2^40 candidate seeds, few enough to test offline against a seat's
/// own opening hand. SHA-256 is not invertible, so finding the seed from this
/// costs a search of all 2^64 seeds, the same search a seat's hand alone
/// already demands.
fn table_seed_of(preset: &GamePreset, state: &baylee_engine::state::GameState) -> u64 {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"baylee table surface v1\0");
    hash.update(preset.seed.to_le_bytes());
    hash.update(crate::record::seen(state).to_le_bytes());
    hash.update((preset.seats.len() as u64).to_le_bytes());
    let digest = hash.finalize();
    let low = u64::from(digest[0]) | u64::from(digest[1]) << 8 | u64::from(digest[2]) << 16;
    low | 0x0100_0000
}

impl Session {
    /// Starts a game from a preset; Open seats become humans, AI seats
    /// get a heuristic agent.
    #[must_use]
    pub fn new(preset: &GamePreset) -> Option<Self> {
        if !challenge::valid(preset) {
            return None;
        }
        let seats: Vec<SeatKind> = preset
            .seats
            .iter()
            .enumerate()
            .map(|(i, s)| match &s.controller {
                SeatController::Ai(profile) => Some(SeatKind::Ai(
                    HeuristicAgent::new(*profile)
                        // Not `preset.seed`: that stream dealt the hands.
                        // A table nobody has named yet gets the derivation
                        // with no identifier, which is still independent of
                        // the shuffle; `describe` supplies the real one.
                        .with_seed(challenge::policy_seed(
                            &preset.house_rules,
                            "",
                            seat_byte(i),
                        ))
                        .with_teams(preset.seats.iter().map(|s| s.team).collect()),
                )),
                _ => Some(SeatKind::Human),
            })
            .collect::<Option<_>>()?;
        let engine = Engine::new(preset, RegistryLookup).ok()?;
        let table_seed = table_seed_of(preset, engine.state());
        let log = GameLog::new(engine.state());
        let mut session = Self {
            engine,
            seats,
            scouting_decks: crate::scouting::decks(preset).into(),
            seq: 0,
            decisions: 0,
            house_rules: preset.house_rules.clone(),
            asked_at: vec![0; preset.seats.len()],
            clock: vec![None; preset.seats.len()],
            prints: preset.prints.clone(),
            revealed: preset
                .seats
                .iter()
                .map(|spec| own_prints(spec, preset.prints.len()))
                .collect(),
            teams: preset.seats.iter().map(|s| s.team).collect(),
            house_answered: vec![None; preset.seats.len()],
            policy_acts: vec![Vec::new(); preset.seats.len()],
            settings_made: vec![0; preset.seats.len()],
            policy_counted: vec![0; preset.seats.len()],
            policy_read: 0,
            now_ms: 0,
            lost: Vec::new(),
            roster_dirty: vec![false; preset.seats.len()],
            game_id: String::new(),
            table_seed,
            names: Vec::new(),
            guests: Vec::new(),
            log,
            told: vec![0; preset.seats.len()],
            sent: vec![ViewSender::default(); preset.seats.len()],
            statics_sent: vec![StaticSender::default(); preset.seats.len()],
            showing: vec![SeatSet::new(); preset.seats.len()],
            shown_once: BTreeMap::new(),
            asking: vec![SeatSet::new(); preset.seats.len()],
            declined: BTreeMap::new(),
            ai_hand_share_policy: AiHandSharePolicy::default(),
            record: None,
            minds: vec![None; preset.seats.len()],
            unsaid: vec![false; preset.seats.len()],
            declarations: vec![0; preset.seats.len()],
            retold: vec![(0, 0); preset.seats.len()],
            gallery: spectators::Gallery {
                revealed: vec![false; preset.prints.len()],
                ..spectators::Gallery::default()
            },
            fair_scouting: preset.house_rules.challenge_policy_seed.is_some(),
            #[cfg(any(test, feature = "foresight"))]
            foresight: foresight::Desk::new(preset.seats.len()),
            last_attempt: None,
            frames_read: true,
        };
        // A unit test reads a frame on its own, so it is sent whole views; a
        // debug build goes on proving each patch beside them. The tests that
        // read a stream are `tests/view_patches.rs` and the end-to-end suites.
        session.house_asks_its_teammates();
        if cfg!(test) {
            session.tell_views_whole();
        }
        Some(session)
    }
}
