//! Drowner of Truth // Drowned Jungle — {5}{G/U}{G/U} — Creature — Eldrazi // Land
//! Oracle: Devoid (This card has no color.)
//! Oracle: When you cast this spell, if {C} was spent to cast it, create two 0/1 colorless Eldrazi Spawn creature tokens with "Sacrifice this token: Add {C}."
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G} or {U}.
//! Set: MH3 #253 — Modern Horizons 3 | Scryfall ID: 7a1d3c1d-1373-4ac4-bb26-9780976efc4f | Oracle ID: db19a27a-ee22-4931-ae3c-0ce21f456ea6
//! Face: Drowner of Truth — {5}{G/U}{G/U} — Creature — Eldrazi
//! Face: Drowned Jungle —  — Land
// PARTIAL — devoid as a colour-removing static and the land face complete
// (enters tapped, taps for {G} or {U}); the cast trigger is not expressible.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana_choice(&[
    ManaColor::Green,
    ManaColor::Blue,
])])];

card!(
    index = index::DROWNER_OF_TRUTH,
    oracle_id = "db19a27a-ee22-4931-ae3c-0ce21f456ea6",
    scryfall_id = "7a1d3c1d-1373-4ac4-bb26-9780976efc4f",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[
        face!(
            name = "Drowner of Truth",
            mana_cost = mana!("{5}{G/U}{G/U}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::ELDRAZI],
            power = Some(7),
            toughness = Some(6),
        ),
        face!(
            name = "Drowned Jungle",
            types = TypeSet::LAND,
            abilities = BACK_MANA,
            enter_modifiers = &[EnterModifier::Tapped],
        ),
    ],
    coverage = Coverage::Partial(
        "the cast trigger's \"if {C} was spent to cast it\" clause — the engine tracks no mana provenance"
    ),
    abilities = &[
        // NOT SUPPORTED: "When you cast this spell, if {C} was spent to cast
        // it, create two 0/1 colorless Eldrazi Spawn creature tokens with
        // 'Sacrifice this token: Add {C}.'" — no trigger reads what a cast
        // was paid with, and no Condition states it.
        static_ability!(Filter::This, Modifier::SetColor(ColorSet::EMPTY)),
    ],
);
