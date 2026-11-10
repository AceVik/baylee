//! Empyrial Plate — `{2}` Artifact — Equipment (MRD).
//! Oracle: "Equipped creature gets +1/+1 for each card in your hand. Equip {2}"

use super::*;

fn empyrial_plate() -> CardIndex {
    card_index("e95e100e-4886-4bde-825c-017e878ac825")
}

#[test]
fn empyrial_plate_pumps_equipped_creature_by_your_hand_size() {
    let p0 = PlayerId::new(0);
    let mut r = rig_vs(
        empyrial_plate(),
        llanowar_elves(),
        2,
        &[forest(), forest(), forest()],
        &[llanowar_elves()],
        &[],
    );
    assert_eq!(r.hand_size(), 3);
    r.equip(2);
    assert_eq!(pt(&r.engine, r.host), (4, 4));
    assert_eq!(pt(&r.engine, r.bystander), (2, 1));

    play_land(&mut r.engine, p0, forest());
    assert_eq!(r.hand_size(), 2);
    assert_eq!(pt(&r.engine, r.host), (3, 3));
}
