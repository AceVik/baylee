//! Oracle checklist: own white and white multicolor creatures get +1/+1; nonwhite own creatures and an opposing white creature remain unchanged.
#[allow(clippy::wildcard_imports)]
use super::*;

fn honor_of_the_pure() -> CardIndex {
    card_index("eb4188bb-62df-4309-bde1-f66318fe2f05")
}

#[test]
fn honor_buffs_all_and_only_white_creatures_its_controller_controls() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                honor_of_the_pure(),
                benalish_knight(),
                lightning_angel(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[benalish_knight()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let own_white = on_battlefield(&e, p0, benalish_knight()).unwrap();
    let multicolor = on_battlefield(&e, p0, lightning_angel()).unwrap();
    let own_green = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let opposing_white = on_battlefield(&e, p1, benalish_knight()).unwrap();
    assert_eq!(pt(&e, own_white), (3, 3));
    assert_eq!(
        pt(&e, multicolor),
        (4, 5),
        "white remains one of its colors"
    );
    assert_eq!(pt(&e, own_green), (1, 1));
    assert_eq!(
        pt(&e, opposing_white),
        (2, 2),
        "the enchantment only sees creatures you control"
    );
}
