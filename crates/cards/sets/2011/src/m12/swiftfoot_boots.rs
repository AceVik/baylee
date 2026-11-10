//! Swiftfoot Boots — {2} — Artifact — Equipment
//! Oracle: Equipped creature has hexproof and haste. (It can't be the target of spells or abilities your opponents control. It can attack and {T} no matter when it came under your control.)
//! Oracle: Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
//! Set: MBC #77 — Mystery Booster Commander Edition | Scryfall ID: 03f7e9fc-8e59-45c1-90fc-1d04d929b292 | Oracle ID: c8b143ad-43ec-4e0d-a440-e348daa31391
// IMPLEMENTED — Lightning Greaves' shape with hexproof instead of shroud and
// {1} to equip. The difference matters: the engine reads shroud as "nobody
// may target it" and hexproof as "your opponents may not", so the two are
// separate bits rather than one aliased keyword.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::artifact;

card!(
    index = index::SWIFTFOOT_BOOTS,
    oracle_id = "c8b143ad-43ec-4e0d-a440-e348daa31391",
    scryfall_id = "03f7e9fc-8e59-45c1-90fc-1d04d929b292",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Swiftfoot Boots",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[artifact::EQUIPMENT],
    ),],
    abilities = &[
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddKeyword(KeywordSet::HEXPROOF.union(KeywordSet::HASTE))
        ),
        equip!("{1}"),
    ],
);
