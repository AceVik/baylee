//! Crucible of the Spirit Dragon — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Put a storage counter on this land.
//! Oracle: {T}, Remove X storage counters from this land: Add X mana in any combination of colors. Spend this mana only to cast Dragon spells or activate abilities of Dragons.
//! Set: AFC #231 — Forgotten Realms Commander | Scryfall ID: 257a5ae9-f754-4c45-a47d-5196f1b625b7 | Oracle ID: ecfbebc9-7fc7-474e-8c59-8ede800e082e
// PARTIAL — {C}; {1}, {T} for a storage counter; {T} plus X storage counters
// for X mana in any combination of colors, restricted to Dragon spells. The
// half of the spend restriction that names an ability's source has nothing in
// the vocabulary to say it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::CRUCIBLE_OF_THE_SPIRIT_DRAGON,
    oracle_id = "ecfbebc9-7fc7-474e-8c59-8ede800e082e",
    scryfall_id = "257a5ae9-f754-4c45-a47d-5196f1b625b7",
    faces = &[face!(
        name = "Crucible of the Spirit Dragon",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Partial(
        "\"or activate abilities of Dragons\": a ManaRestriction is one Filter \
         read against the object being paid for, and no Filter in the DSL \
         reaches the source of an ability on the stack"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::AddCounter {
                kind: counters::STORAGE,
                amount: Amount::Fixed(1),
            }]
        ),
        // NOT SUPPORTED: "Spend this mana only to cast Dragon spells or
        // activate abilities of Dragons." — the Dragon-spell half is the
        // ManaRestriction below; the ability half is dropped, because an
        // ability on the stack carries no subtype and no Filter names the
        // permanent it came from.
        mana_ability!(
            cost!(
                TapSelf,
                RemoveCounterSelfX {
                    kind: counters::STORAGE
                }
            ),
            &[Effect::mana_combination(ALL_MANA_COLORS, Amount::X)
                .restricted(&Filter::HasSubtype(creature::DRAGON), SpendRider::None)]
        ),
    ],
);
