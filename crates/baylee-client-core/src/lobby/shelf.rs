//! The deck shelf (the shell design, `DESIGN-v5.md` §6): what a deck tile
//! shows and in which order, decided without a window.
//!
//! - A deck's **picture** is its first commander, else its signature card
//!   (WG-3); its **art** is that printing's Scryfall `art_crop` — and only
//!   with a known artist, who is credited beside it. No artist, no art: the
//!   tile shows its identity gradient alone (principle 4, `docs/legal.md`).
//! - The shelf's **order**: a search over name, commanders and format, then
//!   one of four sorts. A deck whose deletion is waiting for its Undo is not
//!   on the shelf.
//! - The tile's **lines**: the meta line (cards, sideboard, when saved) and
//!   the badge line (format, next game, what will not play), never one line.

use super::DeckSummary;
use crate::i18n::{Lang, Phrase};
use crate::images::{self, ArtSize, Face};
use baylee_core::deckdigest::Leader;

/// A picture a tile may show: Scryfall's `art_crop`, whole, and who painted
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeckArt {
    /// The `art_crop` address, against the art base in force.
    pub url: String,
    /// The artist, credited under the picture.
    pub artist: String,
}

/// The art for a printing, against `base`: `None` without an artist or
/// without a well-formed printing id.
#[must_use]
pub fn art_at(base: &str, picture: &Leader) -> Option<DeckArt> {
    let artist = picture.artist.trim();
    if artist.is_empty() {
        return None;
    }
    let url = images::art_url_at(base, &picture.scryfall_id, Face::Front, ArtSize::ArtCrop)?;
    Some(DeckArt {
        url,
        artist: artist.to_string(),
    })
}

impl DeckSummary {
    /// What pictures this deck: its first commander, else its signature.
    #[must_use]
    pub fn picture(&self) -> Option<&Leader> {
        self.leaders.first().or(self.signature.as_ref())
    }

    /// The art its tile shows, against the art base in force.
    #[must_use]
    pub fn art(&self) -> Option<DeckArt> {
        self.picture().and_then(|p| art_at(&images::art_base(), p))
    }
}

/// How the shelf is ordered.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Sort {
    /// Newest save first: the gateway's own order.
    #[default]
    LastSaved,
    /// By name, A to Z.
    Name,
    /// By format, then name.
    Format,
    /// By colour identity in WUBRG order, then name.
    Colours,
}

impl Sort {
    /// Every sort, in the menu's order.
    pub const ALL: [Self; 4] = [Self::LastSaved, Self::Name, Self::Format, Self::Colours];

    /// The sort's name in the menu.
    #[must_use]
    pub const fn phrase(self) -> Phrase {
        match self {
            Self::LastSaved => Phrase::DecksSortSaved,
            Self::Name => Phrase::DecksSortName,
            Self::Format => Phrase::DecksSortFormat,
            Self::Colours => Phrase::DecksSortColours,
        }
    }
}

/// Folds text for a search: lower case, the common accents off.
#[must_use]
pub fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

/// Whether a deck answers a folded query: name, commanders or format.
fn matches(deck: &DeckSummary, query: &str) -> bool {
    query.is_empty()
        || fold(&deck.name).contains(query)
        || deck.commanders.iter().any(|c| fold(c).contains(query))
        || fold(&deck.format).contains(query)
}

/// The identity's place in WUBRG order, colourless last.
fn colour_key(identity: &str) -> (usize, Vec<usize>) {
    let order = |c: char| "WUBRG".find(c).unwrap_or(5);
    let mut letters: Vec<usize> = identity.chars().map(order).collect();
    letters.sort_unstable();
    // Fewer colours first; colourless after the five mono decks' order.
    let count = if letters.is_empty() { 6 } else { letters.len() };
    (count, letters)
}

