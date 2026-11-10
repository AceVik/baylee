//! Renegade Rallier — {1}{G}{W} — Creature — Human Warrior
//! Oracle: Revolt — When this creature enters, if a permanent left the battlefield under your control this turn, return target permanent card with mana value 2 or less from your graveyard to the battlefield.
//! Set: AER #133 — Aether Revolt | Scryfall ID: 90bad312-80e3-45b0-9556-60ce06808a47 | Oracle ID: 6fa07b6c-f01a-4416-b0fc-986b0fc4e412

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PERMANENT_CARD_MV_2: Filter = Filter::And(&[
    Filter::CmcAtMost(2),
    Filter::LacksType(TypeSet::INSTANT.union(TypeSet::SORCERY)),
]);

card!(
    index = index::RENEGADE_RALLIER,
    oracle_id = "6fa07b6c-f01a-4416-b0fc-986b0fc4e412",
    scryfall_id = "90bad312-80e3-45b0-9556-60ce06808a47",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Renegade Rallier",
        mana_cost = mana!("{1}{G}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARRIOR],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::reanimate(TargetSpec::CardInGraveyard(
            &PERMANENT_CARD_MV_2,
            PlayerRel::You
        ))],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &PERMANENT_CARD_MV_2,
            PlayerRel::You,
        ))),
        condition = Some(Condition::Revolt),
    ),],
);
