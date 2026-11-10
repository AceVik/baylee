//! `cards/artifacts/equipment/mv_1/sylvok_lifestaff.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sylvok_lifestaff() -> CardIndex {
    card_index("e69b8388-1311-427f-9300-42badd5338ff")
}

/// Sylvok Lifestaff — "Equipped creature gets +1/+0. Whenever equipped
/// creature dies, you gain 3 life." and "Equip {1}".
#[test]
fn sylvok_lifestaff_gains_three_when_its_host_dies() {
    plain_pump(sylvok_lifestaff(), 1, 1, 0);

    let elf = llanowar_elves();
    let mut engine = rig(1, &[], &[sylvok_lifestaff(), elf, elf], &[]);
    let p0 = PlayerId::new(0);
    let elves = all_on_battlefield(&engine, p0, elf);
    let (host, other) = (elves[0], elves[1]);
    equip(&mut engine, sylvok_lifestaff(), 1, host);

    kill(&mut engine, other);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "a bystander's death is free"
    );
    kill(&mut engine, host);
    assert_eq!(
        engine.state().players[0].life,
        23,
        "the host's death gains 3"
    );
}
