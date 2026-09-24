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
//!
//! # The gateway is asked first, and Scryfall second
//!
//! Every ability the sheet draws says what the *card* says, in the player's
//! language, and there are no other words anywhere — a client that composed
//! its own prose for an ability it could not look up was drawing three
//! different things depending on what happened to be reachable. So there is
//! one source with two doors: the gateway's catalog, which knows the
//! player's language and falls back to English printing by printing, and
//! [`scryfall`] behind it for whatever the gateway did not answer — a
//! gateway with no ingest, a gateway that is not running at all, or a
//! printing its catalog has never seen.
//!
//! Scryfall is asked by **printing id**, so what comes back is that piece of
//! cardboard's own text. A deck names English printings, so in practice the
//! fallback is the English fallback the owner asked for; it is not a second
//! translator, and it is not meant to be one — the gateway is where a
//! language is chosen and this is where a hole is filled.

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

/// Where the fallback's answers gather.
///
/// Scryfall takes at most [`scryfall::BATCH`] identifiers in one call, so a
/// table of two decks is two or three requests and they finish in whatever
/// order they finish in. A slot that could only hold one answer would file
/// the first and drop the rest, which is a hole that looks exactly like a
/// card the catalog has never heard of.
type Gathering = Arc<Mutex<Gather>>;

/// The fallback's answers, and how many requests are still out.
#[derive(Default)]
struct Gather {
    /// Requests sent and not yet answered. Zero is the signal.
    outstanding: usize,
    /// What has come back so far.
    entries: Vec<CardTextEntry>,
}

/// Card text for the current game.
#[derive(Resource, Default)]
pub struct CardTexts {
    /// Text by printing, in the language actually served.
    by_print: HashMap<PrintRef, CardTextEntry>,
    /// What the fetch is doing.
    state: Fetch,
    /// The language everything here was fetched for; a change re-fetches.
    lang: String,
    /// How many printings the last request covered.
    ///
    /// The print table grows during a game: a seat earns an opponent's
    /// printing the first time it sees the card. Without this the text for
    /// everything the opponent plays would be missing for the rest of the
    /// game, because the one request went out before any of it was known.
    covered: usize,
}

/// State of the one in-flight request.
#[derive(Default)]
enum Fetch {
    /// Nothing requested yet.
    #[default]
    Idle,
    /// A request is out; the slot receives the decoded answer.
    Waiting(Slot),
    /// The gateway has answered and left gaps, and Scryfall is being asked
    /// about them.
    ///
    /// A state of its own rather than a second `Waiting`, because the two
    /// are asked different questions and only one of them leads on to the
    /// other: a gap the fallback could not fill is a card nobody has text
    /// for, and asking the gateway about it a second time would be a
    /// request loop against an answer that has already been given.
    ///
    /// The whole road is walked again when the print table *grows* — a seat
    /// earns an opponent's printing by seeing the card — because that is a
    /// new question rather than the same one re-asked. [`request`] is where
    /// that happens.
    Falling(Gathering),
    /// Finished, successfully or not. Either way the client stops asking:
    /// a gateway that is not there will not appear mid-game, and a retry
    /// loop against a dead endpoint costs a frame every time.
    Settled,
}

