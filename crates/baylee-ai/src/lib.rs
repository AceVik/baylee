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

use baylee_core::ids::{Defender, ObjectId, PlayerId};
pub use baylee_core::preset::AIProfile;
use baylee_core::preset::{GamePreset, Politics};
use baylee_engine::choice::{Pending, PlayerAction, YesNoPrompt};
use baylee_engine::engine::Engine;
use baylee_engine::state::{CardLookup, GameState};
use baylee_engine::win::GameResult;

/// A greedy one-ply heuristic controller. Deterministic given the same
/// state (the engine's seeded RNG does all randomness).
#[derive(Clone, Debug)]
pub struct HeuristicAgent {
    /// Difficulty knobs. `politics` steers who this seat attacks; the
    /// evaluation knobs (lookahead, temperature, mulligan skill, hold-up)
    /// are still read by nobody and wait on the evaluator.
    profile: AIProfile,
}

impl HeuristicAgent {
    /// A default-profile agent.
    #[must_use]
    pub fn new(profile: AIProfile) -> Self {
        Self { profile }
    }

    /// Who this seat swings at, per its politics profile.
    ///
    /// In a duel every policy picks the only opponent, so this only starts
    /// to matter at three seats and up — where always taking
    /// `defenders.first()` meant one player absorbed every attack in the game
    /// purely for sitting in the lowest seat.
    fn pick_defender(&self, state: &GameState, me: PlayerId, defenders: &[PlayerId]) -> PlayerId {
        let life = |p: &PlayerId| state.players[p.get() as usize].life;
        match self.profile.politics {
            // Spread the aggression around without breaking determinism: the
            // journal position is the seed, so the same game always replays
            // the same way. `std::random` here would be a replay bug.
            Politics::Random => {
                let n = state
                    .journal
                    .last_seq()
                    .wrapping_add(u64::from(me.get()))
                    .wrapping_mul(0x9E37_79B9_7F4A_7C15);
                defenders[(n >> 33) as usize % defenders.len()]
            }
            // Whoever is closest to winning the race; their board breaks ties.
            Politics::AttackLeader => *defenders
                .iter()
                .max_by_key(|p| (life(p), board_pressure(state, **p)))
                .unwrap_or(&defenders[0]),
            // Archenemy: the biggest board is the threat, however low their
            // life has dropped — a player on 2 life with an empty board is
            // not what loses this game.
            Politics::Archenemy => *defenders
                .iter()
                .max_by_key(|p| (board_pressure(state, **p), life(p)))
                .unwrap_or(&defenders[0]),
        }
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
                let opponents: Vec<PlayerId> = engine
                    .state()
                    .players
                    .iter()
                    .filter(|p| p.id != player && !p.has_lost)
                    .map(|p| p.id)
                    .collect();
                if opponents.is_empty() {
                    return PlayerAction::DeclareAttackers { attackers: vec![] };
                }
                let victim = self.pick_defender(engine.state(), player, &opponents);
                let squad: Vec<ObjectId> = engine
                    .state()
                    .zones
                    .list(baylee_engine::zone::ZoneLocation::Battlefield)
                    .iter()
                    .copied()
                    .filter(|id| baylee_engine::combat::can_attack(engine.state(), player, *id))
                    .collect();
                let defender = aim_at(engine.state(), player, victim, &squad);
                let attackers = squad.into_iter().map(|id| (id, defender)).collect();
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
                // Kicker and tax are declined to keep the mana; a draw is
                // declined because the house AI has no match score to protect,
                // so accepting would only ever be a game given away.
                YesNoPrompt::Kicker
                | YesNoPrompt::PayTax { .. }
                | YesNoPrompt::DrawOffer { .. } => PlayerAction::YesNo(false),
                YesNoPrompt::Generic => PlayerAction::YesNo(true),
            },
            Pending::GameOver(_) => PlayerAction::PassPriority, // unreachable in the driver
        }
    }
}

