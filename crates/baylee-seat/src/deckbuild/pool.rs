//! The cards a deck of one format may hold: those the engine plays
//! rules-complete, the host's catalog plays in that format, and its
//! operator has not blocked ([`super::check::Checker::playable`]). The
//! model searches and reads only these, and the deck generator lends from
//! them.

use super::check::{Checker, name_of};
use baylee_account::ranked::Format;
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use serde::Deserialize;
use std::fmt::Write as _;

/// The most cards one page of a search lists.
pub const PAGE: usize = 25;

/// The most of a card's rules text a search line carries, in characters.
const SNIPPET: usize = 180;

/// One card of the pool.
#[derive(Clone, Debug)]
pub struct Card {
    /// The card.
    pub index: CardIndex,
    /// The name a deck row spells it by.
    pub name: String,
    /// Its mana value (front face).
    pub mana_value: u32,
    /// Its colour identity.
    pub identity: ColorSet,
    /// Whether it may lead a Commander deck on its own or with a partner.
    pub leader: bool,
    /// Its type line (front face), lower case, for a search.
    types: String,
    /// Everything a search's words are found in, lower case.
    haystack: String,
    /// What a search shows of it.
    line: String,
}

/// The pool of one format.
#[derive(Clone, Debug)]
pub struct Pool {
    /// Its format.
    pub format: Format,
    /// Its cards, by name.
    cards: Vec<Card>,
}

/// A search the model asks for: every field narrows it.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Query {
    /// Words that must all be in the card's name, type line or rules text.
    pub text: Option<String>,
    /// Words that must all be in its type line (`legendary creature`,
    /// `instant`).
    pub types: Option<String>,
    /// The colours its identity keeps within, `WUBRG` letters; `C` for
    /// colourless only.
    pub colors: Option<String>,
    /// The least mana value.
    pub mv_min: Option<u32>,
    /// The most.
    pub mv_max: Option<u32>,
    /// Only cards that may be a commander.
    pub commander: Option<bool>,
    /// The page, from 1.
    pub page: Option<u32>,
}

/// `colors` as a colour identity: `WUBRG` letters, `C` for none.
///
/// # Errors
/// A letter that is none of them.
pub fn identity_of(colors: &str) -> Result<ColorSet, String> {
    let mut set = ColorSet::EMPTY;
    for letter in colors.chars().filter(|c| !c.is_whitespace() && *c != ',') {
        let color = match letter.to_ascii_uppercase() {
            'W' => Color::White,
            'U' => Color::Blue,
            'B' => Color::Black,
            'R' => Color::Red,
            'G' => Color::Green,
            'C' => continue,
            other => return Err(format!("«{other}» is no colour: use W, U, B, R, G or C")),
        };
        set = set.union(ColorSet::from_slice(&[color]));
    }
    Ok(set)
}

/// `identity` in `WUBRG` letters, `C` for none.
#[must_use]
pub fn letters(identity: ColorSet) -> String {
    let named: String = [
        (Color::White, 'W'),
        (Color::Blue, 'U'),
        (Color::Black, 'B'),
        (Color::Red, 'R'),
        (Color::Green, 'G'),
    ]
    .into_iter()
    .filter(|(color, _)| identity.contains(*color))
    .map(|(_, letter)| letter)
    .collect();
    if named.is_empty() { "C".into() } else { named }
}

impl Pool {
    /// The pool of `format` under `checker`'s facts, at `now`.
    #[must_use]
    pub fn of(checker: &Checker<'_>, format: Format, now: &str) -> Self {
        let mut cards: Vec<Card> = baylee_cards::all()
            .filter(|def| checker.playable(format, def.index, now))
            .map(card)
            .collect();
        cards.sort_by(|a, b| a.name.cmp(&b.name));
        Self { format, cards }
    }

    /// How many cards it holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// Whether it holds none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Its cards, by name.
    #[must_use]
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    /// Whether it holds `card`.
    #[must_use]
    pub fn holds(&self, card: CardIndex) -> bool {
        self.cards.iter().any(|c| c.index == card)
    }

