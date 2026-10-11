//! Restless Vents — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B} or {R}.
//! Oracle: {1}{B}{R}: Until end of turn, this land becomes a 2/3 black and red Insect creature with menace. It's still a land.
//! Oracle: Whenever this land attacks, you may discard a card. If you do, draw a card.
//! Set: LCI #284 — The Lost Caverns of Ixalan | Scryfall ID: e628e89b-bee9-408d-bb05-1784fda6b8a1 | Oracle ID: 696e7ddb-bdc7-40ee-bc5c-59e98f4a7401

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RESTLESS_VENTS,
    oracle_id = "696e7ddb-bdc7-40ee-bc5c-59e98f4a7401",
    scryfall_id = "e628e89b-bee9-408d-bb05-1784fda6b8a1",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Restless Vents",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Black, ManaColor::Red])]),
        activated!(
            cost!("{1}{B}{R}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(subtypes::creature::INSECT),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetColor(ColorSet::from_slice(&[Color::Black, Color::Red])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(2, 3),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::MENACE),
                    Duration::UntilEndOfTurn,
                ),
            ],
        ),
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::PlayerMayPayCostThen {
                player: PlayerRel::You,
                cost: &CostPart::Discard(&Filter::Any),
                effect: &Effect::draw(1),
            }]
        ),
    ],
);
