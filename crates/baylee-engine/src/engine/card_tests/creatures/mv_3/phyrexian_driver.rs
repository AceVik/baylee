//! `cards/creatures/mv_3/phyrexian_driver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn phyrexian_driver_etb_pumps_other_mercenaries_only() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[card_index("e8d12d8e-afd2-438b-b121-5911fca479f9")])
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                index::CATERAN_PERSUADER,
                index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let persuader = on_battlefield(&engine, p0, index::CATERAN_PERSUADER).expect("Mercenary");
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).expect("Grizzly Bears");
    assert_eq!(pt(&engine, persuader), (2, 1));
    assert_eq!(pt(&engine, bears), (2, 2));

    cast_from_hand(
        &mut engine,
        p0,
        card_index("e8d12d8e-afd2-438b-b121-5911fca479f9"),
    );
    pass_until(&mut engine, stack_is_empty);

    let driver = on_battlefield(
        &engine,
        p0,
        card_index("e8d12d8e-afd2-438b-b121-5911fca479f9"),
    )
    .expect("Phyrexian Driver");
    assert_eq!(pt(&engine, persuader), (3, 2), "other Mercenary gets +1/+1");
    assert_eq!(pt(&engine, bears), (2, 2), "non-Mercenary unchanged");
    assert_eq!(
        pt(&engine, driver),
        (1, 1),
        "Phyrexian Driver itself unchanged"
    );
}
