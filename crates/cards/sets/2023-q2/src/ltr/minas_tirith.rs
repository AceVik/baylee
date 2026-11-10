//! Minas Tirith — (no cost) — Legendary Land
//! Oracle: Minas Tirith enters tapped unless you control a legendary creature.
//! Oracle: {T}: Add {W}.
//! Oracle: {1}{W}, {T}: Draw a card. Activate only if you attacked with two or more creatures this turn.
//! Set: LTR #256 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: b38b6760-616f-4b11-8ce7-ac1223c7fd53 | Oracle ID: 7b0d7e62-0287-454a-8702-b0bfa7b41245

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MINAS_TIRITH,
    oracle_id = "7b0d7e62-0287-454a-8702-b0bfa7b41245",
    scryfall_id = "b38b6760-616f-4b11-8ce7-ac1223c7fd53",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Minas Tirith",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
        enter_modifiers = &[EnterModifier::TappedUnless(&f!(your LEGENDARY_CREATURE))],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        activated!(
            cost!("{1}{W}", TapSelf),
            &[Effect::draw(1)],
            condition = Some(Condition::AttackedWithAtLeast(2))
        )
    ],
);
