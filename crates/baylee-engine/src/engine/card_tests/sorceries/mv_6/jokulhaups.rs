//! `cards/sorceries/mv_6/jokulhaups.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "7bdfe368-353c-4f8d-851f-05b7ea9f905d"
fn jokulhaups() -> CardIndex {
    card_index("7bdfe368-353c-4f8d-851f-05b7ea9f905d")
}

/// Jokulhaups: "Destroy all artifacts, creatures, and lands. They can't be
/// regenerated."
///
/// Every land of both seats, an artifact, and a creature that holds a
/// regeneration shield go; an enchantment, which is none of the three, is the
/// only permanent left.
#[test]
fn jokulhaups_destroys_artifacts_creatures_and_lands_through_a_shield() {
    let lands = [vec![mountain(); 6], vec![swamp()]].concat();
    let mut engine = table(
        963,
        &[jokulhaups()],
        &lands,
        &[sedge_troll(), quiet_artifact()],
        &[forest(), llanowar_elves(), their_enchantment()],
    );
    let troll = on_battlefield(&engine, me(), sedge_troll()).expect("my Troll");
    tap_only(&mut engine, swamp());
    raise_a_shield(&mut engine, me(), troll, 1);
    tap_only(&mut engine, mountain());
    cast_with_floating(&mut engine, me(), jokulhaups());
    settle(&mut engine);

    assert!(
        in_graveyard(&engine, me(), sedge_troll()).is_some(),
        "shield or not"
    );
    assert!(
        in_graveyard(&engine, me(), quiet_artifact()).is_some(),
        "an artifact"
    );
    assert!(
        in_graveyard(&engine, me(), mountain()).is_some(),
        "a land of mine"
    );
    assert!(
        in_graveyard(&engine, them(), forest()).is_some(),
        "a land of theirs"
    );
    assert!(
        in_graveyard(&engine, them(), llanowar_elves()).is_some(),
        "a creature of theirs"
    );
    assert_eq!(battlefield_len(&engine), 1, "only the enchantment is left");
    assert!(on_battlefield(&engine, them(), their_enchantment()).is_some());
}