impl CardTexts {
    /// The text for a printing's face, if it has arrived.
    #[must_use]
    pub fn get(&self, print: PrintRef, face: u8) -> Option<CardText> {
        CardText::of(self.by_print.get(&print)?, face as usize)
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

    /// One printing's text, filed directly, for a test elsewhere in the crate
    /// that is about what a name is *drawn* as rather than about how text is
    /// fetched. The ordinary road in is [`Self::absorb`], which needs a
    /// `GameStatic` to turn a Scryfall id into a [`PrintRef`].
    #[cfg(test)]
    pub(crate) fn filed(print: PrintRef, entry: CardTextEntry) -> Self {
        let mut texts = Self::default();
        texts.by_print.insert(print, entry);
        texts
    }

    /// Files entries against the print table.
    ///
    /// The catalog answers by Scryfall id; the renderer asks by [`PrintRef`].
    /// This is where the two meet, and an entry for a printing the game does
    /// not contain — or which this seat has not been shown — is dropped rather
    /// than kept for a game that will never ask.
    fn absorb(&mut self, statics: &GameStatic, entries: Vec<CardTextEntry>) {
        for entry in entries {
            let found = statics.prints.iter().position(|p| {
                p.as_ref()
                    .is_some_and(|p| p.scryfall_id.eq_ignore_ascii_case(&entry.scryfall_id))
            });
            if let Some(index) = found {
                self.by_print.insert(PrintRef::new(index as u16), entry);
            }
        }
    }

    /// The printings this seat has been shown and still has no text for.
    ///
    /// The gap between [`known_ids`] and what is filed, which is what the
    /// fallback is asked about. It is computed after the gateway has
    /// answered rather than from its answer, because the two disagree in
    /// both directions: the catalog resolves an id to another printing of
    /// the same card and answers under the id that was *asked* for, and a
    /// cached entry from a previous session fills a printing this request
    /// never mentioned.
    fn missing_ids(&self, statics: &GameStatic) -> Vec<String> {
        statics
            .prints
            .iter()
            .enumerate()
            .filter(|(index, _)| !self.by_print.contains_key(&PrintRef::new(*index as u16)))
            .filter_map(|(_, p)| p.as_ref().map(|p| p.scryfall_id.clone()))
            .collect()
    }

    /// Everything filed, for the on-disk cache.
    ///
    /// The cache is replaced wholesale, so it is written from the table and
    /// never from one answer: storing the fallback's entries alone would
    /// throw away the gateway's, and a language a gateway serves well would
    /// come back from disk as the handful of cards Scryfall filled in.
    fn filed_entries(&self) -> Vec<CardTextEntry> {
        self.by_print.values().cloned().collect()
    }
}

/// The printings this seat has actually been shown.
///
/// The print table has holes in it: a seat is entitled to its own deck from
/// the start and to the rest only as it sees the cards. Asking the catalog
/// about a hole would be asking about a card this client is not allowed to
/// know, and would send an empty id that reads like a bug at the other end.
fn known_ids(statics: &GameStatic) -> Vec<&str> {
    statics
        .prints
        .iter()
        .filter_map(|p| p.as_ref().map(|p| p.scryfall_id.as_str()))
        .collect()
}

/// How many printings this seat has been shown.
///
/// Counted rather than collected: this runs every frame, and the ids are only
/// needed on the frame a request actually goes out.
fn known_count(statics: &GameStatic) -> usize {
    statics.prints.iter().filter(|p| p.is_some()).count()
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
    let known = known_count(statics);
    if !matches!(texts.state, Fetch::Idle) && texts.lang == settings.lang && known <= texts.covered
    {
        return;
    }
    if texts.lang != settings.lang {
        texts.by_print.clear();
        texts.state = Fetch::Idle;
    }
    // A printing this seat has just earned: ask again, for the whole table.
    // The catalog answers from its own cache, and one request is cheaper than
    // tracking which ids of a few dozen are new.
    if known > texts.covered && matches!(texts.state, Fetch::Settled) {
        texts.state = Fetch::Idle;
    }
    if !matches!(texts.state, Fetch::Idle) {
        return;
    }
    texts.lang.clone_from(&settings.lang);
    texts.covered = known;

    // Whatever a previous session stored is usable immediately, and covers
    // the whole game when nothing changed — the request that follows only
    // has to fill gaps.
    let cached = cache::load(&settings.lang);
    if !cached.is_empty() {
        texts.absorb(statics, cached);
    }

    let ids: Vec<&str> = known_ids(statics);
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
        if let Ok(mut slot) = target.lock() {
            *slot = Some(entries);
        }
    });
    texts.state = Fetch::Waiting(slot);
    bevy::log::info!(cards = ids.len(), "requesting card text");
}

