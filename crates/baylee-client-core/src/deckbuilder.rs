//! The deck builder: search a pool, fill two zones, know whether it is legal.
//!
//! Like [`crate::lobby`] this is the decisions only — no renderer, no HTTP —
//! so all of it is testable as arithmetic. The shell asks for a
//! [`crate::lobby::LobbyRequest`] and hands the answer back; everything
//! between a keystroke and a saved deck happens here.
//!
//! Two rules shape the design.
//!
//! **The pool is what the engine can play.** It arrives whole from
//! `GET /pool` — a few hundred rows — and every filter runs locally, so search
//! answers at keystroke latency and a player is never offered a card that
//! cannot be put in a deck. [`Coverage`] rides along, because "the engine
//! knows this card" and "the engine plays this card correctly" are different
//! claims and a builder that conflated them would be lying.
//!
//! **The rules here are the gateway's rules.** [`DeckBuilder::problems`]
//! mirrors what `POST /decks` enforces, separated into what would be *refused*
//! and what is merely worth saying. Saving must never surprise: if the button
//! is live, the deck saves.

use crate::lobby::{FieldKind, LobbyRequest};
use baylee_core::deckrow::{PrintChoice, Row};
use baylee_core::preset::Finish;
use serde::{Deserialize, Serialize};

/// Hard cap on a deck's expanded card count, matching the gateway.
pub const MAX_DECK_CARDS: u32 = 250;
/// Hard cap on how many distinct lines a deck may have, matching the gateway.
pub const MAX_DECK_LINES: usize = 250;
/// Copies of one card a deck may hold, unless it is a basic land.
pub const MAX_COPIES: u16 = 4;
/// The smallest constructed deck the rules allow. Advice, not a refusal — the
/// gateway saves a shorter one happily, and a half-built deck is a normal
/// thing to keep.
pub const MIN_CONSTRUCTED: u32 = 60;
/// The usual sideboard limit. Advice for the same reason.
pub const MAX_SIDEBOARD: u32 = 15;
/// Mana values the curve distinguishes; the last bucket is "that or more".
pub const CURVE_BUCKETS: usize = 8;

/// How completely the engine implements a card.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Coverage {
    /// Rules-complete and tested.
    Implemented,
    /// Playable, with a gap its author described.
    Partial,
    /// A stub. It exists in the registry, so a deck holding it saves, but the
    /// card does nothing.
    #[default]
    Unimplemented,
}

impl Coverage {
    /// Whether a deck holding this card plays as printed.
    #[must_use]
    pub fn trustworthy(self) -> bool {
        self == Self::Implemented
    }
}

/// One card in the playable pool, as `GET /pool` sends it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolCard {
    /// Registry index: the rules identity.
    pub index: u32,
    /// Name in the player's language.
    pub name: String,
    /// English name — what a saved deck line is written with.
    pub english_name: String,
    /// Mana cost in `{1}{W}` notation; empty for a land.
    pub mana_cost: String,
    /// Mana value.
    pub cmc: u32,
    /// The card's own colors, as `WUBRG` letters.
    pub colors: String,
    /// Color identity (CR 903.4).
    pub identity: String,
    /// Printed type line, in the player's language.
    pub type_line: String,
    /// Card types as English words, for grouping and filtering.
    pub kinds: Vec<String>,
    /// Power/toughness or loyalty, when the card prints either.
    pub stats: Option<String>,
    /// Rules text, when the gateway has a catalog.
    #[serde(default)]
    pub oracle_text: String,
    /// How completely the engine implements it.
    #[serde(default)]
    pub coverage: Coverage,
    /// Why a partial card is only partly there.
    #[serde(default)]
    pub note: Option<String>,
    /// Whether it may lead a commander deck.
    #[serde(default)]
    pub commander: bool,
    /// Basic lands are the one card a deck may hold any number of.
    #[serde(default)]
    pub basic_land: bool,
    /// The printing the registry names: the art key, and what a row that
    /// picks nothing is served as.
    #[serde(default)]
    pub scryfall_id: String,
    /// Rules identity — what `GET /printings` is keyed on.
    #[serde(default)]
    pub oracle_id: String,
    /// Every other name this card is printed under, across languages.
    ///
    /// The pool sends one row per card, not one per printing, so this is how
    /// a player who knows the card as "Blitzschlag" finds the row a deck
    /// stores as "Lightning Bolt". Empty when the gateway has no catalog.
    #[serde(default)]
    pub alt_names: Vec<String>,
}

impl PoolCard {
    /// Whether this card belongs to a type, by its English name.
    #[must_use]
    pub fn is(&self, kind: &str) -> bool {
        self.kinds.iter().any(|k| k == kind)
    }

    /// The group a deck list files this card under.
    #[must_use]
    pub fn group(&self) -> Group {
        // Order matters: a Land Creature is a land on the battlefield and a
        // land in a deck list, and an Artifact Creature is a creature.
        for (kind, group) in [
            ("Land", Group::Land),
            ("Creature", Group::Creature),
            ("Planeswalker", Group::Planeswalker),
            ("Instant", Group::Instant),
            ("Sorcery", Group::Sorcery),
            ("Artifact", Group::Artifact),
            ("Enchantment", Group::Enchantment),
            ("Battle", Group::Battle),
        ] {
            if self.is(kind) {
                return group;
            }
        }
        Group::Other
    }

    /// Which curve bucket the card falls in. Lands are not on the curve —
    /// they are what pays for it.
    #[must_use]
    pub fn bucket(&self) -> Option<usize> {
        if self.is("Land") {
            return None;
        }
        Some((self.cmc as usize).min(CURVE_BUCKETS - 1))
    }
}

/// One printing of a card, as `GET /printings` sends it.
///
/// A printing is a piece of cardboard, not a card: two of these with the same
/// `oracle_id` are the same card in the rules and two different things to
/// own. Every field defaults, because a gateway with no catalog answers with
/// one printing that knows only its own id.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Printing {
    /// Printing id — the art key, and the deck row's `scryfall=` form.
    #[serde(default)]
    pub scryfall_id: String,
    /// Rules identity, shared with every other printing of this card.
    #[serde(default)]
    pub oracle_id: String,
    /// Two-letter language code.
    #[serde(default)]
    pub lang: String,
    /// Set code, as a deck row writes it.
    #[serde(default)]
    pub set: String,
    /// Full set name.
    #[serde(default)]
    pub set_name: String,
    /// Collector number within the set.
    #[serde(default)]
    pub collector_number: String,
    /// Rarity.
    #[serde(default)]
    pub rarity: String,
    /// Release date, ISO-8601 — the order the carousel runs in.
    #[serde(default)]
    pub released_at: String,
    /// Illustrator.
    #[serde(default)]
    pub artist: String,
    /// Finishes it was sold in: `nonfoil`, `foil`, `etched`.
    #[serde(default)]
    pub finishes: Vec<String>,
    /// Frame treatments (`showcase`, `extendedart`, …).
    #[serde(default)]
    pub frame_effects: Vec<String>,
    /// Border color, `borderless` included.
    #[serde(default)]
    pub border_color: String,
    /// Front-face name in this printing's language.
    #[serde(default)]
    pub name: String,
    /// Whether it is a promo.
    #[serde(default)]
    pub promo: bool,
}

impl Printing {
    /// The finishes this printing was sold in, in the order a picker shows
    /// them, and never empty.
    ///
    /// A printing that names no finish at all was still sold plain — offering
    /// nothing would be a card that cannot be added in any form.
    #[must_use]
    pub fn offered(&self) -> Vec<Finish> {
        let mut out = Vec::new();
        for (tag, finish) in [
            ("nonfoil", Finish::Normal),
            ("foil", Finish::Foil),
            ("etched", Finish::Etched),
        ] {
            if self.finishes.iter().any(|f| f == tag) {
                out.push(finish);
            }
        }
        if out.is_empty() {
            out.push(Finish::Normal);
        }
        out
    }

    /// Whether it was sold in this finish.
    #[must_use]
    pub fn has(&self, finish: Finish) -> bool {
        self.offered().contains(&finish)
    }

    /// What the carousel writes under the art: set, number, and whatever
    /// makes this printing look different from the plain one.
    #[must_use]
    pub fn label(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if !self.set.is_empty() {
            parts.push(self.set.to_uppercase());
        }
        if !self.collector_number.is_empty() {
            parts.push(format!("#{}", self.collector_number));
        }
        for effect in &self.frame_effects {
            parts.push(pretty_effect(effect));
        }
        if self.border_color == "borderless" {
            parts.push("Borderless".to_string());
        }
        if self.promo {
            parts.push("Promo".to_string());
        }
        if parts.is_empty() {
            // The registry's own reference printing knows nothing but its id.
            return "This build's printing".to_string();
        }
        parts.join(" · ")
    }
}

/// Scryfall's frame-effect tags as words a player would recognise.
fn pretty_effect(effect: &str) -> String {
    match effect {
        "extendedart" => "Extended art".to_string(),
        "showcase" => "Showcase".to_string(),
        "inverted" => "Inverted".to_string(),
        "etched" => "Etched".to_string(),
        "fullart" => "Full art".to_string(),
        // Anything Scryfall adds later still reads as *something*, which is
        // better than a printing that looks identical to the one above it.
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

/// The sections a deck list is drawn in, in the order they are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    /// Creatures.
    Creature,
    /// Planeswalkers.
    Planeswalker,
    /// Instants.
    Instant,
    /// Sorceries.
    Sorcery,
    /// Artifacts that are not creatures.
    Artifact,
    /// Enchantments that are not creatures.
    Enchantment,
    /// Battles.
    Battle,
    /// Lands, last, as every deck list prints them.
    Land,
    /// Anything the list above does not name.
    Other,
}

impl Group {
    /// The heading this group is drawn under.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Creature => "Creatures",
            Self::Planeswalker => "Planeswalkers",
            Self::Instant => "Instants",
            Self::Sorcery => "Sorceries",
            Self::Artifact => "Artifacts",
            Self::Enchantment => "Enchantments",
            Self::Battle => "Battles",
            Self::Land => "Lands",
            Self::Other => "Other",
        }
    }
}

