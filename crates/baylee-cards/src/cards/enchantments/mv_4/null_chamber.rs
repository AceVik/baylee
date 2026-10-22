//! Null Chamber — {3}{W} — World Enchantment
//! Oracle: As this enchantment enters, you and an opponent each choose a card name other than a basic land card name.
//! Oracle: Spells with the chosen names can't be cast and lands with the chosen names can't be played.
//! Set: MIR #31 — Mirage | Scryfall ID: 814f8976-6612-438f-a04a-8edb63edb1e7 | Oracle ID: cb75aca6-dfc7-460d-b90c-80b7988f39a8
// PARTIAL: EnterModifier::ChooseCardName asks only the controller for any card name and the engine has no static modifier forbidding casting spells or playing lands with chosen names; as this World enchantment enters you choose a card name and activated abilities of sources with that name cannot be activated

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NULL_CHAMBER,
    oracle_id = "cb75aca6-dfc7-460d-b90c-80b7988f39a8",
    scryfall_id = "814f8976-6612-438f-a04a-8edb63edb1e7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "EnterModifier::ChooseCardName asks only the controller for any card name and the engine has no static modifier forbidding casting spells or playing lands with chosen names; as this World enchantment enters you choose a card name and activated abilities of sources with that name cannot be activated"
    ),
    faces = &[face!(
        name = "Null Chamber",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
        enter_modifiers = &[EnterModifier::ChooseCardName],
    ),],
    abilities = &[static_ability!(
        Filter::Any,
        Modifier::ChosenNameCantActivate
    )],
);
