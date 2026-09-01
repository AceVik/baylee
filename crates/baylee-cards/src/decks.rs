//! Deck loading: acceptance-suite text format → registry-resolved
//! presets. Used by self-play tests and the local play harness.

use crate::by_index;
use baylee_core::acceptance::{Zone, parse_decks};
use baylee_core::deckrow::PrintChoice;
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, FormatId, GamePreset, PrintInfo, SeatController, SeatSpec,
};
use baylee_core::preset::{Finish, HouseRules};

/// One copy of one card in a deck, with the printing its owner chose.
///
/// Rules identity and print identity travel together because a deck list is
/// where they are decided together and nowhere else: the engine takes the
/// first and never reads the second, and the client takes the second and
/// cannot derive it from the first. Two copies of the same card with
/// different finishes are two `DeckCard`s and one `CardIndex`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeckCard {
    /// Rules identity.
    pub index: CardIndex,
    /// Which physical printing this copy is.
    pub print: PrintInfo,
}

impl DeckCard {
    /// A copy printed as the registry's reference printing: English, non-foil.
    #[must_use]
    pub fn plain(index: CardIndex) -> Self {
        Self {
            index,
            print: reference_print(index),
        }
    }

    /// A copy printed as a deck row asked for, falling back field by field to
    /// the registry's reference printing.
    #[must_use]
    pub fn chosen(index: CardIndex, choice: &PrintChoice) -> Self {
        let mut print = reference_print(index);
        // Only the id names a *printing*; set and collector number narrow
        // towards one, and resolving them needs a catalog this crate does not
        // have. They are carried in the deck row and resolved by whoever
        // stores it, so by the time a deck is loaded either an id is set or
        // the reference printing is the answer.
        if let Some(id) = &choice.scryfall_id
            && let Ok(id) = uuid::Uuid::parse_str(id)
        {
            print.scryfall_id = id;
        }
        if let Some(lang) = &choice.lang {
            print.lang.clone_from(lang);
        }
        if let Some(finish) = choice.finish {
            print.finish = finish;
        }
        Self { index, print }
    }
}

/// The registry's own printing of a card: what a deck that named none gets.
#[must_use]
pub fn reference_print(card: CardIndex) -> PrintInfo {
    let id = by_index(card).map_or_else(uuid::Uuid::nil, |def| {
        uuid::Uuid::parse_str(def.scryfall_id).unwrap_or_default()
    });
    PrintInfo {
        scryfall_id: id,
        lang: "en".to_string(),
        finish: Finish::Normal,
    }
}

