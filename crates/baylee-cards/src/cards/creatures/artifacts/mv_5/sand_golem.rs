//! Sand Golem — {5} — Artifact Creature — Golem
//! Oracle: When a spell or ability an opponent controls causes you to discard this card, return this card from your graveyard to the battlefield with a +1/+1 counter on it at the beginning of the next end step.
//! Set: MIR #318 — Mirage | Scryfall ID: 84e4a955-ce9a-4386-b6ac-c00fd25de882 | Oracle ID: 0f9b78ca-1556-4157-a91c-55276fd78127
// PARTIAL: Trigger::DiscardedThisByOpponent records the pre-move hand version without a graveyard departure successor, so the delayed end-step trigger uses ReturnFromGraveyardOf to return a creature card from your graveyard without a +1/+1 counter.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SAND_GOLEM,
    oracle_id = "0f9b78ca-1556-4157-a91c-55276fd78127",
    scryfall_id = "84e4a955-ce9a-4386-b6ac-c00fd25de882",
    coverage = Coverage::Partial(
        "Trigger::DiscardedThisByOpponent records the pre-move hand version without a graveyard departure successor, so the delayed end-step trigger uses ReturnFromGraveyardOf to return a creature card from your graveyard without a +1/+1 counter",
    ),
    faces = &[face!(
        name = "Sand Golem",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::DiscardedThisByOpponent,
        &[Effect::AtNextEndStep {
            effects: &[Effect::ReturnFromGraveyardOf {
                whose: PlayerRel::You,
                filter: &Filter::CREATURE,
            }],
        }],
    )],
);
