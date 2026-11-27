//! The house ladder measured: every stage's fifty builds against each other
//! (balance within a stage), and every build against the stage before
//! (the ladder rises). The report holds counts and shapes, never a build.
use crate::{
    duel::{self, Bounds},
    jobs,
    starters::commander,
    tally::{Tally, round},
};
use baylee_tower::{
    cards::Catalog,
    v2::{
        MAX_STAGES,
        catalog::Rarity,
        package::{Build, Cohort},
    },
};
use baylee_tower_content::{Content, Ladder};

/// One planned duel: (stage, archetype) of each chair, and its seed.
type Match = ((u8, usize), (u8, usize), u64);

/// Measure `ladder`; `seeds` deals per ordered pair within a stage, and
/// `rise` opponents from the stage below per build.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one measurement, read top to bottom
pub fn measure(
    content: &Content,
    cohort: &Cohort,
    ladder: &Ladder,
    seeds: u32,
    rise: usize,
    base: u64,
    threads: usize,
    stages: &[u8],
    detail: bool,
) -> serde_json::Value {
    let catalog: &Catalog = &content.catalog;
    let count = content.starters.len();
    let wanted = |stage: u8| stages.is_empty() || stages.contains(&stage);
    let mut matches: Vec<Match> = Vec::new();
    for stage in (1..=MAX_STAGES).filter(|stage| wanted(*stage)) {
        for first in 0..count {
            for second in (0..count).filter(|second| *second != first) {
                for deal in 0..seeds {
                    let (low, high) = (first.min(second) as u64, first.max(second) as u64);
                    let seed = duel::mix(
                        base,
                        u64::from(stage) << 48 | low << 32 | high << 16 | u64::from(deal),
                    );
                    matches.push(((stage, first), (stage, second), seed));
                }
            }
        }
        if stage > 1 {
            for archetype in 0..count {
                for step in 0..rise {
                    let below = (archetype + 1 + step * 7) % count;
                    let seed = duel::mix(
                        base ^ 0x7269_7365,
                        u64::from(stage) << 48 | (archetype as u64) << 32 | step as u64,
                    );
                    // Both chairs: the higher stage first, then second.
                    matches.push(((stage, archetype), (stage - 1, below), seed));
                    matches.push(((stage - 1, below), (stage, archetype), seed));
                }
            }
        }
    }
    let build = |(stage, archetype): (u8, usize)| -> &Build {
        &ladder.stages[usize::from(stage) - 1][archetype]
    };
    let results = jobs::run(
        &matches,
        threads,
        |&(first, second, seed)| {
            duel::play(
                catalog,
                cohort,
                first.0.max(second.0),
                build(first),
                build(second),
                seed,
                Bounds::default(),
            )
        },
        |done, total| {
            if done % 500 == 0 || done == total {
                eprintln!("ladder: {done}/{total} duels");
            }
        },
    );
    let stages_n = usize::from(MAX_STAGES);
    let mut cells = vec![vec![Tally::default(); count]; stages_n];
    let mut rising = vec![Tally::default(); stages_n];
    for (&(first, second, _), played) in matches.iter().zip(&results) {
        match first.0.cmp(&second.0) {
            std::cmp::Ordering::Equal => {
                cells[usize::from(first.0) - 1][first.1].add(played, true);
                cells[usize::from(second.0) - 1][second.1].add(played, false);
            }
            std::cmp::Ordering::Greater => rising[usize::from(first.0) - 1].add(played, true),
            std::cmp::Ordering::Less => rising[usize::from(second.0) - 1].add(played, false),
        }
    }
    let mut report = Vec::new();
    for stage in (1..=MAX_STAGES).filter(|stage| wanted(*stage)) {
        let index = usize::from(stage) - 1;
        let mut scores: Vec<f64> = cells[index].iter().filter_map(Tally::score).collect();
        scores.sort_by(f64::total_cmp);
        let builds = &ladder.stages[index];
        let items: Vec<usize> = builds.iter().map(|build| build.active.len()).collect();
        let rarity = |wanted: Rarity| -> usize {
            builds
                .iter()
                .flat_map(|build| &build.active)
                .filter(|item| item.rarity == wanted)
                .count()
        };
        let potions: u32 = builds
            .iter()
            .map(|build| u32::from(build.potions.iter().sum::<u8>()))
            .sum();
        let changed = builds
            .iter()
            .zip(&content.starters)
            .filter(|(build, starter)| build.deck != starter.deck)
            .count();
        let outliers: Vec<_> = cells[index]
            .iter()
            .enumerate()
            .filter(|(_, tally)| tally.score().is_some_and(|s| !(0.30..=0.70).contains(&s)))
            .map(|(archetype, tally)| {
                serde_json::json!({
                    "starter": archetype + 1,
                    "commander": commander(&content.starters[archetype]),
                    "result": tally.report(),
                })
            })
            .collect();
        let unfinished: u32 = cells[index].iter().map(|tally| tally.unfinished).sum();
        if detail {
            // Private diagnosis only: names each build's equipment on stderr.
            for (archetype, tally) in cells[index].iter().enumerate() {
                let items: Vec<String> = builds[archetype]
                    .active
                    .iter()
                    .map(|item| format!("{:?}/{:?}", item.family, item.rarity))
                    .collect();
                eprintln!(
                    "detail stage {stage} starter {} {:?} {}: {}",
                    archetype + 1,
                    tally.score().map(round),
                    commander(&content.starters[archetype]),
                    items.join(", ")
                );
            }
        }
        report.push(serde_json::json!({
            "stage": stage,
            "within": {
                "min": scores.first().copied().map(round),
                "median": scores.get(scores.len() / 2).copied().map(round),
                "max": scores.last().copied().map(round),
                "outside_30_70": outliers,
                "unfinished": unfinished,
            },
            "against_stage_below": (stage > 1).then(|| rising[index].report()),
            "loadout": {
                "mean_items": round(
                    f64::from(u32::try_from(items.iter().sum::<usize>()).unwrap_or(u32::MAX))
                        / f64::from(u32::try_from(builds.len()).unwrap_or(u32::MAX)),
                ),
                "common": rarity(Rarity::Common),
                "uncommon": rarity(Rarity::Uncommon),
                "rare": rarity(Rarity::Rare),
                "mythic": rarity(Rarity::MythicRare),
                "potions": potions,
                "decks_changed": changed,
            },
        }));
    }
    serde_json::json!({
        "kind": "ladder",
        "seeds": seeds,
        "rise": rise,
        "duels": matches.len(),
        "stages": report,
    })
}
