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
    pending: u64,
}

/// The open question, in full.
///
/// This was `discriminant(&pending)` — the *kind* of question — and that is
/// blind to the one place a game legitimately asks two different questions
/// over an unchanged board: a cast in progress. The wizard lives on
/// `Engine`, not in the `GameState` the hash is taken of, so "cast a spell
/// with kicker, decline the kicker" repeated a key it had not repeated a
/// situation of, and a game was called a loop at turn 34.
///
/// Making the key finer can only lose detections, never invent them, and
/// what it loses is a loop that varies its question — which the action cap
/// still catches, now reported as [`Halt::CapReached`] rather than
/// misfiled as a loop.
fn pending_fingerprint(pending: &Pending) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    format!("{pending:?}").hash(&mut h);
    h.finish()
}

/// Why a harness game stopped.
#[derive(Clone, Debug)]
pub enum Halt {
    /// The game ended by its own rules — somebody won, or it was a draw.
    Finished(GameResult),
    /// The same decision point came round again, exactly: a real loop, and
    /// the deterministic agents would spin on it forever.
    Repeated {
        /// The action number the state was first seen at.
        first_seen: usize,
    },
    /// The action cap ran out with the game still moving. Not the same
    /// thing as a loop at all: the game was going somewhere, just slowly.
    CapReached,
}

/// What one seat looked like when the game stopped.
#[derive(Clone, Copy, Debug)]
pub struct SeatSnapshot {
    /// Life total.
    pub life: i32,
    /// Cards in hand.
    pub hand: usize,
    /// Cards left in the library — a game that is going nowhere and a game
    /// about to end on an empty draw look identical without this.
    pub library: usize,
    /// Permanents controlled.
    pub permanents: usize,
    /// How many of those are creatures.
    pub creatures: usize,
}

/// What the agents actually did, counted over the whole game.
///
/// The end state says how a game finished and not how it was played, and
/// those turned out to be very different questions: every acceptance game
/// ends, and none of them ends in combat.
#[derive(Clone, Copy, Debug, Default)]
pub struct Tally {
    /// Lands played.
    pub lands: usize,
    /// Spells cast.
    pub spells: usize,
    /// Abilities activated by hand (mana abilities not counted).
    pub abilities: usize,
    /// Attack declarations that named at least one creature.
    pub attacks: usize,
    /// Attack declarations that named none.
    pub attacks_declined: usize,
    /// Block declarations that named at least one blocker.
    pub blocks: usize,
    /// Block declarations that named none.
    pub blocks_declined: usize,
    /// Mana abilities activated.
    pub taps: usize,
    /// Actions the engine refused because the cost could not be paid.
    ///
    /// The agent compares a card's mana *value* against the pool's total,
    /// which is a comparison that ignores colour, so this is the count of
    /// times it tapped up and then could not pay after all.
    pub refused_cost: usize,
    /// Actions the engine refused for any other reason.
    pub refused_other: usize,
}

/// A played harness game, with enough of the end state to say *why* it
/// stopped.
///
/// [`play_game`] answers `Option<GameResult>`, and a `None` there covers
/// two situations that want opposite fixes: a genuine loop, and a game
/// still making progress when the cap ran out. The soak asserts on a count
/// of those `None`s and so could never explain itself.
#[derive(Clone, Debug)]
pub struct Report {
    /// Why it stopped.
    pub halt: Halt,
    /// Actions taken.
    pub actions: usize,
    /// Turn number reached.
    pub turn: u32,
    /// Phase and step it stopped in, and the question that was open —
    /// `Debug` text, because this exists to be read by a person looking at
    /// a game that stopped somewhere it should not have.
    pub at: String,
    /// Every seat, in seat order.
    pub seats: Vec<SeatSnapshot>,
    /// What the agents did, in seat order.
    pub tally: Vec<Tally>,
    /// The last handful of question/answer pairs, newest last, and empty
    /// for a game that ended properly.
    ///
    /// A repeat says *that* the game came round again and never *how*, and
    /// the how is one line: the question asked, and what was answered to
    /// it. Kept as a short ring rather than a full trace because it is
    /// carried by every game the soak plays and only ever read by the few
    /// that stop badly.
    pub trail: Vec<String>,
}

