//! Gisela, the Broken Blade — {2}{W}{W} — Legendary Creature — Angel Horror
//! Oracle: Flying, first strike, lifelink
//! Oracle: At the beginning of your end step, if you both own and control Gisela and a creature named Bruna, the Fading Light, exile them, then meld them into Brisela, Voice of Nightmares.
//! Set: INR #24 — Innistrad Remastered | Scryfall ID: 04506bad-3856-4184-8dda-941ded60f41a | Oracle ID: f3e23d5e-bd88-4e7c-a3fb-db2a8cb05b22
// IMPLEMENTED

use crate::tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GISELA_THE_BROKEN_BLADE,
    oracle_id = "f3e23d5e-bd88-4e7c-a3fb-db2a8cb05b22",
    scryfall_id = "04506bad-3856-4184-8dda-941ded60f41a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FLYING
        .union(KeywordSet::FIRST_STRIKE)
        .union(KeywordSet::LIFELINK),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gisela, the Broken Blade",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ANGEL, subtypes::creature::HORROR],
        power = Some(4),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::End,
            whose: PlayerRel::You,
        },
        &[Effect::MeldWithNamed {
            source_name: "Gisela, the Broken Blade",
            partner_name: "Bruna, the Fading Light",
            melded: &tokens::BRISELA_VOICE_OF_NIGHTMARES,
        }]
    )],
);
