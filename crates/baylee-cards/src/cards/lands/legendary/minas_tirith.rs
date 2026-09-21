//! Minas Tirith — (no cost) — Legendary Land
//! Oracle: Minas Tirith enters tapped unless you control a legendary creature.
//! Oracle: {T}: Add {W}.
//! Oracle: {1}{W}, {T}: Draw a card. Activate only if you attacked with two or more creatures this turn.
//! Set: LTR #256 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: b38b6760-616f-4b11-8ce7-ac1223c7fd53 | Oracle ID: 7b0d7e62-0287-454a-8702-b0bfa7b41245
// PARTIAL — the as-it-enters condition and the {W} mana ability; the gated
// draw ability is left off because no `Condition` variant can say it.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MINAS_TIRITH,
    oracle_id = "7b0d7e62-0287-454a-8702-b0bfa7b41245",
    scryfall_id = "b38b6760-616f-4b11-8ce7-ac1223c7fd53",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "the {1}{W}, {T} ability prints \"Activate only if you attacked with two or more creatures this turn\", and no Condition variant counts attackers"
    ),
    faces = &[face!(
        name = "Minas Tirith",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
        enter_modifiers = &[EnterModifier::TappedUnless(&f!(your LEGENDARY_CREATURE))],
    ),],
    // NOT SUPPORTED: "{1}{W}, {T}: Draw a card. Activate only if you attacked
    // with two or more creatures this turn." — `Condition` has no
    // attackers-this-turn sentence, and the ability is left off rather than
    // offered unconditionally.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])],
);