/// The shelf: the indices into `decks` that answer `query`, in `sort`'s
/// order, leaving out the deck named `hidden` (one waiting for its Undo).
#[must_use]
pub fn order(decks: &[DeckSummary], query: &str, sort: Sort, hidden: Option<&str>) -> Vec<usize> {
    let query = fold(query.trim());
    let mut shown: Vec<usize> = decks
        .iter()
        .enumerate()
        .filter(|(_, d)| hidden != Some(d.id.as_str()) && matches(d, &query))
        .map(|(i, _)| i)
        .collect();
    let name = |i: usize| fold(&decks[i].name);
    match sort {
        // The gateway lists newest first; a list from an older one keeps
        // its order, which is the same promise.
        Sort::LastSaved => shown.sort_by(|a, b| decks[*b].updated_at.cmp(&decks[*a].updated_at)),
        Sort::Name => shown.sort_by_key(|i| name(*i)),
        Sort::Format => shown.sort_by_key(|i| (fold(&decks[*i].format), name(*i))),
        Sort::Colours => shown.sort_by_key(|i| (colour_key(&decks[*i].identity), name(*i))),
    }
    shown
}

/// A deck format in the player's language: the store's spelling is a word
/// for machines.
#[must_use]
pub fn format_label(lang: Lang, format: &str) -> String {
    match format {
        "commander" => Phrase::FormatCommander.text(lang).to_string(),
        "freeform" | "" => Phrase::FormatFreeform.text(lang).to_string(),
        other => {
            let mut chars = other.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        }
    }
}

/// When a deck was saved, from `now` (unix seconds): "saved 2 days ago".
/// Empty when the gateway did not say (`then` is zero).
#[must_use]
pub fn saved_ago(lang: Lang, now: u64, then: u64) -> String {
    if then == 0 {
        return String::new();
    }
    let secs = now.saturating_sub(then);
    let n = |v: u64| v.to_string();
    match secs {
        0..60 => Phrase::SavedJustNow.text(lang).to_string(),
        60..3600 => Phrase::SavedMinutesAgo.fill(lang, &[&n(secs / 60)]),
        3600..86_400 => Phrase::SavedHoursAgo.fill(lang, &[&n(secs / 3600)]),
        86_400..172_800 => Phrase::SavedYesterday.text(lang).to_string(),
        _ => Phrase::SavedDaysAgo.fill(lang, &[&n(secs / 86_400)]),
    }
}

/// The tile's meta line: "100 · SB 0 · saved 2 days ago".
#[must_use]
pub fn meta_line(lang: Lang, deck: &DeckSummary, now: u64) -> String {
    let (main, side) = if deck.copies > 0 || deck.cards == 0 {
        (deck.copies.to_string(), deck.side_copies.to_string())
    } else {
        // An older gateway sends lines, not copies; they are still a count.
        (deck.cards.to_string(), deck.sideboard.to_string())
    };
    let mut line = Phrase::DeckMeta.fill(lang, &[&main, &side]);
    let saved = saved_ago(lang, now, deck.updated_at);
    if !saved.is_empty() {
        line.push_str(" · ");
        line.push_str(&saved);
    }
    line
}

/// The tile's badges, in order: its format, whether it is the next game's
/// deck, and how many of its cards will not play.
#[must_use]
pub fn badges(lang: Lang, deck: &DeckSummary, next: bool) -> Vec<String> {
    let mut said = vec![format_label(lang, &deck.format)];
    if next {
        said.push(Phrase::DecksNextGame.text(lang).to_string());
    }
    if deck.unplayable > 0 {
        let n = deck.unplayable.to_string();
        said.push(
            Phrase::counted(
                usize::try_from(deck.unplayable).unwrap_or(usize::MAX),
                Phrase::DecksUnplayableOne,
                Phrase::DecksUnplayableMany,
            )
            .fill(lang, &[&n]),
        );
    }
    said
}

