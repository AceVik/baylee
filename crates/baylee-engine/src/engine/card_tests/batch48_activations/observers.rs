#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch48_haunting_wind_observes_tapping_and_tapless_activation_once_each() {
    let mut e = prepared(&[], &[index::HAUNTING_WIND, index::BASALT_MONOLITH], &[]);
    let before = e.state.players[0].life;
    ability(&mut e, P0, index::BASALT_MONOLITH, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.players[0].life,
        before - 1,
        "tap cost does not double trigger"
    );
    ability(&mut e, P0, index::BASALT_MONOLITH, 2);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, before - 2);
    assert!(!is_tapped(&e, source(&e, P0, index::BASALT_MONOLITH)));
}

#[test]
fn batch48_powerleech_only_observes_opponent_artifacts() {
    let mut e = prepared(
        &[],
        &[index::POWERLEECH, index::BASALT_MONOLITH],
        &[index::BASALT_MONOLITH],
    );
    let before = e.state.players[0].life;
    ability(&mut e, P0, index::BASALT_MONOLITH, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, before);
    ability(&mut e, P1, index::BASALT_MONOLITH, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, before + 1);
    ability(&mut e, P1, index::BASALT_MONOLITH, 2);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, before + 2);
}

#[test]
fn batch48_artifact_possession_binds_both_trigger_halves_to_enchanted_artifact() {
    let mut e = prepared(
        &[index::ARTIFACT_POSSESSION],
        &[],
        &[index::BASALT_MONOLITH, index::SOL_RING],
    );
    let basalt = source(&e, P1, index::BASALT_MONOLITH);
    cast(&mut e, index::ARTIFACT_POSSESSION);
    targets(&mut e, vec![basalt], vec![]);
    pass_until(&mut e, stack_is_empty);
    let before = e.state.players[1].life;
    ability(&mut e, P1, index::SOL_RING, 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, before);
    ability(&mut e, P1, index::BASALT_MONOLITH, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, before - 2);
    ability(&mut e, P1, index::BASALT_MONOLITH, 2);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, before - 4);
}
