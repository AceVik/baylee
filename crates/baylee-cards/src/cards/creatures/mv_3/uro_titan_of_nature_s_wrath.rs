//! Uro, Titan of Nature's Wrath — {1}{G}{U} — Legendary Creature — Elder Giant
//! Oracle: When Uro enters, sacrifice it unless it escaped.
//! Oracle: Whenever Uro enters or attacks, you gain 3 life and draw a card, then you may put a land card from your hand onto the battlefield.
//! Oracle: Escape—{G}{G}{U}{U}, Exile five other cards from your graveyard. (You may cast this card from your graveyard for its escape cost.)
//! Set: THB #229 — Theros Beyond Death | Scryfall ID: a0b6a71e-56cb-4d25-8f2b-7a4f1b60900d | Oracle ID: ee302659-59ed-4eef-babe-451b9ccf7f14
// IMPLEMENTED — escape from the graveyard for {G}{G}{U}{U} and five other
// cards (`FaceDef.escape`, CR 702.138a); the enters trigger sacrifices it
// unless it escaped (`Condition::Escaped`, CR 702.138b); and the
// enters-or-attacks trigger gains 3, draws, then may put a land card from
// hand onto the battlefield (no land drop spent, CR 305.4).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SACRIFICE_UNLESS_ESCAPED: &[Effect] = &[Effect::IfCondition {
    condition: Condition::Escaped,
    then: &[],
    otherwise: &[Effect::SacrificeSelf],
}];

static GROW: &[Effect] = &[
    Effect::gain_life(3),
    Effect::draw(1),
    Effect::PutFromHandOntoBattlefield {
        filter: &Filter::LAND,
        mana_value: None,
        optional: true,
    },
];

card!(
    index = index::URO_TITAN_OF_NATURE_S_WRATH,
    oracle_id = "ee302659-59ed-4eef-babe-451b9ccf7f14",
    scryfall_id = "a0b6a71e-56cb-4d25-8f2b-7a4f1b60900d",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Uro, Titan of Nature's Wrath",
        mana_cost = mana!("{1}{G}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELDER, subtypes::creature::GIANT],
        power = Some(6),
        toughness = Some(6),
        escape = Some(Escape {
            cost: mana!("{G}{G}{U}{U}"),
            exile: 5,
        }),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(Trigger::ETB, SACRIFICE_UNLESS_ESCAPED),
        triggered!(Trigger::ETB, GROW),
        triggered!(Trigger::Attacks(&Filter::This), GROW),
    ],
);
