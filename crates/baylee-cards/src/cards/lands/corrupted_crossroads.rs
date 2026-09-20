//! Corrupted Crossroads — (no cost) — Land
//! Oracle: {T}: Add {C}. ({C} represents colorless mana.)
//! Oracle: {T}, Pay 1 life: Add one mana of any color. Spend this mana only to cast a spell with devoid.
//! Set: OGW #169 — Oath of the Gatewatch | Scryfall ID: 7043e193-8051-4f1b-9633-9c1f250d3607 | Oracle ID: 6276a985-7630-476d-a94f-6c6adc88f6c4
// PARTIAL — {T}: Add {C}. The second ability is off the card rather than
// printed without its rider: see NOT SUPPORTED below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CORRUPTED_CROSSROADS,
    oracle_id = "6276a985-7630-476d-a94f-6c6adc88f6c4",
    scryfall_id = "7043e193-8051-4f1b-9633-9c1f250d3607",
    faces = &[face!(name = "Corrupted Crossroads", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "{T}, Pay 1 life: Add one mana of any color is not implemented — its mana \
         may be spent only on a spell with devoid, and devoid is not a keyword the \
         engine reads, so `ManaRestriction.filter` has no way to name that spell"
    ),
    abilities = &[
        // NOT SUPPORTED: {T}, Pay 1 life: Add one mana of any color. Spend this mana
        // only to cast a spell with devoid. — `Effect::restricted` can carry the
        // rider, but the filter it needs is a spell with devoid and no `Filter`
        // spells that (`HasKeyword` would need a bit nothing reads). Written
        // without the rider it would be a strictly different card — any-color
        // mana for any spell — so the ability comes off rather than play wrong.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    ],
);
