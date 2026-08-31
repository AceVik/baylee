//! Card text, fetched from the gateway once per game and kept on disk.
//!
//! # Why the gateway and not the binary
//!
//! Rules text is the one part of a card that is neither rules data nor art: it
//! changes with every oracle update, it exists in a dozen languages, and it is
//! far too large to compile into a client. It travels the same road as card
//! images — Scryfall to the gateway, gateway to the client — and is cached at
//! both ends, so a game costs one request and a second launch costs none.
//!
//! # Why a whole game in one request
//!
//! The print table is sent once, when the client attaches, and it names every
//! card that can appear in the game. Asking for all of it at that moment means
//! the text is there before the first card is drawn, instead of a request per
//! card arriving during play.

use baylee_client_core::card_face::{CardText, CardTextEntry};
use baylee_core::ids::PrintRef;
use baylee_view::GameStatic;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::sync::{Arc, Mutex};

/// Where the fetch callback leaves its answer.
///
/// A channel would be the obvious choice, but `Receiver` is not `Sync` and a
/// Bevy resource must be — and there is only ever one answer, so a slot is
/// both smaller and enough.
type Slot = Arc<Mutex<Option<Vec<CardTextEntry>>>>;

/// Card text for the current game.
#[derive(Resource, Default)]
pub struct CardTexts {
    /// Text by printing, in the language actually served.
    by_print: HashMap<PrintRef, CardTextEntry>,
    /// What the fetch is doing.
    state: Fetch,
    /// The language everything here was fetched for; a change re-fetches.
    lang: String,
}

/// State of the one in-flight request.
#[derive(Default)]
enum Fetch {
    /// Nothing requested yet.
    #[default]
    Idle,
    /// A request is out; the slot receives the decoded answer.
    Waiting(Slot),
    /// Finished, successfully or not. Either way the client stops asking:
    /// a gateway that is not there will not appear mid-game, and a retry
    /// loop against a dead endpoint costs a frame every time.
    Settled,
}

impl CardTexts {
    /// The text for a printing's face, if it has arrived.
    #[must_use]
    pub fn get(&self, print: PrintRef, face: u8) -> Option<CardText> {
        self.by_print.get(&print)?.face(face as usize)
    }

    /// Whether any text is available at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_print.is_empty()
    }

    /// How many printings have text.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_print.len()
    }

    /// Files entries against the print table.
    ///
    /// The catalog answers by Scryfall id; the renderer asks by [`PrintRef`].
    /// This is where the two meet, and an entry for a printing the game does
    /// not contain is dropped rather than kept for a game that will never ask.
    fn absorb(&mut self, statics: &GameStatic, entries: Vec<CardTextEntry>) {
        for entry in entries {
            let found = statics
                .prints
                .iter()
                .position(|p| p.scryfall_id.eq_ignore_ascii_case(&entry.scryfall_id));
            if let Some(index) = found {
                self.by_print.insert(PrintRef::new(index as u16), entry);
            }
        }
    }
}

/// Starts the fetch once the print table is known.
pub fn request(
    mut texts: ResMut<CardTexts>,
    duel: Res<crate::Duel>,
    settings: Res<crate::settings::ClientSettings>,
) {
    let Some(statics) = duel.statics.as_ref() else {
        return;
    };
    // A language change invalidates everything; the simplest correct answer is
    // to ask again rather than to translate what is already here.
    if !matches!(texts.state, Fetch::Idle) && texts.lang == settings.lang {
        return;
    }
    if texts.lang != settings.lang {
        texts.by_print.clear();
        texts.state = Fetch::Idle;
    }
    if !matches!(texts.state, Fetch::Idle) {
        return;
    }
    texts.lang.clone_from(&settings.lang);

    // Whatever a previous session stored is usable immediately, and covers
    // the whole game when nothing changed — the request that follows only
    // has to fill gaps.
    let cached = cache::load(&settings.lang);
    if !cached.is_empty() {
        texts.absorb(statics, cached);
    }

    let ids: Vec<&str> = statics
        .prints
        .iter()
        .map(|p| p.scryfall_id.as_str())
        .collect();
    if ids.is_empty() {
        texts.state = Fetch::Settled;
        return;
    }
    let url = format!(
        "{}/catalog/text?lang={}&ids={}",
        crate::settings::gateway_url(),
        settings.lang,
        ids.join(",")
    );
    let slot: Slot = Arc::default();
    let target = Arc::clone(&slot);
    let lang = settings.lang.clone();
    ehttp::fetch(ehttp::Request::get(&url), move |result| {
        let entries = match result {
            Ok(response) if response.ok => response
                .text()
                .and_then(|body| serde_json::from_str::<Vec<CardTextEntry>>(body).ok())
                .unwrap_or_default(),
            Ok(response) => {
                bevy::log::warn!(status = response.status, "card text request refused");
                Vec::new()
            }
            Err(err) => {
                // Not an error worth interrupting a game for: every card still
                // renders, just without rules text.
                bevy::log::info!("card text unavailable: {err}");
                Vec::new()
            }
        };
        if !entries.is_empty() {
            cache::store(&lang, &entries);
        }
        if let Ok(mut slot) = target.lock() {
            *slot = Some(entries);
        }
    });
    texts.state = Fetch::Waiting(slot);
    bevy::log::info!(cards = ids.len(), "requesting card text");
}

