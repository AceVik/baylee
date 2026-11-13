//! Force of Vigor — {2}{G}{G} — Instant
//! Oracle: If it's not your turn, you may exile a green card from your hand rather than pay this spell's mana cost.
//! Oracle: Destroy up to two target artifacts and/or enchantments.
//! Set: MH1 #164 — Modern Horizons | Scryfall ID: 017c415b-d635-43c6-92b8-8c95d1c4ff8d | Oracle ID: 5f627e45-deee-4a4f-8128-57b0d8ebc553
// IMPLEMENTED — off-turn pitch casting and up to two artifact/enchantment targets.

use baylee_cards_dsl::prelude::*;

static GREEN_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Green]));

card!(
    index = index::FORCE_OF_VIGOR,
    oracle_id = "5f627e45-deee-4a4f-8128-57b0d8ebc553",
    scryfall_id = "017c415b-d635-43c6-92b8-8c95d1c4ff8d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Force of Vigor",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::INSTANT,
        alternative_costs = &[AlternativeCost {
            cost: cost!(ExileFromHand(&GREEN_CARD)),
            condition: AltCondition::NotYourTurn,
        }],
    ),],
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(
            &Filter::ARTIFACT_OR_ENCHANTMENT
        ))],
        targets = Some(TargetReq::up_to(
            TargetSpec::Object(&Filter::ARTIFACT_OR_ENCHANTMENT),
            2
        ))
    )],
);