/// Which list a card is being put in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Zone {
    /// The deck itself.
    #[default]
    Main,
    /// Cards outside the game a seat may reach.
    Side,
}

/// How the result list is ordered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    /// Alphabetical.
    #[default]
    Name,
    /// Cheapest first, then alphabetical.
    Cost,
    /// Grouped by card type, then by cost.
    Type,
}

impl Sort {
    /// The label on the control that cycles the order.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "A–Z",
            Self::Cost => "Cost",
            Self::Type => "Type",
        }
    }

    /// The next order, so one control can cycle all three.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Cost,
            Self::Cost => Self::Type,
            Self::Type => Self::Name,
        }
    }
}

/// One row of a deck list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Where the card sits in the pool.
    pub slot: usize,
    /// How many copies.
    pub count: u16,
    /// The printing its owner chose, if they chose one.
    ///
    /// Two entries with the same slot and different printings are two rows
    /// — that is what a deck list says and what a collection holds. The copy
    /// limit does not follow: it is on the card.
    pub print: PrintChoice,
}

/// The printing picker: one card, every printing of it, and the choice.
///
/// The pool shows one row per *card* — a player asking "do I own this" wants
/// one answer, not one per set it appeared in. The picker is where the other
/// question is asked, and it is only ever open for one card at a time.
#[derive(Clone, Debug, Default)]
pub struct Picker {
    /// The pool slot being picked for.
    slot: usize,
    /// Which list the confirmed pick lands in.
    zone: Zone,
    /// Registry index, so an answer that arrives after the dialog was closed
    /// and reopened on another card is dropped instead of misfiled.
    card: u32,
    /// Every printing the gateway knew, newest set first.
    printings: Vec<Printing>,
    /// Distinct languages, in the order they first appear.
    langs: Vec<String>,
    /// Which language the carousel is limited to; `None` is all of them.
    lang: Option<String>,
    /// Where the carousel is, as an index into [`Picker::visible`].
    at: usize,
    /// The finish the pick will name.
    finish: Finish,
    /// Whether the answer is still in flight.
    loading: bool,
    /// Whether these came from a catalog, or are the one printing this build
    /// records. A picker that did not say so would imply a card was printed
    /// exactly once.
    from_catalog: bool,
}

impl Picker {
    /// The pool slot being picked for.
    #[must_use]
    pub fn slot(&self) -> usize {
        self.slot
    }

    /// Which list the pick lands in.
    #[must_use]
    pub fn zone(&self) -> Zone {
        self.zone
    }

    /// Whether the printings are still on their way.
    #[must_use]
    pub fn loading(&self) -> bool {
        self.loading
    }

    /// Whether a catalog answered, or this is the build's own printing.
    #[must_use]
    pub fn from_catalog(&self) -> bool {
        self.from_catalog
    }

    /// Every language these printings exist in, in first-seen order.
    #[must_use]
    pub fn langs(&self) -> &[String] {
        &self.langs
    }

    /// The language filter, or `None` for all of them.
    #[must_use]
    pub fn lang(&self) -> Option<&str> {
        self.lang.as_deref()
    }

    /// The chosen finish.
    #[must_use]
    pub fn finish(&self) -> Finish {
        self.finish
    }

    /// Where the carousel is.
    #[must_use]
    pub fn at(&self) -> usize {
        self.at
    }

    /// The printings the language filter admits, in carousel order.
    #[must_use]
    pub fn visible(&self) -> Vec<&Printing> {
        self.printings
            .iter()
            .filter(|p| self.lang.as_ref().is_none_or(|l| &p.lang == l))
            .collect()
    }

    /// How many the carousel can move through.
    #[must_use]
    pub fn len(&self) -> usize {
        self.visible().len()
    }

    /// Whether there is nothing to pick. Only true while loading, or when the
    /// language filter admits nothing — which the filter itself prevents.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The printing the carousel is on.
    #[must_use]
    pub fn current(&self) -> Option<&Printing> {
        let visible = self.visible();
        visible.get(self.at).copied()
    }

    /// The finishes the current printing can be had in.
    #[must_use]
    pub fn finishes(&self) -> Vec<Finish> {
        self.current().map(Printing::offered).unwrap_or_default()
    }

    /// Keeps the carousel and the finish inside what the current filter
    /// admits. Called after anything that changes either.
    fn settle(&mut self) {
        let len = self.len();
        if len == 0 {
            self.at = 0;
            return;
        }
        if self.at >= len {
            self.at = len - 1;
        }
        // A printing sold only plain must not stay marked as a foil pick:
        // the row would name a finish that was never printed.
        if let Some(printing) = self.current()
            && !printing.has(self.finish)
        {
            self.finish = Finish::Normal;
        }
    }
}

/// Something worth telling the player about the deck.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// Whether the gateway would refuse to save this. Advice is not a refusal:
    /// a deck of 12 cards is a perfectly good thing to keep working on.
    pub blocking: bool,
    /// What to show, written to be read as-is.
    pub message: String,
}

/// What a deck adds up to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Cards in the deck.
    pub main: u32,
    /// Cards in the sideboard.
    pub side: u32,
    /// Lands in the deck.
    pub lands: u32,
    /// Creatures in the deck.
    pub creatures: u32,
    /// Everything else in the deck.
    pub spells: u32,
    /// Cards in the deck the engine does not fully implement.
    pub shaky: u32,
}

/// A text box in the builder.
///
/// The builder owns its own caret for the same reason [`crate::lobby::Lobby`]
/// owns the sign-in form's: a browser shell has to hand the focused box to a
/// real `<input>`, and it can only do that if "which box" is a decision that
/// was made here rather than in the renderer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BuildField {
    /// The search box over the pool.
    #[default]
    Search,
    /// The deck's name.
    Name,
}

impl BuildField {
    /// What a platform should offer for this box. Both are plain names — a
    /// search box is not an address and never a password.
    #[must_use]
    pub fn kind(self) -> FieldKind {
        FieldKind::Name
    }
}

/// The deck builder's whole state.
#[derive(Clone, Debug, Default)]
pub struct DeckBuilder {
    pool: Vec<PoolCard>,
    /// Indices into `pool`, filtered and sorted. Rebuilt whenever a filter
    /// changes rather than on every draw: the shell redraws far more often
    /// than a player types.
    results: Vec<usize>,
    text: String,
    colors: Vec<char>,
    kind: Option<String>,
    cmc: Option<u32>,
    playable_only: bool,
    sort: Sort,
    main: Vec<Entry>,
    side: Vec<Entry>,
    zone: Zone,
    name: String,
    editing: Option<String>,
    /// Rows a loaded deck named that the pool cannot resolve *yet*, because
    /// the pool has not arrived. Held rather than dropped; see
    /// [`DeckBuilder::load`].
    pending: Vec<(u16, String, Zone, PrintChoice)>,
    /// Cards a loaded deck named that the pool does not have. Kept so the
    /// player is told, rather than losing them silently on the next save.
    missing: Vec<String>,
    dirty: bool,
    has_text: bool,
    /// The card whose full text is on screen, as a slot in the pool.
    inspecting: Option<usize>,
    /// The deck's commander, as a slot in the pool.
    ///
    /// A slot rather than a name so it survives a language change: the row a
    /// deck stores is the English name, and the pool is what maps between
    /// them. `None` for every deck that is not a commander deck, which is
    /// most of them.
    commander: Option<usize>,
    /// A loaded deck's commander name, until the pool can resolve it.
    pending_commander: Option<String>,
    /// The open printing picker, if a card is being picked for.
    picker: Option<Picker>,
    focus: BuildField,
    /// Bumped on every placement of the caret, including onto the box it is
    /// already in — a shell that raises a keyboard needs the tap, not the
    /// field. Mirrors [`crate::lobby::Lobby::focus_epoch`].
    focus_epoch: u64,
}

impl DeckBuilder {
    /// An empty builder with no pool yet.
    #[must_use]
    pub fn new() -> Self {
        Self {
            playable_only: true,
            name: String::new(),
            ..Self::default()
        }
    }

    /// Whether the pool has arrived.
    #[must_use]
    pub fn loaded(&self) -> bool {
        !self.pool.is_empty()
    }

    /// Whether the gateway could serve rules text. A builder that knows it
    /// cannot search rules text can say so once instead of looking broken.
    #[must_use]
    pub fn has_text(&self) -> bool {
        self.has_text
    }

    /// The pool, for a shell that needs to draw a row.
    #[must_use]
    pub fn pool(&self) -> &[PoolCard] {
        &self.pool
    }

    /// One pool card.
    #[must_use]
    pub fn card(&self, slot: usize) -> Option<&PoolCard> {
        self.pool.get(slot)
    }

    /// The filtered, sorted search results as pool slots.
    #[must_use]
    pub fn results(&self) -> &[usize] {
        &self.results
    }

    /// The current search text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The deck's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The id of the deck being edited, if this is not a new one.
    #[must_use]
    pub fn editing(&self) -> Option<&str> {
        self.editing.as_deref()
    }

    /// Whether anything has changed since the deck was loaded or saved.
    #[must_use]
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Which list an "add" goes to.
    #[must_use]
    pub fn zone(&self) -> Zone {
        self.zone
    }

    /// The colors the filter is restricted to, empty for "any".
    #[must_use]
    pub fn colors(&self) -> &[char] {
        &self.colors
    }

    /// The card type the filter is restricted to.
    #[must_use]
    pub fn kind(&self) -> Option<&str> {
        self.kind.as_deref()
    }

