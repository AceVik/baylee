//! Card text, asked for by card and kept on disk.
//!
//! # Keyed by card, not by printing
//!
//! Text is a property of the card and not of a printing
//! (`docs/protocol.md` §"Card text"): the gateway picks one printing per card
//! and language by `baylee_cardtext::pick`, and every face comes from it. A
//! printing id was therefore never the key, and it could not be one: a copy's
//! abilities are printed on a card the copy is not, so a table keyed by the
//! copy's printing had nothing to say about them. Keyed by card, a Spark
//! Double showing Sheoldred reads Sheoldred's German, and a second game
//! reuses the first game's text because a card says the same in every game.
//!
//! # What is asked, and when
//!
//! The cards this seat's view names ([`PlayerView::cards`]) and nothing else.
//! That walk is the one the print table's entitlement walks, so a seat never
//! asks about a card it could not see. Each card is asked once per language,
//! in one request at a time; cards the view names while one is out wait for
//! the next. The library is not prefetched — a card drawn is asked for when
//! it reaches the hand — which is a request a turn at most.
//!
//! Under English nothing is asked at all. The gateway would answer with the
//! English Oracle (`printed` is `null` under `en`), and that is compiled in.
//!
//! A gateway that does not answer, or refuses `oracle_ids` because it
//! predates them, is not asked again until the language or the game changes:
//! a retry per card drawn against a dead endpoint is noise, not a recovery.
//! Until then Scryfall is asked instead ([`scryfall::search`]), one card at a
//! time and three a second at most, and its printings go through the
//! gateway's own rule ([`baylee_cardtext::card_entry`]), so an offline game
//! in German reads the German the gateway would have served.
//!
//! # Under all of it, the English Oracle
//!
//! A sentence is never drawn from this table alone: [`sentence`] answers
//! from the player's language where the served printing lines up with the
//! compiled English Oracle (`baylee_cardtext::align`, placed once when the
//! entry is filed), and from that Oracle where it does not. A card face
//! draws the same floor ([`CardTexts::face`]). What a row or a face says
//! never depends on whether a request got through — the owner's rule,
//! "Fallback ist immer englisch".

use baylee_cardtext::Aligned;
use baylee_client_core::card_face::{CardText, CardTextEntry, TextBlock, split_blocks};
use baylee_core::ids::CardIndex;
use baylee_view::{PlayerView, StackText};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use std::sync::{Arc, Mutex};

/// Where the fetch callback leaves its answer.
///
/// A channel would be the obvious choice, but `Receiver` is not `Sync` and a
/// Bevy resource must be — and there is only ever one answer, so a slot is
/// both smaller and enough.
type Slot = Arc<Mutex<Option<Reply>>>;

/// What one request came back with.
enum Reply {
    /// The entries the gateway knew; a card it lacks is simply absent.
    Answered(Vec<CardTextEntry>),
    /// No answer: unreachable, refused, or unreadable.
    Failed,
}

/// The gateway card text is asked of, and the session it answers (#270).
///
/// `/catalog/text` answers a signed-in session only: open to anyone, it
/// re-served Scryfall's data to whoever asked. So the text follows the
/// lobby's session as the art mirror does
/// (`lobby::systems::text_follows_the_session`): signed in, the gateway the
/// player signed in to and that one only; otherwise none, and the Scryfall
/// door answers instead. A duel with no lobby behind it, a seat ticket from
/// the environment among them, is never signed in.
#[derive(Resource, Default, Clone, PartialEq, Eq, Debug)]
pub struct TextGateway(pub Option<SignedIn>);

/// A gateway and a session on it.
#[derive(Clone, PartialEq, Eq)]
pub struct SignedIn {
    /// The gateway's base URL.
    pub base: String,
    /// The bearer token, shown to that gateway and nowhere else.
    pub token: String,
}

impl std::fmt::Debug for SignedIn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignedIn")
            .field("base", &self.base)
            .field("token", &"<token>")
            .finish()
    }
}

/// The request for `ids`' text in `lang`: to the gateway the session is on,
/// carrying it. None without a session, which the gateway would refuse.
fn text_request(gateway: Option<&SignedIn>, lang: &str, ids: &[&str]) -> Option<ehttp::Request> {
    let gateway = gateway?;
    let mut request = ehttp::Request::get(format!(
        "{}/catalog/text?lang={lang}&oracle_ids={}",
        gateway.base.trim_end_matches('/'),
        ids.join(",")
    ));
    request
        .headers
        .insert("Authorization", format!("Bearer {}", gateway.token));
    Some(request)
}

/// One card's text as served, with each face's printed sentences already
/// placed against the compiled Oracle.
struct Filed {
    entry: CardTextEntry,
    /// Per face: the printed lines in the Oracle's positions, or `None` where
    /// the face was not translated or does not line up — the Oracle is drawn
    /// there. Placed once, when the entry is filed, so a frame never aligns.
    aligned: Vec<Option<Aligned>>,
}

/// Card text for this client, in the language it is set to.
#[derive(Resource, Default)]
pub struct CardTexts {
    /// Text by card.
    by_card: HashMap<CardIndex, Filed>,
    /// Moves whenever anything is filed. A panel drawn from this table keeps
    /// the number it was drawn at, and redraws when it differs.
    ///
    /// A count of entries is not that number: a cached entry replaced by the
    /// gateway's fresher one changes the text and not the count.
    generation: u64,
    /// The language everything here is in; a change starts over.
    lang: String,
    /// Cards asked about in [`Self::lang`], answered or not.
    asked: HashSet<CardIndex>,
    /// The view last walked for cards to ask about, by its `seq`.
    walked: Option<u64>,
    /// The game the gateway's reachability was learned in.
    game: String,
    /// The gateway did not answer in this language and game.
    down: bool,
    /// The one request out, if any, and the cards it names.
    waiting: Option<(Slot, Vec<CardIndex>)>,
    /// The second door, asked while this one is [`Self::down`].
    scryfall: Door,
}

/// Scryfall, asked about one card at a time while the gateway does not
/// answer — a game offline, a gateway that is not running.
///
/// One card a request, because Scryfall's search answers one query and the
/// query is the card (`oracleid:… lang:…`); every printing of it in the
/// language comes back, and [`baylee_cardtext::card_entry`] picks from them
/// exactly as the gateway would have.
#[derive(Default)]
struct Door {
    /// Cards asked about in this language, answered or not.
    tried: HashSet<CardIndex>,
    /// The one request out, if any, and its card.
    waiting: Option<(Slot, CardIndex)>,
    /// When the next request may go, on the real clock.
    next_at: f64,
    /// The view last walked and found nothing new in, by its `seq`.
    walked: Option<u64>,
    /// Scryfall did not answer in this language and game.
    down: bool,
}

