//! Underdome — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to pay Un-costs.
//! Set: UND #86 — Unsanctioned | Scryfall ID: d5c362ea-16e2-4c13-bed2-735c8c09d975 | Oracle ID: 8375aaaa-edc2-4a0c-98f6-af07d61ebd0a
// PARTIAL — {T}: Add {C} is built. The any-color ability is dropped rather
// than written without its rider: a `ManaRestriction` names the spells the
// mana may be spent on with a `Filter`, and no filter variant names an Un-set
// card, so unrestricted mana of any color would pay for every spell there is.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNDERDOME,
    oracle_id = "8375aaaa-edc2-4a0c-98f6-af07d61ebd0a",
    scryfall_id = "d5c362ea-16e2-4c13-bed2-735c8c09d975",
    faces = &[face!(name = "Underdome", types = TypeSet::LAND,)],
    coverage = Coverage::Partial(
        "the 'spend this mana only to pay Un-costs' rider has no Filter that names an Un-set card"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

// NOT SUPPORTED: "{T}: Add one mana of any color. Spend this mana only to pay
// Un-costs." — `Effect::mana_of_any_color().restricted(&filter, …)` needs a
// `Filter` over the spell being cast, and nothing in the vocabulary matches a
// card from an Un-set; writing the mana line without the rider would hand the
// land mana spendable on anything, which is a strictly different card.
