//! Irini Sengir — `{2}{B}{B}` 2/2 Legendary Creature — Vampire Dwarf (Homelands)
//!
//! Green enchantment spells and white enchantment spells cost `{2}` more to
//! cast.

#[allow(clippy::wildcard_imports)]
use super::*;

fn irini_sengir() -> CardIndex {
    card_index("2727a7ec-1cf7-4cd7-b5d4-955db940ddd9")
}

#[test]
fn irini_sengir_taxes_green_and_white_enchantment_spells_by_two_and_not_black_enchantments() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(
            0,
            &[
                card_index("9357de36-f8be-4f49-b2c8-9fe9eaf82b07"),
                card_index("090d88a9-7f2d-4bd1-a30a-7c48d05068be"),
            ],
        )
        .battlefield(
            0,
            &[
                irini_sengir(),
                card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0"),
                plains(),
                plains(),
                swamp(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let holy = in_hand(
        &engine,
        p0,
        card_index("9357de36-f8be-4f49-b2c8-9fe9eaf82b07"),
    )
    .unwrap();
    let unholy = in_hand(
        &engine,
        p0,
        card_index("090d88a9-7f2d-4bd1-a30a-7c48d05068be"),
    )
    .unwrap();
    let plains_ids = all_on_battlefield(&engine, p0, plains());
    let swp = on_battlefield(&engine, p0, swamp()).unwrap();

    // Tap 1 Plains + 1 Swamp ({W}{B}): Unholy Strength ({B}) is castable, but Holy Strength ({W} + {2} = {2}{W}) is not!
    tap_mana_where(&mut engine, p0, |id| id == plains_ids[0] || id == swp);
    assert!(
        !priority_offer(&engine).castable.contains(&holy),
        "white enchantment Holy Strength costs {{2}}{{W}} under Irini Sengir"
    );
    assert!(
        priority_offer(&engine).castable.contains(&unholy),
        "black enchantment Unholy Strength still costs {{B}}"
    );

    // Tap the second Plains ({W}{W}{B}, 3 mana total): now Holy Strength ({2}{W}) is castable!
    tap_mana_where(&mut engine, p0, |id| id == plains_ids[1]);
    assert!(
        priority_offer(&engine).castable.contains(&holy),
        "Holy Strength is castable once {{2}}{{W}} is available"
    );
}

/// "Spells" is every player's: Irini on the opponent's side taxes the green
/// and the white enchantment its owner's opponent casts, and not a black one.
#[test]
fn irini_sengir_taxes_the_opponents_green_and_white_enchantments_too() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(
            0,
            &[
                card_index("9357de36-f8be-4f49-b2c8-9fe9eaf82b07"),
                rancor(),
                card_index("090d88a9-7f2d-4bd1-a30a-7c48d05068be"),
            ],
        )
        .battlefield(
            0,
            &[
                card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0"),
                plains(),
                forest(),
                swamp(),
            ],
        )
        .battlefield(1, &[irini_sengir()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let holy = in_hand(
        &engine,
        p0,
        card_index("9357de36-f8be-4f49-b2c8-9fe9eaf82b07"),
    )
    .unwrap();
    let rancor_card = in_hand(&engine, p0, rancor()).unwrap();
    let unholy = in_hand(
        &engine,
        p0,
        card_index("090d88a9-7f2d-4bd1-a30a-7c48d05068be"),
    )
    .unwrap();
    let plains_id = on_battlefield(&engine, p0, plains()).unwrap();
    let forest_id = on_battlefield(&engine, p0, forest()).unwrap();
    let swamp_id = on_battlefield(&engine, p0, swamp()).unwrap();

    // One {B}: the black Aura costs {B}, the white and the green one cost 3.
    tap_mana_where(&mut engine, p0, |id| id == swamp_id);
    let offer = priority_offer(&engine);
    assert!(
        offer.castable.contains(&unholy),
        "{{B}} pays Unholy Strength"
    );
    assert!(
        !offer.castable.contains(&holy),
        "Holy Strength costs {{2}}{{W}} under the opponent's Irini"
    );
    assert!(
        !offer.castable.contains(&rancor_card),
        "Rancor costs {{2}}{{G}} under the opponent's Irini"
    );

    // {B}{W}{G}: three mana, and both taxed Auras are castable.
    tap_mana_where(&mut engine, p0, |id| id == plains_id || id == forest_id);
    let offer = priority_offer(&engine);
    assert!(offer.castable.contains(&holy), "{{2}}{{W}} is floating");
    assert!(
        offer.castable.contains(&rancor_card),
        "{{2}}{{G}} is floating"
    );
}