/// A loaded deck: main-deck entries plus commander(s).
#[derive(Clone, Debug)]
pub struct LoadedDeck {
    /// Deck name.
    pub name: String,
    /// Main-deck entries (one per copy).
    pub main: Vec<DeckCard>,
    /// Sideboard entries (one per copy). Reachable by wishes, never
    /// shuffled into the library — folding these into `main` quietly
    /// turned a 60-card deck with a 15-card sideboard into 75 cards.
    pub sideboard: Vec<DeckCard>,
    /// Commander card(s).
    pub commanders: Vec<DeckCard>,
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
            target.push(DeckCard::plain(index));
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

/// The print-table slot for one printing, deduplicated.
///
/// Deduplicated on the *whole* printing rather than on the id: a foil and a
/// non-foil copy of the same card share an image and must not share a slot,
/// because the slot is where the finish is written and one of the two would
/// come out wearing the other's.
fn print_ref_for(prints: &mut Vec<PrintInfo>, print: &PrintInfo) -> PrintRef {
    if let Some(pos) = prints.iter().position(|p| p == print) {
        return PrintRef::new(pos as u16);
    }
    prints.push(print.clone());
    PrintRef::new((prints.len() - 1) as u16)
}

/// Builds a two-player preset from two loaded decks.
#[must_use]
pub fn preset_for(seed: u64, a: &LoadedDeck, b: &LoadedDeck) -> GamePreset {
    let mut prints: Vec<PrintInfo> = Vec::new();
    // One closure for both lists: the print table is shared, so two closures
    // holding `prints` would each want it mutably.
    let mut entries = |cards: &[DeckCard]| -> Vec<DeckEntry> {
        cards
            .iter()
            .map(|card| DeckEntry {
                card: card.index,
                print: print_ref_for(&mut prints, &card.print),
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

/// A preset for a table of any size, one seat per deck.
///
/// [`preset_for`] is the two-deck case and the one most callers want; a room
/// where the host chose how many chairs there are needs this. The print table
/// is still shared and still deduplicated across every deck at the table —
/// which is exactly why this cannot be a fold over `preset_for`.
///
/// Every seat comes out as an AI, as in the two-deck case: who actually sits
/// where is the caller's business, and the gateway overwrites the controller
/// per seat before the engine ever sees it.
#[must_use]
pub fn preset_for_all(seed: u64, decks: &[&LoadedDeck]) -> GamePreset {
    let mut prints: Vec<PrintInfo> = Vec::new();
    let mut entries = |cards: &[DeckCard]| -> Vec<DeckEntry> {
        cards
            .iter()
            .map(|card| DeckEntry {
                card: card.index,
                print: print_ref_for(&mut prints, &card.print),
            })
            .collect()
    };
    let seats = decks
        .iter()
        .map(|deck| SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: baylee_core::preset::SeatCapabilities::default(),
            deck: entries(&deck.main),
            sideboard: entries(&deck.sideboard),
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        })
        .collect();
    GamePreset {
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints,
        seats,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some_card() -> CardIndex {
        by_name("Forest").expect("the registry has Forest")
    }

    #[test]
    fn a_deck_that_names_no_printing_gets_the_registrys_own() {
        let card = some_card();
        let plain = DeckCard::plain(card);
        assert_eq!(plain.print, reference_print(card));
        assert_eq!(plain.print.finish, Finish::Normal);
        assert_eq!(plain.print.lang, "en");
        assert!(
            !plain.print.scryfall_id.is_nil(),
            "a nil id is one guaranteed 404 per card"
        );
    }

    #[test]
    fn a_row_that_names_a_printing_gets_that_one() {
        let card = some_card();
        let chosen = DeckCard::chosen(
            card,
            &PrintChoice {
                lang: Some("de".to_string()),
                finish: Some(Finish::Etched),
                scryfall_id: Some("11111111-2222-3333-4444-555555555555".to_string()),
                ..PrintChoice::default()
            },
        );
        assert_eq!(chosen.index, card, "the rules identity is untouched");
        assert_eq!(chosen.print.lang, "de");
        assert_eq!(chosen.print.finish, Finish::Etched);
        assert_eq!(
            chosen.print.scryfall_id.to_string(),
            "11111111-2222-3333-4444-555555555555"
        );
    }

    /// A choice narrows field by field: naming only the finish keeps the
    /// reference printing's id and language.
    #[test]
    fn an_unstated_field_falls_back_to_the_reference_printing() {
        let card = some_card();
        let foil = DeckCard::chosen(
            card,
            &PrintChoice {
                finish: Some(Finish::Foil),
                ..PrintChoice::default()
            },
        );
        assert_eq!(foil.print.scryfall_id, reference_print(card).scryfall_id);
        assert_eq!(foil.print.lang, "en");
        assert_eq!(foil.print.finish, Finish::Foil);
    }

    /// A garbled id is ignored rather than fatal: the deck still plays, with
    /// the reference art. Refusing to start a game over an image would be the
    /// wrong trade every time.
    #[test]
    fn an_unreadable_printing_id_falls_back_rather_than_failing() {
        let card = some_card();
        let broken = DeckCard::chosen(
            card,
            &PrintChoice {
                scryfall_id: Some("not-a-uuid".to_string()),
                ..PrintChoice::default()
            },
        );
        assert_eq!(broken.print.scryfall_id, reference_print(card).scryfall_id);
    }

    /// The print table is deduplicated, and the finish is part of what makes
    /// two printings different: a foil and a non-foil copy sharing a slot
    /// would put one of them in the other's finish.
    #[test]
    fn a_foil_and_a_plain_copy_do_not_share_a_print_slot() {
        let card = some_card();
        let plain = DeckCard::plain(card);
        let foil = DeckCard::chosen(
            card,
            &PrintChoice {
                finish: Some(Finish::Foil),
                ..PrintChoice::default()
            },
        );
        let deck = LoadedDeck {
            name: "mixed".to_string(),
            main: vec![plain.clone(), foil.clone(), plain.clone()],
            sideboard: vec![],
            commanders: vec![],
        };
        let preset = preset_for(1, &deck, &deck);
        assert_eq!(
            preset.prints.len(),
            2,
            "two printings, three copies: {:?}",
            preset.prints
        );
        let refs: Vec<_> = preset.seats[0].deck.iter().map(|e| e.print).collect();
        assert_eq!(refs[0], refs[2], "the two plain copies share a slot");
        assert_ne!(refs[0], refs[1], "the foil does not");
        assert_eq!(
            preset.prints[refs[1].get() as usize].finish,
            Finish::Foil,
            "and its slot is the one carrying the foil"
        );
    }

    /// The acceptance decks name no printings, so every card in them lands on
    /// the registry's own — which is what makes their art load at all.
    #[test]
    fn the_acceptance_decks_load_with_real_printings() {
        let text = include_str!("../../../data/acceptance-decks.txt");
        let deck = load_acceptance(text, "Allytifact").expect("Allytifact loads");
        assert!(!deck.main.is_empty());
        for card in &deck.main {
            assert!(
                !card.print.scryfall_id.is_nil(),
                "{:?} has no printing",
                card.index
            );
            assert_eq!(card.print.finish, Finish::Normal);
        }
    }
}