/// Files the answer when it arrives, and fills what it left out.
pub fn poll(mut texts: ResMut<CardTexts>, duel: Res<crate::Duel>) {
    // Asked before the answer is taken, and not after: an answer taken with
    // nowhere to file it is gone — the slot is emptied, the state stays
    // `Waiting`, and `request` has no reason to ask again — so the whole
    // game would run with no text because one frame arrived between the
    // duel closing and the fetch returning.
    let Some(statics) = duel.statics.as_ref() else {
        return;
    };
    // Whichever door is open, its answer is taken here and the borrow of
    // `state` ends with it: everything below writes to the table.
    let arrived = match &texts.state {
        Fetch::Waiting(slot) => slot
            .lock()
            .ok()
            .and_then(|mut s| s.take())
            .map(|entries| (entries, true)),
        Fetch::Falling(gathering) => gathering
            .lock()
            .ok()
            .filter(|gather| gather.outstanding == 0)
            .map(|mut gather| (std::mem::take(&mut gather.entries), false)),
        Fetch::Idle | Fetch::Settled => None,
    };
    let Some((entries, from_the_gateway)) = arrived else {
        return;
    };
    texts.absorb(statics, entries);

    // The gateway has had its turn. Whatever it left is a hole in what the
    // player reads, and there is exactly one thing to do about a hole: ask
    // the other door.
    let missing = if from_the_gateway {
        texts.missing_ids(statics)
    } else {
        Vec::new()
    };
    if !missing.is_empty() {
        bevy::log::info!(
            printings = missing.len(),
            "asking Scryfall for the text the catalog did not have"
        );
        texts.state = Fetch::Falling(scryfall::fill(&missing));
        return;
    }
    texts.state = Fetch::Settled;
    cache::store(&texts.lang, &texts.filed_entries());
    bevy::log::info!(printings = texts.len(), "card text ready");
}

/// The door behind the gateway.
///
/// Scryfall's own guidelines ask for a `User-Agent`, at most ten requests a
/// second and no more than 75 identifiers per `/cards/collection` call. A
/// whole table is one or two of those and they go out once per game, which
/// is well inside all three — there is no pacing here because there is
/// nothing to pace.
pub(crate) mod scryfall {
    use super::{Gather, Gathering};
    use baylee_client_core::card_face::{CardTextEntry, FaceText};
    use std::sync::{Arc, Mutex};

    /// How many identifiers Scryfall takes in one call.
    pub const BATCH: usize = 75;

    /// The endpoint that answers about many printings at once.
    const COLLECTION: &str = "https://api.scryfall.com/cards/collection";

    /// Sends one request per batch and hands back where they gather.
    pub(super) fn fill(ids: &[String]) -> Gathering {
        let bodies = bodies(ids);
        let gathering: Gathering = Arc::new(Mutex::new(Gather {
            outstanding: bodies.len(),
            entries: Vec::new(),
        }));
        for body in bodies {
            let target = Arc::clone(&gathering);
            ehttp::fetch(request(body), move |result| {
                let entries = match result {
                    Ok(response) if response.ok => response.text().map(parse).unwrap_or_default(),
                    Ok(response) => {
                        bevy::log::warn!(status = response.status, "Scryfall refused a collection");
                        Vec::new()
                    }
                    Err(err) => {
                        // The same non-error as a gateway that is not there:
                        // every card still renders, this one without words.
                        bevy::log::info!("Scryfall unavailable: {err}");
                        Vec::new()
                    }
                };
                if let Ok(mut gather) = target.lock() {
                    gather.entries.extend(entries);
                    gather.outstanding = gather.outstanding.saturating_sub(1);
                }
            });
        }
        gathering
    }

    /// One `/cards/collection` call, headers and all.
    ///
    /// The headers are **set**, not added, and that is the whole reason this
    /// is a function with a test on it. `ehttp::Headers::insert` appends —
    /// "if the key already exists, it will also be kept" — and
    /// `Request::post` has already written `Content-Type: text/plain`, so
    /// inserting `application/json` beside it sent both and Scryfall read
    /// the first: every request answered `400` and every card at the table
    /// came up wordless. Nothing in this module could see it, because a
    /// body is not a request.
    ///
    /// Scryfall's guidelines also ask for a `User-Agent`, and this is the
    /// only place the client identifies itself to anyone.
    pub fn request(body: String) -> ehttp::Request {
        ehttp::Request {
            headers: ehttp::Headers::new(&[
                ("Accept", "application/json"),
                ("Content-Type", "application/json"),
                (
                    "User-Agent",
                    concat!("baylee-client/", env!("CARGO_PKG_VERSION")),
                ),
            ]),
            ..ehttp::Request::post(COLLECTION, body.into_bytes())
        }
    }

    /// One request body per batch, and none at all for nothing to ask.
    ///
    /// Split out from [`fill`] because the batching is the half that can be
    /// wrong without anything failing: 76 ids in one body is a `422` from
    /// Scryfall and reads here as a table whose text simply never arrived.
    #[must_use]
    pub fn bodies(ids: &[String]) -> Vec<String> {
        ids.chunks(BATCH).map(body).collect()
    }

