//! One monomorphization for every measured game, including forks and replay.
use baylee_core::preset::GamePreset;

#[cfg(feature = "without-loop-detection")]
pub(super) type Detection = baylee_engine::engine::NoLoopDetection;
#[cfg(not(feature = "without-loop-detection"))]
pub(super) type Detection = baylee_engine::engine::DetectLoops;

#[cfg(feature = "tower")]
pub(super) type Registry = baylee_engine::extensions::NativeLookup<
    baylee_gamehost::RegistryLookup,
    baylee_engine_server::tower::Pack,
>;
#[cfg(not(feature = "tower"))]
pub(super) type Registry = baylee_gamehost::RegistryLookup;

pub(super) type Session = baylee_gamehost::Session<Detection, Registry>;
#[cfg(feature = "simulation")]
pub(super) type SimulationSession = baylee_gamehost::SimulationSession<Detection, Registry>;

// Keep the same caller contract in every artifact; native admission can fail.
#[cfg_attr(not(feature = "tower"), allow(clippy::unnecessary_wraps))]
pub(super) fn prepare(preset: &mut GamePreset) -> anyhow::Result<()> {
    if cfg!(feature = "without-loop-detection") {
        preset.house_rules.loop_policy = baylee_core::preset::LoopPolicy::Off;
    }
    #[cfg(feature = "tower")]
    prepare_tower(preset)?;
    Ok(())
}

// A human and the frozen Expert policy, with all four native rarities exercised
// across the fixed seeds/decks. Policy randomness is independent of the shuffle.
#[cfg(feature = "tower")]
fn prepare_tower(preset: &mut GamePreset) -> anyhow::Result<()> {
    use baylee_core::preset::{AIProfile, FormatId, SeatController};
    use baylee_tower::v2::{
        catalog::{Definition, Family, Rarity},
        loadout,
    };
    anyhow::ensure!(preset.seats.len() == 2, "Tower workload requires two seats");
    let first = usize::try_from(preset.seed % 4)?;
    let resources = [0_u8, 1].map(|seat| {
        loadout::resources(
            &[
                Definition {
                    family: Family::Grove,
                    rarity: Rarity::ALL[(first + usize::from(seat) * 2) % 4],
                },
                Definition {
                    family: Family::Heartwood,
                    rarity: Rarity::Common,
                },
            ],
            [1, 1],
            seat,
        )
    });
    let [human, echo] = resources;
    let resources = [human?, echo?];
    preset.format = FormatId::Custom;
    preset.house_rules = loadout::rules(&resources, 0x7067_6f2d_746f_7765)?;
    preset.seats[0].controller = SeatController::Open;
    preset.seats[1].controller = SeatController::Ai(AIProfile::TOWER_EXPERT);
    for (seat, resource) in preset.seats.iter_mut().zip(resources) {
        seat.starting_life = Some(resource.life);
        seat.sideboard.clear();
        seat.commanders.clear();
    }
    loadout::validate(preset)?;
    Ok(())
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

    #[cfg(feature = "tower")]
    #[test]
    fn native_workloads_admit_the_pack_and_reject_incompatible_modes() {
        for game in super::super::games(false).unwrap() {
            baylee_tower::v2::loadout::validate(&game.preset).unwrap();
            assert!(Session::with_detection(&game.preset).is_some());
            assert!(baylee_gamehost::Session::<Detection>::with_detection(&game.preset).is_none());
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
