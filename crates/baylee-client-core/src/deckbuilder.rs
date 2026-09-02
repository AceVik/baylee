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
    /// Whether the card is printed on both sides, and so has a back to turn
    /// over in the preview.
    #[serde(default)]
    pub two_faced: bool,
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

mod builder;

#[cfg(test)]
mod tests;