    /// The request body: `{"identifiers":[{"id":"…"},…]}`.
    ///
    /// Written rather than serialised through a type, because the whole
    /// shape is one key and a list of one-key objects and a `serde` struct
    /// for it would be three declarations saying the same thing. The ids
    /// are escaped all the same — one arrives from a `GameStatic` a
    /// gateway sent, which is not this client's to vouch for.
    #[must_use]
    pub fn body(ids: &[String]) -> String {
        let list: Vec<String> = ids
            .iter()
            .map(|id| format!("{{\"id\":{}}}", escape(id)))
            .collect();
        format!("{{\"identifiers\":[{}]}}", list.join(","))
    }

    /// One JSON string literal, quotes included.
    fn escape(text: &str) -> String {
        serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
    }

    /// Scryfall's answer, as the entries the catalog would have sent.
    ///
    /// `not_found` is not read: an id nobody has heard of is a printing
    /// with no text, which is what the caller already had. What matters is
    /// that the entry is filed under the id that was **asked** for — the
    /// same rule the catalog's own query obeys, because `PrintRef` is
    /// resolved through the print table and a different id files nowhere.
    #[must_use]
    pub fn parse(body: &str) -> Vec<CardTextEntry> {
        let Ok(list) = serde_json::from_str::<Collection>(body) else {
            return Vec::new();
        };
        list.data
            .into_iter()
            .map(|card| CardTextEntry {
                lang: card.lang.clone(),
                faces: card.faces(),
                scryfall_id: card.id,
            })
            .collect()
    }

    /// The `/cards/collection` envelope.
    #[derive(serde::Deserialize)]
    struct Collection {
        #[serde(default)]
        data: Vec<Payload>,
    }

    /// One printing, in the shape `baylee-catalog` reads it.
    ///
    /// Named for the payload rather than for the card, because every field
    /// on it is Scryfall's own spelling and cannot be renamed — a struct
    /// called `Card` with a `card_faces` on it is what clippy's
    /// `struct_field_names` objects to, and the wire is not ours to move.
    #[derive(serde::Deserialize, Default)]
    struct Payload {
        #[serde(default)]
        id: String,
        #[serde(default = "english")]
        lang: String,
        #[serde(flatten)]
        top: Face,
        #[serde(default)]
        card_faces: Vec<Face>,
    }

    /// The fields a face carries, wherever they sit.
    ///
    /// Scryfall writes them on the card for a single-faced printing and on
    /// each entry of `card_faces` otherwise, and a split or an adventure
    /// writes *both* — so the faces win whenever there are any, which is
    /// the normalisation `baylee_catalog::scryfall::Card::faces` performs
    /// on the ingest side.
    #[derive(serde::Deserialize, Default)]
    struct Face {
        #[serde(default)]
        name: String,
        printed_name: Option<String>,
        type_line: Option<String>,
        printed_type_line: Option<String>,
        oracle_text: Option<String>,
        printed_text: Option<String>,
        mana_cost: Option<String>,
    }

    /// What a record with no `lang` is.
    fn english() -> String {
        "en".to_string()
    }

    impl Payload {
        /// This printing's faces, one code path for one face or two.
        fn faces(&self) -> Vec<FaceText> {
            if self.card_faces.is_empty() {
                vec![self.top.text()]
            } else {
                self.card_faces.iter().map(Face::text).collect()
            }
        }
    }

    impl Face {
        /// Field by field, the printed form where there is one.
        ///
        /// A printing may be translated and still carry no translated rules
        /// text — 6489 of 59 465 German faces, by the catalog's own count —
        /// so half a card is taken rather than none, exactly as
        /// `Catalog::text` does it.
        fn text(&self) -> FaceText {
            FaceText {
                name: self
                    .printed_name
                    .clone()
                    .unwrap_or_else(|| self.name.clone()),
                english_name: self.name.clone(),
                type_line: self
                    .printed_type_line
                    .clone()
                    .or_else(|| self.type_line.clone())
                    .unwrap_or_default(),
                oracle_text: self
                    .printed_text
                    .clone()
                    .or_else(|| self.oracle_text.clone())
                    .unwrap_or_default(),
                mana_cost: self.mana_cost.clone().unwrap_or_default(),
            }
        }
    }
}

