//! One monomorphization for every measured game, including forks and replay.
use baylee_core::preset::GamePreset;

#[cfg(feature = "without-loop-detection")]
pub(super) type Detection = baylee_engine::engine::NoLoopDetection;
#[cfg(not(feature = "without-loop-detection"))]
pub(super) type Detection = baylee_engine::engine::DetectLoops;

pub(super) type Session = baylee_gamehost::Session<Detection>;
#[cfg(feature = "simulation")]
pub(super) type SimulationSession = baylee_gamehost::SimulationSession<Detection>;

pub(super) fn prepare(preset: &mut GamePreset) {
    if cfg!(feature = "without-loop-detection") {
        preset.house_rules.loop_policy = baylee_core::preset::LoopPolicy::Off;
    }
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
}
