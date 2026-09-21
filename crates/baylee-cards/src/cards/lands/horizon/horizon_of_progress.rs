//! Horizon of Progress — (no cost) — Land
//! Oracle: {T}, Pay 1 life: Add one mana of any type that a land you control could produce.
//! Oracle: {3}, {T}: You may put a land card from your hand onto the battlefield tapped.
//! Oracle: {1}, {T}, Sacrifice this land: Draw a card.
//! Set: M3C #78 — Modern Horizons 3 Commander | Scryfall ID: 5ae3a9c8-194e-421b-b77d-9c8784442651 | Oracle ID: 59a82f57-fe2f-4834-a4ee-4b948eef1e12
// PARTIAL — the {T}, Pay 1 life mana ability (Reflecting Pool's own sentence,
// `ManaSource::LandColor { mine: true }`) and the {1}, {T}, Sacrifice draw
// ability are built; the {3}, {T} one has no effect to be written with, so
// the card is not playable in full.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HORIZON_OF_PROGRESS,
    oracle_id = "59a82f57-fe2f-4834-a4ee-4b948eef1e12",
    scryfall_id = "5ae3a9c8-194e-421b-b77d-9c8784442651",
    faces = &[face!(name = "Horizon of Progress", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the {3}, {T} ability — put a land card from your hand onto the \
         battlefield tapped — has no Effect: nothing in the DSL moves a card \
         from a hand to the battlefield (SearchLibrary reads a library, \
         PutFromHandOnTop writes to the library, and PlayLandsFromGraveyard \
         / ExtraLandDrops are a permission to play, untapped, not a put)",
    ),
    abilities = &[
        mana_ability!(cost!(TapSelf, PayLife(1)), &[Effect::mana_land_color(true)]),
        // NOT SUPPORTED: {3}, {T}: You may put a land card from your hand onto
        // the battlefield tapped. No effect reaches a hand → battlefield, and
        // an extra land drop would be a land played untapped at sorcery speed
        // on your own turn, which is a different card.
        activated!(cost!("{1}", TapSelf, SacrificeSelf), &[Effect::draw(1)]),
    ],
);
