//! The five *Legends* "bands with other legendary creatures" lands:
//! - Adventurers' Guildhouse (bf29b215-d4f5-4641-97ef-b65d4c463e72)
//! - Cathedral of Serra (4b9922b9-c5b5-467e-8a5b-1e45e862194d)
//! - Mountain Stronghold (af147bb6-aa46-4fc9-af96-de3740c19fd5)
//! - Seafarer's Quay (f71f01bc-caae-426b-a25a-bfbfeb9feb67)
//! - Unholy Citadel (d410027b-1c22-461e-939d-06a2c851352e)

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn legends_banding_lands_grant_bands_with_other_legendary_creatures_by_color() {
    let p0 = PlayerId::new(0);
    let adventurers_guildhouse = card_index("bf29b215-d4f5-4641-97ef-b65d4c463e72");
    let cathedral_of_serra = card_index("4b9922b9-c5b5-467e-8a5b-1e45e862194d");
    let mountain_stronghold = card_index("af147bb6-aa46-4fc9-af96-de3740c19fd5");
    let seafarers_quay = card_index("f71f01bc-caae-426b-a25a-bfbfeb9feb67");
    let unholy_citadel = card_index("d410027b-1c22-461e-939d-06a2c851352e");

    // Multicolored legendary creatures (covering Green, White, Blue, Black, Red):
    // Jasmine Boreal (GW), Tobias Andrion (WU), Barktooth Warbeard (BR)
    let jasmine = card_index("942407bf-862a-4a0b-b19d-b9f8ee833026"); // GW legendary
    let ramirez = card_index("55dfe722-7487-4d25-b90c-e6ad2c82a671"); // WU legendary (Tobias Andrion)
    let barktooth = card_index("4a221518-c850-4e8a-a697-765c0a0e76e7"); // BR legendary
    let grizzly_bears = card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0"); // non-legendary G

    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                adventurers_guildhouse,
                cathedral_of_serra,
                mountain_stronghold,
                seafarers_quay,
                unholy_citadel,
                jasmine,
                ramirez,
                barktooth,
                grizzly_bears,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let jasmine_id = on_battlefield(&engine, p0, jasmine).unwrap();
    let ramirez_id = on_battlefield(&engine, p0, ramirez).unwrap();
    let barktooth_id = on_battlefield(&engine, p0, barktooth).unwrap();
    let bears_id = on_battlefield(&engine, p0, grizzly_bears).unwrap();

    // Jasmine (Green/White legendary) can band with Ramirez (legendary) and Barktooth (legendary),
    // but NOT with Grizzly Bears (non-legendary).
    assert!(crate::combat::can_band_with_other(
        engine.state(),
        jasmine_id,
        ramirez_id
    ));
    assert!(crate::combat::can_band_with_other(
        engine.state(),
        ramirez_id,
        barktooth_id
    ));
    assert!(crate::combat::can_band_with_other(
        engine.state(),
        barktooth_id,
        jasmine_id
    ));
    assert!(!crate::combat::can_band_with_other(
        engine.state(),
        jasmine_id,
        bears_id
    ));
    assert!(!crate::combat::can_band_with_other(
        engine.state(),
        bears_id,
        jasmine_id
    ));
}
