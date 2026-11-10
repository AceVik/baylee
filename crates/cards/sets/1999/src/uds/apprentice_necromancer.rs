//! Apprentice Necromancer — {1}{B} — Creature — Zombie Wizard
//! Oracle: {B}, {T}, Sacrifice this creature: Return target creature card from your graveyard to the battlefield. That creature gains haste. At the beginning of the next end step, sacrifice it.
//! Set: UMA #84 — Ultimate Masters | Scryfall ID: 18d86ad3-e0a1-43da-9dc1-a9028b719c99 | Oracle ID: d1a6aa5f-2ad6-4bc7-82f0-4262f66af730
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::APPRENTICE_NECROMANCER,
    oracle_id = "d1a6aa5f-2ad6-4bc7-82f0-4262f66af730",
    scryfall_id = "18d86ad3-e0a1-43da-9dc1-a9028b719c99",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Apprentice Necromancer",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{B}", TapSelf, SacrificeSelf),
        &[
            Effect::reanimate(TargetSpec::CardInGraveyard(
                &Filter::CREATURE,
                PlayerRel::You,
            )),
            Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::HASTE),
                Duration::Indefinitely,
            ),
            Effect::AtNextEndStep {
                effects: &[Effect::SacrificeEvent],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        ))),
    )],
);
