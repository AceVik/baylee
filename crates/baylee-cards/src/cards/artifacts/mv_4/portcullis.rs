//! Portcullis — {4} — Artifact
//! Oracle: Whenever a creature enters, if there are two or more other creatures on the battlefield, exile that creature. Return that card to the battlefield under its owner's control when this artifact leaves the battlefield.
//! Set: STH #139 — Stronghold | Scryfall ID: a4de4af2-fa75-4a87-b0e1-0117727917a5 | Oracle ID: 6230c179-0263-4fa4-b0ff-f2cb621de147
// PARTIAL: counting two or more creatures specifically other than the entering creature if it left before resolution and returning the exiled cards when this artifact leaves the battlefield are not supported; checks for at least three creatures on the battlefield and exiles the entering creature.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PORTCULLIS,
    oracle_id = "6230c179-0263-4fa4-b0ff-f2cb621de147",
    scryfall_id = "a4de4af2-fa75-4a87-b0e1-0117727917a5",
    coverage = Coverage::Partial(
        "counting two or more creatures specifically other than the entering creature if it left before resolution and returning the exiled cards when this artifact leaves the battlefield are not supported; checks for at least three creatures on the battlefield and exiles the entering creature",
    ),
    faces = &[face!(
        name = "Portcullis",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::EntersBattlefield(&Filter::CREATURE),
        &[Effect::exile(TargetSpec::EventObject)],
        condition = Some(Condition::BattlefieldCount(&Filter::CREATURE, 3)),
    ),],
);
