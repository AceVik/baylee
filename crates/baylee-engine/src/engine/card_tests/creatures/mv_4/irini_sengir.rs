//! Irini Sengir — `{2}{B}{B}` 2/2 Legendary Creature — Vampire Dwarf (Homelands)
//!
//! Green enchantment spells and white enchantment spells cost `{2}` more to
//! cast.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn irini_sengir() -> CardIndex {
    card_index("2727a7ec-1cf7-4cd7-b5d4-955db940ddd9")
}

#[test]
fn irini_sengir_taxes_green_and_white_enchantment_spells_by_two_and_not_black_enchantments() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[index::HOLY_STRENGTH, index::UNHOLY_STRENGTH])
        .battlefield(
            0,
            &[
                irini_sengir(),
                index::GRIZZLY_BEARS,
                plains(),
                plains(),
                swamp(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let holy = in_hand(&engine, p0, index::HOLY_STRENGTH).unwrap();
    let unholy = in_hand(&engine, p0, index::UNHOLY_STRENGTH).unwrap();
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
