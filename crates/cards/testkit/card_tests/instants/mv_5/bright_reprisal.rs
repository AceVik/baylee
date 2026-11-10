//! `cards/instants/mv_5/bright_reprisal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "1de2c216-0921-45eb-9613-a68d18fbee77"
fn bright_reprisal() -> CardIndex {
    card_index("1de2c216-0921-45eb-9613-a68d18fbee77")
}

/// Bright Reprisal: "Destroy target attacking creature. Draw a card."
///
/// The only place "attacking" can be found is a combat phase with the creature
/// declared. An Ogre attacks, an Elf stays home, and the menu holds the first
/// and not the second. The draw is read as a move: the spell left the hand and
/// a card from the library replaced it.
#[test]
fn bright_reprisal_destroys_an_attacker_and_draws_a_card() {
    let mut engine = table(
        910,
        &[bright_reprisal()],
        &[plains(); 5],
        &[gray_ogre()],
        &[llanowar_elves()],
    );
    let attacker = on_battlefield(&engine, me(), gray_ogre()).expect("my Ogre");
    let bystander = on_battlefield(&engine, them(), llanowar_elves()).expect("their Elf");
    declare_attack(&mut engine, attacker);

    tap_all_mana(&mut engine, me());
    let (hand_before, library_before) = (hand_len(&engine, me()), library_len(&engine, me()));
    cast_with_floating(&mut engine, me(), bright_reprisal());
    let menu = aim_at(&mut engine, me(), attacker);
    assert!(menu.contains(&attacker), "the attacker: {menu:?}");
    assert!(
        !menu.contains(&bystander),
        "a creature that did not attack is no target: {menu:?}"
    );
    settle(&mut engine);

    assert!(
        in_graveyard(&engine, me(), gray_ogre()).is_some(),
        "the attacker is destroyed"
    );
    assert!(on_battlefield(&engine, them(), llanowar_elves()).is_some());
    assert_eq!(hand_len(&engine, me()), hand_before, "one out, one in");
    assert_eq!(library_len(&engine, me()), library_before - 1);
}
