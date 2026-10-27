//! Banshee's Blade — `{2}` Artifact — Equipment (MRD).
//! Oracle: "Equipped creature gets +1/+1 for each charge counter on this Equipment. Whenever equipped creature deals combat damage, put a charge counter on this Equipment. Equip {2}"

use super::*;

fn banshee_s_blade() -> CardIndex {
    card_index("81a770e9-adea-4799-9fed-f10e97e7e901")
}

#[test]
fn banshee_s_blade_gains_charge_counter_on_combat_damage_and_pumps_equipped_creature() {
    let mut r = rig(banshee_s_blade(), llanowar_elves(), 2);
    r.equip(2);
    assert_eq!(pt(&r.engine, r.host), (1, 1));

    r.swing_unblocked();
    assert_eq!(
        r.engine
            .state()
            .object(r.equipment)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        1
    );
    assert_eq!(pt(&r.engine, r.host), (2, 2));
    assert_eq!(pt(&r.engine, r.bystander), (2, 1));
}
