//! Honest mana sources for short starters. Conditional creature-only storage
//! lands must not masquerade as unconditional five-colour fixing.
use baylee_cards::deckgen::Pool;
use baylee_core::color::{Color, ColorSet};
use std::collections::BTreeMap;

/// The general builder counts basic-search lands as fixing before allocating
/// basics. A short run must retain a real target for each commander's colour,
/// even when its current spell curve has very few pips of that colour.
pub(super) fn retain_fetch_targets(
    main: &mut [baylee_core::ids::CardIndex],
    identity: ColorSet,
) -> Result<(), super::Error> {
    let basics = baylee_cards::decks::basic_lands();
    for minimum in [1_usize, 2] {
        for color in identity.iter() {
            let basic = basics[color as usize].ok_or(super::Error::Formation)?;
            let have = main.iter().filter(|entry| **entry == basic).count();
            if have >= minimum {
                continue;
            }
            let min_donor = if minimum == 1 { 2 } else { 4 };
            let donor = basics
                .iter()
                .flatten()
                .copied()
                .map(|card| (main.iter().filter(|entry| **entry == card).count(), card))
                .filter(|(count, _)| *count >= min_donor)
                .max()
                .map(|(_, card)| card);
            let Some(donor) = donor else {
                if minimum == 1 {
                    return Err(super::Error::Formation);
                }
                continue;
            };
            let slot = main
                .iter_mut()
                .find(|entry| **entry == donor)
                .ok_or(super::Error::Formation)?;
            *slot = basic;
        }
    }
    Ok(())
}

const FIXING: &[(&str, &str)] = &[
    ("Tundra", "WU"),
    ("Underground Sea", "UB"),
    ("Badlands", "BR"),
    ("Taiga", "RG"),
    ("Savannah", "WG"),
    ("Scrubland", "WB"),
    ("Volcanic Island", "UR"),
    ("Bayou", "BG"),
    ("Plateau", "WR"),
    ("Tropical Island", "UG"),
    ("Hallowed Fountain", "WU"),
    ("Watery Grave", "UB"),
    ("Blood Crypt", "BR"),
    ("Stomping Ground", "RG"),
    ("Temple Garden", "WG"),
    ("Godless Shrine", "WB"),
    ("Steam Vents", "UR"),
    ("Overgrown Tomb", "BG"),
    ("Sacred Foundry", "WR"),
    ("Breeding Pool", "UG"),
    ("Adarkar Wastes", "WU"),
    ("Underground River", "UB"),
    ("Sulfurous Springs", "BR"),
    ("Karplusan Forest", "RG"),
    ("Brushland", "WG"),
    ("Caves of Koilos", "WB"),
    ("Shivan Reef", "UR"),
    ("Llanowar Wastes", "BG"),
    ("Battlefield Forge", "WR"),
    ("Yavimaya Coast", "UG"),
    ("Glacial Fortress", "WU"),
    ("Drowned Catacomb", "UB"),
    ("Dragonskull Summit", "BR"),
    ("Rootbound Crag", "RG"),
    ("Sunpetal Grove", "WG"),
    ("Isolated Chapel", "WB"),
    ("Sulfur Falls", "UR"),
    ("Woodland Cemetery", "BG"),
    ("Clifftop Retreat", "WR"),
    ("Hinterland Harbor", "UG"),
    ("Flooded Strand", "WU"),
    ("Polluted Delta", "UB"),
    ("Bloodstained Mire", "BR"),
    ("Wooded Foothills", "RG"),
    ("Windswept Heath", "WG"),
    ("Marsh Flats", "WB"),
    ("Scalding Tarn", "UR"),
    ("Verdant Catacombs", "BG"),
    ("Arid Mesa", "WR"),
    ("Misty Rainforest", "UG"),
    ("Command Tower", "WUBRG"),
    ("City of Brass", "WUBRG"),
    ("Mana Confluence", "WUBRG"),
    ("Evolving Wilds", "WUBRG"),
    ("Terramorphic Expanse", "WUBRG"),
    ("Fabled Passage", "WUBRG"),
    ("Bojuka Bog", "B"),
    ("Halimar Depths", "U"),
    ("Llanowar Reborn", "G"),
    ("Windbrisk Heights", "W"),
    ("Spinerock Knoll", "R"),
    ("Reliquary Tower", ""),
    ("Rogue's Passage", ""),
    ("War Room", ""),
];

pub(super) fn prepare(pool: &mut Pool) {
    let fixing: BTreeMap<_, _> = FIXING
        .iter()
        .filter_map(|(name, colors)| baylee_cards::decks::by_name(name).map(|card| (card, *colors)))
        .collect();
    pool.entries.retain_mut(|entry| {
        if !entry.land {
            return true;
        }
        if entry.basic {
            // The run supplies exactly the five ordinary basic-land identities.
            return baylee_cards::by_index(entry.card).is_some_and(|card| {
                matches!(
                    card.name(),
                    "Plains" | "Island" | "Swamp" | "Mountain" | "Forest"
                )
            });
        }
        let Some(colors) = fixing.get(&entry.card) else {
            return false;
        };
        entry.profile.produced_colors = colors
            .chars()
            .filter_map(Color::from_symbol)
            .fold(ColorSet::EMPTY, |set, color| set.union(ColorSet::of(color)));
        true
    });
}
