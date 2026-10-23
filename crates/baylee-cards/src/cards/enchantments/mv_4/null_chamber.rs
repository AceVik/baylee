//! Null Chamber — {3}{W} — World Enchantment
//! Oracle: As this enchantment enters, you and an opponent each choose a card name other than a basic land card name.
//! Oracle: Spells with the chosen names can't be cast and lands with the chosen names can't be played.
//! Set: MIR #31 — Mirage | Scryfall ID: 814f8976-6612-438f-a04a-8edb63edb1e7 | Oracle ID: cb75aca6-dfc7-460d-b90c-80b7988f39a8
// PARTIAL: only ONE name is chosen, by the controller (EnterModifier::ChooseCardName asks one player; the opponent's name is missing), and a basic land name cannot be refused as an answer, so it is simply no lock (the filter excludes basic lands)
// NOT SUPPORTED: "you and an opponent each choose a card name": the opponent's name is not asked for.

use baylee_cards_dsl::prelude::*;

/// A card with the name the controller chose, unless it is a basic land card.
const CHOSEN_NAME_NOT_A_BASIC_LAND: Filter =
    Filter::And(&[Filter::NamedBySource, Filter::Not(&Filter::BASIC_LAND)]);

card!(
    index = index::NULL_CHAMBER,
    oracle_id = "cb75aca6-dfc7-460d-b90c-80b7988f39a8",
    scryfall_id = "814f8976-6612-438f-a04a-8edb63edb1e7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "only ONE name is chosen, by the controller (EnterModifier::ChooseCardName asks one player, so the opponent's name is missing), and a basic land name cannot be refused as an answer: naming one locks nothing (the filter excludes basic lands); the chosen name can't be cast as a spell or played as a land, for every player"
    ),
    faces = &[face!(
        name = "Null Chamber",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
        enter_modifiers = &[EnterModifier::ChooseCardName],
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::PlayersCantPlay(&CHOSEN_NAME_NOT_A_BASIC_LAND)
    )],
);
