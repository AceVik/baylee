//! Volrath's Laboratory — {5} — Artifact
//! Oracle: As this artifact enters, choose a color and a creature type.
//! Oracle: {5}, {T}: Create a 2/2 creature token of the chosen color and type.
//! Set: TPR #234 — Tempest Remastered | Scryfall ID: aa9b2bbd-0458-4ee8-8992-843169c2b137 | Oracle ID: f2a6e26c-6bd6-4fca-ba9f-d52d01fc03ef
// PARTIAL: applying the chosen color and creature type to the created token is not supported; chooses a color and a creature type as it enters and activates "{5}, {T}" to create a 2/2 creature token.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VOLRATH_S_LABORATORY,
    oracle_id = "f2a6e26c-6bd6-4fca-ba9f-d52d01fc03ef",
    scryfall_id = "aa9b2bbd-0458-4ee8-8992-843169c2b137",
    coverage = Coverage::Partial(
        "applying the chosen color and creature type to the created token is not supported; chooses a color and a creature type as it enters and activates \"{5}, {T}\" to create a 2/2 creature token",
    ),
    faces = &[face!(
        name = "Volrath's Laboratory",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
        enter_modifiers = &[EnterModifier::ChooseColor, EnterModifier::ChooseSubtype],
    ),],
    abilities = &[activated!(
        cost!("{5}", TapSelf),
        &[Effect::CreateToken {
            token: &crate::generated_tokens::CHOSENTYPE_2_2,
        }],
    ),],
);
