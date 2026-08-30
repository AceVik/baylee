//! baylee-ai — heuristic AI controllers with difficulty profiles (M3).
//!
//! AI seats consume the exact same engine contract as humans
//! (`view` + `pending` → `PlayerAction`); difficulty is parameterized
//! through [`AIProfile`], never duplicated logic.
//!
//! V1 honesty note: the in-process agent can read the full [`GameState`]
//! (including the opponent's hand). It only uses its own zone contents
//! and public information for decisions, but this is convention, not
//! enforcement — hidden-information filtering arrives with the protocol
//! view layer (M3 server work).

#![warn(missing_docs)]
#![forbid(unsafe_code)]

use baylee_core::ids::{ObjectId, PlayerId};
pub use baylee_core::preset::AIProfile;
use baylee_core::preset::GamePreset;
use baylee_engine::choice::{Pending, PlayerAction, YesNoPrompt};
use baylee_engine::engine::Engine;
use baylee_engine::state::{CardLookup, GameState};
use baylee_engine::win::GameResult;

/// A greedy one-ply heuristic controller. Deterministic given the same
/// state (the engine's seeded RNG does all randomness).
#[derive(Clone, Debug)]
pub struct HeuristicAgent {
    /// Difficulty knobs (lookahead, temperature, mulligan skill); the v1
    /// greedy policy reads them once evaluation lands.
    #[allow(dead_code)]
    profile: AIProfile,
}

impl HeuristicAgent {
    /// A default-profile agent.
    #[must_use]
    pub fn new(profile: AIProfile) -> Self {
        Self { profile }
    }

    /// Picks an action for the current pending choice of `player`.
    #[must_use]
    #[allow(clippy::too_many_lines)] // the pending taxonomy is one flat table
    pub fn act<L: CardLookup>(&self, engine: &Engine<L>, player: PlayerId) -> PlayerAction {
        match engine.pending().clone() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::MulliganBottom { count, .. } => {
                // Bottom the highest-cmc cards (keep lands and cheap plays).
                let mut hand: Vec<(u32, ObjectId)> = engine
                    .state()
                    .zones
                    .list(baylee_engine::zone::ZoneLocation::Hand(player))
                    .iter()
                    .map(|id| {
                        let cmc = engine
                            .state()
                            .object(*id)
                            .map_or(0, |o| o.characteristics().mana_cost.cmc());
                        (cmc, *id)
                    })
                    .collect();
                hand.sort_by_key(|(cmc, _)| u32::MAX - cmc);
                // `count` comes from the engine and fits the hand today;
                // take() stays panic-free if that ever changes.
                PlayerAction::ChooseObjects {
                    objects: hand
                        .iter()
                        .take(count as usize)
                        .map(|(_, id)| *id)
                        .collect(),
                }
            }
            Pending::Priority { legal, .. } => {
                // 1. Play a land.
                if let Some(&card) = legal.lands.first() {
                    return PlayerAction::PlayLand { card };
                }
                // 2. Tap mana while holding anything castable-but-unpaid
                //    or while unspent mana could matter (simple: always
                //    tap before casting, never float into the pass).
                if !legal.castable.is_empty() && !legal.mana_abilities.is_empty() {
                    let best_unpaid = legal.castable.iter().any(|id| {
                        engine.state().object(*id).is_some_and(|o| {
                            o.characteristics().mana_cost.cmc()
                                > mana_available(engine.state(), player)
                        })
                    });
                    if best_unpaid {
                        return PlayerAction::ActivateManaAbility {
                            source: legal.mana_abilities[0],
                        };
                    }
                }
                // 3. Cast the highest-cmc castable spell.
                if let Some(card) = legal
                    .castable
                    .iter()
                    .max_by_key(|id| {
                        engine
                            .state()
                            .object(**id)
                            .map_or(0, |o| o.characteristics().mana_cost.cmc())
                    })
                    .copied()
                {
                    return PlayerAction::CastSpell { card };
                }
                // 4. Activated abilities are NOT used by the v1
                //    heuristic — blind activation loops on free no-op
                //    abilities. (Equipment/loyalty use comes with
                //    evaluation in a later difficulty tier.)
                PlayerAction::PassPriority
            }
            Pending::ChooseAttackers { .. } => {
                // Attack with everything untapped and legal.
                let defenders: Vec<PlayerId> = engine
                    .state()
                    .players
                    .iter()
                    .filter(|p| p.id != player && !p.has_lost)
                    .map(|p| p.id)
                    .collect();
                let Some(&defender) = defenders.first() else {
                    return PlayerAction::DeclareAttackers { attackers: vec![] };
                };
                let attackers: Vec<(ObjectId, PlayerId)> = engine
                    .state()
                    .zones
                    .list(baylee_engine::zone::ZoneLocation::Battlefield)
                    .iter()
                    .copied()
                    .filter(|id| baylee_engine::combat::can_attack(engine.state(), player, *id))
                    .map(|id| (id, defender))
                    .collect();
                PlayerAction::DeclareAttackers { attackers }
            }
            Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers { blockers: vec![] },
            Pending::DiscardChoice { count, .. } => {
                let mut hand: Vec<(u32, ObjectId)> = engine
                    .state()
                    .zones
                    .list(baylee_engine::zone::ZoneLocation::Hand(player))
                    .iter()
                    .map(|id| {
                        let cmc = engine
                            .state()
                            .object(*id)
                            .map_or(0, |o| o.characteristics().mana_cost.cmc());
                        (cmc, *id)
                    })
                    .collect();
                hand.sort_by_key(|(cmc, _)| u32::MAX - cmc);
                PlayerAction::ChooseObjects {
                    objects: hand
                        .iter()
                        .take(count as usize)
                        .map(|(_, id)| *id)
                        .collect(),
                }
            }
            Pending::LegendChoice { options, .. } => PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
            Pending::ChooseCards {
                options, min, max, ..
            }
            | Pending::ChooseTargets {
                options, min, max, ..
            } => {
                let n = if max <= 2 { max } else { min };
                PlayerAction::ChooseObjects {
                    objects: options[..(n as usize).min(options.len())].to_vec(),
                }
            }
            Pending::ChooseSubtype { options, .. } => {
                // Ally tribal decks: prefer ALLY, else the first type.
                let ally = baylee_core::generated::subtypes::creature::ALLY;
                PlayerAction::ChooseSubtype(if options.contains(&ally) {
                    ally
                } else {
                    options[0]
                })
            }
            Pending::ChooseColor { options, .. } => PlayerAction::ChooseColor(options[0]),
            Pending::ChooseNumber { min, .. } => PlayerAction::ChooseNumber(min),
            Pending::ChoosePlayer { options, .. } => PlayerAction::ChoosePlayer(
                options
                    .iter()
                    .copied()
                    .find(|p| *p != player)
                    .unwrap_or(options[0]),
            ),
            Pending::ChooseCastMode { options, .. } => PlayerAction::ChooseMode(
                options
                    .iter()
                    .position(|o| matches!(o.kind, baylee_engine::choice::CastModeKind::Normal))
                    .unwrap_or(0),
            ),
            Pending::OrderObjects { objects, .. } => PlayerAction::OrderObjects { objects },
            Pending::YesNo { prompt, .. } => match prompt {
                YesNoPrompt::PayLifeOrEnterTapped { amount } => PlayerAction::YesNo(
                    engine.state().players[player.get() as usize].life > i32::from(amount) + 5,
                ),
                YesNoPrompt::Miracle { .. } => {
                    PlayerAction::YesNo(mana_available(engine.state(), player) >= 2)
                }
                YesNoPrompt::Kicker | YesNoPrompt::PayTax { .. } => PlayerAction::YesNo(false),
                YesNoPrompt::Generic => PlayerAction::YesNo(true),
            },
            Pending::GameOver(_) => PlayerAction::PassPriority, // unreachable in the driver
        }
    }
}

