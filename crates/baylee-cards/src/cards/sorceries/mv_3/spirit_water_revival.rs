//! Spirit Water Revival — {4}{U} — Sorcery
//! Oracle: As an additional cost to cast this spell, you may waterbend {6}. (While paying a waterbend cost, you can tap your artifacts and creatures to help. Each one pays for {1}.)
//! Oracle: Draw two cards. If this spell's additional cost was paid, instead shuffle your graveyard into your library, draw seven cards, and you have no maximum hand size for the rest of the game.
//! Oracle: Exile Spirit Water Revival.
//! Set: TLA #74 — Avatar: The Last Airbender | Scryfall ID: 0c019e76-c88e-4d1b-a546-0f4e462ef44a | Oracle ID: 68979160-b5ce-4787-8a1e-1f40e614c3b0
// IMPLEMENTED — waterbend (kicker-style additional cost paid via
// convoke taps on artifacts AND creatures), kick-branched outcome,
// self-exile always.

static KICKED_OUTCOME: &[Effect] = &[
    Effect::ShuffleGraveyardIntoLibrary,
    Effect::DrawCards {
        amount: Amount::Fixed(7),
    },
    Effect::CreateContinuousEffect {
        layer: Layer::Text,
        filter: &Filter::Any,
        modifier: Modifier::NoMaxHandSize,
        duration: Duration::Indefinitely,
    },
];
static NORMAL_OUTCOME: &[Effect] = &[Effect::DrawCards {
    amount: Amount::Fixed(2),
}];

use baylee_cards_dsl::prelude::*;

card! {
    index: 156,
    oracle_id: "68979160-b5ce-4787-8a1e-1f40e614c3b0",
    scryfall_id: "0c019e76-c88e-4d1b-a546-0f4e462ef44a",
    faces: &[face! {
        name: "Spirit Water Revival",
        mana_cost: baylee_core::mana!("{4}{U}"),
        types: TypeSet::SORCERY,
        additional_costs: &[Cost {
            mana: baylee_core::mana!("{6}"),
            parts: &[],
        }],
        convoke: true,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[
            Effect::IfKicked {
                then: KICKED_OUTCOME,
                otherwise: NORMAL_OUTCOME,
            },
            Effect::ExileSource,
        ])],
}
