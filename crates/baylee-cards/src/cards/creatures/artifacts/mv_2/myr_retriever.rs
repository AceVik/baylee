//! Myr Retriever — {2} — Artifact Creature — Myr
//! Oracle: When this creature dies, return another target artifact card from your graveyard to your hand.
//! Set: 2XM #277 — Double Masters | Scryfall ID: 7f0149d4-0731-474a-a1c3-28c25e486c14 | Oracle ID: d07d3be3-f69d-4484-8467-cffd43871788
// IMPLEMENTED — a dies trigger that returns another target artifact card
// from your graveyard to your hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// Named because the trigger says it twice: what may be pointed at, and what
/// comes back. `another` is the card's own word and it is load-bearing here —
/// Myr Retriever is itself an artifact card in that graveyard by the time the
/// trigger is put on the stack.
static ANOTHER_ARTIFACT_CARD: Filter = f!(another ARTIFACT);

card!(
    index = index::MYR_RETRIEVER,
    oracle_id = "d07d3be3-f69d-4484-8467-cffd43871788",
    scryfall_id = "7f0149d4-0731-474a-a1c3-28c25e486c14",
    faces = &[face!(
        name = "Myr Retriever",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::MYR],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::GraveyardToHand {
            target: TargetSpec::CardInGraveyard(&ANOTHER_ARTIFACT_CARD, PlayerRel::You),
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &ANOTHER_ARTIFACT_CARD,
            PlayerRel::You,
        )))
    )],
);

// Engine-level test belongs in baylee-engine (card_tests): a dies trigger
// that needs a target is worth playing once — the Myr lying in the graveyard
// must not be offered to itself, and a graveyard holding no other artifact
// card must drop the trigger rather than ask a question with no answer.
