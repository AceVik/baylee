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
    pending: Vec<(u16, String, Zone)>,
    /// Cards a loaded deck named that the pool does not have. Kept so the
    /// player is told, rather than losing them silently on the next save.
    missing: Vec<String>,
    dirty: bool,
    has_text: bool,
    /// The card whose full text is on screen, as a slot in the pool.
    inspecting: Option<usize>,
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

    /// How many copies of a pool card the deck holds.
    #[must_use]
    pub fn count_of(&self, slot: usize, zone: Zone) -> u16 {
        self.entries(zone)
            .iter()
            .find(|e| e.slot == slot)
            .map_or(0, |e| e.count)
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
        if filled >= MAX_DECK_CARDS {
            return false;
        }
        let entries = match zone {
            Zone::Main => &mut self.main,
            Zone::Side => &mut self.side,
        };
        match entries.iter_mut().find(|e| e.slot == slot) {
            Some(entry) if entry.count >= limit => return false,
            Some(entry) => entry.count += 1,
            None => {
                if entries.len() >= MAX_DECK_LINES {
                    return false;
                }
                entries.push(Entry { slot, count: 1 });
            }
        }
        self.sort_zone(zone);
        self.dirty = true;
        true
    }

    /// Removes one copy, dropping the row when the last one goes.
    pub fn remove(&mut self, slot: usize, zone: Zone) -> bool {
        let entries = match zone {
            Zone::Main => &mut self.main,
            Zone::Side => &mut self.side,
        };
        let Some(at) = entries.iter().position(|e| e.slot == slot) else {
            return false;
        };
        entries[at].count -= 1;
        if entries[at].count == 0 {
            entries.remove(at);
        }
        self.dirty = true;
        true
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

    // ---------------------------------------------------------- the wire

    /// One zone as the `"N Card Name"` rows the gateway stores.
    ///
    /// Always the English name: a deck saved by a player reading German has to
    /// be the same deck when the gateway resolves it against the registry.
    #[must_use]
    pub fn rows(&self, zone: Zone) -> Vec<String> {
        self.entries(zone)
            .iter()
            .filter_map(|entry| {
                let card = self.pool.get(entry.slot)?;
                Some(format!("{} {}", entry.count, card.english_name))
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
    pub fn load(&mut self, id: &str, name: &str, cards: &[String], sideboard: &[String]) {
        self.start_new();
        self.editing = Some(id.to_string());
        self.name = name.to_string();
        for (rows, zone) in [(cards, Zone::Main), (sideboard, Zone::Side)] {
            for row in rows {
                match parse_row(row) {
                    Some((count, card_name)) => self.pending.push((count, card_name, zone)),
                    // A malformed row will never resolve, whatever the pool
                    // holds, so it is missing right away.
                    None => self.missing.push(row.clone()),
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
        if self.pending.is_empty() {
            return;
        }
        let held = std::mem::take(&mut self.pending);
        for (count, name, zone) in held {
            match self.slot_of(&name) {
                Some(slot) => {
                    let entries = match zone {
                        Zone::Main => &mut self.main,
                        Zone::Side => &mut self.side,
                    };
                    match entries.iter_mut().find(|e| e.slot == slot) {
                        Some(entry) => entry.count = entry.count.saturating_add(count),
                        None => entries.push(Entry { slot, count }),
                    }
                }
                None if self.loaded() => self.missing.push(name),
                // No pool yet: keep holding it.
                None => self.pending.push((count, name, zone)),
            }
        }
        self.sort_zone(Zone::Main);
        self.sort_zone(Zone::Side);
    }
}

/// Splits a `"N Card Name"` row. `None` when it is not one.
fn parse_row(row: &str) -> Option<(u16, String)> {
    let (count, name) = row.split_once(' ')?;
    let count = count.trim().parse::<u16>().ok()?;
    Some((count, name.trim().to_string()))
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
        b.load("d", "Old", &["1 Black Lotus".to_string()], &[]);
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
        b.load("deck-1", "Burn", &["1 Forest".to_string()], &[]);
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
}
