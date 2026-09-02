//! Everybody Lives! — {1}{W} — Instant
//! Oracle: All creatures gain hexproof and indestructible until end of turn. Players gain hexproof until end of turn. Players can't lose life this turn and players can't lose the game or win the game this turn.
//! Set: WHO #18 — Doctor Who | Scryfall ID: 9dab0052-7f0c-4b56-847f-20552666a271 | Oracle ID: 39213de3-6a4a-4879-a7f9-70f45013765e
// IMPLEMENTED — creature hexproof+indestructible EOT, no-life-loss,
// no-lose/no-win suppression, and player hexproof (ChoosePlayer filters
// hexproofed players out).

static HEXPROOF_INDESTRUCTIBLE: KeywordSet = KeywordSet::HEXPROOF.union(KeywordSet::INDESTRUCTIBLE);

use baylee_cards_dsl::prelude::*;

card! {
    index: 47,
    oracle_id: "39213de3-6a4a-4879-a7f9-70f45013765e",
    scryfall_id: "9dab0052-7f0c-4b56-847f-20552666a271",
    faces: &[face! {
        name: "Everybody Lives!",
        mana_cost: baylee_core::mana!("{1}{W}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[
            Effect::CreateContinuousEffect {
                layer: Layer::Ability,
                filter: &Filter::CREATURE,
                modifier: Modifier::AddKeyword(HEXPROOF_INDESTRUCTIBLE),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::Any,
                modifier: Modifier::CantLoseLife,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::Any,
                modifier: Modifier::PlayersCantLose,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::Any,
                modifier: Modifier::PlayerHexproof,
                duration: Duration::UntilEndOfTurn,
            },
        ])],
}
