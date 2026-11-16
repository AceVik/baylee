//! Visual-only synthetic settlement records. Never linked into production and
//! never evidence that a real native replay passed independent verification.
use baylee_tower::{
    cards::Catalog,
    v2::{
        package::{Build, Cohort, Opponent, Origin, Package, Seeds},
        run::{Binding, Command, Outcome, Run, Settlement, WorkerCommand},
    },
};

pub(super) fn prepare(
    mut run: Run,
    publication: &baylee_tower_store::Publication,
    scenario: &str,
) -> Run {
    let (rounds, outcome) = match scenario {
        "loot-win" => (1, Outcome::Victory),
        "loot-loss" => (1, Outcome::Defeat),
        "victory" => (8, Outcome::Victory),
        "defeat" => (5, Outcome::Defeat),
        _ => panic!("unknown synthetic visual scenario"),
    };
    let catalog = &publication.catalog;
    let opponent = Run::begin_for(
        catalog,
        publication.starters[1].clone(),
        &[83; 32],
        publication.cohort.generator,
    )
    .unwrap();
    let build = Build::freeze(&opponent, catalog).unwrap();
    for round in 0..rounds {
        run = settle(run, catalog, &publication.cohort, &build, outcome);
        if round + 1 != rounds {
            let reward = run.reward.as_ref().unwrap();
            let chosen = reward
                .cards
                .iter()
                .take(usize::from(reward.take))
                .copied()
                .collect();
            run = run
                .command(
                    run.revision,
                    &Command::Claim { cards: chosen },
                    catalog,
                    &[83; 32],
                )
                .unwrap();
        }
    }
    // Keep the last reward pending: the real client claims through the authority
    // and reaches the terminal screen via the ordinary transaction/socket path.
    run.validate(catalog).unwrap();
    run
}

fn settle(run: Run, catalog: &Catalog, cohort: &Cohort, build: &Build, outcome: Outcome) -> Run {
    let reserved = Package::reserve(
        &run,
        run.revision,
        catalog,
        cohort,
        Opponent {
            stage: run.shop.stage,
            cohort: cohort.clone(),
            origin: Origin::House { archetype: 1 },
            build: build.clone(),
        },
        Seeds {
            shuffle: u64::from(run.shop.stage),
            policy: 83,
        },
        4096,
    )
    .unwrap();
    let mut run = reserved.run;
    run = run
        .worker(
            run.revision,
            10,
            WorkerCommand::Claim { duration: 1000 },
            catalog,
        )
        .unwrap();
    let execution = run.execution.as_ref().unwrap();
    run = run
        .worker(
            run.revision,
            11,
            WorkerCommand::Append {
                fence: execution.fence,
                expected: execution.prefix,
                chunk: b"synthetic visual record; not a native replay",
            },
            catalog,
        )
        .unwrap();
    let execution = run.execution.as_ref().unwrap();
    run = run
        .worker(
            run.revision,
            12,
            WorkerCommand::Seal {
                fence: execution.fence,
                expected: execution.prefix,
            },
            catalog,
        )
        .unwrap();
    let execution = run.execution.as_ref().unwrap();
    let result = Settlement {
        binding: Binding {
            package: execution.package,
            fence: execution.fence,
            prefix: execution.prefix,
        },
        outcome,
        turns: 24,
        used: [0; 2],
    };
    run.settled_against(run.revision, result, catalog, &[83; 32], &reserved.package)
        .unwrap()
}
