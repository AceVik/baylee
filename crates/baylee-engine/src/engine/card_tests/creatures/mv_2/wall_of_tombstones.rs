//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn wall_of_tombstones_freezes_upkeep_graveyard_count_without_setting_power() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::WALL_OF_TOMBSTONES,
                grizzly_bears(),
                gray_ogre(),
                index::MIND_STONE,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    let wall = on_battlefield(&e, p0, index::WALL_OF_TOMBSTONES).unwrap();
    let dead = [
        on_battlefield(&e, p0, grizzly_bears()).unwrap(),
        on_battlefield(&e, p0, gray_ogre()).unwrap(),
        on_battlefield(&e, p0, index::MIND_STONE).unwrap(),
    ];
    let state = e.dev_state_mut(p0).unwrap();
    let timestamp = state.next_timestamp();
    let filter = crate::effects::EffectFilter::object(state, wall);
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: p0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::PtSet,
        timestamp,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter,
        modifier: baylee_cards_dsl::Modifier::SetPower(7),
    });
    bury(&mut e, &dead);
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(wall)
        .unwrap()
        .counters
        .add(CounterKind::P1P1, 1);
    assert!(walk_to_own_main(&mut e, p0));
    assert_eq!(pt(&e, wall), (8, 4));
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            dead[0],
            ZoneLocation::Exile(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    assert_eq!(
        pt(&e, wall),
        (8, 4),
        "count is not continuously recalculated"
    );
    reach_their_main_phase(&mut e, PlayerId::new(1));
    assert!(walk_to_own_main(&mut e, p0));
    assert_eq!(pt(&e, wall), (8, 3));
}
