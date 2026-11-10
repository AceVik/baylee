//! `cards/sorceries/mv_6/akroma_s_vengeance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "ae19dce5-c9fc-4fbe-99b5-dc6a12124a68"
fn akroma_s_vengeance() -> CardIndex {
    card_index("ae19dce5-c9fc-4fbe-99b5-dc6a12124a68")
}

/// Akroma's Vengeance: "Destroy all artifacts, creatures, and enchantments."
///
/// Both seats lose one of each kind, the Baleful Strix counts for two kinds at
/// once, and the lands of both seats are not among them.
#[test]
fn akroma_s_vengeance_destroys_artifacts_creatures_and_enchantments() {
    let mut engine = table(
        940,
        &[akroma_s_vengeance()],
        &[plains(); 6],
        &[llanowar_elves(), quiet_artifact(), their_enchantment()],
        &[gray_ogre(), baleful_strix(), forest()],
    );
    cast_plain(&mut engine, akroma_s_vengeance());

    assert!(
        in_graveyard(&engine, me(), llanowar_elves()).is_some(),
        "a creature of mine"
    );
    assert!(
        in_graveyard(&engine, me(), quiet_artifact()).is_some(),
        "an artifact of mine"
    );
    assert!(
        in_graveyard(&engine, me(), their_enchantment()).is_some(),
        "an enchantment of mine"
    );
    assert!(
        in_graveyard(&engine, them(), gray_ogre()).is_some(),
        "a creature of theirs"
    );
    assert!(
        in_graveyard(&engine, them(), baleful_strix()).is_some(),
        "an artifact creature"
    );
    assert!(
        on_battlefield(&engine, them(), forest()).is_some(),
        "their land stays"
    );
    assert!(
        on_battlefield(&engine, me(), plains()).is_some(),
        "my lands stay"
    );
}

/// Cycling {3}: from the hand, for three mana, the card is discarded and a card
/// is drawn, and the sweep does not happen.
#[test]
fn akroma_s_vengeance_cycles_for_three_without_sweeping() {
    let mut engine = table(
        941,
        &[akroma_s_vengeance()],
        &[plains(); 3],
        &[llanowar_elves()],
        &[gray_ogre()],
    );
    let card = in_hand(&engine, me(), akroma_s_vengeance()).expect("in hand");
    let (hand_before, library_before) = (hand_len(&engine, me()), library_len(&engine, me()));
    tap_all_mana_but(&mut engine, me(), Some(llanowar_elves()));
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: card,
                ability_index: 0,
            },
        )
        .expect("cycling is offered from the hand");
    settle(&mut engine);

    assert!(
        in_graveyard(&engine, me(), akroma_s_vengeance()).is_some(),
        "discarded"
    );
    assert_eq!(hand_len(&engine, me()), hand_before, "one out, one in");
    assert_eq!(library_len(&engine, me()), library_before - 1);
    assert!(
        on_battlefield(&engine, them(), gray_ogre()).is_some(),
        "nothing was destroyed"
    );
    assert!(on_battlefield(&engine, me(), llanowar_elves()).is_some());
}