impl CardTexts {
    /// The served text for one face of a card, if it has arrived.
    #[must_use]
    pub fn get(&self, card: CardIndex, face: u8) -> Option<CardText> {
        CardText::of(&self.by_card.get(&card)?.entry, usize::from(face))
    }

    /// The text to draw for one face of a card: the served text where it has
    /// arrived, else the card's own English ([`english`]).
    #[must_use]
    pub fn face(&self, card: CardIndex, face: u8) -> Option<CardText> {
        self.get(card, face)
            .or_else(|| english(card, usize::from(face)))
    }

    /// Which filing this table is at; see the field.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// How many cards have text.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_card.len()
    }

    /// Whether any card has text.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_card.is_empty()
    }

    /// One card's text, filed as the gateway's would be, for a test elsewhere
    /// in the crate that is about what is *drawn* rather than how it is
    /// fetched. The entry names its card by `oracle_id`, as the wire does.
    #[cfg(test)]
    pub(crate) fn filed(entry: CardTextEntry) -> Self {
        let mut texts = Self::default();
        texts.absorb(vec![entry]);
        texts
    }

    /// Files entries by the card their `oracle_id` names, and returns how
    /// many were filed.
    ///
    /// An entry naming no card of this pool is dropped — among them every
    /// entry a cache wrote before text was keyed by card, which named a
    /// printing and left `oracle_id` empty.
    fn absorb(&mut self, entries: Vec<CardTextEntry>) -> usize {
        let mut filed = 0;
        for entry in entries {
            let Some(def) = baylee_cards::by_oracle_id(&entry.oracle_id.to_ascii_lowercase())
            else {
                continue;
            };
            let aligned = (0..entry.faces.len())
                .map(|face| {
                    let oracle = baylee_cards::oracle::face(def.index, face)?;
                    let printed = entry.faces[face].printed.as_deref()?;
                    baylee_cardtext::align(oracle, printed, &entry.layout)
                })
                .collect();
            self.by_card.insert(def.index, Filed { entry, aligned });
            filed += 1;
        }
        if filed > 0 {
            self.generation += 1;
        }
        filed
    }

    /// The player's-language sentence standing for one line of a card, if
    /// the served printing has one there.
    fn localized(&self, card: CardIndex, at: StackText) -> Option<&str> {
        let aligned = self
            .by_card
            .get(&card)?
            .aligned
            .get(usize::from(at.face))?
            .as_ref()?;
        let oracle = baylee_cards::oracle::face(card, usize::from(at.face))?;
        baylee_cardtext::localized(oracle, aligned, usize::from(at.line))
    }

    /// Starts over in another language: this one's table from disk, nothing
    /// asked yet.
    fn relang(&mut self, lang: &str) {
        self.by_card.clear();
        self.asked.clear();
        self.walked = None;
        self.down = false;
        self.waiting = None;
        self.scryfall = Door::default();
        self.lang = lang.to_string();
        if lang != "en" {
            self.absorb(cache::load(lang));
        }
        self.generation += 1;
    }

    /// Takes one request's reply, and answers whether anything was filed.
    ///
    /// A failed request gives its cards back: they were never answered, and
    /// the next game — the one place a gateway gets asked again — asks
    /// about them. A card the gateway answered without is left asked.
    fn receive(&mut self, reply: Reply, cards: &[CardIndex]) -> bool {
        match reply {
            Reply::Answered(entries) => self.absorb(entries) > 0,
            Reply::Failed => {
                for card in cards {
                    self.asked.remove(card);
                }
                self.down = true;
                false
            }
        }
    }

    /// The card the Scryfall door asks about next, if it may ask now: the
    /// gateway is down, Scryfall is not, nothing is out, the pace allows,
    /// and the view names a card with no text that has not been tried —
    /// the lowest index, so the same view asks in the same order.
    fn next_for_scryfall(&self, view: &PlayerView, now: f64) -> Option<CardIndex> {
        let door = &self.scryfall;
        if self.lang == "en"
            || !self.down
            || door.down
            || door.waiting.is_some()
            || now < door.next_at
            || door.walked == Some(view.seq)
        {
            return None;
        }
        wanted(view)
            .filter(|card| !self.by_card.contains_key(card) && !door.tried.contains(card))
            .min()
    }

    /// Takes the Scryfall door's reply for `card`, and answers whether
    /// anything was filed. A failed request gives the card back, as the
    /// gateway's does, and closes the door until the next game.
    fn receive_scryfall(&mut self, reply: Reply, card: CardIndex) -> bool {
        match reply {
            Reply::Answered(entries) => self.absorb(entries) > 0,
            Reply::Failed => {
                self.scryfall.tried.remove(&card);
                self.scryfall.down = true;
                false
            }
        }
    }

    /// Everything filed, for the on-disk cache.
    ///
    /// The cache is replaced wholesale, so it is written from the table and
    /// never from one answer: storing one answer alone would throw away
    /// every card asked about before it.
    fn filed_entries(&self) -> Vec<CardTextEntry> {
        self.by_card.values().map(|f| f.entry.clone()).collect()
    }
}

/// The printed sentence at one line of a card's face, as blocks to draw.
///
/// The player's own language where this seat holds the card's text and the
/// served printing lines up with the English the index was counted in; the
/// card's **English Oracle** sentence otherwise. The second half is the
/// owner's rule for every row drawn from card text — "Fallback ist immer
/// englisch" — and it is what takes a row's words off the network: the
/// Oracle is compiled in (`baylee_cards::oracle`), so a client with no
/// gateway, a card nobody translated and a translation whose lines do not
/// pair all draw the card's own English rather than a blank.
///
/// `None` only where no sentence exists at all: a line the table does not
/// have, or a card the pool does not compile. Neither is a coordinate the
/// host or the line table hands out, and a caller that meets one draws no
/// words rather than invented ones.
///
/// And `None` where the host counted another text: `at.of` is how many
/// sentences the face had where the host looked the line up, and a compiled
/// Oracle with a different count is a client built against other card text
/// than its host (`refresh-oracle` moves the text and not the view version).
/// An index merely out of range is caught by anyone; this is the case where
/// it lands *in* range, on the sentence beside the right one, and would be
/// drawn to the player as precise text — worse than no words.
///
/// One door for the three places a sentence is drawn — the ability sheet, the
/// stack and the cast chooser — so no two of them can disagree about what an
/// ability says.
///
/// `card` is the card the sentence is printed on — [`PublicObject::rules`],
/// which for a copy is the copied card, and whose text this table holds
/// under that card.
///
/// [`PublicObject::rules`]: baylee_view::PublicObject::rules
#[must_use]
pub fn sentence(
    texts: Option<&CardTexts>,
    card: CardIndex,
    at: StackText,
) -> Option<Vec<TextBlock>> {
    said(texts, card, at).map(|(blocks, _)| blocks)
}