    /// The mana value the filter is restricted to.
    #[must_use]
    pub fn cmc(&self) -> Option<u32> {
        self.cmc
    }

    /// Whether cards the engine cannot play are hidden.
    #[must_use]
    pub fn playable_only(&self) -> bool {
        self.playable_only
    }

    /// The result order.
    #[must_use]
    pub fn sort(&self) -> Sort {
        self.sort
    }

    /// One zone's rows, in the order a deck list prints them.
    #[must_use]
    pub fn entries(&self, zone: Zone) -> &[Entry] {
        match zone {
            Zone::Main => &self.main,
            Zone::Side => &self.side,
        }
    }

    /// Cards a loaded deck named that the pool no longer has.
    #[must_use]
    pub fn missing(&self) -> &[String] {
        &self.missing
    }

    /// How many copies of a pool card the deck holds, across every
    /// printing of it.
    ///
    /// The copy limit is on the card: four Lightning Bolts are four
    /// Lightning Bolts however many different pieces of cardboard they are.
    #[must_use]
    pub fn count_of(&self, slot: usize, zone: Zone) -> u16 {
        self.entries(zone)
            .iter()
            .filter(|e| e.slot == slot)
            .fold(0u16, |sum, e| sum.saturating_add(e.count))
    }

    /// Where a card's first row sits in a zone's list.
    ///
    /// A card with two printings has two rows; this finds the first, which is
    /// what an action aimed at "this card" should act on.
    #[must_use]
    pub fn row_of(&self, slot: usize, zone: Zone) -> Option<usize> {
        self.entries(zone).iter().position(|e| e.slot == slot)
    }

    // ------------------------------------------------------------- the pool

    /// Takes the pool and rebuilds the results.
    pub fn set_pool(&mut self, cards: Vec<PoolCard>, has_text: bool) {
        self.pool = cards;
        self.has_text = has_text;
        // A deck may have been loaded before the pool arrived; its rows were
        // held by name and become real entries now.
        self.resolve_pending();
        self.refilter();
    }

    /// The pool slot holding a card, by its English name.
    #[must_use]
    pub fn slot_of(&self, english_name: &str) -> Option<usize> {
        self.pool
            .iter()
            .position(|c| c.english_name == english_name)
    }

    // ----------------------------------------------------------- the filter

