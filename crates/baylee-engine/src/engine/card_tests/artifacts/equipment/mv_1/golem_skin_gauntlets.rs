//! Golem-Skin Gauntlets — `{1}` Artifact — Equipment (MRD).
//! Oracle: "Equipped creature gets +1/+0 for each Equipment attached to it. Equip {2}"

use super::*;

fn golem_skin_gauntlets() -> CardIndex {
    card_index("2a956224-8bdb-40d8-8405-2aa29eb228e0")
}

#[test]
fn golem_skin_gauntlets_grants_one_power_per_attached_equipment() {
    let p0 = PlayerId::new(0);
    let gauntlets = golem_skin_gauntlets();
    let sledge = card_index("e1ee79de-e4c3-48f9-87a7-18f72e2f6116");
    let elf = llanowar_elves();
    let mut engine = rig(4, &[], &[gauntlets, sledge, elf, elf], &[elf]);
    let elves = all_on_battlefield(&engine, p0, elf);
    let (host, bystander) = (elves[0], elves[1]);

    equip(&mut engine, gauntlets, 2, host);
    assert_eq!(pt(&engine, host), (2, 1));
    assert_eq!(pt(&engine, bystander), (1, 1));

    equip(&mut engine, sledge, 2, host);
    assert_eq!(pt(&engine, host), (3, 1));
}
