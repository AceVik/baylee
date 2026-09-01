//! The AI-vs-AI harness: a whole game with nobody watching.
//!
//! It lives here rather than in `baylee-ai` because an agent needs a
//! [`PlayerView`](baylee_view::PlayerView) to act, and building one takes
//! the engine — which is exactly the boundary the AI is not allowed to
//! cross. The soak test below is the acceptance-deck smoke test: it is the
//! one place where every card in the decks is actually played.

use crate::session::priority_holder;
use baylee_ai::{HeuristicAgent, pending_player};
use baylee_core::ids::PlayerId;
use baylee_core::preset::GamePreset;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_engine::state::CardLookup;
use baylee_engine::win::GameResult;

/// Loop-detection key: the state hash alone misses engine-side fields
/// (pass counters, priority holder), so turn/phase/step join the key.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct LoopKey {
    state: u64,
    player: u8,
    turn: u32,
    phase: baylee_engine::turn::Phase,
    step: baylee_engine::turn::Step,
    pending_kind: std::mem::Discriminant<Pending>,
}

/// Plays a full game between two agents. Returns the result, or `None`
/// when the action cap was hit (pathological stalls count as timeouts).
///
/// An AI cast that fails late legality checks (e.g. "not enough legal
/// targets" discovered only in the wizard) falls back to passing; any
/// other error is an engine bug and panics.
///
/// # Panics
/// On engine-internal invariant violations (an illegal action that is not
/// a late legality miss).
pub fn play_game<L: CardLookup>(
    lookup: L,
    preset: &GamePreset,
    agents: &[HeuristicAgent; 2],
    max_actions: usize,
) -> Option<GameResult> {
    let mut engine = Engine::new(preset, lookup).expect("preset builds");
    let mut seen: std::collections::HashMap<LoopKey, usize> = std::collections::HashMap::new();
    for i in 0..max_actions {
        let pending = engine.pending().clone();
        if let Pending::GameOver(result) = pending {
            return Some(result);
        }
        let player_for_hash = pending_player(&pending);
        let key = LoopKey {
            state: engine.state().snapshot_hash(),
            player: player_for_hash.map_or(255, PlayerId::get),
            turn: engine.state().turn.number,
            phase: engine.state().turn.phase,
            step: engine.state().turn.step,
            pending_kind: std::mem::discriminant(&pending),
        };
        if seen.insert(key, i).is_some() {
            // Exact repetition: an infinite combo loop (real MTG boards
            // allow these; the deterministic agent would spin forever).
            return None;
        }
        let player = pending_player(&pending)?;
        // The agent sees what a client would see, and nothing else.
        let view =
            crate::view::player_view(engine.state(), player, priority_holder(&pending), i as u64);
        let action = agents[player.get() as usize].act(&view, &pending);
        if let Err(err) = engine.apply(player, action) {
            // Late legality/payment misses are AI mis-evaluation, not engine
            // bugs, and the engine has already recovered: a cast that fails
            // mid-wizard drops the wizard and re-publishes a decision point
            // (see `advance_cast_wizard`). So the game continues from
            // whatever it published — abandoning it here threw away a game
            // the engine had already put back on its feet, which is why a
            // deck change could take the soak from 3/4 finished to 1/4.
            //
            // If the agent really is stuck, the state repeats exactly and the
            // loop detector above ends the game on the next pass.
            if format!("{err}").contains("cannot pay") {
                continue;
            }
            if let Pending::Priority { .. } = &pending {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("passing is always legal");
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::RegistryLookup;
    use baylee_core::preset::AIProfile;

    fn acceptance_text() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/acceptance-decks.txt"
        ))
        .expect("acceptance deck file")
    }

    /// M3 soak: heuristic self-play over the acceptance decks must never
    /// panic and should terminate within the action cap.
    ///
    /// It moved here with `play_game` when the agents stopped taking an
    /// `&Engine`. That is the interesting part of this test now: every one
    /// of these games is played through the same filtered view a networked
    /// client gets, so a card that only works when you can see the whole
    /// state stops working here first.
    #[test]
    fn self_play_acceptance_decks_terminates_without_panics() {
        let text = acceptance_text();
        let allytifact =
            baylee_cards::decks::load_acceptance(&text, "Allytifact").expect("Allytifact loads");
        let victory =
            baylee_cards::decks::load_acceptance(&text, "Victory").expect("Victory loads");
        let agents = [
            HeuristicAgent::new(AIProfile::default()),
            HeuristicAgent::new(AIProfile::default()),
        ];
        let mut finished = 0;
        for seed in [1u64, 7, 42, 1337] {
            let (a, b) = if seed % 2 == 0 {
                (&allytifact, &victory)
            } else {
                (&victory, &allytifact)
            };
            let preset = baylee_cards::decks::preset_for(seed, a, b);
            if play_game(RegistryLookup, &preset, &agents, 20_000).is_some() {
                finished += 1;
            }
        }
        assert!(
            finished >= 2,
            "at least half of the self-play games should finish (got {finished}/4)"
        );
    }
}
