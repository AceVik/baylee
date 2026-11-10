//! Bruna, the Fading Light — {5}{W}{W} — Legendary Creature — Angel Horror
//! Oracle: When you cast this spell, you may return target Angel or Human creature card from your graveyard to the battlefield.
//! Oracle: Flying, vigilance
//! Oracle: (Melds with Gisela, the Broken Blade.)
//! Set: EMN #15 — Eldritch Moon | Scryfall ID: 27907985-b5f6-4098-ab43-15a0c2bf94d5 | Oracle ID: bce7e45d-b2c5-43aa-91a6-3a3c24f30850
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static TARGET1: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Or(&[
        Filter::HasSubtype(subtypes::creature::ANGEL),
        Filter::HasSubtype(subtypes::creature::HUMAN),
    ]),
]);

card!(
    index = index::BRUNA_THE_FADING_LIGHT,
    oracle_id = "bce7e45d-b2c5-43aa-91a6-3a3c24f30850",
    scryfall_id = "27907985-b5f6-4098-ab43-15a0c2bf94d5",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Bruna, the Fading Light",
        mana_cost = mana!("{5}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ANGEL, subtypes::creature::HORROR],
        power = Some(5),
        toughness = Some(7),
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&Filter::This),
        &[Effect::MayDoFor {
            player: PlayerRel::You,
            effects: &[Effect::GraveyardToBattlefield {
                target: TargetSpec::CardInGraveyard(&TARGET1, PlayerRel::You),
                owner_control: false,
                counters: None,
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &TARGET1,
            PlayerRel::You
        )))
    )],
);