/// Chooses what the squad actually swings at once politics has picked the
/// victim: one of their planeswalkers if this attack can finish it off,
/// otherwise the player.
///
/// Killing a walker is worth more than a few points of life, but only if
/// it actually dies — chipping a loyalty counter off a big planeswalker
/// while the controller's life total goes untouched is the worst of both.
/// So the bar is "total attacking power is at least its loyalty", and
/// among the walkers that clear it the cheapest one to kill wins.
///
/// The blockers the defender has not declared yet are not modelled; this
/// is the same one-ply optimism the rest of the heuristic runs on.
fn aim_at(state: &GameState, me: PlayerId, victim: PlayerId, squad: &[ObjectId]) -> Defender {
    let power: i32 = squad
        .iter()
        .filter_map(|id| state.object(*id))
        .map(|o| i32::from(o.characteristics().power.unwrap_or(0)))
        .sum();
    baylee_engine::combat::defender_options(state, me)
        .into_iter()
        .filter_map(|d| {
            let Defender::Planeswalker(id) = d else {
                return None;
            };
            let walker = state.object(id)?;
            if walker.controller != victim {
                return None;
            }
            let loyalty = i32::from(
                walker
                    .counters
                    .get(baylee_engine::object::CounterKind::Loyalty),
            );
            (loyalty > 0 && loyalty <= power).then_some((loyalty, d))
        })
        .min_by_key(|(loyalty, _)| *loyalty)
        .map_or(Defender::Player(victim), |(_, d)| d)
}