    /// The cards `query` finds, on its page, and how many it finds in all.
    ///
    /// # Errors
    /// A colour that is none.
    pub fn search(&self, query: &Query) -> Result<(Vec<&Card>, usize), String> {
        let words = |text: &Option<String>| -> Vec<String> {
            text.as_deref()
                .unwrap_or_default()
                .split_whitespace()
                .map(str::to_lowercase)
                .collect()
        };
        let text = words(&query.text);
        let types = words(&query.types);
        let within = query.colors.as_deref().map(identity_of).transpose()?;
        let found: Vec<&Card> = self
            .cards
            .iter()
            .filter(|c| text.iter().all(|w| c.haystack.contains(w.as_str())))
            .filter(|c| types.iter().all(|w| c.types.contains(w.as_str())))
            .filter(|c| within.is_none_or(|w| c.identity.difference(w).is_empty()))
            .filter(|c| query.mv_min.is_none_or(|least| c.mana_value >= least))
            .filter(|c| query.mv_max.is_none_or(|most| c.mana_value <= most))
            .filter(|c| query.commander != Some(true) || c.leader)
            .collect();
        let total = found.len();
        let page = query.page.unwrap_or(1).max(1) as usize;
        Ok((
            found
                .into_iter()
                .skip((page - 1) * PAGE)
                .take(PAGE)
                .collect(),
            total,
        ))
    }

    /// What a search shows: one line a card, and how many there are.
    ///
    /// # Errors
    /// As [`Pool::search`].
    pub fn search_text(&self, query: &Query) -> Result<String, String> {
        let (cards, total) = self.search(query)?;
        let page = query.page.unwrap_or(1).max(1) as usize;
        let pages = total.div_ceil(PAGE).max(1);
        let mut out = format!(
            "{total} cards of the {} pool match; page {page} of {pages}.\n",
            self.format.as_str()
        );
        for card in cards {
            out.push_str(&card.line);
            out.push('\n');
        }
        Ok(out)
    }

    /// A card's whole text, when the pool holds it; else why not.
    #[must_use]
    pub fn read(&self, name: &str) -> String {
        let name = name.trim();
        let Some(index) = baylee_cards::decks::by_name(name).or_else(|| {
            let folded = crate::lookup::fold(name);
            self.cards
                .iter()
                .find(|c| crate::lookup::fold(&c.name) == folded)
                .map(|c| c.index)
        }) else {
            return format!(
                "«{name}»: no card of this name in the {} pool",
                self.format.as_str()
            );
        };
        let Some(card) = self.cards.iter().find(|c| c.index == index) else {
            return format!(
                "«{}» is not in the {} pool: not legal there, blocked, or not fully scripted",
                name_of(index),
                self.format.as_str()
            );
        };
        let text = crate::lookup::compiled(&card.name).unwrap_or_default();
        let lead = if card.leader {
            " · may be a commander"
        } else {
            ""
        };
        format!(
            "{} (identity {}{lead})\n{}",
            card.name,
            letters(card.identity),
            text.trim_end()
        )
    }
}

/// The name a deck row spells `def` by: its whole name where the printing
/// gives it two faces, else its own.
fn row_name(def: &baylee_cards::dsl::CardDef) -> String {
    baylee_cards::decks::whole_name(def.index)
        .map_or_else(|| def.name().to_string(), str::to_string)
}

fn card(def: &'static baylee_cards::dsl::CardDef) -> Card {
    let face = &def.faces[0];
    let name = row_name(def);
    let leader = baylee_cards::decks::leader_of(def.index).is_some_and(|l| l.eligible());
    let mut rules = String::new();
    let mut haystack = name.to_lowercase();
    for (at, face) in def.faces.iter().enumerate() {
        let type_line = baylee_cards::pool::type_line(face);
        let _ = write!(
            haystack,
            "\n{}\n{}",
            face.name.to_lowercase(),
            type_line.to_lowercase()
        );
        if let Some(text) = baylee_cards::oracle::face(def.index, at) {
            if !rules.is_empty() {
                rules.push_str(" // ");
            }
            rules.push_str(&text.split_whitespace().collect::<Vec<_>>().join(" "));
            haystack.push('\n');
            haystack.push_str(&text.to_lowercase());
        }
    }
    let type_line = baylee_cards::pool::type_line(face);
    let mut line = name.clone();
    let cost = face.mana_cost.to_string();
    if !cost.is_empty() {
        let _ = write!(line, " {cost}");
    }
    let _ = write!(line, " · {type_line}");
    if let Some(stats) = baylee_cards::pool::stats(face) {
        let _ = write!(line, " · {stats}");
    }
    let _ = write!(line, " · {}", letters(def.color_identity));
    if leader {
        line.push_str(" · commander");
    }
    if !rules.is_empty() {
        let snippet: String = rules.chars().take(SNIPPET).collect();
        let cut = if snippet.len() < rules.len() {
            "…"
        } else {
            ""
        };
        let _ = write!(line, " — {snippet}{cut}");
    }
    Card {
        index: def.index,
        name,
        mana_value: face.mana_cost.cmc(),
        identity: def.color_identity,
        leader,
        types: type_line.to_lowercase(),
        haystack,
        line,
    }
}
