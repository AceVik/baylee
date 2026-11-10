//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn setup() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::HALFDANE])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    let source = on_battlefield(&e, p0, index::HALFDANE).unwrap();
    let target = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let options = aim_at(&mut e, p0, target);
    assert!(!options.contains(&source), "Halfdane cannot target itself");
    (e, source, target)
}

#[test]
fn halfdane_reads_signed_current_pt_then_keeps_it_through_end_of_next_upkeep() {
    let p0 = PlayerId::new(0);
    let (mut e, source, target) = setup();
    {
        let chars = e
            .dev_state_mut(p0)
            .unwrap()
            .object_mut(target)
            .unwrap()
            .base_mut();
        chars.power = Some(-2);
        chars.toughness = Some(5);
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        pt(&e, source),
        (-2, 5),
        "reads the target when the ability resolves"
    );
    reach_main_phase(&mut e, p0);
    assert_eq!(
        pt(&e, source),
        (-2, 5),
        "not expired at the end of the current upkeep"
    );
    {
        let chars = e
            .dev_state_mut(p0)
            .unwrap()
            .object_mut(target)
            .unwrap()
            .base_mut();
        chars.power = Some(7);
        chars.toughness = Some(8);
    }
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut e, p0, target);
    assert_eq!(
        pt(&e, source),
        (-2, 5),
        "old value remains during the next upkeep"
    );
    bury(&mut e, &[target]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        pt(&e, source),
        (-2, 5),
        "new ability fizzles but old duration has not expired"
    );
    reach_main_phase(&mut e, p0);
    assert_eq!(pt(&e, source), (3, 3), "old base P/T ends with that upkeep");
}

#[test]
fn halfdane_does_not_change_a_returned_source_incarnation() {
    let p0 = PlayerId::new(0);
    let (mut e, source, _) = setup();
    let state = e.dev_state_mut(p0).unwrap();
    state
        .move_object(
            source,
            ZoneLocation::Exile(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            source,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, source), (3, 3));
}
