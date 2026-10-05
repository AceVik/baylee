//! Metamorphosis — {G} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Add X mana of any one color, where X is 1 plus the sacrificed creature's mana value. Spend this mana only to cast creature spells.
//! Set: CHR #66 — Chronicles | Scryfall ID: fc73bd94-6e14-4798-b9ff-163ba7bdd663 | Oracle ID: 7140d726-0136-43af-84b5-85005a66a186
// IMPLEMENTED — the cast sacrifices a creature, then adds 1 plus its mana
// value in one chosen color, spendable only on creature spells.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::METAMORPHOSIS,
    oracle_id = "7140d726-0136-43af-84b5-85005a66a186",
    scryfall_id = "fc73bd94-6e14-4798-b9ff-163ba7bdd663",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Metamorphosis",
        mana_cost = mana!("{G}"),
        types = TypeSet::SORCERY,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::CREATURE)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::mana_choice_dynamic(
        ALL_MANA_COLORS,
        Amount::Plus {
            base: &Amount::SacrificedManaValue,
            offset: 1,
        },
    )
    .restricted(&Filter::CREATURE, SpendRider::None)])],
);
