//! Unstable Shapeshifter — {3}{U} — Creature — Shapeshifter
//! Oracle: Whenever another creature enters, this creature becomes a copy of that creature, except it has this ability.
//! Set: TMP #100 — Tempest | Scryfall ID: 84e8cbd4-f49d-420d-a027-3be64ca58989 | Oracle ID: 6a71deb3-6659-47ea-8687-136cf75fa381

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ANOTHER_CREATURE: Filter = Filter::And(&[Filter::Another, Filter::CREATURE]);

card!(
    index = index::UNSTABLE_SHAPESHIFTER,
    oracle_id = "6a71deb3-6659-47ea-8687-136cf75fa381",
    scryfall_id = "84e8cbd4-f49d-420d-a027-3be64ca58989",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Unstable Shapeshifter",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(0),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::EntersBattlefield(&ANOTHER_CREATURE),
        &[Effect::BecomeCopyOfTarget {
            until_eot: false,
            mods: &[CopyMod::KeepResolvingAbility],
        }],
        targets = Some(TargetReq::one(TargetSpec::EventObject)),
    )],
);
