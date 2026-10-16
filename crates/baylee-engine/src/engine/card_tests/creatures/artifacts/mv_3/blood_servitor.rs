//! `cards/creatures/artifacts/mv_3/blood_servitor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn blood_servitor() -> CardIndex {
    card_index("5a3f9c4d-20a8-46aa-9cf3-3916f56a7906")
}

/// Blood Servitor — "When this creature enters, create a Blood token": one
/// token, an artifact and not a creature, under its controller.
#[test]
fn blood_servitor_creates_one_blood_token_on_entering() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[blood_servitor()], &[forest(), forest(), forest()]);
    assert!(tokens_of(&engine, p0).is_empty());

    cast_and_settle(&mut engine, blood_servitor());

    let servitor = on_battlefield(&engine, p0, blood_servitor()).expect("it resolved");
    assert_eq!(pt(&engine, servitor), (2, 2));
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Blood token");
    let t = types(&engine, tokens[0]);
    assert!(t.contains(TypeSet::ARTIFACT) && !t.contains(TypeSet::CREATURE));
    assert!(tokens_of(&engine, PlayerId::new(1)).is_empty());
}

/// The token is the Blood the card names: "{1}, {T}, Discard a card, Sacrifice
/// this token: Draw a card." Paid with the mana left floating and a Forest
/// from hand, it goes, the Forest goes to the graveyard and a card is drawn.
#[test]
fn blood_servitor_token_loots_for_one_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(
        &[blood_servitor(), forest()],
        &[forest(), forest(), forest(), forest()],
    );
    cast_and_settle(&mut engine, blood_servitor());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "four Forests, three spent"
    );
    let token = tokens_of(&engine, p0)[0];
    let library_before = library_of(&engine, p0).len();

    let (source, ability_index) = priority_offer(&engine)
        .abilities
        .iter()
        .copied()
        .find(|(s, _)| *s == token)
        .expect("the token's ability is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("it activates");
    let rest = drive_to_rest(&mut engine, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    assert!(tokens_of(&engine, p0).is_empty(), "the Blood was sacrificed");
    assert!(in_graveyard(&engine, p0, forest()).is_some(), "the Forest was discarded");
    assert_eq!(library_of(&engine, p0).len(), library_before - 1, "and a card drawn");
}