/// Which of [`sentence`]'s two branches a sentence came from.
///
/// For the dev harness, which reports it beside a row's words so a driver
/// can tell a German row from a row that fell to English without reading
/// German; nothing that draws branches on it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Said {
    /// The player's language, from the served printing.
    Localized,
    /// The compiled English Oracle.
    Oracle,
}

/// [`sentence`], and which branch answered.
#[must_use]
pub fn said(
    texts: Option<&CardTexts>,
    card: CardIndex,
    at: StackText,
) -> Option<(Vec<TextBlock>, Said)> {
    let oracle = baylee_cards::oracle::face(card, usize::from(at.face))?;
    if baylee_core::oracle::sentence_count(oracle) != usize::from(at.of) {
        return None;
    }
    texts
        .and_then(|texts| texts.localized(card, at))
        .map(|line| (split_blocks(line), Said::Localized))
        .or_else(|| {
            baylee_cards::oracle::sentence(card, usize::from(at.face), at.line)
                .map(|line| (split_blocks(line), Said::Oracle))
        })
}

/// One face as the card prints it in English, out of the compiled registry:
/// the floor under every face drawn.
///
/// A face with no text is a face that says nothing, and before the Oracle was
/// compiled in that was every face in a game with no gateway — an offline
/// duel against the house showed blank cards unless Scryfall happened to be
/// reachable. The name is the printed name, which [`CardText::english_name`]
/// also is, so the clone guard in `CardFace::build` still compares like with
/// like.
#[must_use]
pub fn english(card: CardIndex, face: usize) -> Option<CardText> {
    let printed = baylee_cards::by_index(card)?.faces.get(face)?;
    Some(CardText {
        lang: "en".to_string(),
        name: printed.name.to_string(),
        english_name: printed.name.to_string(),
        type_line: baylee_cards::pool::type_line(printed),
        oracle_text: baylee_cards::oracle::face(card, face)
            .unwrap_or_default()
            .to_string(),
        mana_cost: printed.mana_cost.to_string(),
    })
}

/// How many cards one request names; the gateway reads the first 500.
const BATCH: usize = 500;

/// Asks the gateway about every card the view names that has not been asked
/// about in this language.
pub fn request(
    mut texts: ResMut<CardTexts>,
    duel: Res<crate::Duel>,
    settings: Res<crate::settings::ClientSettings>,
    gateway: Res<TextGateway>,
    time: Res<Time<Real>>,
) {
    if texts.lang != settings.lang {
        texts.relang(&settings.lang);
    }
    let (Some(statics), Some(view)) = (duel.statics.as_ref(), duel.view.as_ref()) else {
        return;
    };
    if texts.game != statics.game_id {
        // A new game may be on another gateway, or on one that is up now.
        texts.game.clone_from(&statics.game_id);
        texts.down = false;
        texts.walked = None;
        texts.scryfall.down = false;
        texts.scryfall.walked = None;
    }
    if texts.down || gateway.0.is_none() {
        // Signed in to no gateway, nothing at one would answer (#270).
        texts.down = true;
        ask_scryfall(&mut texts, view, time.elapsed_secs_f64());
        return;
    }
    if texts.lang == "en" || texts.waiting.is_some() || texts.walked == Some(view.seq) {
        return;
    }
    texts.walked = Some(view.seq);
    let wanted = unasked(view, &texts.asked);
    if wanted.is_empty() {
        return;
    }
    let ids: Vec<&str> = wanted
        .iter()
        .filter_map(|card| baylee_cards::by_index(*card))
        .map(|def| def.oracle_id)
        .collect();
    texts.asked.extend(wanted.iter().copied());
    if ids.is_empty() {
        // Cards this build's pool does not compile: a newer host's. There is
        // nothing to name them by, and nothing to draw them with either.
        return;
    }
    let Some(request) = text_request(gateway.0.as_ref(), &settings.lang, &ids) else {
        return;
    };
    let slot: Slot = Arc::default();
    let target = Arc::clone(&slot);
    ehttp::fetch(request, move |result| {
        let reply = match result {
            Ok(response) if response.ok => response
                .text()
                .and_then(|body| serde_json::from_str::<Vec<CardTextEntry>>(body).ok())
                .map_or(Reply::Failed, Reply::Answered),
            Ok(response) => {
                bevy::log::warn!(status = response.status, "card text request refused");
                Reply::Failed
            }
            Err(err) => {
                // Not an error worth interrupting a game for: every card
                // still draws its English Oracle.
                bevy::log::info!("card text unavailable: {err}");
                Reply::Failed
            }
        };
        if let Ok(mut slot) = target.lock() {
            *slot = Some(reply);
        }
    });
    texts.waiting = Some((slot, wanted));
    bevy::log::info!(cards = ids.len(), "requesting card text");
}

/// The cards `view` names that have not been asked about, each once, at most
/// one request's worth, in index order so a request is the same for the
/// same view.
fn unasked(view: &PlayerView, asked: &HashSet<CardIndex>) -> Vec<CardIndex> {
    let named: std::collections::BTreeSet<CardIndex> =
        wanted(view).filter(|card| !asked.contains(card)).collect();
    named.into_iter().take(BATCH).collect()
}

/// The cards whose text this seat may draw: the ones its view names
/// ([`PlayerView::cards`]), and the spell each of those links as prepared
/// (`AbilityDef::Prepared`), whose name and text a prepared cast's row
/// draws ([`crate::abilities::prepared_words`]).
///
/// The link is printed on the card that names it, so asking for it tells the
/// gateway nothing the view did not already say.
fn wanted(view: &PlayerView) -> impl Iterator<Item = CardIndex> + '_ {
    view.cards()
        .flat_map(|card| std::iter::once(card).chain(crate::abilities::prepared_spell(card)))
}

/// Files the answer when it arrives.
pub fn poll(mut texts: ResMut<CardTexts>) {
    let Some(reply) = texts
        .waiting
        .as_ref()
        .and_then(|(slot, _)| slot.lock().ok()?.take())
    else {
        return;
    };
    let Some((_, cards)) = texts.waiting.take() else {
        return;
    };
    // Cards the view named while this was out are asked about next frame.
    texts.walked = None;
    if texts.receive(reply, &cards) {
        cache::store(&texts.lang, &texts.filed_entries());
        bevy::log::info!(cards = texts.len(), "card text filed");
    }
}

