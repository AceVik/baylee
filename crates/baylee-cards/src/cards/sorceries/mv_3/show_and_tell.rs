//! Show and Tell — {2}{U} — Sorcery
//! Oracle: Each player may put an artifact, creature, enchantment, or land card from their hand onto the battlefield.
//! Set: USG #96 — Urza's Saga | Scryfall ID: 4b851c17-55ed-4671-b471-dc7b34944432 | Oracle ID: b83a3ba0-249e-4c39-bbf0-cb005413f7d2
// PARTIAL: each player putting at most one card from their hand onto the battlefield is not in Effect; each player may put artifact, creature, enchantment, or land cards from their hand onto the battlefield repeating until no one does.

use baylee_cards_dsl::prelude::*;

static ARTIFACT_CREATURE_ENCHANTMENT_OR_LAND: Filter = Filter::HasType(
    TypeSet::ARTIFACT
        .union(TypeSet::CREATURE)
        .union(TypeSet::ENCHANTMENT)
        .union(TypeSet::LAND),
);

card!(
    index = index::SHOW_AND_TELL,
    oracle_id = "b83a3ba0-249e-4c39-bbf0-cb005413f7d2",
    scryfall_id = "4b851c17-55ed-4671-b471-dc7b34944432",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "each player putting at most one card from their hand onto the battlefield is not in Effect; each player may put artifact, creature, enchantment, or land cards from their hand onto the battlefield repeating until no one does",
    ),
    faces = &[face!(
        name = "Show and Tell",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::EachPlayerMayPutFromHandOntoBattlefieldRepeatUntilNone {
            filter: &ARTIFACT_CREATURE_ENCHANTMENT_OR_LAND,
        }
    ]),],
);
