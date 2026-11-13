//! Radiant, Archangel — {3}{W}{W} — Legendary Creature — Angel
//! Oracle: Flying, vigilance
//! Oracle: Radiant gets +1/+1 for each other creature on the battlefield with flying.
//! Set: VMA #40 — Vintage Masters | Scryfall ID: 4d976c91-2377-470a-8129-eeb716451252 | Oracle ID: 859cbe11-e1e6-4309-94da-8725db1b3fc2
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const OTHER_FLYING_CREATURE: Filter = Filter::And(&[
    Filter::Another,
    Filter::CREATURE,
    Filter::HasKeyword(KeywordSet::FLYING),
]);

card!(
    index = index::RADIANT_ARCHANGEL,
    oracle_id = "859cbe11-e1e6-4309-94da-8725db1b3fc2",
    scryfall_id = "4d976c91-2377-470a-8129-eeb716451252",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Radiant, Archangel",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::ModifyPTByCount(PtCount::OnBattlefield(&OTHER_FLYING_CREATURE)),
    )],
);
