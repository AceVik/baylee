//! Lindblum, Industrial Regency // Mage Siege — (no cost) — Land — Town // Instant — Adventure
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R}.
//! Oracle: Create a 0/1 black Wizard creature token with "Whenever you cast a noncreature spell, this token deals 1 damage to each opponent."
//! Set: FIN #285 — Final Fantasy | Scryfall ID: 548dd152-f0b6-4e8f-9afc-a4ec1671b648 | Oracle ID: 4cc014f3-05e0-442e-9dee-03eab1aa65a3
//! Face: Lindblum, Industrial Regency —  — Land — Town
//! Face: Mage Siege — {2}{R} — Instant — Adventure
// PARTIAL — the land half is built: it enters tapped and taps for {R}. Mage
// Siege's token has no shape in the DSL and is left off (see NOT SUPPORTED).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LINDBLUM_INDUSTRIAL_REGENCY,
    oracle_id = "4cc014f3-05e0-442e-9dee-03eab1aa65a3",
    scryfall_id = "548dd152-f0b6-4e8f-9afc-a4ec1671b648",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Lindblum, Industrial Regency",
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::TOWN],
            enter_modifiers = &[EnterModifier::Tapped],
        ),
        face!(
            name = "Mage Siege",
            mana_cost = mana!("{2}{R}"),
            types = TypeSet::INSTANT,
            subtypes = &[subtypes::spell::ADVENTURE],
        ),
    ],
    // NOT SUPPORTED: "Create a 0/1 black Wizard creature token with 'Whenever you cast a
    // noncreature spell, this token deals 1 damage to each opponent.'" — no Effect deals
    // damage to each opponent (DealDamage reads the targets that were chosen, and
    // PlayerRel::EachOpponent is a seat set no DealDamage arm reads), and the 0/1 black
    // Wizard is not in the pool's token registry, so the token cannot be created either.
    coverage = Coverage::Partial(
        "Mage Siege: no Effect deals damage to each opponent, and the 0/1 black Wizard token is not in the pool's registry",
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])],
);