    /// Sets the search text.
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.refilter();
    }

    /// Types one character into the search box.
    pub fn type_char(&mut self, ch: char) {
        self.text.push(ch);
        self.refilter();
    }

    /// Deletes the last character of the search box.
    pub fn backspace(&mut self) {
        self.text.pop();
        self.refilter();
    }

    /// Turns one color on or off. No colors means every color.
    pub fn toggle_color(&mut self, color: char) {
        if let Some(at) = self.colors.iter().position(|c| *c == color) {
            self.colors.remove(at);
        } else {
            self.colors.push(color);
        }
        self.refilter();
    }

    /// Restricts to one card type, or clears the restriction.
    pub fn set_kind(&mut self, kind: Option<&str>) {
        self.kind = kind.map(str::to_string);
        self.refilter();
    }

    /// Restricts to one mana value, or clears the restriction. This is what a
    /// click on a curve bar does.
    pub fn set_cmc(&mut self, cmc: Option<u32>) {
        self.cmc = if self.cmc == cmc { None } else { cmc };
        self.refilter();
    }

    /// Shows or hides the cards the engine cannot play.
    pub fn toggle_playable_only(&mut self) {
        self.playable_only = !self.playable_only;
        self.refilter();
    }

    /// Moves to the next result order.
    pub fn cycle_sort(&mut self) {
        self.sort = self.sort.next();
        self.refilter();
    }

    /// Clears every filter, including the text.
    pub fn clear_filters(&mut self) {
        self.text.clear();
        self.colors.clear();
        self.kind = None;
        self.cmc = None;
        self.refilter();
    }

    /// Whether anything is narrowing the results.
    #[must_use]
    pub fn filtered(&self) -> bool {
        !self.text.is_empty()
            || !self.colors.is_empty()
            || self.kind.is_some()
            || self.cmc.is_some()
    }

    /// Recomputes the result list.
    fn refilter(&mut self) {
        let needle = self.text.trim().to_lowercase();
        let mut hits: Vec<usize> = (0..self.pool.len())
            .filter(|slot| self.matches(&self.pool[*slot], &needle))
            .collect();
        let sort = self.sort;
        // `sort_by` rather than `sort_unstable_by`: every comparison ends in
        // the name, so the order is total, but a stable sort keeps it obvious
        // that two runs of the same filter cannot disagree.
        hits.sort_by(|a, b| {
            let (x, y) = (&self.pool[*a], &self.pool[*b]);
            match sort {
                Sort::Name => x.name.cmp(&y.name),
                Sort::Cost => x.cmc.cmp(&y.cmc).then_with(|| x.name.cmp(&y.name)),
                Sort::Type => x
                    .group()
                    .cmp(&y.group())
                    .then_with(|| x.cmc.cmp(&y.cmc))
                    .then_with(|| x.name.cmp(&y.name)),
            }
        });
        self.results = hits;
    }

    /// Whether one card survives the current filter.
    fn matches(&self, card: &PoolCard, needle: &str) -> bool {
        if self.playable_only && card.coverage == Coverage::Unimplemented {
            return false;
        }
        if let Some(kind) = &self.kind
            && !card.is(kind)
        {
            return false;
        }
        if let Some(cmc) = self.cmc
            && (card.is("Land") || card.cmc != cmc)
        {
            return false;
        }
        if !self.colors.is_empty() && !self.color_match(card) {
            return false;
        }
        if needle.is_empty() {
            return true;
        }
        // Every name the card answers to, in every language it was printed
        // in. A player searching for their own copy types what is on it.
        card.name.to_lowercase().contains(needle)
            || card.english_name.to_lowercase().contains(needle)
            || card
                .alt_names
                .iter()
                .any(|n| n.to_lowercase().contains(needle))
            || card.type_line.to_lowercase().contains(needle)
            || card.oracle_text.to_lowercase().contains(needle)
    }

    /// Whether a card is within the chosen colors.
    ///
    /// Within, not overlapping: picking W and U asks for the cards a
    /// white-blue deck could play, so a card that also needs black is out.
    /// Colorless cards belong to every deck and always survive.
    fn color_match(&self, card: &PoolCard) -> bool {
        card.identity.chars().all(|c| self.colors.contains(&c))
    }

    // -------------------------------------------------- the printing picker

    /// The open picker, if there is one.
    #[must_use]
    pub fn picker(&self) -> Option<&Picker> {
        self.picker.as_ref()
    }

    /// Opens the picker on a pool card, and asks the gateway for its
    /// printings.
    ///
    /// The dialog opens *before* the answer arrives, showing the printing the
    /// pool row already names: a picker that appeared only once the network
    /// answered would feel like a dropped tap.
    pub fn open_picker(&mut self, slot: usize, zone: Zone) -> Option<LobbyRequest> {
        let card = self.pool.get(slot)?;
        let reference = Printing {
            scryfall_id: card.scryfall_id.clone(),
            oracle_id: card.oracle_id.clone(),
            lang: "en".to_string(),
            name: card.english_name.clone(),
            ..Printing::default()
        };
        let index = card.index;
        self.picker = Some(Picker {
            slot,
            zone,
            card: index,
            langs: vec!["en".to_string()],
            printings: vec![reference],
            loading: true,
            ..Picker::default()
        });
        Some(LobbyRequest::LoadPrintings { card: index })
    }

    /// Closes the picker without adding anything.
    pub fn close_picker(&mut self) {
        self.picker = None;
    }

    /// The gateway's answer.
    ///
    /// Matched on the registry index rather than accepted blindly: a slow
    /// answer for a card the player has already moved on from would otherwise
    /// replace the printings of the one they are looking at.
    pub fn set_printings(&mut self, card: u32, printings: Vec<Printing>, from_catalog: bool) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        if picker.card != card {
            return;
        }
        picker.loading = false;
        picker.from_catalog = from_catalog;
        if printings.is_empty() {
            return;
        }
        let mut langs: Vec<String> = Vec::new();
        for printing in &printings {
            if !printing.lang.is_empty() && !langs.contains(&printing.lang) {
                langs.push(printing.lang.clone());
            }
        }
        picker.printings = printings;
        picker.langs = langs;
        picker.at = 0;
        picker.settle();
    }

    /// Moves the carousel, wrapping at both ends.
    ///
    /// Wrapping rather than stopping because the carousel is a ring of art
    /// with no beginning: a player flicking through twelve printings should
    /// not have to notice which one the list happened to start at.
    pub fn picker_step(&mut self, by: i32) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        let len = picker.len();
        if len == 0 {
            return;
        }
        let len_i = i64::try_from(len).unwrap_or(1);
        let at = i64::try_from(picker.at).unwrap_or(0);
        let next = (at + i64::from(by)).rem_euclid(len_i);
        picker.at = usize::try_from(next).unwrap_or(0);
        picker.settle();
    }

    /// Jumps the carousel to one printing.
    pub fn picker_go(&mut self, at: usize) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        picker.at = at;
        picker.settle();
    }

    /// Limits the carousel to one language, or to all of them.
    pub fn picker_set_lang(&mut self, lang: Option<&str>) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        picker.lang = lang.map(str::to_string);
        // The card the player was looking at is almost certainly not at the
        // same offset in a shorter list, so the carousel restarts rather than
        // landing somewhere arbitrary.
        picker.at = 0;
        picker.settle();
    }

    /// Chooses a finish, if the current printing was sold in it.
    pub fn picker_set_finish(&mut self, finish: Finish) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        if picker.current().is_some_and(|p| p.has(finish)) {
            picker.finish = finish;
        }
    }

    /// Adds the picked printing to the deck and closes the dialog.
    ///
    /// Returns whether it was added: the copy limit still applies, and it
    /// applies to the *card* — four Lightning Bolts are four Lightning Bolts
    /// however many different pieces of cardboard they are.
    pub fn picker_confirm(&mut self) -> bool {
        let Some(picker) = self.picker.as_ref() else {
            return false;
        };
        let (slot, zone) = (picker.slot, picker.zone);
        let choice = self.picked_choice();
        let added = self.add_print(slot, zone, choice);
        self.picker = None;
        added
    }

    /// The deck row the current pick writes.
    ///
    /// Narrow, not exhaustive: a row records what the player *chose*, and a
    /// choice that changes nothing writes nothing. Picking the default
    /// printing of a card leaves `4 Lightning Bolt` exactly as it was, which
    /// is what keeps a deck built before this feature existed from growing
    /// noise the first time it is saved.
    #[must_use]
    fn picked_choice(&self) -> PrintChoice {
        let Some(picker) = self.picker.as_ref() else {
            return PrintChoice::default();
        };
        let Some(printing) = picker.current() else {
            return PrintChoice::default();
        };
        let reference = self
            .pool
            .get(picker.slot)
            .map(|c| c.scryfall_id.as_str())
            .unwrap_or_default();

        let mut choice = PrintChoice {
            finish: (picker.finish != Finish::Normal).then_some(picker.finish),
            ..PrintChoice::default()
        };
        if !printing.lang.is_empty() && printing.lang != "en" {
            choice.lang = Some(printing.lang.clone());
        }
        if !printing.set.is_empty() {
            choice.set = Some(printing.set.to_uppercase());
            if !printing.collector_number.is_empty() {
                choice.collector_number = Some(printing.collector_number.clone());
            }
        } else if !printing.scryfall_id.is_empty() && printing.scryfall_id != reference {
            // No set to name it by, and not the printing the row would
            // resolve to anyway: the id is the only thing that pins it.
            choice.scryfall_id = Some(printing.scryfall_id.clone());
        }
        choice
    }

    // ------------------------------------------------------------ the deck

    /// Which list the next add goes to.
    pub fn set_zone(&mut self, zone: Zone) {
        self.zone = zone;
    }

    /// Adds one copy, up to what the format allows.
    ///
    /// Returns whether anything changed, so a shell can say why a click did
    /// nothing rather than looking broken.
    pub fn add(&mut self, slot: usize, zone: Zone) -> bool {
        self.add_print(slot, zone, PrintChoice::default())
    }

    /// Adds one copy of a card in a printing the player chose.
    ///
    /// Two copies with different printings are two rows, because that is what
    /// a deck list says and what a collection holds — but the copy limit is
    /// on the *card*: four Lightning Bolts are four Lightning Bolts however
    /// many different pieces of cardboard they are.
    pub fn add_print(&mut self, slot: usize, zone: Zone, print: PrintChoice) -> bool {
        let Some(card) = self.pool.get(slot) else {
            return false;
        };
        let limit = if card.basic_land {
            u16::MAX
        } else {
            MAX_COPIES
        };
        // The gateway caps each list on its own, so this does too: a full
        // main deck must not be what stops a sideboard being built.
        let counts = self.counts();
        let filled = match zone {
            Zone::Main => counts.main,
            Zone::Side => counts.side,
        };
        if filled >= MAX_DECK_CARDS || self.count_of(slot, zone) >= limit {
            return false;
        }
        let entries = match zone {
            Zone::Main => &mut self.main,
            Zone::Side => &mut self.side,
        };
        if let Some(entry) = entries
            .iter_mut()
            .find(|e| e.slot == slot && e.print == print)
        {
            entry.count += 1;
        } else {
            if entries.len() >= MAX_DECK_LINES {
                return false;
            }
            entries.push(Entry {
                slot,
                count: 1,
                print,
            });
        }
        self.dirty = true;
        self.sort_zone(zone);
        true
    }

    /// Removes one copy, dropping the row when the last one goes.
    ///
    /// From the *last* row of that card, so it undoes the most recent add:
    /// picking a foil and then changing your mind takes the foil back, not
    /// one of the plain copies that were already there.
    pub fn remove(&mut self, slot: usize, zone: Zone) -> bool {
        let entries = match zone {
            Zone::Main => &mut self.main,
            Zone::Side => &mut self.side,
        };
        let Some(at) = entries.iter().rposition(|e| e.slot == slot) else {
            return false;
        };
        Self::take_one(entries, at);
        self.dirty = true;
        true
    }

    /// Removes one copy from a named row of the deck list.
    ///
    /// The list addresses rows, not cards: two printings of the same card are
    /// two lines, and a player tapping one of them means that one.
    pub fn remove_at(&mut self, at: usize, zone: Zone) -> bool {
        let entries = match zone {
            Zone::Main => &mut self.main,
            Zone::Side => &mut self.side,
        };
        if at >= entries.len() {
            return false;
        }
        Self::take_one(entries, at);
        self.dirty = true;
        true
    }

    /// One copy off a row, and the row itself when that was the last.
    fn take_one(entries: &mut Vec<Entry>, at: usize) {
        entries[at].count -= 1;
        if entries[at].count == 0 {
            entries.remove(at);
        }
    }

    /// Empties both lists, keeping the name and the deck being edited.
    pub fn clear_deck(&mut self) {
        self.main.clear();
        self.side.clear();
        self.pending.clear();
        self.missing.clear();
        self.dirty = true;
    }

    /// Sets the deck's name.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
        self.dirty = true;
    }

    /// Types one character into the name.
    pub fn type_name(&mut self, ch: char) {
        self.name.push(ch);
        self.dirty = true;
    }

    /// Deletes the last character of the name.
    pub fn backspace_name(&mut self) {
        self.name.pop();
        self.dirty = true;
    }

    // ----------------------------------------------------------- one card

    /// The card whose full text is being read, if any.
    #[must_use]
    pub fn inspecting(&self) -> Option<usize> {
        self.inspecting
    }

    /// Opens a card. Reading one is a separate act from adding it: on a touch
    /// screen there is no hover to read with, and a builder where a card
    /// cannot be read is not one.
    pub fn inspect(&mut self, slot: usize) {
        self.inspecting = (slot < self.pool.len()).then_some(slot);
    }

    /// Closes it again.
    pub fn stop_inspecting(&mut self) {
        self.inspecting = None;
    }

    // ------------------------------------------------------------ the caret

    /// Which box the caret is in.
    #[must_use]
    pub fn focus(&self) -> BuildField {
        self.focus
    }

    /// How many times the caret has been placed. See [`BuildField`].
    #[must_use]
    pub fn focus_epoch(&self) -> u64 {
        self.focus_epoch
    }

    /// Puts the caret in a box.
    pub fn focus_on(&mut self, field: BuildField) {
        self.focus = field;
        self.focus_epoch = self.focus_epoch.wrapping_add(1);
    }

    /// Moves the caret to the other box.
    pub fn cycle_focus(&mut self) {
        self.focus_on(match self.focus {
            BuildField::Search => BuildField::Name,
            BuildField::Name => BuildField::Search,
        });
    }

    /// What the focused box holds.
    #[must_use]
    pub fn focused_text(&self) -> &str {
        match self.focus {
            BuildField::Search => &self.text,
            BuildField::Name => &self.name,
        }
    }

    /// Replaces the focused box wholesale, as a platform text field does.
    pub fn set_focused(&mut self, value: &str) {
        match self.focus {
            BuildField::Search => self.set_text(value),
            BuildField::Name => self.set_name(value),
        }
    }

    /// Types one character into the focused box.
    pub fn type_focused(&mut self, ch: char) {
        match self.focus {
            BuildField::Search => self.type_char(ch),
            BuildField::Name => self.type_name(ch),
        }
    }

    /// Deletes the last character of the focused box.
    pub fn backspace_focused(&mut self) {
        match self.focus {
            BuildField::Search => self.backspace(),
            BuildField::Name => self.backspace_name(),
        }
    }

    /// Keeps a zone in deck-list order: by group, then cost, then name.
    /// Files a zone's entries the way a deck list is printed.
    ///
    /// Same-card entries sort next to each other and then by the printing, so
    /// the plain copies come before the foils and the order does not shuffle
    /// between saves.
    fn sort_zone(&mut self, zone: Zone) {
        let pool = &self.pool;
        let entries = match zone {
            Zone::Main => &mut self.main,
            Zone::Side => &mut self.side,
        };
        entries.sort_by(|a, b| {
            let (x, y) = (&pool[a.slot], &pool[b.slot]);
            x.group()
                .cmp(&y.group())
                .then_with(|| x.cmc.cmp(&y.cmc))
                .then_with(|| x.name.cmp(&y.name))
                .then_with(|| print_key(&a.print).cmp(&print_key(&b.print)))
        });
    }

    // ------------------------------------------------------- what it adds up to

    /// What the deck adds up to.
    #[must_use]
    pub fn counts(&self) -> Counts {
        let mut counts = Counts::default();
        for entry in &self.main {
            let Some(card) = self.pool.get(entry.slot) else {
                continue;
            };
            let n = u32::from(entry.count);
            counts.main += n;
            if card.is("Land") {
                counts.lands += n;
            } else if card.is("Creature") {
                counts.creatures += n;
            } else {
                counts.spells += n;
            }
            if !card.coverage.trustworthy() {
                counts.shaky += n;
            }
        }
        for entry in &self.side {
            counts.side += u32::from(entry.count);
        }
        counts
    }

    /// The mana curve of the deck's non-land cards. The last bucket is
    /// "that mana value or more".
    #[must_use]
    pub fn curve(&self) -> [u16; CURVE_BUCKETS] {
        let mut curve = [0u16; CURVE_BUCKETS];
        for entry in &self.main {
            if let Some(card) = self.pool.get(entry.slot)
                && let Some(bucket) = card.bucket()
            {
                curve[bucket] = curve[bucket].saturating_add(entry.count);
            }
        }
        curve
    }

    /// Coloured mana symbols in the deck's costs, in `WUBRG` order — the
    /// number a mana base is actually built against.
    #[must_use]
    pub fn pips(&self) -> [u16; 5] {
        let mut pips = [0u16; 5];
        for entry in &self.main {
            let Some(card) = self.pool.get(entry.slot) else {
                continue;
            };
            for symbol in card.mana_cost.chars() {
                if let Some(at) = "WUBRG".find(symbol) {
                    pips[at] = pips[at].saturating_add(entry.count);
                }
            }
        }
        pips
    }

    /// Everything worth telling the player, refusals first.
    ///
    /// The blocking half is exactly what `POST /decks` enforces. Keeping the
    /// two in step is what lets the save button be trusted: if it is live, the
    /// deck saves.
    #[must_use]
    pub fn problems(&self) -> Vec<Problem> {
        let mut out = Vec::new();
        let counts = self.counts();
        let name = self.name.trim();
        if name.is_empty() {
            out.push(Problem {
                blocking: true,
                message: "The deck needs a name.".to_string(),
            });
        } else if name.len() > 64 {
            out.push(Problem {
                blocking: true,
                message: "That name is too long (64 characters at most).".to_string(),
            });
        }
        if self.main.is_empty() {
            out.push(Problem {
                blocking: true,
                message: "The deck is empty.".to_string(),
            });
        }
        if self.main.len() > MAX_DECK_LINES || self.side.len() > MAX_DECK_LINES {
            out.push(Problem {
                blocking: true,
                message: format!("At most {MAX_DECK_LINES} different cards per list."),
            });
        }
        if counts.main > MAX_DECK_CARDS || counts.side > MAX_DECK_CARDS {
            out.push(Problem {
                blocking: true,
                message: format!("At most {MAX_DECK_CARDS} cards in each list."),
            });
        }
        for name in &self.missing {
            out.push(Problem {
                blocking: true,
                message: format!("{name} is no longer in the card pool."),
            });
        }
        // Advice from here down. None of it stops a save.
        if counts.main > 0 && counts.main < MIN_CONSTRUCTED {
            out.push(Problem {
                blocking: false,
                message: format!(
                    "{} cards — a constructed deck wants at least {MIN_CONSTRUCTED}.",
                    counts.main
                ),
            });
        }
        if counts.side > MAX_SIDEBOARD {
            out.push(Problem {
                blocking: false,
                message: format!("A sideboard is usually at most {MAX_SIDEBOARD} cards."),
            });
        }
        if counts.main >= MIN_CONSTRUCTED && counts.lands * 3 < counts.main {
            out.push(Problem {
                blocking: false,
                message: format!(
                    "{} lands in {} cards is thin for this curve.",
                    counts.lands, counts.main
                ),
            });
        }
        if counts.shaky > 0 {
            out.push(Problem {
                blocking: false,
                message: format!(
                    "{} card(s) are not fully implemented yet and will not play as printed.",
                    counts.shaky
                ),
            });
        }
        out
    }

    /// Whether the deck would save.
    #[must_use]
    pub fn saveable(&self) -> bool {
        !self.problems().iter().any(|p| p.blocking)
    }

    // ------------------------------------------------------- the commander

    /// The deck's commander, as a slot in the pool.
    #[must_use]
    pub fn commander(&self) -> Option<usize> {
        self.commander
    }

    /// The commander's English name — what the gateway is told.
    #[must_use]
    pub fn commander_name(&self) -> Option<&str> {
        self.commander
            .and_then(|slot| self.pool.get(slot))
            .map(|card| card.english_name.as_str())
    }

    /// Makes a card the deck's commander.
    ///
    /// Refused for a card the rules cannot seat as one — the pool says which,
    /// and offering the choice on a card that would be rejected on save is
    /// worse than not offering it.
    ///
    /// A commander is also a card in the deck, so this puts one there if it
    /// is not already: choosing a leader that is not in the ninety-nine is a
    /// deck nobody meant to build.
    pub fn set_commander(&mut self, slot: usize) -> bool {
        if !self.pool.get(slot).is_some_and(|card| card.commander) {
            return false;
        }
        if self.count_of(slot, Zone::Main) == 0 {
            self.add(slot, Zone::Main);
        }
        if self.commander != Some(slot) {
            self.commander = Some(slot);
            self.dirty = true;
        }
        true
    }

    /// Takes the commander mark off, leaving the card in the deck.
    pub fn clear_commander(&mut self) {
        if self.commander.take().is_some() {
            self.dirty = true;
        }
    }

    // -------------------------------------------------- between the zones

    /// Moves one copy of an entry to the other zone, printing and all.
    ///
    /// Not remove-then-add at the call site, because that would drop the
    /// chosen printing: a foil moved to the sideboard has to arrive as the
    /// same piece of cardboard it left as.
    pub fn move_entry(&mut self, at: usize, from: Zone, to: Zone) -> bool {
        if from == to {
            return false;
        }
        let Some(entry) = self.entries(from).get(at) else {
            return false;
        };
        let (slot, print) = (entry.slot, entry.print.clone());
        if !self.remove_at(at, from) {
            return false;
        }
        self.add_print(slot, to, print)
    }

    // ---------------------------------------------------------- the wire

    /// One zone as the `"N Card Name"` rows the gateway stores.
    ///
    /// Always the English name: a deck saved by a player reading German has to
    /// be the same deck when the gateway resolves it against the registry.
    /// The deck as rows, in the form `docs/deck-format.md` specifies.
    ///
    /// This is the stored form *and* the exported form: what comes out here
    /// is what a player can paste into a text file, and what
    /// `baylee_core::deckrow::parse` reads back is this deck. A printing the
    /// player chose travels with the row.
    #[must_use]
    pub fn rows(&self, zone: Zone) -> Vec<String> {
        self.entries(zone)
            .iter()
            .filter_map(|entry| {
                let card = self.pool.get(entry.slot)?;
                Some(
                    Row {
                        count: u32::from(entry.count),
                        name: card.english_name.clone(),
                        print: entry.print.clone(),
                    }
                    .to_string(),
                )
            })
            .collect()
    }

    /// The request that saves this deck, or `None` when it would be refused.
    #[must_use]
    pub fn save(&self) -> Option<LobbyRequest> {
        if !self.saveable() {
            return None;
        }
        Some(LobbyRequest::SaveDeck {
            deck_id: self.editing.clone(),
            name: self.name.trim().to_string(),
            cards: self.rows(Zone::Main),
            sideboard: self.rows(Zone::Side),
            commander: self.commander_name().map(ToString::to_string),
        })
    }

    /// Marks the deck as saved.
    pub fn saved(&mut self, deck_id: Option<&str>) {
        self.dirty = false;
        // A new deck becomes the deck being edited the moment it has an id.
        // Without this the next save would post it a second time, and the
        // player would find two decks where they saved one.
        if let Some(id) = deck_id {
            self.editing = Some(id.to_string());
        }
    }

    /// Starts a new, empty deck.
    pub fn start_new(&mut self) {
        self.main.clear();
        self.side.clear();
        self.pending.clear();
        self.missing.clear();
        self.name.clear();
        self.editing = None;
        self.zone = Zone::Main;
        self.dirty = false;
        self.inspecting = None;
        self.commander = None;
        self.pending_commander = None;
        // A nameless deck cannot be saved, so that is where the caret starts.
        self.focus_on(BuildField::Name);
    }

    /// Loads a stored deck for editing.
    ///
    /// The rows are card *names*; resolving them takes the pool, and the pool
    /// may not have arrived yet — the two requests race. So a row that cannot
    /// be resolved is held, not dropped, and [`DeckBuilder::set_pool`] tries
    /// again. What is still unresolved once the pool is here is genuinely
    /// missing, and [`DeckBuilder::problems`] refuses to save over it: losing
    /// a card silently is the one outcome a deck builder must not have.
    pub fn load(
        &mut self,
        id: &str,
        name: &str,
        cards: &[String],
        sideboard: &[String],
        commander: Option<&str>,
    ) {
        self.start_new();
        self.editing = Some(id.to_string());
        self.name = name.to_string();
        // The commander is a name too, and races the pool the same way its
        // rows do.
        self.pending_commander = commander.map(ToString::to_string);
        for (rows, zone) in [(cards, Zone::Main), (sideboard, Zone::Side)] {
            for row in rows {
                match baylee_core::deckrow::parse(row) {
                    // The printing travels with the row: a deck reopened and
                    // saved again has to come back out the way it went in, or
                    // editing one line would quietly strip every other line's
                    // foils.
                    Ok(parsed) => self.pending.push((
                        u16::try_from(parsed.count).unwrap_or(u16::MAX),
                        parsed.name,
                        zone,
                        parsed.print,
                    )),
                    // A malformed row will never resolve, whatever the pool
                    // holds, so it is missing right away.
                    Err(_) => self.missing.push(row.clone()),
                }
            }
        }
        self.resolve_pending();
        self.dirty = false;
        // This one already has a name; what is wanted is the next card.
        self.focus_on(BuildField::Search);
    }

    /// Turns held rows into deck entries, as far as the pool allows.
    fn resolve_pending(&mut self) {
        if let Some(name) = self.pending_commander.clone()
            && let Some(slot) = self.slot_of(&name)
        {
            self.commander = Some(slot);
            self.pending_commander = None;
        }
        if self.pending.is_empty() {
            return;
        }
        let held = std::mem::take(&mut self.pending);
        for (count, name, zone, print) in held {
            match self.slot_of(&name) {
                Some(slot) => {
                    let entries = match zone {
                        Zone::Main => &mut self.main,
                        Zone::Side => &mut self.side,
                    };
                    // Rows merge only when they name the same printing; two
                    // that do not are two lines in the list they came from.
                    match entries
                        .iter_mut()
                        .find(|e| e.slot == slot && e.print == print)
                    {
                        Some(entry) => entry.count = entry.count.saturating_add(count),
                        None => entries.push(Entry { slot, count, print }),
                    }
                }
                None if self.loaded() => self.missing.push(name),
                // No pool yet: keep holding it.
                None => self.pending.push((count, name, zone, print)),
            }
        }
        self.sort_zone(Zone::Main);
        self.sort_zone(Zone::Side);
    }
}