/// Files the answer when it arrives.
pub fn poll(mut texts: ResMut<CardTexts>, duel: Res<crate::Duel>) {
    let Fetch::Waiting(slot) = &texts.state else {
        return;
    };
    let Some(entries) = slot.lock().ok().and_then(|mut s| s.take()) else {
        return;
    };
    texts.state = Fetch::Settled;
    if let Some(statics) = duel.statics.as_ref() {
        texts.absorb(statics, entries);
        bevy::log::info!(printings = texts.len(), "card text ready");
    }
}

/// The on-disk half: what was fetched once does not need fetching again.
///
/// Card text changes only when an oracle update lands, so a stale entry is a
/// cosmetic problem for a few days at worst, while a cold start with no
/// network is a game that cannot be read at all. The cache therefore has no
/// expiry — a fresh fetch overwrites it whenever one succeeds.
mod cache {
    use baylee_client_core::card_face::CardTextEntry;

    /// Reads the cached entries for a language.
    pub fn load(lang: &str) -> Vec<CardTextEntry> {
        read(lang)
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Replaces the cache for a language.
    pub fn store(lang: &str, entries: &[CardTextEntry]) {
        if let Ok(text) = serde_json::to_string(entries) {
            write(lang, &text);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn path(lang: &str) -> Option<std::path::PathBuf> {
        // Same directory as the settings file; the language is part of the
        // name so switching back and forth costs no re-fetch.
        let base = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|v| !v.is_empty())
            .map_or_else(
                || std::env::var("HOME").ok().map(|h| format!("{h}/.config")),
                Some,
            )?;
        let lang: String = lang
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        Some(std::path::PathBuf::from(base).join(format!("baylee/card-text-{lang}.json")))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn read(lang: &str) -> Option<String> {
        std::fs::read_to_string(path(lang)?).ok()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn write(lang: &str, text: &str) {
        let Some(path) = path(lang) else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, text);
    }

    #[cfg(target_arch = "wasm32")]
    fn read(lang: &str) -> Option<String> {
        storage()?
            .get_item(&format!("baylee:card-text:{lang}"))
            .ok()
            .flatten()
    }

    #[cfg(target_arch = "wasm32")]
    fn write(lang: &str, text: &str) {
        // A browser caps localStorage at a few megabytes and throws when it is
        // full. A game's text is far below that, but a player who has played
        // in several languages could get there, and losing the cache is not
        // worth an exception.
        if let Some(storage) = storage() {
            let _ = storage.set_item(&format!("baylee:card-text:{lang}"), text);
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::card_face::FaceText;
    use baylee_view::{Finish, PrintEntry, SeatIdentity};

    fn statics(ids: &[&str]) -> GameStatic {
        GameStatic {
            view_version: baylee_view::VIEW_VERSION,
            game_id: "test".to_string(),
            your_seat: baylee_core::ids::PlayerId::new(0),
            seats: vec![SeatIdentity {
                player: baylee_core::ids::PlayerId::new(0),
                display_name: "You".to_string(),
                is_ai: false,
                team: None,
            }],
            prints: ids
                .iter()
                .map(|id| PrintEntry {
                    scryfall_id: (*id).to_string(),
                    lang: "en".to_string(),
                    finish: Finish::Normal,
                })
                .collect(),
        }
    }

    fn entry(id: &str, name: &str) -> CardTextEntry {
        CardTextEntry {
            scryfall_id: id.to_string(),
            lang: "en".to_string(),
            faces: vec![FaceText {
                name: name.to_string(),
                english_name: name.to_string(),
                type_line: "Instant".to_string(),
                oracle_text: "Draw a card.".to_string(),
                mana_cost: "{U}".to_string(),
            }],
        }
    }

    #[test]
    fn entries_are_filed_against_the_print_table() {
        let statics = statics(&["aaa", "bbb"]);
        let mut texts = CardTexts::default();
        texts.absorb(&statics, vec![entry("bbb", "Brainstorm")]);

        let found = texts.get(PrintRef::new(1), 0).expect("print 1 has text");
        assert_eq!(found.name, "Brainstorm");
        assert!(texts.get(PrintRef::new(0), 0).is_none());
    }

    /// The catalog answers with whatever ids it could resolve; one for a game
    /// this client is not in must not land in the table under a wrong index.
    #[test]
    fn text_for_a_printing_this_game_does_not_have_is_dropped() {
        let statics = statics(&["aaa"]);
        let mut texts = CardTexts::default();
        texts.absorb(&statics, vec![entry("zzz", "Something Else")]);
        assert!(texts.is_empty());
    }

    /// Scryfall ids are lowercase hex, but a preset assembled by hand may not
    /// be, and a case mismatch would silently cost every card its text.
    #[test]
    fn print_matching_ignores_case() {
        let statics = statics(&["AAA-BBB"]);
        let mut texts = CardTexts::default();
        texts.absorb(&statics, vec![entry("aaa-bbb", "Brainstorm")]);
        assert_eq!(texts.len(), 1);
    }

    /// A card with one face must not answer for a second one — a renderer
    /// asking for the back of a single-faced card gets nothing, not face 0.
    #[test]
    fn a_missing_face_has_no_text() {
        let statics = statics(&["aaa"]);
        let mut texts = CardTexts::default();
        texts.absorb(&statics, vec![entry("aaa", "Brainstorm")]);
        assert!(texts.get(PrintRef::new(0), 0).is_some());
        assert!(texts.get(PrintRef::new(0), 1).is_none());
    }
}
