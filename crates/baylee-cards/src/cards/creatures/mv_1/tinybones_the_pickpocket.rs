//! Tinybones, the Pickpocket — {B} — Legendary Creature — Skeleton Rogue
//! Oracle: Deathtouch
//! Oracle: Whenever Tinybones deals combat damage to a player, you may cast target nonland permanent card from that player's graveyard, and mana of any type can be spent to cast that spell.
//! Set: OTJ #109 — Outlaws of Thunder Junction | Scryfall ID: 3d3025a2-4a17-4137-ba0b-bd676c6f5f88 | Oracle ID: 7bc4c7e2-6758-4a85-84e7-03ab93981106
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NONLAND_PERMANENT_CARD: Filter = Filter::And(&[
    Filter::HasType(
        TypeSet::ARTIFACT
            .union(TypeSet::BATTLE)
            .union(TypeSet::CREATURE)
            .union(TypeSet::ENCHANTMENT)
            .union(TypeSet::PLANESWALKER),
    ),
    Filter::NONLAND,
]);

card!(
    index = index::TINYBONES_THE_PICKPOCKET,
    oracle_id = "7bc4c7e2-6758-4a85-84e7-03ab93981106",
    scryfall_id = "3d3025a2-4a17-4137-ba0b-bd676c6f5f88",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::DEATHTOUCH,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tinybones, the Pickpocket",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::SKELETON, subtypes::creature::ROGUE],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::DealsCombatDamageToPlayer(&Filter::This),
        &[Effect::MayCastTargetAnyMana],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &NONLAND_PERMANENT_CARD,
            PlayerRel::DamagedPlayer,
        ))),
    )],
);
