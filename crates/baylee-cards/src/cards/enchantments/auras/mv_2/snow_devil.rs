//! Snow Devil — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has flying.
//! Oracle: Enchanted creature has first strike as long as it's blocking and you control a snow land.
//! Set: ICE #100 — Ice Age | Scryfall ID: 2be3a9a5-2ac5-4ea4-915d-8cff35c0e72f | Oracle ID: e743b4ec-65bf-4e8d-ad87-6746a2e6ba73
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SNOW_LAND: Filter = Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::SNOW)]);

card!(
    index = index::SNOW_DEVIL,
    oracle_id = "e743b4ec-65bf-4e8d-ad87-6746a2e6ba73",
    scryfall_id = "2be3a9a5-2ac5-4ea4-915d-8cff35c0e72f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Snow Devil",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddKeyword(KeywordSet::FLYING)
        ),
        static_ability!(
            Filter::And(&[Filter::AttachedToBySource, Filter::Blocking]),
            Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
            condition = Some(Condition::ControlCount(&SNOW_LAND, 1)),
        ),
    ],
);
