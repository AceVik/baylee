//! Taniwha — {3}{U}{U} — Legendary Creature — Serpent
//! Oracle: Trample
//! Oracle: Phasing (This phases in or out before you untap during each of your untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Oracle: At the beginning of your upkeep, all lands you control phase out. (They phase in before you untap during your next untap step.)
//! Set: MIR #95 — Mirage | Scryfall ID: 72d12315-4220-470d-9628-b9a3ea904ca7 | Oracle ID: 371c993a-69b8-4a51-af89-982b79940b2d
// IMPLEMENTED — trample, phasing, and upkeep trigger phasing out all lands you control.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TANIWHA,
    oracle_id = "371c993a-69b8-4a51-af89-982b79940b2d",
    scryfall_id = "72d12315-4220-470d-9628-b9a3ea904ca7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::TRAMPLE.union(KeywordSet::PHASING),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Taniwha",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::SERPENT],
        power = Some(7),
        toughness = Some(7),
        keywords = KeywordSet::TRAMPLE.union(KeywordSet::PHASING),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::PhaseOutFilter(&Filter::YOUR_LAND)],
    )],
);
