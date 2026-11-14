use super::*;
use baylee_core::preset::LoopPolicy;
use baylee_engine::engine::NoLoopDetection;

#[test]
fn the_off_transport_driver_admits_only_off_games() {
    let mut runner = EngineRunner::<NoLoopDetection>::default();
    let legacy = duel(60);
    let refused = setup(&mut runner, &legacy);
    assert!(runner.session().is_none());
    assert!(matches!(
        refused.as_slice(),
        [Envelope {
            msg: Some(v1::envelope::Msg::GameEnded(_))
        }]
    ));
    let mut off = legacy;
    off.house_rules.loop_policy = LoopPolicy::Off;
    let mut runner = EngineRunner::<NoLoopDetection>::default();
    setup(&mut runner, &off);
    assert!(runner.session().is_some());
}

#[test]
fn the_enabled_transport_driver_does_not_silently_enable_an_off_game() {
    let mut preset = duel(60);
    preset.house_rules.loop_policy = LoopPolicy::Off;
    let mut runner = EngineRunner::new();
    let refused = setup(&mut runner, &preset);
    assert!(runner.session().is_none());
    assert!(matches!(
        refused.as_slice(),
        [Envelope {
            msg: Some(v1::envelope::Msg::GameEnded(_))
        }]
    ));
}
