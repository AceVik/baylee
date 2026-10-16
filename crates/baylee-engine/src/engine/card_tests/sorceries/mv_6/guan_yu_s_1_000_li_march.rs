//! `cards/sorceries/mv_6/guan_yu_s_1_000_li_march.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "a1b87d5d-9493-4001-8c7e-553f40de61ea"
fn guan_yu_s_1_000_li_march() -> CardIndex {
    card_index("a1b87d5d-9493-4001-8c7e-553f40de61ea")
}

/// Guan Yu's 1,000-Li March: "Destroy all tapped creatures."
///
/// The Elf is tapped by paying for the spell with its own mana, and the Ogre
/// beside it and the Ogre across the table are untapped. Only the tapped
/// creature dies.
#[test]
fn guan_yu_s_march_destroys_only_tapped_creatures() {
    let mut engine = table(
        961,
        &[guan_yu_s_1_000_li_march()],
        &[plains(); 6],
        &[llanowar_elves(), gray_ogre()],
        &[llanowar_elves()],
    );
    let elf = on_battlefield(&engine, me(), llanowar_elves()).expect("my Elf");
    tap_all_mana(&mut engine, me());
    assert!(is_tapped(&engine, elf), "the Elf was tapped for mana");
    cast_with_floating(&mut engine, me(), guan_yu_s_1_000_li_march());
    settle(&mut engine);

    assert!(in_graveyard(&engine, me(), llanowar_elves()).is_some(), "the tapped creature dies");
    assert!(on_battlefield(&engine, me(), gray_ogre()).is_some(), "an untapped one stays");
    assert!(
        on_battlefield(&engine, them(), llanowar_elves()).is_some(),
        "so does an untapped creature of theirs"
    );
}