impl Report {
    /// Whether the game reached its own ending.
    #[must_use]
    pub const fn finished(&self) -> bool {
        matches!(self.halt, Halt::Finished(_))
    }

    /// Attaches the last few question/answer pairs.
    #[must_use]
    fn with_trail(mut self, trail: Vec<String>) -> Self {
        self.trail = trail;
        self
    }
}

/// Plays a full game between the agents at the table. Returns the result,
/// or `None` when the game did not reach one — see [`play_report`] for
/// which of the two ways that happens.
///
/// # Panics
/// As [`play_report`].
pub fn play_game<L: CardLookup>(
    lookup: L,
    preset: &GamePreset,
    agents: &[HeuristicAgent],
    max_actions: usize,
) -> Option<GameResult> {
    match play_report(lookup, preset, agents, max_actions).halt {
        Halt::Finished(result) => Some(result),
        Halt::Repeated { .. } | Halt::CapReached => None,
    }
}

/// Plays a full game and reports how it ended.
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
pub fn play_report<L: CardLookup>(
    lookup: L,
    preset: &GamePreset,
    agents: &[HeuristicAgent],
    max_actions: usize,
) -> Report {
    assert_eq!(
        agents.len(),
        preset.seats.len(),
        "one agent per seat: {} agents for {} seats",
        agents.len(),
        preset.seats.len()
    );
    let mut engine = Engine::new(preset, lookup).expect("preset builds");
    let mut seen: std::collections::HashMap<LoopKey, usize> = std::collections::HashMap::new();
    let mut tally = vec![Tally::default(); preset.seats.len()];
    let mut trail: Vec<String> = Vec::with_capacity(TRAIL);
    for i in 0..max_actions {
        let pending = engine.pending().clone();
        if let Pending::GameOver(result) = pending {
            return report(&engine, Halt::Finished(result), i, tally);
        }
        let player_for_hash = pending_player(&pending);
        let key = LoopKey {
            state: engine.state().snapshot_hash(),
            player: player_for_hash.map_or(255, PlayerId::get),
            turn: engine.state().turn.number,
            phase: engine.state().turn.phase,
            step: engine.state().turn.step,
            pending: pending_fingerprint(&pending),
        };
        if let Some(first_seen) = seen.insert(key, i) {
            // Exact repetition: an infinite combo loop (real MTG boards
            // allow these; the deterministic agent would spin forever).
            return report(&engine, Halt::Repeated { first_seen }, i, tally).with_trail(trail);
        }
        // `GameOver` is the only pending nobody answers, and it returned
        // above.
        let player = player_for_hash.expect("a decision point has a seat");
        // The agent sees what a client would see, and nothing else.
        let view = crate::view::player_view(
            engine.state(),
            player,
            priority_holder(&pending),
            i as u64,
            Some(&pending),
            engine.automation(player).hold.suppresses(),
        );
        let action = agents[player.get() as usize].act(&view, &pending);
        if trail.len() == TRAIL {
            trail.remove(0);
        }
        trail.push(format!("{i}: {} → {action:?}", short(&pending)));
        // Counted before it is applied, and only for the shapes that say
        // something about how the game is being *played*: an answer to a
        // trigger is not a decision anybody watches for.
        let t = &mut tally[player.get() as usize];
        match &action {
            PlayerAction::PlayLand { .. } => t.lands += 1,
            PlayerAction::CastSpell { .. } => t.spells += 1,
            PlayerAction::ActivateAbility { .. } => t.abilities += 1,
            PlayerAction::ActivateManaAbility { .. } => t.taps += 1,
            PlayerAction::DeclareAttackers { attackers } => {
                if attackers.is_empty() {
                    t.attacks_declined += 1;
                } else {
                    t.attacks += 1;
                }
            }
            PlayerAction::DeclareBlockers { blockers } => {
                if blockers.is_empty() {
                    t.blocks_declined += 1;
                } else {
                    t.blocks += 1;
                }
            }
            _ => {}
        }
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
                tally[player.get() as usize].refused_cost += 1;
                continue;
            }
            tally[player.get() as usize].refused_other += 1;
            if let Pending::Priority { .. } = &pending {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("passing is always legal");
            }
        }
    }
    report(&engine, Halt::CapReached, max_actions, tally).with_trail(trail)
}

