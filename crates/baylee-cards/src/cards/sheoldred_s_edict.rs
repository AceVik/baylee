//! Sheoldred's Edict — {1}{B} — Instant
//! Oracle: Choose one —
//! Oracle: • Each opponent sacrifices a nontoken creature of their choice.
//! Oracle: • Each opponent sacrifices a creature token of their choice.
//! Oracle: • Each opponent sacrifices a planeswalker of their choice.
//! Set: ONE #108 — Phyrexia: All Will Be One | Scryfall ID: a9225cc3-90f0-448f-a8d9-7c6c2796d077 | Oracle ID: 217062f5-96f1-454c-9507-17f34ef37070
// IMPLEMENTED — all three edict modes (per-opponent sacrifice choice).

static NONTOKEN_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::IsToken)]);
static CREATURE_TOKEN: Filter = Filter::And(&[Filter::CREATURE, Filter::IsToken]);

use baylee_cards_dsl::prelude::*;

card! {
    index: 144,
    oracle_id: "217062f5-96f1-454c-9507-17f34ef37070",
    scryfall_id: "a9225cc3-90f0-448f-a8d9-7c6c2796d077",
    faces: &[face! {
        name: "Sheoldred's Edict",
        mana_cost: baylee_core::mana!("{1}{B}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::ModalSpell {
        modes: &[
            mode!(&[Effect::SacrificeFilter {
                    who: PlayerRel::EachOpponent,
                    filter: &NONTOKEN_CREATURE,
                }]),
            mode!(&[Effect::SacrificeFilter {
                    who: PlayerRel::EachOpponent,
                    filter: &CREATURE_TOKEN,
                }]),
            mode!(&[Effect::SacrificeFilter {
                    who: PlayerRel::EachOpponent,
                    filter: &Filter::PLANESWALKER,
                }]),
        ],
    }],
}
