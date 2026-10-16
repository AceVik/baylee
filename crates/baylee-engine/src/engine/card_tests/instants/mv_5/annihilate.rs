//! `cards/instants/mv_5/annihilate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "28f55673-7aa0-497e-8434-aa70df1b5e04"
fn annihilate() -> CardIndex {
    card_index("28f55673-7aa0-497e-8434-aa70df1b5e04")
}

/// Annihilate: "Destroy target nonblack creature. It can't be regenerated.
/// Draw a card."
///
/// One board answers every clause. The Sedge Troll (red, so a legal target)
/// buys a regeneration shield first, which only the second sentence gets past;
/// the black Lotleth Troll across the table is on the board for the first
/// word and must be missing from the menu; and the hand is counted either side
/// for the draw, which is a move and not a feeling.
#[test]
fn annihilate_destroys_a_nonblack_creature_through_a_shield_and_draws() {
    let mut engine = table(
        901,
        &[annihilate()],
        &[swamp(); 6],
        &[sedge_troll()],
        &[lotleth_troll()],
    );
    let red = on_battlefield(&engine, me(), sedge_troll()).expect("the Sedge Troll is out");
    let black = on_battlefield(&engine, them(), lotleth_troll()).expect("their Troll is out");

    tap_all_mana(&mut engine, me());
    raise_a_shield(&mut engine, me(), red, 1);
    let hand_before = hand_len(&engine, me());

    cast_with_floating(&mut engine, me(), annihilate());
    let menu = aim_at(&mut engine, me(), red);
    assert!(menu.contains(&red), "a red creature is a legal target: {menu:?}");
    assert!(
        !menu.contains(&black),
        "\"nonblack\" keeps the black Troll off the menu: {menu:?}"
    );
    settle(&mut engine);

    assert!(
        in_graveyard(&engine, me(), sedge_troll()).is_some(),
        "destroyed, and the standing shield did not save it"
    );
    assert!(
        on_battlefield(&engine, them(), lotleth_troll()).is_some(),
        "the black creature was never touched"
    );
    assert_eq!(
        hand_len(&engine, me()),
        hand_before,
        "the spell left the hand and one card replaced it"
    );
}
