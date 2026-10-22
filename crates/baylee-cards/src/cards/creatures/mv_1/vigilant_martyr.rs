//! Vigilant Martyr — {W} — Creature — Human Cleric
//! Oracle: Sacrifice this creature: Regenerate target creature.
//! Oracle: {W}{W}, {T}, Sacrifice this creature: Counter target spell that targets an enchantment.
//! Set: MIR #45 — Mirage | Scryfall ID: a02e3c10-cc21-4e06-a987-b03aee61bd50 | Oracle ID: 122dfe9f-e0e2-4f69-9637-289c956fd54a
// PARTIAL — sacrifice to regenerate target creature; {W}{W}, {T}, sacrifice counters target spell (no "spell that targets an enchantment" target filter yet).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VIGILANT_MARTYR,
    oracle_id = "122dfe9f-e0e2-4f69-9637-289c956fd54a",
    scryfall_id = "a02e3c10-cc21-4e06-a987-b03aee61bd50",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "sacrifice to regenerate target creature; {W}{W}, {T}, sacrifice counters target spell (no 'spell that targets an enchantment' target filter yet)",
    ),
    faces = &[face!(
        name = "Vigilant Martyr",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        activated!(
            cost!(SacrificeSelf),
            &[Effect::regenerate(TargetSpec::Object(&Filter::CREATURE))],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
        activated!(
            cost!("{W}{W}", TapSelf, SacrificeSelf),
            &[Effect::CounterTargetSpell],
            target = Some(TargetSpec::Spell(&Filter::Any)),
        ),
    ],
);
