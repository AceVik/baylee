//! Watermarket — (no cost) — Land
//! Oracle: {T}: Add {C}{C}. Spend this mana only to cast spells with watermarks.
//! Set: UST #166 — Unstable | Scryfall ID: bf98bb09-c479-4dba-b542-ac5475fac697 | Oracle ID: 84d89a3d-4b28-4e19-8298-737ec6a06238
// PARTIAL — the {T} ability is built; the spend restriction is not sayable.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WATERMARKET,
    oracle_id = "84d89a3d-4b28-4e19-8298-737ec6a06238",
    scryfall_id = "bf98bb09-c479-4dba-b542-ac5475fac697",
    faces = &[face!(name = "Watermarket", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the mana is produced without its restriction: no Filter can read a watermark"
    ),
    abilities = &[
        // NOT SUPPORTED: "Spend this mana only to cast spells with watermarks."
        // `Effect::restricted` takes a `Filter` and a `SpendRider`, and the
        // `Manasource`/`HasColor`/`HasType`/… algebra has no clause that reads
        // a card's watermark — so the mana is made, and it is made spendable
        // on anything.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 2)]),
    ],
);
