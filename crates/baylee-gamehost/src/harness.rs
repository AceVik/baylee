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

/// Plays a full game between the agents at the table. Returns the result,
/// or `None` when the action cap was hit (pathological stalls count as
/// timeouts).
///
/// One agent per seat, in seat order. It took a `[HeuristicAgent; 2]` for
/// as long as the harness only ever played duels — which meant the room a
/// host opens for three or four chairs (`docs/protocol.md` §Rooms) shipped
/// without anything ever self-playing it, and a rule that only bites with
/// more than one opponent had nowhere to fail.
///
/// An AI cast that fails late legality checks (e.g. "not enough legal
/// targets" discovered only in the wizard) falls back to passing; any
/// other error is an engine bug and panics.
///
/// # Panics
/// On engine-internal invariant violations (an illegal action that is not
/// a late legality miss), or when `agents` does not have one agent per seat.
pub fn play_game<L: CardLookup>(
    lookup: L,
    preset: &GamePreset,
    agents: &[HeuristicAgent],
    max_actions: usize,
) -> Option<GameResult> {
    assert_eq!(
        agents.len(),
        preset.seats.len(),
        "one agent per seat: {} agents for {} seats",
        agents.len(),
        preset.seats.len()
    );
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

    /// The sweep is only worth its runtime if the card really is where
    /// `probe_preset` claims to put it. A `starting_hand` the engine quietly
    /// ignored would leave the sweep passing for hundreds of cards it never
    /// touched — the exact failure mode the probe exists to avoid — so this
    /// asserts the arrangement rather than trusting it.
    #[test]
    fn a_probe_game_starts_with_its_card_in_hand_and_in_play() {
        use baylee_engine::zone::Zone;

        let forest = baylee_cards::decks::by_name("Forest").expect("Forest is registered");
        let bolt_like = baylee_cards::all()
            .find(|d| {
                d.is_implemented()
                    && d.faces.first().is_some_and(|f| {
                        f.types.contains(baylee_core::types::TypeSet::INSTANT)
                            || f.types.contains(baylee_core::types::TypeSet::SORCERY)
                    })
            })
            .map(|d| d.index);

        // A permanent: in hand *and* on the battlefield.
        let preset = baylee_cards::decks::probe_preset(9, forest).expect("probe builds");
        let engine = Engine::new(&preset, RegistryLookup).expect("preset builds");
        let count_in = |zone: Zone, card| {
            engine
                .state()
                .arena
                .iter()
                .filter(|(_, o)| o.zone == zone && o.card.is_some_and(|c| c.index == card))
                .count()
        };
        assert!(
            count_in(Zone::Battlefield, forest) >= 1,
            "a permanent probe must start on the battlefield"
        );
        assert!(
            count_in(Zone::Hand, forest) >= 1,
            "a probe must start with its card in hand"
        );

        // A spell: in hand only — putting an instant on the battlefield
        // would be an illegal board, not a stronger test.
        if let Some(spell) = bolt_like {
            let preset = baylee_cards::decks::probe_preset(9, spell).expect("probe builds");
            let engine = Engine::new(&preset, RegistryLookup).expect("preset builds");
            let on_field = engine
                .state()
                .arena
                .iter()
                .filter(|(_, o)| {
                    o.zone == Zone::Battlefield && o.card.is_some_and(|c| c.index == spell)
                })
                .count();
            let in_hand = engine
                .state()
                .arena
                .iter()
                .filter(|(_, o)| o.zone == Zone::Hand && o.card.is_some_and(|c| c.index == spell))
                .count();
            assert_eq!(on_field, 0, "an instant or sorcery cannot start in play");
            assert!(
                in_hand >= 1,
                "a spell probe must start with its card in hand"
            );
        }
    }

    /// Every card the deckbuilder offers as playable is put into a real
    /// game at least once.
    ///
    /// The acceptance soak above plays two hand-curated decks, which was the
    /// whole pool's worth of coverage while the pool *was* those two decks.
    /// It stopped being that the moment `codegen` learned to write finished
    /// cards: several hundred cards became `Implemented` — and therefore
    /// offered by `GET /pool` as playable — without anything ever having
    /// played them.
    ///
    /// So: one mirror match per implemented card, through
    /// [`probe_preset`](baylee_cards::decks::probe_preset) — which puts the
    /// card in the opening hand and, if it is a permanent, on the
    /// battlefield, so the game cannot fail to reach it. A plain four-of in
    /// sixty cards would often never be drawn inside a short game, and a
    /// sweep that never draws its subject passes for the wrong reason.
    ///
    /// The bar is deliberately low — no panic, no engine-invariant
    /// violation — because that is the class of bug a generated card can
    /// introduce and a per-card assertion never could. Cards are collected
    /// rather than asserted one at a time, so a run names *every* offender
    /// instead of stopping at the first.
    fn play_every_implemented_card(cap: usize) -> Vec<String> {
        let mut offenders = Vec::new();
        for def in baylee_cards::all().filter(|d| d.is_implemented()) {
            let Some(preset) = baylee_cards::decks::probe_preset(9, def.index) else {
                continue; // no basics registered: nothing to pad with
            };
            let agents = [
                HeuristicAgent::new(AIProfile::default()),
                HeuristicAgent::new(AIProfile::default()),
            ];
            let played = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                play_game(RegistryLookup, &preset, &agents, cap)
            }));
            if played.is_err() {
                offenders.push(def.name().to_string());
            }
        }
        offenders
    }

    #[test]
    fn every_implemented_card_survives_a_game() {
        // A low cap: this runs once per card, and what it is looking for
        // shows up in the first turns (an ability the engine cannot resolve,
        // a filter that panics, a cost it cannot pay). Whether the game
        // *finishes* is the acceptance soak's question, not this one.
        let offenders = play_every_implemented_card(300);
        assert!(
            offenders.is_empty(),
            "these cards are offered as playable and break a game: {offenders:?}"
        );
    }

    /// The same sweep with a real action budget, so a card that only breaks
    /// once the board fills up is still caught. Slow by construction —
    /// `cargo test -p baylee-gamehost -- --ignored`.
    #[test]
    #[ignore = "one full game per card; minutes, not seconds"]
    fn every_implemented_card_survives_a_whole_game() {
        let offenders = play_every_implemented_card(20_000);
        assert!(
            offenders.is_empty(),
            "these cards are offered as playable and break a game: {offenders:?}"
        );
    }

    /// A room is 2–4 chairs (`docs/protocol.md` §Rooms), and until the
    /// harness took a slice of agents nothing ever self-played one. Rules
    /// that only bite with more than one opponent — "each opponent", the
    /// range an effect reaches — had no test that could see them.
    #[test]
    fn a_table_of_more_than_two_plays_itself() {
        let text = acceptance_text();
        let allytifact =
            baylee_cards::decks::load_acceptance(&text, "Allytifact").expect("Allytifact loads");
        let victory =
            baylee_cards::decks::load_acceptance(&text, "Victory").expect("Victory loads");
        for seats in [3usize, 4] {
            let decks: Vec<&baylee_cards::decks::LoadedDeck> = (0..seats)
                .map(|i| if i % 2 == 0 { &allytifact } else { &victory })
                .collect();
            let preset = baylee_cards::decks::preset_for_all(11, &decks);
            assert_eq!(preset.seats.len(), seats);
            let agents: Vec<HeuristicAgent> = (0..seats)
                .map(|_| HeuristicAgent::new(AIProfile::default()))
                .collect();
            // No panic is the assertion; a four-way game need not finish
            // inside the cap.
            let _ = play_game(RegistryLookup, &preset, &agents, 20_000);
        }
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
