//! The decks an AI account offers: files on the runner's machine, since a
//! runner session cannot read the account's library at Central (baylee-central
//! `docs/ai-players.md` §2). Each is frozen once as the native client
//! freezes a deck (notes dropped, the rows the commitment binds), so the
//! deck the host deals is the one Central was offered and the one the mind
//! is told it holds, and its commitment is computed exactly as a person's
//! search computes it (`baylee_cards::ranked::registration`).
//!
//! A [`Shelf`] holds the account's decks: per format the newest version in
//! its deck directory that passes ([`crate::deckbuild::history`]), checked
//! against the host's legality facts kept beside them
//! ([`crate::deckbuild::facts`]) where there are any; a newer version that
//! fails is passed over for the one before it, and the runner says why.
//! Every deck the shelf ever offered stays on it, so a match made with it
//! is played with it, however the directory changed since.

use crate::deckbuild::{facts::Facts, history};
use anyhow::{Context as _, anyhow, bail};
use baylee_account::RankedDeck;
use baylee_account::ranked::Format;
use baylee_cards::decks::LoadedDeck;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A deck frozen for ranked play.
#[derive(Clone)]
pub struct Frozen {
    /// What the host is sent (`POST /ranked/{match}/play`).
    pub deck: RankedDeck,
    /// The commitment Central matches it under.
    pub digest: [u8; 32],
    /// The same deck as the bridge tells its mind ([`crate::deck::Deck`]).
    pub bridge: crate::deck::Deck,
    /// The deck the commitment was computed from, for the host's checks.
    pub loaded: LoadedDeck,
}

impl std::fmt::Debug for Frozen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frozen")
            .field("format", &self.deck.format)
            .field("name", &self.bridge.name)
            .finish_non_exhaustive()
    }
}

impl Frozen {
    /// The deck file `path` (Baylee text, or anything a player can export),
    /// for `format`.
    ///
    /// # Errors
    /// An unreadable file, a deck of another format, or one that cannot be
    /// registered (a card this build does not know).
    pub fn from_file(path: &Path, format: Format) -> anyhow::Result<Self> {
        let read = crate::deck::Deck::from_file(path)?;
        let mut cards = read.cards.clone();
        // The commanders go in with the cards, as a shelf deck lists them;
        // the registration takes each one out again.
        cards.extend(read.commanders.iter().map(|name| format!("1 {name}")));
        Self::from_rows(
            &read.name,
            format.as_str(),
            format,
            &cards,
            &read.sideboard,
            &read.commanders,
        )
        .with_context(|| format!("the deck {}", path.display()))
    }

    /// The deck from its shelf rows.
    ///
    /// # Errors
    /// As [`Frozen::from_file`].
    pub fn from_rows(
        name: &str,
        shelf_format: &str,
        format: Format,
        cards: &[String],
        sideboard: &[String],
        commanders: &[String],
    ) -> anyhow::Result<Self> {
        if shelf_format != format.as_str() {
            bail!(
                "the deck «{name}» is a {shelf_format} deck, not {}",
                format.as_str()
            );
        }
        let plain = |rows: &[String]| -> anyhow::Result<Vec<String>> {
            rows.iter()
                .map(|line| {
                    let mut row = baylee_core::deckrow::parse(line)
                        .map_err(|_| anyhow!("the deck «{name}» has a row it cannot read"))?;
                    row.note = None;
                    Ok(row.to_string())
                })
                .collect()
        };
        let registration = baylee_cards::ranked::registration(
            format.as_str(),
            &plain(cards)?,
            &plain(sideboard)?,
            commanders,
        )
        .map_err(|e| anyhow!("the deck «{name}» cannot be registered: {e}"))?;
        let [cards, sideboard, commanders] =
            baylee_cards::ranked::playable_rows(&registration.deck)
                .map_err(|e| anyhow!("the deck «{name}»: {e}"))?;
        let mut bridge = crate::deck::Deck {
            name: name.to_owned(),
            cards: without_leaders(&cards, &commanders)?,
            sideboard: sideboard.clone(),
            commanders: commanders.clone(),
            list: crate::mind::DeckList::default(),
        };
        bridge
            .resolve()
            .with_context(|| format!("the deck «{name}» as the mind is told it"))?;
        Ok(Self {
            deck: RankedDeck {
                format,
                cards,
                sideboard,
                commanders,
            },
            digest: registration.digest,
            bridge,
            loaded: registration.deck,
        })
    }

    /// Whether the host would seat it, as far as its legality facts say:
    /// legal in the format by its catalog, and no card blocked now.
    ///
    /// # Errors
    /// Why it would not.
    pub fn admitted(&self, facts: &Facts) -> Result<(), String> {
        baylee_cards::ranked::admit(self.deck.format.as_str(), &self.loaded, &facts.table())?;
        let now = crate::deckbuild::facts::now_utc();
        for card in self
            .loaded
            .main
            .iter()
            .chain(&self.loaded.sideboard)
            .chain(&self.loaded.commanders)
        {
            if facts.blocked_at(card.index.get(), &now).is_some() {
                let name = baylee_cards::decks::whole_name(card.index).unwrap_or("a card");
                return Err(format!("{name} is blocked on the host"));
            }
        }
        Ok(())
    }
}

