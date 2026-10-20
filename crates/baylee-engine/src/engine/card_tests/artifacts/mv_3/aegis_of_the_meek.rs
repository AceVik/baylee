//! `cards/artifacts/mv_3/aegis_of_the_meek.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn aegis_of_the_meek() -> CardIndex {
    card_index("0cf78877-bf08-413c-8ecb-8c3265ac1c4a")
}

fn wall_of_wood() -> CardIndex {
    card_index("aa5e6894-8c97-46f8-a0fd-07c6db51e9a7")
}

/// Aegis of the Meek (924083a2-f08e-4821-b956-6f101980d0ab) — {3} Artifact:
/// - "{1}, {T}: Target 1/1 creature gets +1/+2 until end of turn."
///
/// Only creatures with current power 1 and toughness 1 are legal targets; a 2/2
/// or 0/3 is excluded. The target becomes 2/3 until end of turn and reverts in
/// cleanup.
#[test]
fn aegis_of_the_meek_targets_only_one_one_creatures_and_pumps_plus_one_plus_two_until_eot() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                aegis_of_the_meek(),
                forest(),
                llanowar_elves(),
                gray_ogre(),
                wall_of_wood(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let aegis = on_battlefield(&engine, P0, aegis_of_the_meek()).expect("Aegis seated");
    let my_elf = on_battlefield(&engine, P0, llanowar_elves()).expect("my 1/1 Elf");
    let their_elf = on_battlefield(&engine, P1, llanowar_elves()).expect("their 1/1 Elf");
    let ogre = on_battlefield(&engine, P0, gray_ogre()).expect("2/2 Gray Ogre");
    let wall = on_battlefield(&engine, P0, wall_of_wood()).expect("0/3 Wall of Wood");

    tap_all_mana_but(&mut engine, P0, Some(llanowar_elves()));
    activate(&mut engine, P0, aegis_of_the_meek(), 0);

    let menu = aim_at(&mut engine, P0, my_elf);
    assert!(
        menu.contains(&my_elf) && menu.contains(&their_elf),
        "1/1 creatures on either side are legal targets: {menu:?}"
    );
    assert!(
        !menu.contains(&ogre) && !menu.contains(&wall),
        "2/2 and 0/3 creatures are not 1/1 and must be excluded: {menu:?}"
    );
    assert!(is_tapped(&engine, aegis));

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, my_elf),
        (2, 3),
        "+1/+2 makes the 1/1 into a 2/3"
    );
    assert_eq!(pt(&engine, their_elf), (1, 1), "untouched 1/1 stays 1/1");

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        pt(&engine, my_elf),
        (1, 1),
        "+1/+2 wears off at end of turn"
    );
}
