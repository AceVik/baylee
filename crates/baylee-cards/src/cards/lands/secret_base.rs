//! Secret Base — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a spell that shares a watermark with this land.
//! Set: UST #165a — Unstable | Scryfall ID: 9a4306c2-1b92-4ccc-8d92-9f27f1505113 | Oracle ID: 1017088c-08a3-45d9-a7f7-01fb2f309717
// PARTIAL — {T}: Add {C}; the any-color ability is off the card, see the
// NOT SUPPORTED line at the abilities list.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SECRET_BASE,
    oracle_id = "1017088c-08a3-45d9-a7f7-01fb2f309717",
    scryfall_id = "9a4306c2-1b92-4ccc-8d92-9f27f1505113",
    faces = &[face!(name = "Secret Base", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "\"Spend this mana only to cast a spell that shares a watermark with this land\" — the ability is dropped rather than made unrestricted"
    ),
    // NOT SUPPORTED: {T}: Add one mana of any color. Spend this mana only to
    // cast a spell that shares a watermark with this land. — ManaRestriction
    // names a Filter and a SpendRider, and a watermark is no characteristic
    // any Filter can read, so the second ability is written nowhere. Left on
    // the card it would be `Effect::mana_of_any_color()` with no rider, which
    // is a strictly stronger card than the printing.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