/// How many question/answer pairs a [`Report`] keeps.
const TRAIL: usize = 24;

/// One question, short enough to put on a line.
fn short(pending: &Pending) -> String {
    // The variant, not the whole payload: a `Priority` carries every legal
    // action, and printing those is a page per line.
    match pending {
        Pending::Priority { player, .. } => format!("Priority({})", player.get()),
        other => format!("{other:?}").chars().take(200).collect(),
    }
}

/// The end state, as one line per seat.
fn report<L: CardLookup>(
    engine: &Engine<L>,
    halt: Halt,
    actions: usize,
    tally: Vec<Tally>,
) -> Report {
    use baylee_core::types::TypeSet;
    use baylee_engine::zone::ZoneLocation;

    let state = engine.state();
    let battlefield = state.zones.list(ZoneLocation::Battlefield);
    let seats = (0..state.players.len())
        .map(|i| {
            let seat = PlayerId::new(u8::try_from(i).expect("a table fits in a u8"));
            let mine = || {
                battlefield
                    .iter()
                    .filter_map(|id| state.object(*id))
                    .filter(move |o| o.controller == seat)
            };
            SeatSnapshot {
                life: state.players[i].life,
                hand: state.zones.list(ZoneLocation::Hand(seat)).len(),
                library: state.zones.list(ZoneLocation::Library(seat)).len(),
                permanents: mine().count(),
                creatures: mine()
                    .filter(|o| o.characteristics().types.contains(TypeSet::CREATURE))
                    .count(),
            }
        })
        .collect();
    Report {
        halt,
        actions,
        turn: state.turn.number,
        at: format!(
            "{:?}/{:?} {}",
            state.turn.phase,
            state.turn.step,
            short(engine.pending())
        ),
        seats,
        tally,
        trail: Vec::new(),
    }
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
        let mut attacks = 0;
        let mut lines = Vec::new();
        for seed in [1u64, 7, 42, 1337] {
            let (a, b) = if seed % 2 == 0 {
                (&allytifact, &victory)
            } else {
                (&victory, &allytifact)
            };
            let preset = baylee_cards::decks::preset_for(seed, a, b);
            let r = play_report(RegistryLookup, &preset, &agents, 20_000);
            if r.finished() {
                finished += 1;
            }
            attacks += r.tally.iter().map(|t| t.attacks).sum::<usize>();
            let seats: Vec<String> = r
                .seats
                .iter()
                .zip(&r.tally)
                .map(|(s, t)| {
                    format!(
                        "{}life h{} l{} b{}/{}c · {}land {}spell {}abil {}atk/{} {}blk/{} · {}tap {}refused-cost {}refused-other",
                        s.life,
                        s.hand,
                        s.library,
                        s.permanents,
                        s.creatures,
                        t.lands,
                        t.spells,
                        t.abilities,
                        t.attacks,
                        t.attacks_declined,
                        t.blocks,
                        t.blocks_declined,
                        t.taps,
                        t.refused_cost,
                        t.refused_other,
                    )
                })
                .collect();
            lines.push(format!(
                "seed {seed}: {:?} after {} actions on turn {} at {}\n    {}",
                r.halt,
                r.actions,
                r.turn,
                r.at,
                seats.join("\n    ")
            ));
            for step in &r.trail {
                lines.push(format!("      {step}"));
            }
        }
        // The report rather than the count, because "2/4" says nothing about
        // whether the other two looped or were still playing when the cap ran
        // out, and those want opposite fixes.
        let report = lines.join("\n");
        assert!(
            finished >= 3,
            "self-play games should finish (got {finished}/4)\n{report}"
        );
        // The bar used to be two of four and every one of them was reached
        // by decking the opponent out over 180 turns: nobody tapped a land,
        // nobody cast a creature, and both seats declared an empty attack
        // ninety times each. A count of finished games could not see that,
        // so this asks the question that could.
        assert!(
            attacks > 0,
            "nobody attacked in any of the four games\n{report}"
        );
        println!("{report}");
    }

    /// A cracked fetchland has to *find*.
    ///
    /// `Tally::abilities` counts the activation and a zero `refused_cost`
    /// says the engine accepted it — neither of them says a land arrived.
    /// The step in between is the agent's own answer to the search
    /// (`Pending::ChooseCards`), and an answer of "nothing" would sacrifice
    /// the land for no land at all. On a report that reads exactly like
    /// mana screw, which is the one symptom the soak is least able to tell
    /// apart from bad luck.
    ///
    /// So: an empty opening hand and a library of one basic. No land drop
    /// is possible, and every land that reaches this seat's battlefield
    /// came out of the library.
    #[test]
    fn cracking_a_fetchland_puts_a_land_onto_the_battlefield() {
        use baylee_engine::zone::ZoneLocation;

        let mesa = baylee_cards::decks::by_name("Arid Mesa").expect("Arid Mesa is registered");
        let mountain = baylee_cards::decks::by_name("Mountain").expect("Mountain is registered");
        // The probe preset already puts the Mesa on the battlefield and
        // builds a print table for everything it uses; the deck it pads
        // with is what gets replaced, not the scaffolding.
        let mut preset = baylee_cards::decks::probe_preset(9, mesa).expect("the probe deck builds");
        let basic = *preset.seats[0]
            .deck
            .iter()
            .find(|e| e.card == mountain)
            .expect("a colourless card's probe deck pads with every basic");
        for seat in &mut preset.seats {
            seat.deck = vec![basic; 40];
            seat.starting_hand = Some(vec![]);
        }
        let mut engine = Engine::new(&preset, RegistryLookup).expect("preset builds");
        let agent = HeuristicAgent::new(AIProfile::default());
        let me = PlayerId::new(0);
        let mine = |engine: &Engine<RegistryLookup>, card| {
            let state = engine.state();
            state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .filter_map(|id| state.object(*id))
                .filter(|o| o.controller == me && o.card.is_some_and(|c| c.index == card))
                .count()
        };
        assert_eq!(mine(&engine, mesa), 1, "the Mesa starts on the battlefield");
        // What the battlefield held when the Mesa was cracked. The
        // sacrifice is a *cost*, so by the time the activation is applied
        // the land is already gone and only the find is still outstanding —
        // and nothing can be played while the ability is on the stack. So a
        // count that moves from here moved because the search found.
        let mut cracked: Option<usize> = None;
        for i in 0..200u64 {
            let pending = engine.pending().clone();
            if matches!(pending, Pending::GameOver(_)) {
                break;
            }
            let player = pending_player(&pending).expect("a decision point has a seat");
            let view = crate::view::player_view(
                engine.state(),
                player,
                priority_holder(&pending),
                i,
                Some(&pending),
                engine.automation(player).hold.suppresses(),
            );
            let action = agent.act(&view, &pending);
            let activating = player == me
                && cracked.is_none()
                && matches!(action, PlayerAction::ActivateAbility { .. });
            let before = mine(&engine, mountain);
            if engine.apply(player, action).is_err() && matches!(pending, Pending::Priority { .. })
            {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("passing is always legal");
            }
            if activating {
                cracked = Some(before);
            }
            if cracked.is_some_and(|before| mine(&engine, mountain) > before) {
                break;
            }
        }
        let before = cracked.expect("the agent never cracked the fetchland");
        assert_eq!(
            mine(&engine, mountain),
            before + 1,
            "the fetchland was cracked and the search found nothing"
        );
        assert_eq!(mine(&engine, mesa), 0, "a cracked fetchland is sacrificed");
    }
}