/// The on-disk half: what was fetched once does not need fetching again.
///
/// Card text changes only when an oracle update lands, so a stale entry is a
/// cosmetic problem for a few days at worst, while a cold start with no
/// network is a game that cannot be read at all. The cache therefore has no
/// expiry — a fresh fetch overwrites it whenever one succeeds.
/// # One back end, not two
///
/// This used to carry its own copy of both halves of `settings::store` — the
/// XDG path arithmetic natively, the `localStorage` wrapper in a browser —
/// which meant every property of the client's persistence had to be won
/// twice. It was not: the settings store writes through a temporary and
/// renames, and this one truncated in place. The cache is a named document
/// like any other now, so the atomic write and the rescue of an unreadable
/// file arrived here for free, and whatever that seam learns next will too.
///
/// The browser note the old copy carried still applies and now applies once:
/// `localStorage` is capped at a few megabytes and throws when it is full. A
/// game's text is far below that; a player who has played in nineteen
/// languages is the case that would reach it, and the write is best-effort
/// either way.
pub(crate) mod cache {
    use baylee_client_core::card_face::CardTextEntry;

    /// The document one language's text lives in.
    ///
    /// Filtered to what a filename may hold because the language arrives
    /// from a setting a player can type into.
    fn name(lang: &str) -> String {
        let lang: String = lang
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        format!("card-text-{lang}.json")
    }

    /// Reads the cached entries for a language.
    ///
    /// A document that cannot be read is set aside rather than left for the
    /// next successful fetch to overwrite. Nothing here was authored by a
    /// player — every entry can be fetched again — so this is the cheap half
    /// of the rule the deck file needs it for; it is applied anyway, because
    /// a cache that empties itself in silence is how an afternoon goes into
    /// debugging the fetch path when the writer was at fault.
    pub fn load(lang: &str) -> Vec<CardTextEntry> {
        let name = name(lang);
        let Some(text) = crate::settings::store::read_named(&name) else {
            return Vec::new();
        };
        serde_json::from_str(&text).unwrap_or_else(|_| {
            crate::settings::store::set_aside(&name);
            Vec::new()
        })
    }

