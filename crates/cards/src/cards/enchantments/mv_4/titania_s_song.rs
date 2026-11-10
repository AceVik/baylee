//! Titania's Song — {3}{G} — Enchantment
//! Oracle: Each noncreature artifact loses all abilities and becomes an artifact creature with power and toughness each equal to its mana value. If this enchantment leaves the battlefield, this effect continues until end of turn.
//! Set: ME4 #170 — Masters Edition IV | Scryfall ID: 4bb135e7-f9e3-4abc-8374-180316498fc3 | Oracle ID: 30ac0f06-3dd3-4827-8136-4cf8adbf9b12

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TITANIA_S_SONG,
    oracle_id = "30ac0f06-3dd3-4827-8136-4cf8adbf9b12",
    scryfall_id = "4bb135e7-f9e3-4abc-8374-180316498fc3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Titania's Song",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::ARTIFACT,
        Modifier::AnimateNoncreatureArtifactWithoutAbilitiesUntilEndOfTurnAfterDeparture
    ),],
);
