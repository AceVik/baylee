//! Sand Golem — {5} — Artifact Creature — Golem
//! Oracle: When a spell or ability an opponent controls causes you to discard this card, return this card from your graveyard to the battlefield with a +1/+1 counter on it at the beginning of the next end step.
//! Set: MIR #318 — Mirage | Scryfall ID: 84e4a955-ce9a-4386-b6ac-c00fd25de882 | Oracle ID: 0f9b78ca-1556-4157-a91c-55276fd78127
// PARTIAL: the Sand Golem card that was discarded is returned by name (a Sand Golem card in your graveyard, chosen as the trigger resolves; Trigger::DiscardedThisByOpponent records only the pre-move hand version, so the exact card cannot be followed) and it comes back without the +1/+1 counter, a 3/3 instead of a 4/4

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SAND_GOLEM,
    oracle_id = "0f9b78ca-1556-4157-a91c-55276fd78127",
    scryfall_id = "84e4a955-ce9a-4386-b6ac-c00fd25de882",
    coverage = Coverage::Partial(
        "the Sand Golem card that was discarded is returned by name (a Sand Golem card in your graveyard, chosen as the trigger resolves; Trigger::DiscardedThisByOpponent records only the pre-move hand version, so the exact card cannot be followed) and it comes back without the +1/+1 counter, a 3/3 instead of a 4/4",
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
                filter: &Filter::Named("Sand Golem"),
            }],
        }],
    )],
);
