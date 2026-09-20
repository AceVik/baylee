//! Eclipsed Realms — (no cost) — Land
//! Oracle: As this land enters, choose Elemental, Elf, Faerie, Giant, Goblin, Kithkin, Merfolk, or Treefolk.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a spell of the chosen type or activate an ability of a source of the chosen type.
//! Set: ECL #263 — Lorwyn Eclipsed | Scryfall ID: a174f0db-8b4f-4c37-9583-44c92d37b9c0 | Oracle ID: 5715ed43-395c-4877-99a7-8e28e7bf9dce
// PARTIAL — the {C} ability, the any-color mana ability and the as-it-enters
// type choice are built; the printed spend restriction and the printed list
// of eight types are not expressible (see the NOT SUPPORTED note below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ECLIPSED_REALMS,
    oracle_id = "5715ed43-395c-4877-99a7-8e28e7bf9dce",
    scryfall_id = "a174f0db-8b4f-4c37-9583-44c92d37b9c0",
    faces = &[face!(
        name = "Eclipsed Realms",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::ChooseSubtype],
    )],
    coverage = Coverage::Partial(
        "the any-color mana's spend restriction — \"only to cast a spell of the chosen type \
         or activate an ability of a source of the chosen type\" — has no variant: a \
         ManaRestriction names spells and never an activation, and pool mana has no \
         provenance; and the eight types the card prints are not sayable, \
         EnterModifier::ChooseSubtype offering every creature type"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "Spend this mana only to cast a spell of the chosen
        // type or activate an ability of a source of the chosen type." — the
        // mana is made, and made unrestricted.
        mana_ability!(&[Effect::mana_of_any_color()]),
    ],
);
