//! Dauthi Warlord — {1}{B} — Creature — Dauthi Soldier
//! Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.)
//! Oracle: Dauthi Warlord's power is equal to the number of creatures on the battlefield with shadow.
//! Set: TPR #98 — Tempest Remastered | Scryfall ID: d7679a92-52a1-4b8f-99e6-564ac14b0cce | Oracle ID: 42bb6537-fa54-43b7-84f2-dea41adc35fd
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CREATURE_WITH_SHADOW: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::SHADOW)]);

card!(
    index = index::DAUTHI_WARLORD,
    oracle_id = "42bb6537-fa54-43b7-84f2-dea41adc35fd",
    scryfall_id = "d7679a92-52a1-4b8f-99e6-564ac14b0cce",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::SHADOW,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dauthi Warlord",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DAUTHI, subtypes::creature::SOLDIER],
        power = Some(0),
        toughness = Some(1),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CharacteristicPTSeparate {
            power: PtCount::OnBattlefield(&CREATURE_WITH_SHADOW),
            toughness: PtCount::Fixed(1),
        }
    )],
);
