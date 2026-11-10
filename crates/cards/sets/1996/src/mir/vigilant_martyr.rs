//! Vigilant Martyr — {W} — Creature — Human Cleric
//! Oracle: Sacrifice this creature: Regenerate target creature.
//! Oracle: {W}{W}, {T}, Sacrifice this creature: Counter target spell that targets an enchantment.
//! Set: MIR #45 — Mirage | Scryfall ID: a02e3c10-cc21-4e06-a987-b03aee61bd50 | Oracle ID: 122dfe9f-e0e2-4f69-9637-289c956fd54a
// PARTIAL — {W}{W}, {T}, Sacrifice this creature: Counter target spell that targets an enchantment is not played: no filter says a spell that targets an enchantment.
// NOT SUPPORTED: "{W}{W}, {T}, Sacrifice this creature: Counter target spell that targets an enchantment." No filter says "a spell that targets an enchantment", and counting any spell would counter more than is printed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VIGILANT_MARTYR,
    oracle_id = "122dfe9f-e0e2-4f69-9637-289c956fd54a",
    scryfall_id = "a02e3c10-cc21-4e06-a987-b03aee61bd50",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "{W}{W}, {T}, Sacrifice this creature: Counter target spell that targets an enchantment is not played: no filter says a spell that targets an enchantment",
    ),
    faces = &[face!(
        name = "Vigilant Martyr",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!(SacrificeSelf),
        &[Effect::regenerate(TargetSpec::Object(&Filter::CREATURE))],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