/// A printing choice as something sortable.
///
/// `PrintChoice` is a bag of options with no natural order; a deck list needs
/// one, or two saves of the same deck would differ only in row order.
fn print_key(print: &PrintChoice) -> (String, String, String, u8) {
    (
        print.set.clone().unwrap_or_default(),
        print.collector_number.clone().unwrap_or_default(),
        print.lang.clone().unwrap_or_default(),
        match print.finish.unwrap_or_default() {
            Finish::Normal => 0,
            Finish::Foil => 1,
            Finish::Etched => 2,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(index: u32, name: &str, cost: &str, cmc: u32, kinds: &[&str]) -> PoolCard {
        let colors: String = cost.chars().filter(|c| "WUBRG".contains(*c)).collect();
        PoolCard {
            index,
            name: name.to_string(),
            english_name: name.to_string(),
            mana_cost: cost.to_string(),
            cmc,
            colors: colors.clone(),
            identity: colors,
            type_line: kinds.join(" "),
            kinds: kinds.iter().map(|k| (*k).to_string()).collect(),
            stats: None,
            oracle_text: String::new(),
            coverage: Coverage::Implemented,
            note: None,
            commander: false,
            basic_land: kinds.contains(&"Land") && name == "Forest",
            ..PoolCard::default()
        }
    }

    fn pool() -> Vec<PoolCard> {
        vec![
            card(1, "Forest", "", 0, &["Land"]),
            card(2, "Grizzly Bears", "{1}{G}", 2, &["Creature"]),
            card(3, "Lightning Bolt", "{R}", 1, &["Instant"]),
            card(4, "Wrath of God", "{2}{W}{W}", 4, &["Sorcery"]),
            card(5, "Sol Ring", "{1}", 1, &["Artifact"]),
        ]
    }

    fn builder() -> DeckBuilder {
        let mut b = DeckBuilder::new();
        b.set_pool(pool(), true);
        b.set_name("Test");
        b
    }

    /// Everything the builder offers has to come from the pool, and the pool
    /// is what the engine can play. A card that is only a stub is hidden by
    /// default: offering it would be offering a card that does nothing.
    #[test]
    fn a_stub_is_hidden_until_it_is_asked_for() {
        let mut b = DeckBuilder::new();
        let mut cards = pool();
        cards[4].coverage = Coverage::Unimplemented;
        b.set_pool(cards, true);
        assert_eq!(b.results().len(), 4);
        b.toggle_playable_only();
        assert_eq!(b.results().len(), 5, "asking for them shows them");
    }

    /// Search reaches the name a player knows, whichever of the two it is,
    /// and the rules text when the gateway had it.
    #[test]
    fn search_looks_where_a_player_would() {
        let mut b = builder();
        b.set_text("bears");
        assert_eq!(b.results().len(), 1);
        assert_eq!(
            b.card(b.results()[0]).unwrap().english_name,
            "Grizzly Bears"
        );
        b.set_text("instant");
        assert_eq!(b.results().len(), 1, "the type line is searched too");
        b.set_text("nothing at all");
        assert!(b.results().is_empty());
    }

    /// A color filter asks "what could a deck of these colors play", so a card
    /// needing a colour that was not picked is out, and colorless is in.
    #[test]
    fn colors_filter_to_what_a_deck_could_play() {
        let mut b = builder();
        b.toggle_color('G');
        let names: Vec<&str> = b
            .results()
            .iter()
            .map(|s| b.card(*s).unwrap().english_name.as_str())
            .collect();
        assert!(names.contains(&"Grizzly Bears"));
        assert!(names.contains(&"Forest"), "a land is colorless");
        assert!(names.contains(&"Sol Ring"), "so is an artifact");
        assert!(!names.contains(&"Lightning Bolt"));
        b.toggle_color('R');
        assert!(
            b.results()
                .iter()
                .any(|s| b.card(*s).unwrap().english_name == "Lightning Bolt"),
            "adding red admits it"
        );
    }

    /// Clicking a bar of the curve filters to that mana value, and clicking it
    /// again clears the filter — the same control both ways.
    #[test]
    fn a_curve_bar_filters_and_unfilters() {
        let mut b = builder();
        b.set_cmc(Some(1));
        let names: Vec<&str> = b
            .results()
            .iter()
            .map(|s| b.card(*s).unwrap().english_name.as_str())
            .collect();
        assert_eq!(names, vec!["Lightning Bolt", "Sol Ring"]);
        assert!(!names.contains(&"Forest"), "lands are not on the curve");
        b.set_cmc(Some(1));
        assert_eq!(b.results().len(), 5, "the same bar clears it");
    }

    /// Four of anything, any number of basics — the rule the gateway enforces,
    /// enforced here too so the player is told before they try to save.
    #[test]
    fn copies_are_capped_except_for_basics() {
        let mut b = builder();
        let bears = b.slot_of("Grizzly Bears").unwrap();
        for _ in 0..4 {
            assert!(b.add(bears, Zone::Main));
        }
        assert!(!b.add(bears, Zone::Main), "the fifth is refused");
        assert_eq!(b.count_of(bears, Zone::Main), 4);

        let forest = b.slot_of("Forest").unwrap();
        for _ in 0..20 {
            assert!(b.add(forest, Zone::Main));
        }
        assert_eq!(b.count_of(forest, Zone::Main), 20);
    }

    /// The deck and the sideboard are separate lists holding the same cards.
    #[test]
    fn the_two_zones_count_separately() {
        let mut b = builder();
        let bolt = b.slot_of("Lightning Bolt").unwrap();
        b.add(bolt, Zone::Main);
        b.add(bolt, Zone::Side);
        b.add(bolt, Zone::Side);
        assert_eq!(b.count_of(bolt, Zone::Main), 1);
        assert_eq!(b.count_of(bolt, Zone::Side), 2);
        let counts = b.counts();
        assert_eq!(counts.main, 1);
        assert_eq!(counts.side, 2);
    }

    /// Removing the last copy takes the row away rather than leaving a zero.
    #[test]
    fn the_last_copy_takes_its_row_with_it() {
        let mut b = builder();
        let bolt = b.slot_of("Lightning Bolt").unwrap();
        b.add(bolt, Zone::Main);
        assert_eq!(b.entries(Zone::Main).len(), 1);
        b.remove(bolt, Zone::Main);
        assert!(b.entries(Zone::Main).is_empty());
        assert!(
            !b.remove(bolt, Zone::Main),
            "and removing again does nothing"
        );
    }

    /// A deck list prints creatures first and lands last, whatever order the
    /// cards were added in.
    #[test]
    fn a_deck_list_is_in_deck_list_order() {
        let mut b = builder();
        for name in ["Forest", "Wrath of God", "Grizzly Bears", "Sol Ring"] {
            let slot = b.slot_of(name).unwrap();
            b.add(slot, Zone::Main);
        }
        let order: Vec<&str> = b
            .entries(Zone::Main)
            .iter()
            .map(|e| b.card(e.slot).unwrap().english_name.as_str())
            .collect();
        assert_eq!(
            order,
            vec!["Grizzly Bears", "Wrath of God", "Sol Ring", "Forest"]
        );
    }

    /// The curve counts spells by mana value and leaves lands out — they are
    /// what pays for the curve, not part of it.
    #[test]
    fn the_curve_counts_spells_and_not_lands() {
        let mut b = builder();
        let forest = b.slot_of("Forest").unwrap();
        for _ in 0..10 {
            b.add(forest, Zone::Main);
        }
        let bolt = b.slot_of("Lightning Bolt").unwrap();
        b.add(bolt, Zone::Main);
        b.add(bolt, Zone::Main);
        let bears = b.slot_of("Grizzly Bears").unwrap();
        b.add(bears, Zone::Main);
        let curve = b.curve();
        assert_eq!(curve[0], 0, "no lands on the curve");
        assert_eq!(curve[1], 2);
        assert_eq!(curve[2], 1);
        assert_eq!(b.counts().lands, 10);
    }

    /// Pips are what a mana base is built against, so hybrid and generic
    /// symbols must not be counted as coloured requirements.
    #[test]
    fn pips_count_coloured_symbols_only() {
        let mut b = builder();
        let wrath = b.slot_of("Wrath of God").unwrap();
        b.add(wrath, Zone::Main);
        // {2}{W}{W} — two white pips, and the {2} is not one of them.
        assert_eq!(b.pips(), [2, 0, 0, 0, 0]);
    }

    /// The save button may only be live when the save will succeed, so the
    /// blocking problems have to be the gateway's refusals exactly.
    #[test]
    fn a_deck_that_cannot_save_says_why() {
        let mut b = DeckBuilder::new();
        b.set_pool(pool(), true);
        assert!(!b.saveable(), "no name, no cards");
        let problems = b.problems();
        let blocking: Vec<&str> = problems
            .iter()
            .filter(|p| p.blocking)
            .map(|p| p.message.as_str())
            .collect();
        assert_eq!(blocking.len(), 2, "{blocking:?}");
        b.set_name("Mono Green");
        let forest = b.slot_of("Forest").unwrap();
        b.add(forest, Zone::Main);
        assert!(b.saveable());
        assert!(b.save().is_some());
    }

    /// Advice is not a refusal. A half-built deck is a normal thing to keep,
    /// and the builder has to let it be kept.
    #[test]
    fn advice_never_blocks_a_save() {
        let mut b = builder();
        let forest = b.slot_of("Forest").unwrap();
        b.add(forest, Zone::Main);
        assert!(b.saveable(), "one card is savable");
        let problems = b.problems();
        let advice: Vec<&Problem> = problems.iter().filter(|p| !p.blocking).collect();
        assert!(
            advice.iter().any(|p| p.message.contains("at least 60")),
            "the short deck is mentioned: {advice:?}"
        );
    }

    /// A deck is stored under English names whatever language it was built in,
    /// or the gateway would not resolve it against the registry.
    #[test]
    fn rows_are_written_in_english() {
        let mut b = DeckBuilder::new();
        let mut cards = pool();
        cards[1].name = "Grislibären".to_string();
        b.set_pool(cards, true);
        b.set_name("Deutsch");
        let bears = b.slot_of("Grizzly Bears").expect("found by English name");
        b.add(bears, Zone::Main);
        b.add(bears, Zone::Main);
        assert_eq!(b.rows(Zone::Main), vec!["2 Grizzly Bears"]);
    }

    /// Loading a stored deck reproduces it exactly, and a save afterwards
    /// updates that deck rather than creating a second one.
    #[test]
    fn a_loaded_deck_round_trips() {
        let mut b = builder();
        b.load(
            "deck-1",
            "Burn",
            &["4 Lightning Bolt".to_string(), "20 Forest".to_string()],
            &["2 Wrath of God".to_string()],
            None,
        );
        assert_eq!(b.name(), "Burn");
        assert_eq!(b.editing(), Some("deck-1"));
        assert!(!b.dirty(), "loading is not an edit");
        assert_eq!(b.counts().main, 24);
        assert_eq!(b.counts().side, 2);
        assert_eq!(
            b.rows(Zone::Main),
            vec!["4 Lightning Bolt".to_string(), "20 Forest".to_string()]
        );
        let Some(LobbyRequest::SaveDeck { deck_id, .. }) = b.save() else {
            panic!("a save request");
        };
        assert_eq!(deck_id.as_deref(), Some("deck-1"), "it updates the deck");
    }

    /// A card the pool no longer has is named, not dropped. Silently losing a
    /// card on the next save is the one outcome a deck builder must not have.
    #[test]
    fn a_card_the_pool_lost_is_reported_not_dropped() {
        let mut b = builder();
        b.load("d", "Old", &["1 Black Lotus".to_string()], &[], None);
        assert_eq!(b.missing(), ["Black Lotus"]);
        assert!(!b.saveable(), "and it refuses to save over the loss");
        assert!(
            b.problems()
                .iter()
                .any(|p| p.blocking && p.message.contains("Black Lotus"))
        );
    }

    /// A deck cannot grow past what the gateway will store, and the builder
    /// stops it at the click rather than at the save.
    #[test]
    fn each_lists_cap_holds_on_its_own() {
        let mut b = builder();
        let forest = b.slot_of("Forest").unwrap();
        for _ in 0..MAX_DECK_CARDS {
            b.add(forest, Zone::Main);
        }
        assert_eq!(b.counts().main, MAX_DECK_CARDS);
        assert!(!b.add(forest, Zone::Main), "and no further");
        assert!(b.saveable(), "at the cap it still saves");
        // The gateway caps each list separately, so a full main deck is not
        // what stops a sideboard being built.
        assert!(b.add(forest, Zone::Side), "the sideboard has its own room");
    }

    /// Starting a new deck forgets the one being edited, or the next save
    /// would quietly overwrite it.
    #[test]
    fn a_new_deck_is_not_the_old_one() {
        let mut b = builder();
        b.load("deck-1", "Burn", &["1 Forest".to_string()], &[], None);
        b.start_new();
        assert_eq!(b.editing(), None);
        assert!(b.name().is_empty());
        assert!(b.entries(Zone::Main).is_empty());
    }

    #[test]
    fn a_card_can_be_read_without_being_added() {
        let mut b = builder();
        b.inspect(0);
        assert_eq!(b.inspecting(), Some(0));
        assert!(
            b.entries(Zone::Main).is_empty(),
            "reading a card is not taking it"
        );
        b.stop_inspecting();
        assert_eq!(b.inspecting(), None);
        // A slot the pool does not have would draw a panel with nothing in it.
        b.inspect(9_999);
        assert_eq!(b.inspecting(), None);
        // And a new deck starts with nothing open.
        b.inspect(0);
        b.start_new();
        assert_eq!(b.inspecting(), None);
    }
    /// A player searches for the card they own, which is the card in their
    /// hand, which is not necessarily printed in English.
    #[test]
    fn a_card_is_found_under_any_name_it_was_printed_with() {
        let mut builder = DeckBuilder::default();
        builder.set_pool(
            vec![
                PoolCard {
                    index: 1,
                    name: "Lightning Bolt".to_string(),
                    english_name: "Lightning Bolt".to_string(),
                    alt_names: vec!["Blitzschlag".to_string(), "稲妻".to_string()],
                    kinds: vec!["Instant".to_string()],
                    ..PoolCard::default()
                },
                PoolCard {
                    index: 2,
                    name: "Counterspell".to_string(),
                    english_name: "Counterspell".to_string(),
                    alt_names: vec!["Gegenzauber".to_string()],
                    kinds: vec!["Instant".to_string()],
                    ..PoolCard::default()
                },
            ],
            false,
        );

        for needle in ["blitz", "稲妻", "Lightning"] {
            builder.set_text(needle);
            let hits = builder.results();
            assert_eq!(hits.len(), 1, "{needle} matched {} cards", hits.len());
            assert_eq!(hits[0], 0, "{needle} found the wrong card");
        }

        // One row per card, never one per name. "l" is in this card's English
        // name *and* in its German one; a builder that searched printings
        // would list it twice.
        builder.set_text("l");
        let hits = builder.results();
        assert_eq!(hits.len(), 2, "{hits:?}");
        assert_eq!(hits.iter().filter(|slot| **slot == 0).count(), 1);
    }
    /// A printing, as the picker's tests need one.
    fn printing(set: &str, number: &str, lang: &str, finishes: &[&str]) -> Printing {
        Printing {
            scryfall_id: format!("{set}-{number}-{lang}"),
            oracle_id: "bolt".to_string(),
            lang: lang.to_string(),
            set: set.to_string(),
            set_name: format!("Set {set}"),
            collector_number: number.to_string(),
            finishes: finishes.iter().map(|f| (*f).to_string()).collect(),
            name: "Lightning Bolt".to_string(),
            ..Printing::default()
        }
    }

    /// A builder holding one card, with the picker open on it.
    fn picking() -> DeckBuilder {
        let mut builder = DeckBuilder::new();
        builder.set_pool(
            vec![PoolCard {
                index: 7,
                name: "Lightning Bolt".to_string(),
                english_name: "Lightning Bolt".to_string(),
                oracle_id: "bolt".to_string(),
                scryfall_id: "reference".to_string(),
                kinds: vec!["Instant".to_string()],
                type_line: "Instant".to_string(),
                coverage: Coverage::Implemented,
                ..PoolCard::default()
            }],
            false,
        );
        let asked = builder.open_picker(0, Zone::Main);
        assert_eq!(asked, Some(LobbyRequest::LoadPrintings { card: 7 }));
        builder
    }

    /// The dialog opens before the answer arrives, or a tap would feel
    /// dropped. What it shows meanwhile is the printing the row already
    /// names.
    #[test]
    fn the_picker_has_something_to_show_while_it_waits() {
        let builder = picking();
        let picker = builder.picker().expect("the picker is open");
        assert!(picker.loading());
        assert!(!picker.from_catalog());
        assert_eq!(picker.len(), 1);
        assert_eq!(
            picker.current().map(|p| p.scryfall_id.as_str()),
            Some("reference")
        );
        assert_eq!(picker.finish(), Finish::Normal);
    }

    /// An answer for a card the player has already moved on from must not
    /// replace the printings of the one they are looking at.
    #[test]
    fn a_late_answer_for_another_card_is_dropped() {
        let mut builder = picking();
        builder.set_printings(999, vec![printing("m11", "149", "de", &["foil"])], true);
        let picker = builder.picker().expect("still open");
        assert!(
            picker.loading(),
            "the answer it is waiting for has not come"
        );
        assert_eq!(picker.current().map(|p| p.set.as_str()), Some(""));
    }

    /// The carousel is a ring: twelve printings have no beginning, and a
    /// player flicking through them should not have to notice which one the
    /// list happened to start at.
    #[test]
    fn the_carousel_wraps_at_both_ends() {
        let mut builder = picking();
        builder.set_printings(
            7,
            vec![
                printing("m11", "149", "en", &["nonfoil", "foil"]),
                printing("a25", "141", "en", &["nonfoil"]),
                printing("sta", "42", "ja", &["nonfoil", "etched"]),
            ],
            true,
        );
        let at = |b: &DeckBuilder| b.picker().and_then(Picker::current).map(|p| p.set.clone());

        assert_eq!(at(&builder).as_deref(), Some("m11"));
        builder.picker_step(-1);
        assert_eq!(at(&builder).as_deref(), Some("sta"), "back from the first");
        builder.picker_step(1);
        assert_eq!(at(&builder).as_deref(), Some("m11"), "and forward again");
        builder.picker_step(2);
        assert_eq!(at(&builder).as_deref(), Some("sta"));
    }

    /// A finish that was never printed must not survive a move to a printing
    /// that does not have it, or the row would name cardboard that does not
    /// exist.
    #[test]
    fn a_finish_does_not_outlive_the_printing_that_offered_it() {
        let mut builder = picking();
        builder.set_printings(
            7,
            vec![
                printing("m11", "149", "en", &["nonfoil", "foil"]),
                printing("a25", "141", "en", &["nonfoil"]),
            ],
            true,
        );
        builder.picker_set_finish(Finish::Foil);
        assert_eq!(builder.picker().map(Picker::finish), Some(Finish::Foil));

        builder.picker_step(1);
        assert_eq!(
            builder.picker().map(Picker::finish),
            Some(Finish::Normal),
            "this one was only ever sold plain"
        );
        // And it cannot be chosen while that printing is showing.
        builder.picker_set_finish(Finish::Foil);
        assert_eq!(builder.picker().map(Picker::finish), Some(Finish::Normal));
    }

    /// Filtering by language narrows the carousel and never leaves it
    /// pointing past the end.
    #[test]
    fn a_language_filter_narrows_the_carousel() {
        let mut builder = picking();
        builder.set_printings(
            7,
            vec![
                printing("m11", "149", "en", &["nonfoil"]),
                printing("a25", "141", "en", &["nonfoil"]),
                printing("sta", "42", "ja", &["nonfoil"]),
            ],
            true,
        );
        assert_eq!(
            builder.picker().map(Picker::langs),
            Some(&["en".to_string(), "ja".to_string()][..])
        );

        builder.picker_step(2);
        builder.picker_set_lang(Some("ja"));
        let picker = builder.picker().expect("open");
        assert_eq!(picker.len(), 1);
        assert_eq!(picker.at(), 0, "a shorter list starts over");
        assert_eq!(picker.current().map(|p| p.set.as_str()), Some("sta"));

        builder.picker_set_lang(None);
        assert_eq!(builder.picker().map(Picker::len), Some(3));
    }

    /// A choice that changes nothing writes nothing: picking the default
    /// printing leaves the row exactly as a deck built before any of this
    /// existed would have written it.
    #[test]
    fn picking_the_default_printing_writes_a_plain_row() {
        let mut builder = picking();
        builder.set_printings(7, Vec::new(), false);
        assert!(builder.picker_confirm());
        assert_eq!(builder.rows(Zone::Main), vec!["1 Lightning Bolt"]);
    }

    /// And a real pick writes every part of itself, in a form
    /// `baylee_core::deckrow::parse` reads back.
    #[test]
    fn a_picked_printing_reaches_the_deck_row() {
        let mut builder = picking();
        builder.set_printings(
            7,
            vec![printing("m11", "149", "de", &["nonfoil", "foil"])],
            true,
        );
        builder.picker_set_finish(Finish::Foil);
        assert!(builder.picker_confirm());
        assert!(builder.picker().is_none(), "confirming closes it");

        let rows = builder.rows(Zone::Main);
        assert_eq!(rows, vec!["1 Lightning Bolt (M11) 149 [de] *F*"]);
        // The row the builder writes is the row the parser reads.
        let parsed = baylee_core::deckrow::parse(&rows[0]).expect("round-trips");
        assert_eq!(parsed.name, "Lightning Bolt");
        assert_eq!(parsed.print.finish, Some(Finish::Foil));
        assert_eq!(parsed.print.lang.as_deref(), Some("de"));
    }

    /// Two printings of one card are two rows and still four copies: the
    /// limit is on the card, which is the rule the gateway enforces.
    #[test]
    fn a_second_printing_is_a_second_row_and_not_a_fifth_copy() {
        let mut builder = picking();
        for _ in 0..2 {
            builder.add(0, Zone::Main);
        }
        builder.set_printings(
            7,
            vec![printing("m11", "149", "en", &["nonfoil", "foil"])],
            true,
        );
        builder.picker_set_finish(Finish::Foil);
        assert!(builder.picker_confirm());

        assert_eq!(builder.count_of(0, Zone::Main), 3);
        assert_eq!(builder.entries(Zone::Main).len(), 2, "two rows");

        // Up to four, and no further.
        assert!(builder.add(0, Zone::Main));
        assert!(!builder.add(0, Zone::Main), "the fifth copy is refused");
        assert_eq!(builder.count_of(0, Zone::Main), 4);
    }

    /// A deck reopened and saved again has to come back out the way it went
    /// in — editing one line must not strip every other line's printing.
    #[test]
    fn a_loaded_deck_keeps_the_printings_it_was_saved_with() {
        let mut builder = picking();
        builder.close_picker();
        builder.load(
            "id",
            "Shiny",
            &[
                "2 Lightning Bolt (M11) 149 [de] *F*".to_string(),
                "1 Lightning Bolt".to_string(),
            ],
            &[],
            None,
        );
        assert!(builder.missing().is_empty(), "{:?}", builder.missing());
        assert_eq!(
            builder.entries(Zone::Main).len(),
            2,
            "two printings, two rows"
        );
        assert_eq!(builder.count_of(0, Zone::Main), 3);
        assert_eq!(
            builder.rows(Zone::Main),
            vec!["1 Lightning Bolt", "2 Lightning Bolt (M11) 149 [de] *F*"],
            "plain before foil, and stable between saves"
        );
    }

    /// Undoing an add takes back what was just added, not one of the copies
    /// that were already there.
    #[test]
    fn removing_takes_the_most_recent_printing_first() {
        let mut builder = picking();
        builder.close_picker();
        builder.load(
            "id",
            "Shiny",
            &[
                "1 Lightning Bolt".to_string(),
                "1 Lightning Bolt (M11) 149 *F*".to_string(),
            ],
            &[],
            None,
        );
        assert!(builder.remove(0, Zone::Main));
        assert_eq!(builder.rows(Zone::Main), vec!["1 Lightning Bolt"]);
    }

    /// A pool where one card may lead a deck and the rest may not.
    fn commander_pool() -> DeckBuilder {
        let mut cards = pool();
        let mut general = card(
            6,
            "Nissa, Who Shakes the World",
            "{3}{G}{G}",
            5,
            &["Legendary", "Planeswalker"],
        );
        general.commander = true;
        cards.push(general);
        let mut b = DeckBuilder::new();
        b.set_pool(cards, true);
        b.set_name("Test");
        b
    }

    /// The pool says which cards the rules can seat as a commander, and the
    /// gateway rejects the rest on save. Offering the choice on a card that
    /// would be refused is worse than not offering it at all.
    #[test]
    fn a_card_that_cannot_lead_a_deck_is_refused_as_its_commander() {
        let mut b = commander_pool();
        let bears = b.slot_of("Grizzly Bears").unwrap();
        assert!(!b.set_commander(bears));
        assert_eq!(b.commander(), None);
    }

    /// A commander is one of the cards in the deck, so naming one that is not
    /// in it yet puts it there — a leader outside the list is a deck nobody
    /// meant to build.
    #[test]
    fn naming_a_commander_seats_it_in_the_deck() {
        let mut b = commander_pool();
        let nissa = b.slot_of("Nissa, Who Shakes the World").unwrap();
        assert!(b.set_commander(nissa));
        assert_eq!(b.commander(), Some(nissa));
        assert_eq!(b.count_of(nissa, Zone::Main), 1);
        assert_eq!(b.commander_name(), Some("Nissa, Who Shakes the World"));
    }

    /// Clearing the mark leaves the card where it is: a player demoting their
    /// commander is not asking to lose the card.
    #[test]
    fn clearing_the_commander_keeps_the_card() {
        let mut b = commander_pool();
        let nissa = b.slot_of("Nissa, Who Shakes the World").unwrap();
        b.set_commander(nissa);
        b.clear_commander();
        assert_eq!(b.commander(), None);
        assert_eq!(b.count_of(nissa, Zone::Main), 1);
    }

    /// The commander rides the save request and comes back on load — and it
    /// is a *name* on the wire, so it races the pool exactly as the rows do.
    #[test]
    fn a_commander_survives_a_save_and_a_reload() {
        let mut b = commander_pool();
        let nissa = b.slot_of("Nissa, Who Shakes the World").unwrap();
        b.set_commander(nissa);
        let Some(crate::lobby::LobbyRequest::SaveDeck { commander, .. }) = b.save() else {
            panic!("a named deck with cards saves");
        };
        assert_eq!(commander.as_deref(), Some("Nissa, Who Shakes the World"));

        // Loaded into a builder whose pool has not arrived yet.
        let mut fresh = DeckBuilder::new();
        fresh.load(
            "d",
            "Superfriends",
            &["1 Nissa, Who Shakes the World".to_string()],
            &[],
            Some("Nissa, Who Shakes the World"),
        );
        assert_eq!(
            fresh.commander(),
            None,
            "no pool, nothing to resolve against"
        );
        let mut cards = pool();
        let mut general = card(
            6,
            "Nissa, Who Shakes the World",
            "{3}{G}{G}",
            5,
            &["Legendary", "Planeswalker"],
        );
        general.commander = true;
        cards.push(general);
        fresh.set_pool(cards, true);
        assert_eq!(fresh.commander_name(), Some("Nissa, Who Shakes the World"));
    }

    /// Moving a card between the lists must not quietly reprint it: the
    /// sideboard copy is the same piece of cardboard the deck held.
    #[test]
    fn a_card_moved_to_the_sideboard_keeps_its_printing() {
        let mut builder = picking();
        builder.picker_set_finish(Finish::Foil);
        assert!(builder.picker_confirm());
        let before = builder.rows(Zone::Main);
        assert_eq!(before.len(), 1);
        assert!(builder.move_entry(0, Zone::Main, Zone::Side));
        assert!(builder.rows(Zone::Main).is_empty());
        assert_eq!(builder.rows(Zone::Side), before);
    }

    /// A move to the zone the card is already in is not a move, and must not
    /// silently duplicate or drop it.
    #[test]
    fn moving_a_card_to_the_zone_it_is_in_does_nothing() {
        let mut b = builder();
        let forest = b.slot_of("Forest").unwrap();
        b.add(forest, Zone::Main);
        assert!(!b.move_entry(0, Zone::Main, Zone::Main));
        assert_eq!(b.count_of(forest, Zone::Main), 1);
    }
}
