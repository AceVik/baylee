//! `cards/artifacts/mv_1/juju_bubble.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn juju_bubble() -> CardIndex {
    card_index("9344ae2e-cb2e-400d-9157-e8158ce31813")
}

/// `Juju Bubble` ({1}, Artifact, `Coverage::Partial`): `{2}: You gain 1 life`,
/// and sacrifices itself when you cast a spell.
#[test]
fn juju_bubble_gains_life_and_sacrifices_when_you_cast_a_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[juju_bubble(), GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, juju_bubble());
    pass_until(&mut engine, stack_is_empty);
    let bubble = on_battlefield(&engine, p0, juju_bubble()).unwrap();

    activate(&mut engine, p0, juju_bubble(), 2);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 21);

    // Casting Grizzly Bears triggers Juju Bubble's sacrifice ability.
    cast_with_floating(&mut engine, p0, GRIZZLY_BEARS);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bubble).unwrap().zone, Zone::Graveyard);
}
