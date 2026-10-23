//! Lurking Evil — {B}{B}{B} — Enchantment
//! Oracle: Pay half your life, rounded up: This enchantment becomes a 4/4 Phyrexian Horror creature with flying.
//! Set: USG #140 — Urza's Saga | Scryfall ID: f5ef7201-4443-4a77-b9e1-6f1f39e8f993 | Oracle ID: 81d6b42e-9c41-4d1e-bd28-4662e1fde572
// PARTIAL

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LURKING_EVIL,
    oracle_id = "81d6b42e-9c41-4d1e-bd28-4662e1fde572",
    scryfall_id = "f5ef7201-4443-4a77-b9e1-6f1f39e8f993",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "paying half your life rounded up as an activation cost is not in CostPart; activates for Pay 1 life to become a 4/4 Phyrexian Horror creature with flying indefinitely",
    ),
    faces = &[face!(
        name = "Lurking Evil",
        mana_cost = mana!("{B}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!(PayLife(1)),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::PHYREXIAN,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::AddSubtype(subtypes::creature::HORROR),
                Duration::Indefinitely,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::FLYING),
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(4, 4), Duration::Indefinitely),
        ],
    )],
);