/// The main rows without one copy of each commander: the registered rows
/// list a commander among its cards, and the bridge's deck takes it by
/// name, once (as [`crate::deck::Deck::from_text`] does).
fn without_leaders(cards: &[String], commanders: &[String]) -> anyhow::Result<Vec<String>> {
    let mut leaders = commanders.to_vec();
    let mut out = Vec::with_capacity(cards.len());
    for line in cards {
        let mut row = baylee_core::deckrow::parse(line)
            .map_err(|_| anyhow!("a registered row the bridge cannot read"))?;
        if let Some(at) = leaders.iter().position(|name| *name == row.name) {
            leaders.remove(at);
            row.count = row.count.saturating_sub(1);
        }
        if row.count > 0 {
            out.push(row.to_string());
        }
    }
    Ok(out)
}

/// The account's decks: what it offers per format, and every deck it ever
/// offered, by commitment.
#[derive(Debug, Default)]
pub struct Shelf {
    /// The deck directory (`--decks`), or none for a fixed shelf.
    dir: Option<PathBuf>,
    /// The formats it offers in.
    formats: Vec<Format>,
    /// What it offers now, per format.
    offered: BTreeMap<Format, Arc<Frozen>>,
    /// Every deck it offered, by commitment.
    kept: HashMap<[u8; 32], Arc<Frozen>>,
    /// The path each offered version was read from, so an unchanged
    /// directory is not read again.
    read: BTreeMap<Format, PathBuf>,
    /// The versions passed over, under the facts' version they failed
    /// against: not read, or told, again.
    passed_over: HashSet<(PathBuf, Option<String>)>,
    /// Whether it was read once: a format with no deck is told the first
    /// time, and again only when it loses the deck it offered.
    refreshed: bool,
}

impl Shelf {
    /// A shelf of these decks, one per format, that never changes (a
    /// test's).
    ///
    /// # Errors
    /// Two decks of one format.
    pub fn fixed(decks: impl IntoIterator<Item = Frozen>) -> anyhow::Result<Self> {
        let mut shelf = Self::default();
        for deck in decks {
            let format = deck.deck.format;
            if shelf.offered.contains_key(&format) {
                bail!("two {} decks: an AI offers one per format", format.as_str());
            }
            shelf.formats.push(format);
            shelf.offer(format, Arc::new(deck));
        }
        Ok(shelf)
    }

    /// The decks in `dir` for `formats` (each `<dir>/<format>/vNNNN-….txt`,
    /// [`history`]); read with [`Shelf::refresh`].
    #[must_use]
    pub fn directory(dir: PathBuf, formats: Vec<Format>) -> Self {
        Self {
            dir: Some(dir),
            formats,
            ..Self::default()
        }
    }

    /// The deck directory, if it has one.
    #[must_use]
    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    fn offer(&mut self, format: Format, deck: Arc<Frozen>) {
        self.kept.insert(deck.digest, Arc::clone(&deck));
        self.offered.insert(format, deck);
    }

    /// Reads the directory again: per format the newest version that
    /// freezes and that `facts` admit (where there are facts), else the one
    /// before it. A format with no deck that passes offers none. Says what
    /// it passed over, and why, through `told`.
    pub fn refresh(&mut self, facts: Option<&Facts>, mut told: impl FnMut(String)) {
        let Some(dir) = self.dir.clone() else {
            return;
        };
        let stamp = facts.map(|f| f.version().to_owned());
        for format in self.formats.clone() {
            let mut chosen = None;
            for version in history::versions(&dir, format).into_iter().rev() {
                let key = (version.deck.clone(), stamp.clone());
                if self.passed_over.contains(&key) {
                    continue;
                }
                let current = self
                    .offered
                    .get(&format)
                    .filter(|_| self.read.get(&format) == Some(&version.deck))
                    .cloned();
                let deck = match current {
                    Some(deck) => deck,
                    None => match Frozen::from_file(&version.deck, format) {
                        Ok(frozen) => Arc::new(frozen),
                        Err(e) => {
                            told(format!("passing over {}: {e:#}", version.deck.display()));
                            self.passed_over.insert(key);
                            continue;
                        }
                    },
                };
                if let Some(Err(why)) = facts.map(|f| deck.admitted(f)) {
                    told(format!(
                        "passing over {}: the host would not seat it: {why}",
                        version.deck.display()
                    ));
                    self.passed_over.insert(key);
                    continue;
                }
                chosen = Some((version.deck, deck));
                break;
            }
            if let Some((path, deck)) = chosen {
                if self.read.get(&format) != Some(&path) {
                    told(format!(
                        "offering «{}» in {} ({})",
                        deck.bridge.name,
                        format.as_str(),
                        path.display()
                    ));
                }
                self.read.insert(format, path);
                self.offer(format, deck);
            } else {
                if self.offered.remove(&format).is_some() || !self.refreshed {
                    told(format!(
                        "no {} deck to offer in {}",
                        format.as_str(),
                        history::format_dir(&dir, format).display()
                    ));
                }
                self.read.remove(&format);
            }
        }
        self.refreshed = true;
    }

    /// What the account offers now: one deck per format.
    #[must_use]
    pub fn offers(&self) -> Vec<baylee_account::ranked::ai::Offer> {
        self.offered
            .iter()
            .map(|(format, deck)| baylee_account::ranked::ai::Offer {
                format: *format,
                deck: deck.digest,
            })
            .collect()
    }

    /// The deck the account offered under `digest`, if it ever did.
    #[must_use]
    pub fn by_digest(&self, digest: &[u8; 32]) -> Option<Arc<Frozen>> {
        self.kept.get(digest).cloned()
    }
}
