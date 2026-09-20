//! Primal Beyond — (no cost) — Land
//! Oracle: As this land enters, you may reveal an Elemental card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast an Elemental spell or activate an ability of an Elemental.
//! Set: ECC #159 — Lorwyn Eclipsed Commander | Scryfall ID: cf01949c-1aa3-4ce6-aabf-00c6d0498f5c | Oracle ID: 541744d9-449d-420a-a5a1-2fffba18450f
// PARTIAL — {C}, and one mana of any color restricted to Elemental spells.
// NOT SUPPORTED: "As this land enters, you may reveal an Elemental card from
// your hand. If you don't, this land enters tapped." — no `EnterModifier`
// offers a reveal from hand (`TappedUnless` asks about a permanent on the
// battlefield), so the land enters untapped here whatever is in hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

// "An Elemental spell" — the whole of what the second mana may be spent on.
static ELEMENTAL: Filter = Filter::HasSubtype(creature::ELEMENTAL);

card!(
    index = index::PRIMAL_BEYOND,
    oracle_id = "541744d9-449d-420a-a5a1-2fffba18450f",
    scryfall_id = "cf01949c-1aa3-4ce6-aabf-00c6d0498f5c",
    faces = &[face!(name = "Primal Beyond", types = TypeSet::LAND,),],
    coverage = Coverage::Partial("no reveal-from-hand enter clause; restriction is spells-only"),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "or activate an ability of an Elemental" — a
        // `ManaRestriction` names the spells its mana may be spent on, and an
        // activation is not a spell.
        mana_ability!(&[Effect::mana_of_any_color().restricted(&ELEMENTAL, SpendRider::None)]),
    ],
);
