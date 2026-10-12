//! Completion-time activation checks differ from the earlier tap-cost event.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn artifact_possession_has_no_enchanted_artifact_after_tapless_self_sacrifice_cost() {
    let mut e = prepared(
        &[index::ARTIFACT_POSSESSION],
        &[],
        &[index::COMMANDER_S_SPHERE],
    );
    let sphere = source(&e, P1, index::COMMANDER_S_SPHERE);
    cast(&mut e, index::ARTIFACT_POSSESSION);
    targets(&mut e, vec![sphere], vec![]);
    pass_until(&mut e, stack_is_empty);
    let aura = source(&e, P0, index::ARTIFACT_POSSESSION);
    let before = e.state.players[1].life;
    let library = library_size(&e, P1);
    ability(&mut e, P1, index::COMMANDER_S_SPHERE, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(sphere).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state.object(aura).unwrap().zone, Zone::Graveyard);
    assert_eq!(library_size(&e, P1), library - 1);
    assert_eq!(
        e.state.players[1].life, before,
        "no enchanted artifact when activation completes"
    );
}

#[test]
fn artifact_possession_tap_cost_triggers_before_the_later_sacrifice_cost() {
    let mut e = prepared(&[index::ARTIFACT_POSSESSION], &[], &[index::MIND_STONE]);
    let stone = source(&e, P1, index::MIND_STONE);
    cast(&mut e, index::ARTIFACT_POSSESSION);
    targets(&mut e, vec![stone], vec![]);
    pass_until(&mut e, stack_is_empty);
    let before = e.state.players[1].life;
    let library = library_size(&e, P1);
    ability(&mut e, P1, index::MIND_STONE, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(library_size(&e, P1), library - 1);
    assert_eq!(
        e.state.players[1].life,
        before - 2,
        "tap event triggers once; no tapless activation trigger"
    );
}

#[test]
fn artifact_possession_observes_black_lotus_tap_before_sacrifice_and_mana_resolution() {
    let mut e = prepared(&[index::ARTIFACT_POSSESSION], &[], &[index::BLACK_LOTUS]);
    let lotus = source(&e, P1, index::BLACK_LOTUS);
    cast(&mut e, index::ARTIFACT_POSSESSION);
    targets(&mut e, vec![lotus], vec![]);
    pass_until(&mut e, stack_is_empty);
    let before = e.state.players[1].life;
    let mana = e.state.players[1].mana_pool.available(ManaColor::Blue);
    ability(&mut e, P1, index::BLACK_LOTUS, 0);
    if matches!(e.pending(), Pending::ChooseColor { .. }) {
        e.apply(P1, PlayerAction::ChooseColor(ManaColor::Blue))
            .unwrap();
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(lotus).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        e.state.players[1].mana_pool.available(ManaColor::Blue),
        mana + 3
    );
    assert_eq!(
        e.state.players[1].life,
        before - 2,
        "earlier tap event triggers, tap-symbol activation does not trigger again"
    );
}