/// How long the Scryfall door waits between two requests, in seconds.
///
/// Scryfall asks clients for no more than ten a second. A game asking while
/// it is being played is a guest there, and a hand of seven is two seconds
/// of it at this pace.
const PACE: f64 = 0.3;

/// Asks Scryfall about the next card the view names without text, if the
/// door is open; see [`Door`].
fn ask_scryfall(texts: &mut CardTexts, view: &PlayerView, now: f64) {
    if texts.scryfall.waiting.is_some() || now < texts.scryfall.next_at {
        return;
    }
    let Some(card) = texts.next_for_scryfall(view, now) else {
        texts.scryfall.walked = Some(view.seq);
        return;
    };
    texts.scryfall.tried.insert(card);
    texts.scryfall.next_at = now + PACE;
    let Some(request) =
        baylee_cards::by_index(card).and_then(|def| scryfall::search(def.oracle_id, &texts.lang))
    else {
        return;
    };
    let lang = texts.lang.clone();
    let slot: Slot = Arc::default();
    let target = Arc::clone(&slot);
    ehttp::fetch(request, move |result| {
        let reply = match result {
            Ok(response) if response.ok => Reply::Answered(
                response
                    .text()
                    .and_then(|body| scryfall::entry(&lang, body))
                    .into_iter()
                    .collect(),
            ),
            // A search that finds nothing is a 404: a card never printed in
            // this language. That is an answer, not an outage.
            Ok(response) if response.status == 404 => Reply::Answered(Vec::new()),
            Ok(response) => {
                bevy::log::warn!(status = response.status, "Scryfall refused a card search");
                Reply::Failed
            }
            Err(err) => {
                bevy::log::info!("Scryfall unavailable: {err}");
                Reply::Failed
            }
        };
        if let Ok(mut slot) = target.lock() {
            *slot = Some(reply);
        }
    });
    texts.scryfall.waiting = Some((slot, card));
}

/// Files the Scryfall door's answer when it arrives.
pub fn poll_scryfall(mut texts: ResMut<CardTexts>) {
    let Some(reply) = texts
        .scryfall
        .waiting
        .as_ref()
        .and_then(|(slot, _)| slot.lock().ok()?.take())
    else {
        return;
    };
    let Some((_, card)) = texts.scryfall.waiting.take() else {
        return;
    };
    texts.scryfall.walked = None;
    if texts.receive_scryfall(reply, card) {
        cache::store(&texts.lang, &texts.filed_entries());
        bevy::log::info!(cards = texts.len(), "card text filed from Scryfall");
    }
}

/// Scryfall, asked about one card's printings in one language.
///
/// Two doors ask it: the game's while the gateway does not answer
/// ([`Door`](super::Door)), and the deck builder's (`lobby::localization`)
/// when the gateway has no translated catalog. Both send [`search`] and read
/// the answer with [`entry`], which is the gateway's own rule
/// ([`baylee_cardtext::card_entry`]) over Scryfall's rows, so the lobby, the
/// game and the gateway pick a printing by one rule.
pub(crate) mod scryfall {
    use baylee_cardtext::{TextFace, TextPrinting};
    use baylee_client_core::card_face::CardTextEntry;

    /// Who is asking, which Scryfall asks every client to say. The art
    /// reader says it too (`crate::artreader`).
    pub(crate) const AGENT: &str = concat!("baylee-client/", env!("CARGO_PKG_VERSION"));

    /// The search for every printing of one card in one language.
    ///
    /// `None` for an id that is not a plain UUID, or a language with no
    /// letters: neither is pasted into a URL. What is pasted needs no
    /// escaping, since a UUID is hex and dashes and the language is kept to
    /// letters and digits (`zhs`, `ph`).
    ///
    /// `Accept` is said once, in the constructor: `ehttp::Headers::insert`
    /// appends, and `Request::get` has already written `*/*`.
    #[must_use]
    pub fn search(oracle_id: &str, lang: &str) -> Option<ehttp::Request> {
        let plain =
            !oracle_id.is_empty() && oracle_id.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
        let lang: String = lang.chars().filter(char::is_ascii_alphanumeric).collect();
        if !plain || lang.is_empty() {
            return None;
        }
        let url = format!(
            "https://api.scryfall.com/cards/search?unique=prints&order=released\
             &include_multilingual=true&q=oracleid%3A{oracle_id}%20lang%3A{lang}"
        );
        Some(ehttp::Request::new(
            ehttp::Method::GET,
            url,
            &[("Accept", "application/json"), ("User-Agent", AGENT)],
        ))
    }

    /// The printings in a Scryfall list answer, in the order
    /// [`baylee_cardtext::card_entry`] reads them: the catalog's `ORDER BY`.
    ///
    /// Newest first with an undated one last, then the shortest collector
    /// number, then the number, then the id. The last two compare bytes
    /// where Postgres compares by its collation, which can differ only
    /// between two printings of one day whose numbers are one length.
    #[must_use]
    pub fn printings(body: &str) -> Vec<TextPrinting> {
        let Ok(list) = serde_json::from_str::<Collection>(body) else {
            return Vec::new();
        };
        let mut printings: Vec<TextPrinting> =
            list.data.into_iter().map(Payload::printing).collect();
        printings.sort_by(|a, b| {
            b.released_at
                .cmp(&a.released_at)
                .then_with(|| {
                    a.collector_number
                        .chars()
                        .count()
                        .cmp(&b.collector_number.chars().count())
                })
                .then_with(|| a.collector_number.cmp(&b.collector_number))
                .then_with(|| a.scryfall_id.cmp(&b.scryfall_id))
        });
        printings
    }

    /// The entry the gateway would have served for the card this answer is
    /// about, in `lang`.
    #[must_use]
    pub fn entry(lang: &str, body: &str) -> Option<CardTextEntry> {
        baylee_cardtext::card_entry(lang, &printings(body))
    }

