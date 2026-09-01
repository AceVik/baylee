//! Deck loading: acceptance-suite text format → registry-resolved
//! presets. Used by self-play tests and the local play harness.

use crate::by_index;
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
    /// Sideboard entries (one per copy). Reachable by wishes, never
    /// shuffled into the library — folding these into `main` quietly
    /// turned a 60-card deck with a 15-card sideboard into 75 cards.
    pub sideboard: Vec<CardIndex>,
    /// Commander card(s).
    pub commanders: Vec<CardIndex>,
}

/// Resolves a card name to its registry index (linear scan; the
/// acceptance pool is small).
#[must_use]
pub fn by_name(name: &str) -> Option<CardIndex> {
    (0..crate::count())
        .map(|i| by_index(CardIndex::new(i as u32)))
        .zip(0..crate::count())
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
    let mut sideboard = Vec::new();
    let mut commanders = Vec::new();
    for row in rows.iter().filter(|r| r.deck == deck_name) {
        let index = by_name(&row.name).ok_or_else(|| format!("unknown card: {}", row.name))?;
        let target = match row.zone {
            Zone::Main => &mut main,
            Zone::Sideboard => &mut sideboard,
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
        sideboard,
        commanders,
    })
}

/// The print-table entry for a card: the registry's reference printing,
/// deduplicated. Clients key artwork off this — a table of nil UUIDs (the
/// previous placeholder) is why no card image ever loaded.
fn print_ref_for(prints: &mut Vec<PrintInfo>, card: CardIndex) -> PrintRef {
    let id = by_index(card).map_or_else(uuid::Uuid::nil, |def| {
        uuid::Uuid::parse_str(def.scryfall_id).unwrap_or_default()
    });
    if let Some(pos) = prints.iter().position(|p| p.scryfall_id == id) {
        return PrintRef::new(pos as u16);
    }
    prints.push(PrintInfo {
        scryfall_id: id,
        lang: "EN".into(),
        finish: Finish::Normal,
    });
    PrintRef::new((prints.len() - 1) as u16)
}

/// Builds a two-player preset from two loaded decks.
#[must_use]
pub fn preset_for(seed: u64, a: &LoadedDeck, b: &LoadedDeck) -> GamePreset {
    let mut prints: Vec<PrintInfo> = Vec::new();
    // One closure for both lists: the print table is shared, so two closures
    // holding `prints` would each want it mutably.
    let mut entries = |cards: &[CardIndex]| -> Vec<DeckEntry> {
        cards
            .iter()
            .map(|card| DeckEntry {
                card: *card,
                print: print_ref_for(&mut prints, *card),
            })
            .collect()
    };
    let seat = |entries: Vec<DeckEntry>, side: Vec<DeckEntry>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: entries,
        sideboard: side,
        starting_life: None,
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    let seats = vec![
        seat(entries(&a.main), entries(&a.sideboard)),
        seat(entries(&b.main), entries(&b.sideboard)),
    ];
    GamePreset {
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints,
        seats,
    }
}
