//! `cards/instants/mv_5/consume_the_meek.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "c2555c38-1bfc-48c1-85aa-7e40117db78b"
fn consume_the_meek() -> CardIndex {
    card_index("c2555c38-1bfc-48c1-85aa-7e40117db78b")
}

/// Consume the Meek: "Destroy each creature with mana value 3 or less. They
/// can't be regenerated."
///
/// The edge is mana value three: the Ogre and the Sedge Troll (both `{2}{R}`)
/// and the Elf die, and the five-mana Earth Elemental across the table lives.
/// The Troll holds a regeneration shield, which is the second sentence.
#[test]
fn consume_the_meek_destroys_the_cheap_creatures_through_a_shield() {
    let mut engine = table(
        912,
        &[consume_the_meek()],
        &[swamp(); 6],
        &[sedge_troll(), gray_ogre()],
        &[llanowar_elves(), earth_elemental()],
    );
    let troll = on_battlefield(&engine, me(), sedge_troll()).expect("my Troll");
    tap_all_mana(&mut engine, me());
    raise_a_shield(&mut engine, me(), troll, 1);
    cast_with_floating(&mut engine, me(), consume_the_meek());
    assert!(
        !matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "\"each\" names no target"
    );
    settle(&mut engine);

    assert!(in_graveyard(&engine, me(), sedge_troll()).is_some(), "mana value 3, shield or not");
    assert!(in_graveyard(&engine, me(), gray_ogre()).is_some(), "mana value 3");
    assert!(in_graveyard(&engine, them(), llanowar_elves()).is_some(), "mana value 1");
    assert!(
        on_battlefield(&engine, them(), earth_elemental()).is_some(),
        "mana value 5 is past the edge"
    );
}