/// How much a player's board threatens: a point per permanent plus its
/// power, which reads an army of small creatures and one huge one as
/// comparably dangerous.
fn board_pressure(state: &GameState, player: PlayerId) -> i32 {
    state
        .zones
        .list(baylee_engine::zone::ZoneLocation::Battlefield)
        .iter()
        .filter_map(|id| state.object(*id))
        .filter(|o| o.controller == player)
        .map(|o| 1 + i32::from(o.characteristics().power.unwrap_or(0)))
        .sum()
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

/// The player who must answer a pending choice.
fn pending_player(pending: &Pending) -> Option<PlayerId> {
    match pending {
        Pending::Mulligan { player, .. }
        | Pending::MulliganBottom { player, .. }
        | Pending::Priority { player, .. }
        | Pending::ChooseAttackers { player, .. }
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

    fn island() -> CardIndex {
        baylee_cards::by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
            .expect("island")
            .index
    }
    fn ondu_cleric() -> CardIndex {
        baylee_cards::by_oracle_id("f4232466-dd6a-49bf-be6c-95905c3ded17")
            .expect("cleric")
            .index
    }

    /// Three seats: one opponent ahead on life with nothing out, one on the
    /// back foot with a board. The two policies should disagree about them.
    fn three_seat_engine() -> baylee_engine::engine::Engine<RegistryLookup> {
        use baylee_core::ids::PrintRef;
        use baylee_core::preset::{
            DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatController,
            SeatSpec,
        };
        let deck: Vec<DeckEntry> = (0..60)
            .map(|_| DeckEntry {
                card: island(),
                print: PrintRef::new(0),
            })
            .collect();
        let seat = |life: i32, board: usize| SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            deck: deck.clone(),
            sideboard: vec![],
            starting_life: Some(life),
            starting_hand: None,
            starting_battlefield: (0..board)
                .map(|_| DeckEntry {
                    card: ondu_cleric(),
                    print: PrintRef::new(0),
                })
                .collect(),
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            format: FormatId::Freeform,
            seed: 3,
            dev_mode: false,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: Finish::Normal,
            }],
            // seat 0 is us; seat 1 is ahead on life; seat 2 has the board.
            seats: vec![seat(40, 0), seat(40, 0), seat(5, 3)],
        };
        baylee_engine::engine::Engine::new(&preset, RegistryLookup).expect("engine builds")
    }

    /// The two threat policies read the same table differently: one goes for
    /// the player who is winning the race, the other for the biggest board.
    #[test]
    fn politics_decides_who_gets_attacked() {
        let engine = three_seat_engine();
        let me = PlayerId::new(0);
        let defenders = [PlayerId::new(1), PlayerId::new(2)];

        let leader = HeuristicAgent::new(AIProfile {
            politics: Politics::AttackLeader,
            ..AIProfile::default()
        });
        assert_eq!(
            leader.pick_defender(engine.state(), me, &defenders),
            PlayerId::new(1),
            "attack-leader goes for the player on 40 life"
        );

        let archenemy = HeuristicAgent::new(AIProfile {
            politics: Politics::Archenemy,
            ..AIProfile::default()
        });
        assert_eq!(
            archenemy.pick_defender(engine.state(), me, &defenders),
            PlayerId::new(2),
            "archenemy goes for the board, not the life total"
        );
    }

    /// "Random" must still be a function of the game state — a real RNG here
    /// would make replays and the soak diverge.
    #[test]
    fn random_politics_stays_deterministic() {
        let engine = three_seat_engine();
        let me = PlayerId::new(0);
        let defenders = [PlayerId::new(1), PlayerId::new(2)];
        let agent = HeuristicAgent::new(AIProfile {
            politics: Politics::Random,
            ..AIProfile::default()
        });
        let first = agent.pick_defender(engine.state(), me, &defenders);
        for _ in 0..10 {
            assert_eq!(agent.pick_defender(engine.state(), me, &defenders), first);
        }
        assert!(defenders.contains(&first));
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

    /// A duel where seat 1 has Jace (3 loyalty) out and seat 0 has
    /// `creatures` Ondu Clerics (1/1 each) ready to swing.
    fn walker_duel(creatures: usize) -> baylee_engine::engine::Engine<RegistryLookup> {
        use baylee_core::ids::PrintRef;
        use baylee_core::preset::{
            DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatController,
            SeatSpec,
        };
        let entry = |card: CardIndex| DeckEntry {
            card,
            print: PrintRef::new(0),
        };
        let deck: Vec<DeckEntry> = (0..60).map(|_| entry(island())).collect();
        let seat = |board: Vec<CardIndex>| SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            deck: deck.clone(),
            sideboard: vec![],
            starting_life: None,
            starting_hand: Some(vec![]),
            starting_battlefield: board.into_iter().map(entry).collect(),
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            format: FormatId::Freeform,
            seed: 5,
            dev_mode: false,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: Finish::Normal,
            }],
            seats: vec![
                seat((0..creatures).map(|_| ondu_cleric()).collect()),
                seat(vec![jace()]),
            ],
        };
        let mut engine =
            baylee_engine::engine::Engine::new(&preset, RegistryLookup).expect("engine builds");
        // The game has to actually start: loyalty counters are placed when
        // the opening board enters, not when the preset is read.
        for _ in 0..2 {
            let Pending::Mulligan { player, .. } = engine.pending().clone() else {
                panic!("expected a mulligan, got {:?}", engine.pending())
            };
            engine
                .apply(player, PlayerAction::MulliganKeep)
                .expect("keep");
        }
        engine
    }

    fn jace() -> CardIndex {
        baylee_cards::by_oracle_id("7f77a84e-5a4b-4834-aefa-3cecc175ae8e")
            .expect("jace")
            .index
    }

    fn my_creatures(engine: &baylee_engine::engine::Engine<RegistryLookup>) -> Vec<ObjectId> {
        engine
            .state()
            .zones
            .list(baylee_engine::zone::ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| {
                engine.state().object(*id).is_some_and(|o| {
                    o.controller == PlayerId::new(0)
                        && o.characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::CREATURE)
                })
            })
            .collect()
    }

    /// A planeswalker is worth attacking only when the attack kills it:
    /// three 1/1s finish a 3-loyalty Jace, so they go for the walker.
    #[test]
    fn a_squad_that_can_finish_a_planeswalker_goes_for_it() {
        let engine = walker_duel(3);
        let squad = my_creatures(&engine);
        assert_eq!(squad.len(), 3, "the squad did not deploy");
        let aim = aim_at(engine.state(), PlayerId::new(0), PlayerId::new(1), &squad);
        assert!(
            matches!(aim, Defender::Planeswalker(_)),
            "three power went to the player instead of killing the walker"
        );
    }

    /// Two 1/1s only chip it, which is the worst of both — so they hit the
    /// player instead.
    #[test]
    fn a_squad_that_would_only_chip_a_planeswalker_hits_the_player() {
        let engine = walker_duel(2);
        let squad = my_creatures(&engine);
        assert_eq!(squad.len(), 2, "the squad did not deploy");
        let aim = aim_at(engine.state(), PlayerId::new(0), PlayerId::new(1), &squad);
        assert_eq!(
            aim,
            Defender::Player(PlayerId::new(1)),
            "the squad chipped a walker it could not kill"
        );
    }
}
