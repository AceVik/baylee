//! Villainous Hideout — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a Villain spell or to activate an ability of a Villain source.
//! Oracle: {3}, {T}: Target Villain you control connives. Activate only as a sorcery. (Draw a card, then discard a card. If you discarded a nonland card, put a +1/+1 counter on that creature.)
//! Set: MSH #276 — Marvel Super Heroes | Scryfall ID: 822b0249-e1df-453d-8b60-75a5196ed818 | Oracle ID: cd2888aa-71f3-47ee-ba33-7bb95d5bc836
// PARTIAL — {C}, and any color restricted to a Villain spell. Connive has no
// Effect, and the restriction cannot name an ability's source.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::VILLAINOUS_HIDEOUT,
    oracle_id = "cd2888aa-71f3-47ee-ba33-7bb95d5bc836",
    scryfall_id = "822b0249-e1df-453d-8b60-75a5196ed818",
    faces = &[face!(name = "Villainous Hideout", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "connive is not an Effect, and ManaRestriction::filter is read against the \
         spell being cast — it has no word for an ability of a Villain source"
    ),
    abilities = &[
        // {T}: Add {C}.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // {T}: Add one mana of any color, spendable only on a Villain spell.
        // NOT SUPPORTED: "or to activate an ability of a Villain source" —
        // ManaRestriction reaches the spell being cast and nothing else.
        mana_ability!(&[Effect::mana_of_any_color()
            .restricted(&Filter::HasSubtype(creature::VILLAIN), SpendRider::None)]),
        // NOT SUPPORTED: "{3}, {T}: Target Villain you control connives." —
        // no Effect draws then discards and reads the discarded card back to
        // put a +1/+1 counter on the conniving creature.
    ],
);
