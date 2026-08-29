//! Deck loading: acceptance-suite text format → registry-resolved
//! presets. Used by self-play tests and the local play harness.

use baylee_cards::by_index;
use baylee_core::acceptance::{Zone, parse_decks};
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, FormatId, GamePreset, PrintInfo, SeatController, SeatSpec,
};
use baylee_core::preset::{Finish, HouseRules};

/// A loaded deck: main-deck entries plus commander(s).
#[derive(Clone, Debug)]
pub struct LoadedDeck {
    /// Deck name.
    pub name: String,
    /// Main-deck entries (one per copy).
    pub main: Vec<CardIndex>,
    /// Commander card(s).
    pub commanders: Vec<CardIndex>,
}

/// Resolves a card name to its registry index (linear scan; the
/// acceptance pool is small).
#[must_use]
pub fn by_name(name: &str) -> Option<CardIndex> {
    (0..baylee_cards::count())
        .map(|i| by_index(CardIndex::new(i as u32)))
        .zip(0..baylee_cards::count())
        .find_map(|(def, i)| match def {
            Some(def) if def.name() == name => Some(CardIndex::new(i as u32)),
            _ => None,
        })
}

/// Loads a named deck from the acceptance text.
///
/// # Errors
/// Returns the first unresolvable card name or the parse error.
pub fn load_acceptance(text: &str, deck_name: &str) -> Result<LoadedDeck, String> {
    let rows = parse_decks(text).map_err(|e| e.to_string())?;
    let mut main = Vec::new();
    let mut commanders = Vec::new();
    for row in rows.iter().filter(|r| r.deck == deck_name) {
        let index = by_name(&row.name).ok_or_else(|| format!("unknown card: {}", row.name))?;
        let target = match row.zone {
            Zone::Main | Zone::Sideboard => &mut main,
            Zone::Commander => &mut commanders,
        };
        for _ in 0..row.count {
            target.push(index);
        }
    }
    if main.is_empty() {
        return Err(format!("deck not found: {deck_name}"));
    }
    Ok(LoadedDeck {
        name: deck_name.to_string(),
        main,
        commanders,
    })
}

/// Builds a two-player preset from two loaded decks.
#[must_use]
pub fn preset_for(seed: u64, a: &LoadedDeck, b: &LoadedDeck) -> GamePreset {
    let mk = |deck: &LoadedDeck| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        deck: deck
            .main
            .iter()
            .map(|card| DeckEntry {
                card: *card,
                print: PrintRef::new(0),
            })
            .collect(),
        starting_life: None,
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed,
        dev_mode: false,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![mk(a), mk(b)],
    }
}