    /// A Scryfall list envelope (`{"data": […]}`).
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
        #[serde(default)]
        oracle_id: String,
        #[serde(default)]
        released_at: String,
        #[serde(default)]
        collector_number: String,
        #[serde(default)]
        layout: String,
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
        /// This printing as [`baylee_cardtext::card_entry`] reads one.
        fn printing(self) -> TextPrinting {
            let faces = if self.card_faces.is_empty() {
                vec![self.top.fields()]
            } else {
                self.card_faces.iter().map(Face::fields).collect()
            };
            TextPrinting {
                scryfall_id: self.id,
                oracle_id: self.oracle_id,
                lang: self.lang,
                released_at: self.released_at,
                collector_number: self.collector_number,
                layout: self.layout,
                faces,
            }
        }
    }

    impl Face {
        /// The face as Scryfall wrote it, for [`Payload::printing`].
        fn fields(&self) -> TextFace {
            TextFace {
                name: self.name.clone(),
                printed_name: self.printed_name.clone(),
                type_line: self.type_line.clone(),
                printed_type_line: self.printed_type_line.clone(),
                oracle_text: self.oracle_text.clone(),
                printed_text: self.printed_text.clone(),
                mana_cost: self.mana_cost.clone(),
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

/// Card text for tests elsewhere in the crate that are about what is
/// *drawn*: real cards of the pool, with the German their printings carry.
///
/// Real because the table is keyed by card and a sentence is placed against
/// the card's compiled Oracle, so a fixture filing invented words under an
/// invented index would test a path no game takes.
#[cfg(test)]
pub(crate) mod fixture {
    use baylee_client_core::card_face::{CardTextEntry, FaceText};
    use baylee_core::ids::CardIndex;
    use baylee_view::{PublicObject, RulesFace};

    /// A pool card by its English name.
    pub fn card(name: &str) -> CardIndex {
        baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"))
    }

    /// The German entry the gateway serves for a pool card with one face.
    /// `printed` is the face's German rules text, or `None` where only the
    /// name is translated.
    pub fn german(english: &str, name: &str, printed: Option<&str>) -> CardTextEntry {
        let def = baylee_cards::by_index(card(english)).expect("compiled");
        let oracle = baylee_cards::oracle::face(def.index, 0).unwrap_or_default();
        CardTextEntry {
            scryfall_id: String::new(),
            oracle_id: def.oracle_id.to_string(),
            lang: "de".to_string(),
            layout: "normal".to_string(),
            faces: vec![FaceText {
                name: name.to_string(),
                english_name: english.to_string(),
                type_line: baylee_cards::pool::type_line(&def.faces[0]),
                oracle_text: printed.unwrap_or(oracle).to_string(),
                mana_cost: def.faces[0].mana_cost.to_string(),
                printed: printed.map(str::to_string),
            }],
        }
    }

    /// `object` showing `card`, as the host sends a permanent: the card is
    /// both what it is and whose rules it has.
    pub fn showing(mut object: PublicObject, card: CardIndex) -> PublicObject {
        if let Some(shown) = object.card.as_mut() {
            shown.index = card;
        }
        object.rules = Some(RulesFace { card, face: 0 });
        object
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::card_face::FaceText;
    use baylee_client_core::test_support::{ViewBuilder, printed};

    /// The pool's Mind Stone and its oracle id.
    fn mind_stone_card() -> (CardIndex, &'static str) {
        let card = baylee_cards::decks::by_name("Mind Stone").expect("in the pool");
        (
            card,
            baylee_cards::by_index(card).expect("compiled").oracle_id,
        )
    }

    /// One served entry for a card, one face, named `name`.
    fn entry(oracle_id: &str, name: &str) -> CardTextEntry {
        CardTextEntry {
            oracle_id: oracle_id.to_string(),
            layout: "normal".to_string(),
            scryfall_id: String::new(),
            lang: "de".to_string(),
            faces: vec![FaceText {
                printed: None,
                name: name.to_string(),
                english_name: "Mind Stone".to_string(),
                type_line: "Artefakt".to_string(),
                oracle_text: String::new(),
                mana_cost: "{2}".to_string(),
            }],
        }
    }

    /// An entry is filed under the card its `oracle_id` names, whatever the
    /// case it arrives in: a case mismatch would cost the card its text in
    /// silence.
    #[test]
    fn an_entry_is_filed_under_the_card_its_oracle_id_names() {
        let (card, oracle_id) = mind_stone_card();
        let mut texts = CardTexts::default();
        assert_eq!(
            texts.absorb(vec![entry(
                &oracle_id.to_ascii_uppercase(),
                "Gedankenstein"
            )]),
            1
        );
        assert_eq!(
            texts.get(card, 0).map(|t| t.name),
            Some("Gedankenstein".to_string())
        );
    }

    /// An entry naming no card of this pool files nowhere — and every entry a
    /// cache wrote before text was keyed by card is one, because it named a
    /// printing and left `oracle_id` empty. Nothing filed, nothing to redraw.
    #[test]
    fn an_entry_naming_no_card_of_the_pool_is_dropped() {
        let mut texts = CardTexts::default();
        let before = texts.generation();
        let mut old = entry("", "Gedankenstein");
        old.scryfall_id = "b50fd971-3dd1-4878-889f-81e38970408c".to_string();
        assert_eq!(texts.absorb(vec![old, entry("not-a-card", "Nichts")]), 0);
        assert!(texts.is_empty());
        assert_eq!(texts.generation(), before);
    }

    /// A card with one face does not answer for a second one.
    #[test]
    fn a_missing_face_has_no_text() {
        let (card, oracle_id) = mind_stone_card();
        let texts = CardTexts::filed(entry(oracle_id, "Gedankenstein"));
        assert!(texts.get(card, 0).is_some());
        assert!(texts.get(card, 1).is_none());
    }

    /// The number a panel redraws on moves when a card's text is replaced,
    /// which a count of cards does not: the cache's entry, then the
    /// gateway's fresher one, is one card both times.
    #[test]
    fn a_refiled_card_moves_the_generation_and_not_the_count() {
        let (card, oracle_id) = mind_stone_card();
        let mut texts = CardTexts::filed(entry(oracle_id, "Gedankenstein"));
        let before = texts.generation();
        texts.absorb(vec![entry(oracle_id, "Geiststein")]);
        assert_eq!(texts.len(), 1);
        assert_ne!(texts.generation(), before);
        assert_eq!(
            texts.get(card, 0).map(|t| t.name),
            Some("Geiststein".to_string())
        );
    }

    /// The cache is replaced wholesale, so it is written from the table: a
    /// second answer must not leave the first answer's cards off the disk.
    #[test]
    fn the_cache_is_written_from_the_table_and_not_from_one_answer() {
        let (_, stone) = mind_stone_card();
        let forest =
            baylee_cards::by_index(baylee_cards::decks::by_name("Forest").expect("in the pool"))
                .expect("compiled")
                .oracle_id;
        let mut texts = CardTexts::default();
        texts.absorb(vec![entry(stone, "Gedankenstein")]);
        texts.absorb(vec![entry(forest, "Wald")]);
        let mut names: Vec<String> = texts
            .filed_entries()
            .iter()
            .map(|e| e.faces[0].name.clone())
            .collect();
        names.sort();
        assert_eq!(names, vec!["Gedankenstein".to_string(), "Wald".to_string()]);
    }

    /// A served face is what a card draws, where one has arrived.
    #[test]
    fn a_served_face_is_drawn_over_the_english_one() {
        let (card, oracle_id) = mind_stone_card();
        let texts = CardTexts::filed(entry(oracle_id, "Gedankenstein"));
        let face = texts.face(card, 0).expect("a face");
        assert_eq!(face.name, "Gedankenstein");
        assert_eq!(face.lang, "de");
    }

    /// And where none has — no gateway, English, a card nobody translated —
    /// the card draws its own English, with words on it. Before the floor, an
    /// offline duel drew every face blank.
    #[test]
    fn a_face_with_nothing_served_is_the_card_s_english() {
        let (card, _) = mind_stone_card();
        let face = CardTexts::default().face(card, 0).expect("a face");
        assert_eq!(face.name, "Mind Stone");
        assert_eq!(face.english_name, "Mind Stone");
        assert_eq!(face.lang, "en");
        assert_eq!(face.mana_cost, "{2}");
        assert!(face.type_line.contains("Artifact"), "{}", face.type_line);
        assert!(
            face.oracle_text.contains("Draw a card."),
            "{}",
            face.oracle_text
        );
        assert!(CardTexts::default().face(card, 1).is_none());
    }

    /// A request that failed gives its cards back, so the next game asks
    /// about them; one that was answered keeps even the cards it left out.
    #[test]
    fn a_failed_request_gives_its_cards_back() {
        let (card, oracle_id) = mind_stone_card();
        let other = CardIndex::new(u32::MAX - 1);
        let mut texts = CardTexts::default();
        texts.asked.extend([card, other]);
        assert!(!texts.receive(Reply::Failed, &[card]));
        assert!(texts.down);
        assert!(!texts.asked.contains(&card));
        assert!(texts.asked.contains(&other), "only the request's own cards");

        texts.asked.insert(card);
        let answered = Reply::Answered(vec![entry(oracle_id, "Gedankenstein")]);
        assert!(texts.receive(answered, &[card, other]));
        assert!(texts.asked.contains(&card) && texts.asked.contains(&other));
    }

    /// The search names the card and the language and nothing else, says
    /// `Accept` once and who is asking, and refuses what it would have to
    /// escape.
    #[test]
    fn a_scryfall_search_names_one_card_in_one_language() {
        let (_, oracle_id) = mind_stone_card();
        let request = scryfall::search(oracle_id, "de").expect("a plain id");
        assert!(
            request
                .url
                .ends_with(&format!("q=oracleid%3A{oracle_id}%20lang%3Ade")),
            "{}",
            request.url
        );
        assert!(request.url.contains("unique=prints"), "{}", request.url);
        assert_eq!(
            request.headers.get_all("accept").collect::<Vec<_>>(),
            vec!["application/json"]
        );
        assert!(
            request
                .headers
                .get("user-agent")
                .is_some_and(|agent| agent.starts_with("baylee-client/"))
        );
        assert!(scryfall::search("c97361b5 or lang:en", "de").is_none());
        assert!(scryfall::search("", "de").is_none());
        assert!(scryfall::search(oracle_id, "").is_none());
        let odd = scryfall::search(oracle_id, "d e&x=1").expect("letters remain");
        assert!(odd.url.ends_with("lang%3Adex1"), "{}", odd.url);
    }

    /// The printings come back in the catalog's order, which `card_entry`
    /// reads: newest first, an undated one last, then the shorter number,
    /// then the number, then the id.
    #[test]
    fn scryfall_printings_are_read_in_the_catalog_s_order() {
        let row = |id: &str, date: Option<&str>, number: &str| {
            let mut row = serde_json::json!({"id": id, "lang": "de", "collector_number": number,
                "name": "Mind Stone", "oracle_id": "c97361b5-af16-4a7b-af85-a429dbaf4ad2"});
            if let Some(date) = date {
                row["released_at"] = date.into();
            }
            row
        };
        let body = serde_json::json!({"data": [
            row("a", Some("2024-01-01"), "100"),
            row("bbb", Some("2024-01-01"), "20"),
            row("c", None, "1"),
            row("d", Some("2025-01-01"), "300"),
            row("aaa", Some("2024-01-01"), "20"),
        ]})
        .to_string();
        let printings = scryfall::printings(&body);
        let order: Vec<&str> = printings.iter().map(|p| p.scryfall_id.as_str()).collect();
        assert_eq!(order, ["d", "aaa", "bbb", "a", "c"]);
        assert_eq!(
            printings[0].oracle_id,
            "c97361b5-af16-4a7b-af85-a429dbaf4ad2"
        );
        assert_eq!(printings[0].faces.len(), 1);
    }

    /// The whole door on data: Scryfall's German printings of Mind Stone go
    /// through the gateway's own rule, and the entry it builds is filed and
    /// drawn. The glued c15 printing is newer; `pick` still takes the fic
    /// one, because its lines pair and the c15's do not.
    #[test]
    fn a_scryfall_answer_is_served_the_way_the_gateway_serves_it() {
        let (card, oracle_id) = mind_stone_card();
        let oracle = baylee_cards::oracle::face(card, 0).expect("an Oracle");
        let row = |id: &str, date: &str, printed: &str| {
            serde_json::json!({"id": id, "oracle_id": oracle_id, "lang": "de",
                "released_at": date, "collector_number": "1", "layout": "normal",
                "name": "Mind Stone", "printed_name": "Gedankenstein",
                "type_line": "Artifact", "mana_cost": "{2}",
                "oracle_text": oracle, "printed_text": printed})
        };
        let body = serde_json::json!({"object": "list", "data": [
            row("c15", "2025-12-01", C15),
            row("fic", "2025-06-13", FIC),
        ]})
        .to_string();
        let entry = scryfall::entry("de", &body).expect("an entry");
        assert_eq!(entry.scryfall_id, "fic");
        assert_eq!(entry.oracle_id, oracle_id);
        assert_eq!(entry.faces[0].printed.as_deref(), Some(FIC));

        let mut texts = CardTexts::default();
        assert_eq!(texts.absorb(vec![entry]), 1);
        assert_eq!(
            words(sentence(Some(&texts), card, DRAW)).as_deref(),
            Some("{1}, {T}, opfere dieses Artefakt: Ziehe eine Karte.")
        );
        assert!(scryfall::entry("de", "not json").is_none());
    }

    /// The door opens only where the gateway is down and the language is not
    /// English, asks one card at a time at its pace, skips cards that have
    /// text or were tried, and closes when Scryfall does not answer.
    /// #270: text is asked of the gateway the player is signed in to, with
    /// that session, and of no gateway without one; the token never prints.
    #[test]
    fn text_is_asked_of_the_signed_in_gateway_with_its_session() {
        assert!(
            text_request(None, "de", &["a"]).is_none(),
            "signed in nowhere"
        );
        let signed = SignedIn {
            base: "http://gw.example:28766/".to_string(),
            token: "s3cret-session".to_string(),
        };
        let request = text_request(Some(&signed), "de", &["a", "b"]).expect("signed in");
        assert_eq!(
            request.url,
            "http://gw.example:28766/catalog/text?lang=de&oracle_ids=a,b"
        );
        assert_eq!(
            request.headers.get("Authorization"),
            Some("Bearer s3cret-session")
        );
        assert!(
            !format!("{signed:?}").contains("s3cret"),
            "the token never prints"
        );
    }

    /// Signed in nowhere, the Scryfall door answers from the first view
    /// (#270), because a gateway would refuse the question; signed in, the
    /// gateway is asked first. A sign-in counts from the next game, as a
    /// gateway that is up again does. The view names no card, so nothing
    /// leaves.
    #[test]
    fn a_client_signed_in_nowhere_asks_scryfall_instead() {
        use bevy::ecs::system::RunSystemOnce;
        let signed = SignedIn {
            base: "http://gw.example:28766".to_string(),
            token: "tok".to_string(),
        };
        for (gateway, door) in [(None, true), (Some(signed), false)] {
            let mut app = App::new();
            app.init_resource::<CardTexts>()
                .init_resource::<Time<Real>>()
                .insert_resource(TextGateway(gateway.clone()))
                .insert_resource(crate::settings::ClientSettings {
                    lang: "de".to_string(),
                    ..Default::default()
                })
                .insert_resource(crate::Duel {
                    statics: Some(baylee_client_core::test_support::statics(0)),
                    view: Some(ViewBuilder::new(2).build()),
                    ..Default::default()
                });
            app.world_mut().run_system_once(request).expect("runs");
            let texts = app.world().resource::<CardTexts>();
            assert_eq!(texts.down, door, "{gateway:?}");
            assert!(texts.waiting.is_none(), "no card, no question");
            if gateway.is_none() {
                // Signed in by the next game: that game asks the gateway again.
                app.insert_resource(TextGateway(Some(SignedIn {
                    base: "http://gw.example:28766".to_string(),
                    token: "tok".to_string(),
                })));
                let mut next = baylee_client_core::test_support::statics(0);
                next.game_id = "next-game".to_string();
                app.world_mut().resource_mut::<crate::Duel>().statics = Some(next);
                app.world_mut().run_system_once(request).expect("runs");
                assert!(
                    !app.world().resource::<CardTexts>().down,
                    "the next game, signed in, asks the gateway first"
                );
            }
        }
    }

    #[test]
    fn the_scryfall_door_asks_only_while_the_gateway_is_down() {
        let (stone, oracle_id) = mind_stone_card();
        let view = ViewBuilder::new(2)
            .with_hand(vec![("Seven", 1, 7), ("Nine", 1, 9)])
            .with_battlefield(0, [printed(20, 0, "Mind Stone", 0)])
            .build();
        let mut view = view;
        view.battlefield[0] = crate::cardtext::fixture::showing(view.battlefield[0].clone(), stone);
        let mut texts = CardTexts::filed(entry(oracle_id, "Gedankenstein"));
        texts.lang = "de".to_string();

        assert_eq!(
            texts.next_for_scryfall(&view, 0.0),
            None,
            "the gateway is up"
        );
        texts.down = true;
        assert_eq!(texts.next_for_scryfall(&view, 0.0), Some(CardIndex::new(7)));
        texts.scryfall.tried.insert(CardIndex::new(7));
        assert_eq!(
            texts.next_for_scryfall(&view, 0.0),
            Some(CardIndex::new(9)),
            "Mind Stone has text, so it is not asked about"
        );
        texts.scryfall.next_at = 5.0;
        assert_eq!(texts.next_for_scryfall(&view, 4.9), None, "the pace");

        // A 404 is an answer: the card stays tried. A failure gives it back
        // and closes the door.
        texts.scryfall.tried.insert(CardIndex::new(9));
        assert!(!texts.receive_scryfall(Reply::Answered(Vec::new()), CardIndex::new(9)));
        assert!(texts.scryfall.tried.contains(&CardIndex::new(9)));
        assert!(!texts.receive_scryfall(Reply::Failed, CardIndex::new(9)));
        assert!(!texts.scryfall.tried.contains(&CardIndex::new(9)));
        assert_eq!(
            texts.next_for_scryfall(&view, 9.0),
            None,
            "Scryfall is down"
        );

        texts.scryfall.down = false;
        texts.lang = "en".to_string();
        assert_eq!(
            texts.next_for_scryfall(&view, 9.0),
            None,
            "English is compiled in"
        );
    }

    /// What is asked is a set: a card the view names twice is asked about
    /// once, a card already asked about is not asked again, and one request
    /// names no more than the gateway reads.
    #[test]
    fn a_card_is_asked_about_once() {
        let view = ViewBuilder::new(2)
            .with_hand(vec![("Seven", 1, 7), ("Nine", 1, 9)])
            .with_battlefield(
                0,
                [printed(20, 0, "Seven again", 7), printed(21, 1, "Three", 3)],
            )
            .build();
        let asked = HashSet::from_iter([CardIndex::new(9)]);
        assert_eq!(
            unasked(&view, &asked),
            vec![CardIndex::new(3), CardIndex::new(7)]
        );
        let crowd = ViewBuilder::new(2)
            .with_battlefield(
                0,
                (0..BATCH as u32 + 3).map(|n| printed(n, 0, "One of many", n as u16)),
            )
            .build();
        assert_eq!(unasked(&crowd, &HashSet::default()).len(), BATCH);
    }

    /// A prepared permanent's spell is asked about with it: its row draws
    /// the spell's name and text, and no view ever names that card.
    #[test]
    fn a_prepared_card_s_spell_is_asked_about_too() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, [crate::registry_printed(1, 0, "Emeritus of Woe")])
            .build();
        let emeritus = baylee_cards::decks::by_name("Emeritus of Woe").expect("in the pool");
        let tutor = baylee_cards::decks::by_name("Demonic Tutor").expect("in the pool");
        let mut asked = unasked(&view, &HashSet::default());
        asked.sort_unstable();
        let mut both = vec![emeritus, tutor];
        both.sort_unstable();
        assert_eq!(asked, both);
    }

    /// A two-faced printing writes its faces in `card_faces` and *also*
    /// carries a joined top level, so the faces have to win: a client that
    /// read the top level would draw one face's text on both sides.
    #[test]
    fn a_two_faced_printing_is_read_off_its_faces() {
        let printings = scryfall::printings(
            r#"{"data":[{"id":"ddd","lang":"de","name":"Delver of Secrets // Insectile Aberration",
            "type_line":"Creature — Human Wizard // Creature — Human Insect",
            "card_faces":[
              {"name":"Delver of Secrets","type_line":"Creature — Human Wizard",
               "mana_cost":"{U}","oracle_text":"At the beginning of your upkeep, look at the top card of your library."},
              {"name":"Insectile Aberration","type_line":"Creature — Human Insect",
               "mana_cost":"","oracle_text":"Flying"}]}]}"#,
        );
        assert_eq!(printings[0].faces.len(), 2);
        assert_eq!(printings[0].faces[1].name, "Insectile Aberration");
        assert_eq!(printings[0].faces[1].oracle_text.as_deref(), Some("Flying"));
    }

    /// Everything about the answer is somebody else's to change, so nothing
    /// in it may be able to take the game down: a body that is not JSON, an
    /// empty envelope and a record with no id or text all answer quietly.
    #[test]
    fn an_answer_that_makes_no_sense_costs_no_more_than_the_text() {
        assert!(scryfall::printings("not json at all").is_empty());
        assert!(scryfall::printings("{}").is_empty());
        assert!(scryfall::entry("de", "{}").is_none());
        let printings = scryfall::printings(r#"{"data":[{"lang":"de"}]}"#);
        assert_eq!(printings.len(), 1);
        assert_eq!(printings[0].scryfall_id, "");
        assert_eq!(printings[0].faces[0].oracle_text, None);
        let _quietly = scryfall::entry("de", r#"{"data":[{"lang":"de"}]}"#);
    }

    /// Mind Stone, with one German printing's text filed for it.
    fn mind_stone(printed: &str) -> (CardIndex, CardTexts) {
        let (card, oracle_id) = mind_stone_card();
        let mut served = entry(oracle_id, "Gedankenstein");
        served.faces[0].oracle_text = printed.to_string();
        served.faces[0].printed = Some(printed.to_string());
        (card, CardTexts::filed(served))
    }

    /// The words of a sentence, blocks joined.
    fn words(blocks: Option<Vec<TextBlock>>) -> Option<String> {
        blocks.map(|blocks| {
            blocks
                .iter()
                .map(TextBlock::text)
                .collect::<Vec<_>>()
                .join(" ")
        })
    }

    /// Mind Stone's second ability, as the line table places it.
    const DRAW: StackText = StackText {
        face: 0,
        line: 1,
        of: 2,
    };

    /// Mind Stone's German fic printing, read from the catalog 2026-09-24.
    const FIC: &str = "{T}: Erzeuge {C}.\n{1}, {T}, opfere dieses Artefakt: Ziehe eine Karte.";

    /// Its c15 printing, which prints the two abilities as one line.
    const C15: &str = "{T}: Erhöhe deinen Manavorrat um {1}.{1}, {T}, opfere den Gedankenstein: Ziehe eine Karte.";

    const ENGLISH: &str = "{1}, {T}, Sacrifice this artifact: Draw a card.";

    /// A translation that pairs with the line table is what is drawn.
    #[test]
    fn a_sentence_is_the_player_s_language_when_it_pairs() {
        let (card, texts) = mind_stone(FIC);
        assert_eq!(
            words(sentence(Some(&texts), card, DRAW)).as_deref(),
            Some("{1}, {T}, opfere dieses Artefakt: Ziehe eine Karte.")
        );
    }

    /// The other branch, by each of its three ways in: a translation that
    /// does not pair, an empty table, and no table at all. Each of them drew
    /// a blank row before the Oracle was compiled in.
    #[test]
    fn a_sentence_falls_to_the_english_oracle() {
        let (card, glued) = mind_stone(C15);
        assert_eq!(
            words(sentence(Some(&glued), card, DRAW)).as_deref(),
            Some(ENGLISH)
        );
        let empty = CardTexts::default();
        assert_eq!(
            words(sentence(Some(&empty), card, DRAW)).as_deref(),
            Some(ENGLISH)
        );
        assert_eq!(words(sentence(None, card, DRAW)).as_deref(), Some(ENGLISH));
    }

    /// `said` names the branch that answered, which is what the dev harness
    /// reports as a row's `source`: German that pairs is `Localized`, German
    /// that does not and no table at all are both `Oracle`.
    #[test]
    fn a_sentence_says_which_branch_it_came_from() {
        let branch = |texts: Option<&CardTexts>, card| said(texts, card, DRAW).map(|(_, b)| b);
        let (card, paired) = mind_stone(FIC);
        assert_eq!(branch(Some(&paired), card), Some(Said::Localized));
        let (card, glued) = mind_stone(C15);
        assert_eq!(branch(Some(&glued), card), Some(Said::Oracle));
        assert_eq!(branch(None, card), Some(Said::Oracle));
    }

    /// A sentence is looked up under the card it is printed on, and under no
    /// other: German filed for Mind Stone says nothing about Forest's line.
    #[test]
    fn a_sentence_is_looked_up_under_its_own_card() {
        let (_, texts) = mind_stone(FIC);
        let forest = baylee_cards::decks::by_name("Forest").expect("in the pool");
        let first = StackText {
            face: 0,
            line: 0,
            of: 1,
        };
        assert_eq!(
            words(sentence(Some(&texts), forest, first)),
            words(sentence(None, forest, first))
        );
    }

    /// A coordinate counted against another text draws nothing, in either
    /// language: the German pairs and the Oracle has the line, but the host
    /// said the face had three sentences and this build's Mind Stone has two.
    #[test]
    fn a_line_counted_against_other_text_draws_nothing() {
        let (card, texts) = mind_stone(FIC);
        let skewed = StackText { of: 3, ..DRAW };
        assert_eq!(sentence(Some(&texts), card, skewed), None);
        assert_eq!(sentence(None, card, skewed), None);
        assert!(sentence(None, card, DRAW).is_some(), "the counter-test");
    }

    /// No card prints a sentence past its last one, in any language, and the
    /// door invents none.
    #[test]
    fn a_line_nobody_prints_has_no_words() {
        let (card, texts) = mind_stone(FIC);
        let past = StackText { line: 2, ..DRAW };
        assert_eq!(sentence(Some(&texts), card, past), None);
        assert_eq!(sentence(None, card, StackText { face: 1, ..DRAW }), None);
    }
}