    /// Replaces the cache for a language.
    pub fn store(lang: &str, entries: &[CardTextEntry]) {
        if let Ok(text) = serde_json::to_string(entries) {
            crate::settings::store::write_named(&name(lang), &text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::card_face::FaceText;
    use baylee_view::{Finish, PrintEntry, SeatIdentity};

    fn statics(ids: &[&str]) -> GameStatic {
        GameStatic {
            decision_secs: None,
            reconnect_secs: None,
            view_version: baylee_view::VIEW_VERSION,
            game_id: "test".to_string(),
            your_seat: baylee_core::ids::PlayerId::new(0),
            seats: vec![SeatIdentity {
                player: baylee_core::ids::PlayerId::new(0),
                display_name: "You".to_string(),
                is_ai: false,
                away: false,
                team: None,
            }],
            prints: ids
                .iter()
                .map(|id| {
                    Some(PrintEntry {
                        scryfall_id: (*id).to_string(),
                        lang: "en".to_string(),
                        finish: Finish::Normal,
                    })
                })
                .collect(),
        }
    }

    /// A hole in the print table is a card this seat has not been shown.
    /// Asking the catalog about it would be asking about a card the client is
    /// not entitled to know is in the game.
    #[test]
    fn a_hole_in_the_print_table_is_never_asked_about() {
        let mut statics = statics(&["aaa", "bbb"]);
        statics.prints[0] = None;
        assert_eq!(known_ids(&statics), vec!["bbb"]);
        assert_eq!(known_count(&statics), 1);
    }

    /// And the entry that fills the hole still lands on the right `PrintRef`:
    /// the index is the handle, so a hole may never shorten the table.
    #[test]
    fn a_filled_hole_files_against_its_own_index() {
        let mut statics = statics(&["aaa", "bbb"]);
        statics.prints[0] = None;
        let mut texts = CardTexts::default();
        texts.absorb(&statics, vec![entry("bbb", "Forest")]);
        assert!(texts.get(PrintRef::new(0), 0).is_none());
        assert_eq!(
            texts.get(PrintRef::new(1), 0).map(|t| t.name.clone()),
            Some("Forest".to_string())
        );
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

    /// The order the owner asked for, seen from the client's side: the
    /// gateway answers, and **what it left out** is what the second door is
    /// asked about. Not the whole table, and not nothing.
    #[test]
    fn what_the_gateway_left_out_is_what_scryfall_is_asked_for() {
        let statics = statics(&["aaa", "bbb", "ccc"]);
        let mut texts = CardTexts::default();
        texts.absorb(&statics, vec![entry("bbb", "Brainstorm")]);
        assert_eq!(texts.missing_ids(&statics), vec!["aaa", "ccc"]);
    }

    /// A gateway that is not running answers nothing, and then every card at
    /// the table goes to the fallback. This is the case the owner reported —
    /// a sheet with no words in it — so it is the one pinned by name.
    #[test]
    fn a_gateway_that_answers_nothing_sends_the_whole_table_to_scryfall() {
        let statics = statics(&["aaa", "bbb"]);
        let texts = CardTexts::default();
        assert_eq!(texts.missing_ids(&statics), vec!["aaa", "bbb"]);
    }

    /// And a hole in the print table is still never asked about — by either
    /// door. The fallback would otherwise send an empty identifier and, worse,
    /// ask about a card this seat is not entitled to know is in the game.
    #[test]
    fn the_fallback_never_asks_about_a_hole_either() {
        let mut statics = statics(&["aaa", "bbb"]);
        statics.prints[0] = None;
        let texts = CardTexts::default();
        assert_eq!(texts.missing_ids(&statics), vec!["bbb"]);
    }

    /// Text that arrived from either door is one cache, because the cache is
    /// replaced wholesale: a session that filled two gaps out of sixty must
    /// not come back from disk as two cards.
    #[test]
    fn the_cache_is_written_from_the_table_and_not_from_one_answer() {
        let statics = statics(&["aaa", "bbb"]);
        let mut texts = CardTexts::default();
        texts.absorb(&statics, vec![entry("aaa", "Brainstorm")]);
        texts.absorb(&statics, vec![entry("bbb", "Forest")]);
        let mut names: Vec<String> = texts
            .filed_entries()
            .iter()
            .map(|e| e.faces[0].name.clone())
            .collect();
        names.sort();
        assert_eq!(names, vec!["Brainstorm".to_string(), "Forest".to_string()]);
    }

    /// Scryfall's limit is 75 identifiers, and the whole of a game's text
    /// rides on nobody quietly exceeding it.
    #[test]
    fn a_table_larger_than_one_collection_is_asked_for_in_batches() {
        let ids: Vec<String> = (0..160).map(|n| format!("id-{n}")).collect();
        let bodies = scryfall::bodies(&ids);
        assert_eq!(bodies.len(), 3);
        assert_eq!(bodies[0].matches("\"id\"").count(), scryfall::BATCH);
        assert_eq!(
            bodies[2].matches("\"id\"").count(),
            160 - 2 * scryfall::BATCH
        );
        assert!(bodies[0].starts_with("{\"identifiers\":[{\"id\":\"id-0\"}"));
        assert!(scryfall::bodies(&[]).is_empty());
    }

    /// A request says `application/json` **once**.
    ///
    /// The header the whole fallback turned on: `ehttp::Headers::insert`
    /// appends rather than replaces, and `Request::post` writes
    /// `text/plain` first, so a request built by inserting beside it carried
    /// both and Scryfall answered `400` to every one — a table with no words
    /// on any card, through a road that logged every step as working. The
    /// count is the assertion, not the presence.
    #[test]
    fn a_collection_request_names_json_once_and_says_who_is_asking() {
        let request = scryfall::request(scryfall::bodies(&["aaa".to_string()]).remove(0));
        let kinds: Vec<&str> = request.headers.get_all("content-type").collect();
        assert_eq!(kinds, vec!["application/json"]);
        assert_eq!(request.method, ehttp::Method::POST);
        assert!(
            request
                .headers
                .get("user-agent")
                .is_some_and(|ua| ua.starts_with("baylee-client/")),
            "Scryfall's guidelines ask every client to name itself"
        );
        assert_eq!(request.body, b"{\"identifiers\":[{\"id\":\"aaa\"}]}");
    }

    /// An id is escaped rather than pasted: it arrives in a `GameStatic` a
    /// gateway sent, which is not this client's to vouch for.
    #[test]
    fn an_id_is_escaped_into_the_request() {
        let body = scryfall::bodies(&["a\"b".to_string()]).remove(0);
        assert_eq!(body, "{\"identifiers\":[{\"id\":\"a\\\"b\"}]}");
    }

    /// The whole point of the fallback: the printed sentence comes back and
    /// is filed under the id that was **asked** for, which is the id the
    /// print table names — anything else files nowhere.
    #[test]
    fn scryfall_answers_the_printed_text_of_one_face() {
        let entries = scryfall::parse(
            r#"{"object":"list","data":[{"id":"aaa","lang":"en","name":"Chromatic Sphere",
            "type_line":"Artifact","mana_cost":"{1}",
            "oracle_text":"{1}, {T}, Sacrifice this artifact: Add one mana of any color. Draw a card."}],
            "not_found":[]}"#,
        );
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].scryfall_id, "aaa");
        assert_eq!(entries[0].lang, "en");
        assert_eq!(entries[0].faces.len(), 1);
        assert!(entries[0].faces[0].oracle_text.contains("Add one mana"));
        assert_eq!(entries[0].faces[0].english_name, "Chromatic Sphere");
    }

    /// A translated printing writes its rules text in `printed_text` and
    /// keeps the English in `oracle_text`, and the player reads the first.
    /// This is the same field-by-field fallback `Catalog::text` performs, and
    /// the two must not disagree about which one a player sees.
    #[test]
    fn a_translated_printing_is_read_the_way_the_catalog_reads_it() {
        let entries = scryfall::parse(
            r#"{"data":[{"id":"bbb","lang":"de","name":"Forest","printed_name":"Wald",
            "type_line":"Basic Land — Forest","printed_type_line":"Basisland — Wald",
            "oracle_text":"({T}: Add {G}.)","printed_text":"({T}: Erzeuge {G}.)"}]}"#,
        );
        let face = &entries[0].faces[0];
        assert_eq!(face.name, "Wald");
        assert_eq!(face.english_name, "Forest");
        assert_eq!(face.type_line, "Basisland — Wald");
        assert_eq!(face.oracle_text, "({T}: Erzeuge {G}.)");
    }

    /// Half a card is better than none: a printing may be translated and
    /// carry no translated rules text at all.
    #[test]
    fn a_translated_name_with_untranslated_rules_keeps_both() {
        let entries = scryfall::parse(
            r#"{"data":[{"id":"ccc","lang":"de","name":"Shock","printed_name":"Schock",
            "oracle_text":"Shock deals 2 damage to any target."}]}"#,
        );
        let face = &entries[0].faces[0];
        assert_eq!(face.name, "Schock");
        assert_eq!(face.oracle_text, "Shock deals 2 damage to any target.");
    }

    /// A two-faced printing writes its faces in `card_faces` and *also*
    /// carries a joined top level, so the faces have to win — a client that
    /// read the top level would draw one face's text on both sides.
    #[test]
    fn a_two_faced_printing_is_read_off_its_faces() {
        let entries = scryfall::parse(
            r#"{"data":[{"id":"ddd","lang":"en","name":"Delver of Secrets // Insectile Aberration",
            "type_line":"Creature — Human Wizard // Creature — Human Insect",
            "card_faces":[
              {"name":"Delver of Secrets","type_line":"Creature — Human Wizard",
               "mana_cost":"{U}","oracle_text":"At the beginning of your upkeep, look at the top card of your library."},
              {"name":"Insectile Aberration","type_line":"Creature — Human Insect",
               "mana_cost":"","oracle_text":"Flying"}]}]}"#,
        );
        assert_eq!(entries[0].faces.len(), 2);
        assert_eq!(entries[0].faces[1].name, "Insectile Aberration");
        assert_eq!(entries[0].faces[1].oracle_text, "Flying");
    }

    /// Everything about the answer is somebody else's to change, so nothing
    /// in it may be able to take the game down: a body that is not JSON, a
    /// record with no id and a card with no text all answer quietly.
    #[test]
    fn an_answer_that_makes_no_sense_costs_no_more_than_the_text() {
        assert!(scryfall::parse("not json at all").is_empty());
        assert!(scryfall::parse("{}").is_empty());
        let entries = scryfall::parse(r#"{"data":[{"lang":"en"}]}"#);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].scryfall_id, "");
        assert_eq!(entries[0].faces[0].oracle_text, "");
        // …and a record with no `lang` is English, which is what every
        // record Scryfall has ever written without one is.
        let entries = scryfall::parse(r#"{"data":[{"id":"eee","name":"Forest"}]}"#);
        assert_eq!(entries[0].lang, "en");
    }
}
