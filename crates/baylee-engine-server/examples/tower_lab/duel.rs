//! One native duel through the executor's own package → preset → session path.
//!
//! Seat 0 is the human chair of a real Tower duel (`SeatController::Open`);
//! here it is answered by the frozen Expert heuristics through its own
//! authorized view. Seat 1 is the house chair the session plays itself, as
//! the hosting engine does. Every pairing is measured in both chair
//! assignments, so a difference between the two policies cannot pass for a
//! difference between builds.
use baylee_ai::HeuristicAgent;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::Pending;
use baylee_engine_server::variant::{self, HostedSession};
use baylee_tower::{
    cards::Catalog,
    v2::{
        self, generation,
        package::{Build, Cohort, Origin, Package},
    },
};
use std::time::{Duration, Instant};

/// What one duel ended as. `Unfinished` is never counted as a result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Result {
    /// Seat 0's build won through the rules.
    First,
    /// Seat 1's build won through the rules.
    Second,
    /// A rules draw.
    Draw,
    /// The step, turn or time bound ended it first.
    Unfinished,
}

/// Bounds on one duel; reaching one is a measurement gap, not an outcome.
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    /// Agent answers plus house pump slices.
    pub steps: u32,
    /// Rules turns.
    pub turns: u32,
    /// Wall time.
    pub wall: Duration,
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            steps: 30_000,
            turns: 60,
            wall: Duration::from_secs(300),
        }
    }
}

/// One finished duel.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Played {
    /// Rules result, or a bound.
    pub result: Result,
    /// Rules turns played.
    pub turns: u32,
    /// Wall time in milliseconds.
    pub millis: u64,
}

/// The executor's cohort for in-process duels: this binary's own identity.
/// # Errors
/// A catalog the compiled generation does not admit.
pub fn cohort(catalog: &Catalog) -> anyhow::Result<Cohort> {
    Cohort::for_generation(variant::identity(), catalog, v2::GENERATOR)
        .map_err(|error| anyhow::anyhow!("no cohort for the compiled executor: {error:?}"))
}

/// Play `first` (human chair) against `second` (house chair) at `stage`.
/// The Tower Expert has no foresight tier (`AIProfile::TOWER_EXPERT`), so the
/// house chair answers by its heuristics here exactly as at a hosting engine.
/// # Errors
/// A build the executor refuses: never a duel outcome.
pub fn play(
    catalog: &Catalog,
    cohort: &Cohort,
    stage: u8,
    first: &Build,
    second: &Build,
    seed: u64,
    bounds: Bounds,
) -> anyhow::Result<Played> {
    let identity = variant::identity();
    let package = Package {
        schema: 2,
        cohort: cohort.clone(),
        stage,
        // Attribution only; it changes nothing the duel plays.
        opponent: Origin::House { archetype: 1 },
        builds: [first.clone(), second.clone()],
        shuffle_seed: mix(seed, 1),
        policy_seed: mix(seed, 2),
    };
    let preset = package
        .preset(catalog, &identity, |card| {
            baylee_cards::by_index(card).map(|_| baylee_cards::decks::reference_print(card))
        })
        .map_err(|error| anyhow::anyhow!("the executor refuses the package: {error:?}"))?;
    let mut game = HostedSession::recorded_for_execution(&preset, &identity)
        .ok_or_else(|| anyhow::anyhow!("the compiled executor does not admit this preset"))?;
    let profile = generation::contract(cohort.generator)
        .map_err(|error| anyhow::anyhow!("unknown generation: {error:?}"))?
        .expert();
    let human = PlayerId::new(0);
    let agent = HeuristicAgent::new(profile).with_seed(mix(seed, 3));
    let started = Instant::now();
    let deadline = started + bounds.wall;
    let mut steps = 0_u32;
    while !matches!(game.pending(), Pending::GameOver(_))
        && steps < bounds.steps
        && game.state().turn.number <= bounds.turns
        && Instant::now() < deadline
    {
        if game.needs_pump() {
            game.pump_slice(1);
        } else {
            let Some((pending, view)) = game.view_for_with_context(human) else {
                anyhow::bail!("the session waits on neither chair");
            };
            let action = agent.act(&view, &pending);
            game.act_slice(human, action, 0)
                .map_err(|error| anyhow::anyhow!("the engine refused an answer: {error:?}"))?;
        }
        steps += 1;
    }
    let result = if let Pending::GameOver(result) = game.pending() {
        let winners = game.winning_seats(*result);
        if winners.is_empty() {
            Result::Draw
        } else if winners.contains(&human) {
            Result::First
        } else {
            Result::Second
        }
    } else {
        Result::Unfinished
    };
    Ok(Played {
        result,
        turns: game.state().turn.number,
        millis: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

/// An independent stream per purpose from one job seed (`SplitMix64` finaliser).
#[must_use]
pub const fn mix(seed: u64, stream: u64) -> u64 {
    let mut z = seed ^ stream.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}