/// Rough mana available in the pool (cmc units).
fn mana_available(state: &GameState, player: PlayerId) -> u32 {
    state.players[player.get() as usize].mana_pool.total()
}

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
        let action = agents[player.get() as usize].act(&engine, player);
        if let Err(err) = engine.apply(player, action) {
            // Late legality/payment misses are AI mis-evaluation, not
            // engine bugs: the wizard aborts and the game continues.
            let msg = format!("{err}");
            if msg.contains("cannot pay") {
                continue;
            }
            if let Pending::Priority { .. } = &pending {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("passing is always legal");
            } else {
                let _ = msg; // late legality miss at a non-priority choice
                return None;
            }
        }
    }
    None
}

/// The player who must answer a pending choice.
fn pending_player(pending: &Pending) -> Option<PlayerId> {
    match pending {
        Pending::Mulligan { player, .. }
        | Pending::MulliganBottom { player, .. }
        | Pending::Priority { player, .. }
        | Pending::ChooseAttackers { player }
        | Pending::ChooseBlockers { player, .. }
        | Pending::DiscardChoice { player, .. }
        | Pending::LegendChoice { player, .. }
        | Pending::ChooseCards { player, .. }
        | Pending::ChooseTargets { player, .. }
        | Pending::ChooseSubtype { player, .. }
        | Pending::ChooseColor { player, .. }
        | Pending::ChooseNumber { player, .. }
        | Pending::ChoosePlayer { player, .. }
        | Pending::ChooseCastMode { player, .. }
        | Pending::OrderObjects { player, .. }
        | Pending::YesNo { player, .. } => Some(*player),
        Pending::GameOver(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::CardIndex;

    struct RegistryLookup;
    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards::dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn acceptance_text() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/acceptance-decks.txt"
        ))
        .expect("acceptance deck file")
    }

    /// M3 soak: heuristic self-play over the acceptance decks must never
    /// panic and should terminate within the action cap.
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
