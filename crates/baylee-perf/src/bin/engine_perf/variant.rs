//! One monomorphization for every measured game, including forks and replay.

#[cfg(feature = "without-loop-detection")]
pub(super) type Detection = baylee_engine::engine::NoLoopDetection;
#[cfg(not(feature = "without-loop-detection"))]
pub(super) type Detection = baylee_engine::engine::DetectLoops;

/// The shipping Tower executor's own registry (its run pack), named through the
/// engine server so that the workload and what ships cannot drift apart.
#[cfg(feature = "tower")]
pub(super) type Registry = engine_server::variant::Registry;
#[cfg(not(feature = "tower"))]
pub(super) type Registry = baylee_gamehost::RegistryLookup;

pub(super) type Session = baylee_gamehost::Session<Detection, Registry>;
#[cfg(feature = "simulation")]
pub(super) type SimulationSession = baylee_gamehost::SimulationSession<Detection, Registry>;

/// Fits a house-deck preset to the variant's detector.
#[cfg(not(feature = "tower"))]
pub(super) fn prepare(preset: &mut baylee_core::preset::GamePreset) {
    if cfg!(feature = "without-loop-detection") {
        preset.house_rules.loop_policy = baylee_core::preset::LoopPolicy::Off;
    }
}

/// The Tower tables: two frozen starters each (zero-based, of the fifty), spread
/// over the pool so that every colour and many commanders sit down.
#[cfg(feature = "tower")]
pub(super) const TOWER_TABLES: [(usize, usize); 8] = [
    (0, 25),
    (6, 31),
    (12, 37),
    (18, 43),
    (24, 49),
    (30, 5),
    (36, 11),
    (42, 17),
];

/// Active relics per seat: several at once, as a run deep in the tower has
/// them, so that native definitions and their charges are played.
#[cfg(feature = "tower")]
const TOWER_ACTIVE: usize = 4;

/// The native Tower duels, built the way the shipping executor builds one: the
/// reviewed content's frozen starters (`baylee_tower_content::runic`) in a
/// package of the current generation, turned into the preset its run pack
/// admits. Seat zero is the human chair (`Open`), seat one the frozen Expert.
/// The seats carry relics from across the whole catalog at every rarity, and
/// every other table has its table charges.
#[cfg(feature = "tower")]
pub(super) fn tower_games(four: bool, seeds: &[u64]) -> anyhow::Result<Vec<super::Game>> {
    use anyhow::{Context as _, anyhow};
    use baylee_tower::v2::{
        self,
        catalog::{Definition, Rarity},
        package::{Build, Cohort, Origin, Package},
    };
    anyhow::ensure!(!four, "the native Tower workload seats two");
    let content = baylee_tower_content::runic().context("the Tower content")?;
    let identity = engine_server::variant::identity();
    let cohort = Cohort::for_generation(identity.clone(), &content.catalog, v2::GENERATOR)
        .map_err(|error| anyhow!("no cohort for the compiled executor: {error:?}"))?;
    let families = v2::generation::families(v2::GENERATOR)
        .map_err(|error| anyhow!("the current generation's relics: {error:?}"))?;
    let mut games = Vec::new();
    for (table, &(first, second)) in TOWER_TABLES.iter().enumerate() {
        for &seed in seeds {
            let build = |seat: usize, starter: usize| -> anyhow::Result<Build> {
                let starter = content
                    .starters
                    .get(starter)
                    .with_context(|| format!("the content has no starter {starter}"))?;
                let active = (0..TOWER_ACTIVE)
                    .map(|slot| {
                        // A prime stride over the catalog, so that the tables'
                        // seats meet families from all of it.
                        let at = ((table * 2 + seat) * TOWER_ACTIVE + slot) * 37;
                        Definition {
                            family: families[(at + usize::from(seed % 2 == 0)) % families.len()],
                            rarity: Rarity::ALL[(table + seat + slot) % Rarity::ALL.len()],
                        }
                    })
                    .collect();
                Ok(Build {
                    formation: starter.formation.clone(),
                    deck: starter.deck.clone(),
                    active,
                    potions: if table % 2 == 0 { [0; 2] } else { [1, 1] },
                })
            };
            let package = Package {
                schema: 2,
                cohort: cohort.clone(),
                stage: 1,
                // Attribution only; it changes nothing the duel plays.
                opponent: Origin::House { archetype: 1 },
                builds: [build(0, first)?, build(1, second)?],
                shuffle_seed: seed,
                policy_seed: seed ^ 0x7067_6f2d_746f_7765,
            };
            let preset = package
                .preset(&content.catalog, &identity, |card| {
                    baylee_cards::by_index(card).map(|_| baylee_cards::decks::reference_print(card))
                })
                .map_err(|error| {
                    anyhow!("the executor refuses table {first}-{second}: {error:?}")
                })?;
            games.push(super::Game {
                id: format!("tower-{first}-{second}-{seed}"),
                preset,
            });
        }
    }
    Ok(games)
}

pub(super) fn validate_args(args: &[String]) -> anyhow::Result<()> {
    if cfg!(feature = "tower") {
        anyhow::ensure!(
            args.first().is_some_and(|arg| arg == "frames")
                && !args.iter().any(|arg| arg == "--four"),
            "the native Tower workload supports only frames with two seats"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_engine::engine::LoopDetection;

    #[test]
    fn the_profile_workload_instantiates_its_selected_session() {
        let mut preset = super::super::games(false).unwrap().remove(0).preset;
        assert_eq!(
            preset.house_rules.loop_policy == baylee_core::preset::LoopPolicy::Off,
            cfg!(feature = "without-loop-detection")
        );
        assert!(Detection::supports(preset.house_rules.loop_policy));
        let mut session = Session::recorded_with_detection(&preset, "workload-test").unwrap();
        assert!(!session.take_record().is_empty());
        if cfg!(feature = "without-loop-detection") {
            assert_eq!(std::mem::size_of::<Detection>(), 0);
            preset.house_rules.loop_policy = baylee_core::preset::LoopPolicy::CompRulesDraw;
            assert!(Session::with_detection(&preset).is_none());
        }
    }

    /// The workload plays what the shipping Tower executor admits, and only
    /// that: the current generation's run pack, with relics on every seat; never
    /// the retained pack or the ordinary registry.
    #[cfg(feature = "tower")]
    #[test]
    fn native_workloads_admit_the_run_pack_and_reject_incompatible_modes() {
        use baylee_engine::extensions::NativeRulePack as _;
        let games = super::super::games(false).unwrap();
        assert_eq!(games.len(), TOWER_TABLES.len() * super::super::SEEDS.len());
        for game in &games {
            engine_server::tower::RunPack::validate(&game.preset).unwrap();
            assert!(engine_server::tower::Pack::validate(&game.preset).is_err());
            assert!(Session::with_detection(&game.preset).is_some());
            assert!(baylee_gamehost::Session::<Detection>::with_detection(&game.preset).is_none());
            let pack = game.preset.house_rules.rule_pack.as_ref().unwrap();
            assert_eq!(pack.seats.len(), 2);
            assert!(
                pack.seats
                    .iter()
                    .all(|seat| seat.definitions.len() == TOWER_ACTIVE)
            );
        }
        assert!(super::super::games(true).is_err());
        for args in [
            vec![],
            vec!["house"],
            vec!["record"],
            vec!["frames", "--four"],
        ] {
            assert!(
                validate_args(&args.into_iter().map(str::to_owned).collect::<Vec<_>>()).is_err()
            );
        }
        validate_args(&["frames".into(), "--verify".into()]).unwrap();
    }
}
