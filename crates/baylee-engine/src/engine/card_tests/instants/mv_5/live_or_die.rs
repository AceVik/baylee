//! `cards/instants/mv_5/live_or_die.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "dc367b23-d1f4-4c5c-b300-a44a726e3f74"
fn live_or_die() -> CardIndex {
    card_index("dc367b23-d1f4-4c5c-b300-a44a726e3f74")
}

/// Live or Die: "Choose one — Return target creature card from your graveyard
/// to the battlefield. / Destroy target creature."
///
/// The first mode: the menu holds the creature card in the caster's own
/// graveyard and not the one in the opponent's, and the card named returns.
#[test]
fn live_or_die_returns_a_creature_card_from_your_graveyard() {
    let mut engine = table(
        925,
        &[live_or_die()],
        &[swamp(); 5],
        &[gray_ogre(), grizzly_bears()],
        &[llanowar_elves()],
    );
    let ogre = on_battlefield(&engine, me(), gray_ogre()).expect("my Ogre");
    let elf = on_battlefield(&engine, them(), llanowar_elves()).expect("their Elf");
    bury(&mut engine, &[ogre, elf]);
    let mine = in_graveyard(&engine, me(), gray_ogre()).expect("my Ogre is buried");
    let theirs = in_graveyard(&engine, them(), llanowar_elves()).expect("their Elf is buried");

    tap_all_mana(&mut engine, me());
    cast_in_mode(&mut engine, live_or_die(), 0);
    let menu = aim_at(&mut engine, me(), mine);
    assert!(menu.contains(&mine), "my graveyard: {menu:?}");
    assert!(!menu.contains(&theirs), "not the opponent's: {menu:?}");
    settle(&mut engine);
    assert!(on_battlefield(&engine, me(), gray_ogre()).is_some(), "back on the battlefield");
    assert!(in_graveyard(&engine, them(), llanowar_elves()).is_some(), "theirs stays");
}

/// The second mode: a creature on the battlefield is destroyed.
#[test]
fn live_or_die_destroys_a_creature_in_its_other_mode() {
    let mut engine = table(
        926,
        &[live_or_die()],
        &[swamp(); 5],
        &[gray_ogre()],
        &[llanowar_elves()],
    );
    let ogre = on_battlefield(&engine, me(), gray_ogre()).expect("my Ogre");
    bury(&mut engine, &[ogre]);
    let elf = on_battlefield(&engine, them(), llanowar_elves()).expect("their Elf");
    tap_all_mana(&mut engine, me());
    cast_in_mode(&mut engine, live_or_die(), 1);
    aim_at(&mut engine, me(), elf);
    settle(&mut engine);
    assert!(in_graveyard(&engine, them(), llanowar_elves()).is_some(), "destroyed");
}
