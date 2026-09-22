//! Dungeon Descent — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}, Tap an untapped legendary creature you control: Venture into the dungeon. Activate only as a sorcery. (Enter the first room or advance to the next room.)
//! Set: AFR #255 — Adventures in the Forgotten Realms | Scryfall ID: f4cccdbc-f4f4-42b6-9747-6ef703ff949a | Oracle ID: f086a63c-0c62-4674-bd27-82e7aed12b1a
// PARTIAL — the comes-into-play clause and the colorless mana ability are
// built; the venture ability is off the card (see the NOT SUPPORTED line).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DUNGEON_DESCENT,
    oracle_id = "f086a63c-0c62-4674-bd27-82e7aed12b1a",
    scryfall_id = "f4cccdbc-f4f4-42b6-9747-6ef703ff949a",
    faces = &[face!(
        name = "Dungeon Descent",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage =
        Coverage::Partial("the DSL has no dungeon effect, so the venture ability is dropped"),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{4}, {T}, Tap an untapped legendary creature you
        // control: Venture into the dungeon. Activate only as a sorcery."
        // — dungeons are not in the vocabulary (no Effect names a venture,
        // a dungeon or a room), so the ability comes off rather than being
        // offered as one that does nothing.
    ],
);
