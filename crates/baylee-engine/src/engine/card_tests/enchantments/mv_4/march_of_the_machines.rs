//! March of the Machines animates artifacts and stops creature Equipment equipping.

use super::*;

fn march_of_the_machines() -> CardIndex {
    card_index("51092634-308e-4779-aa51-182715dbc734")
}

fn mask_of_memory() -> CardIndex {
    card_index("d6b2c998-a226-426c-a40d-6e6007041bfe")
}

#[test]
fn march_animates_by_mana_value_and_locks_equipment_abilities() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[march_of_the_machines(), quiet_artifact(), mask_of_memory()],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let ring = on_battlefield(&e, p0, quiet_artifact()).unwrap();
    let equipment = on_battlefield(&e, p0, mask_of_memory()).unwrap();
    assert_eq!(pt(&e, ring), (1, 1));
    assert_eq!(pt(&e, equipment), (2, 2));
    assert!(
        e.state()
            .object(ring)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("expected priority")
    };
    assert!(!legal.abilities.contains(&(equipment, 0)));
}