/// Favourites (S-11): kept in the account's preferences by deck id, and a
/// deck that no longer exists is dropped from them.
#[must_use]
pub fn kept_favourites(favourites: &[String], decks: &[DeckSummary]) -> Vec<String> {
    favourites
        .iter()
        .filter(|id| decks.iter().any(|d| &d.id == *id))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::preset::Finish;

    const ID: &str = "f333ea01-124f-4125-87ab-609be40e774c";

    fn leader(artist: &str) -> Leader {
        Leader {
            name: "Weltenbaum".into(),
            scryfall_id: ID.into(),
            lang: "en".into(),
            finish: Finish::default(),
            has_back_image: false,
            artist: artist.into(),
        }
    }

    fn deck(name: &str, saved: u64, identity: &str, format: &str) -> DeckSummary {
        DeckSummary {
            id: format!("id-{name}"),
            name: name.into(),
            updated_at: saved,
            identity: identity.into(),
            format: format.into(),
            ..DeckSummary::default()
        }
    }

    /// No artist known, no art: the tile shows its gradient alone.
    #[test]
    fn a_picture_without_an_artist_has_no_art() {
        assert_eq!(art_at("https://cards.scryfall.io", &leader("")), None);
        assert_eq!(art_at("https://cards.scryfall.io", &leader("  ")), None);
        let art = art_at("https://cards.scryfall.io", &leader("Ryan Pancoast")).expect("credited");
        assert_eq!(art.artist, "Ryan Pancoast");
        assert_eq!(
            art.url,
            format!("https://cards.scryfall.io/art_crop/front/f/3/{ID}.jpg")
        );
        // A printing id that is not one asks for nothing.
        let mut bad = leader("Somebody");
        bad.scryfall_id = "nope".into();
        assert_eq!(art_at("https://cards.scryfall.io", &bad), None);
    }

    /// A commander pictures its deck; without one, the signature does.
    #[test]
    fn the_first_commander_pictures_the_deck_else_the_signature() {
        let mut d = deck("a", 0, "", "freeform");
        assert!(d.picture().is_none());
        d.signature = Some(leader("Sig"));
        assert_eq!(d.picture().map(|p| p.artist.as_str()), Some("Sig"));
        d.leaders.push(leader("Lead"));
        assert_eq!(d.picture().map(|p| p.artist.as_str()), Some("Lead"));
    }

    /// The four sorts, a search, and a deck waiting for its Undo.
    #[test]
    fn the_shelf_sorts_searches_and_hides_a_deck_being_deleted() {
        let decks = vec![
            deck("Weltenbaum", 300, "WUBRG", "commander"),
            deck("Schwarzrand", 100, "BR", "commander"),
            deck("Great Druid", 200, "G", "freeform"),
            deck("Ätherfluss", 50, "", "freeform"),
        ];
        assert_eq!(order(&decks, "", Sort::LastSaved, None), vec![0, 2, 1, 3]);
        assert_eq!(order(&decks, "", Sort::Name, None), vec![3, 2, 1, 0]);
        assert_eq!(order(&decks, "", Sort::Format, None), vec![1, 0, 3, 2]);
        // Mono green, then two colours, then five, then colourless.
        assert_eq!(order(&decks, "", Sort::Colours, None), vec![2, 1, 0, 3]);
        assert_eq!(order(&decks, "ather", Sort::Name, None), vec![3]);
        assert_eq!(order(&decks, "COMMANDER", Sort::Name, None), vec![1, 0]);
        assert_eq!(
            order(&decks, "", Sort::LastSaved, Some("id-Great Druid")),
            vec![0, 1, 3]
        );
    }

    #[test]
    fn a_save_reads_as_how_long_ago() {
        let day = 86_400;
        assert_eq!(saved_ago(Lang::En, 1_000, 0), "");
        assert_eq!(saved_ago(Lang::En, 1_000, 990), "saved just now");
        assert_eq!(
            saved_ago(Lang::En, 10_000, 10_000 - 5 * 60),
            "saved 5 min ago"
        );
        assert_eq!(
            saved_ago(Lang::En, 10 * day, 10 * day - day - 5),
            "saved yesterday"
        );
        assert_eq!(
            saved_ago(Lang::De, 10 * day, 7 * day),
            "vor 3 Tagen gespeichert"
        );
    }

    /// The badge line names what will not play only when something will not.
    #[test]
    fn only_a_deck_with_unplayable_cards_says_so() {
        let mut d = deck("x", 0, "", "commander");
        assert_eq!(badges(Lang::En, &d, false), vec!["Commander".to_string()]);
        d.unplayable = 2;
        assert_eq!(
            badges(Lang::En, &d, true),
            vec![
                "Commander".to_string(),
                "next game".to_string(),
                "2 cards won\u{2019}t play".to_string()
            ]
        );
        d.unplayable = 1;
        assert_eq!(badges(Lang::En, &d, false)[1], "1 card won\u{2019}t play");
    }

    #[test]
    fn a_favourite_of_a_deck_that_is_gone_is_dropped() {
        let decks = vec![deck("a", 0, "", "")];
        assert_eq!(
            kept_favourites(&["id-a".into(), "id-gone".into()], &decks),
            vec!["id-a".to_string()]
        );
    }
}
