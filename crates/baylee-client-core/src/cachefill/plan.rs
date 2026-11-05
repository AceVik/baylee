//! What the fill fetches, and in which order.
//!
//! What a draw reads, read off the code that draws it:
//!
//! - a deck builder row and everything on the table: `small`, the printing's
//!   front (`lobby::thumbnails`, `board.rs`), and its back for a card turned
//!   over;
//! - the hover preview and the card window: `normal`, front and back
//!   (`lobby::preview`, `buildui::cardwindow`);
//! - every library and every face-down card: the printed back at `normal`
//!   (`textures::BACK_SIZE`).
//!
//! So a plan is that, in two rounds: every thumbnail first, because a row is
//! what a player sees most of and a thumbnail is a seventh of a large picture,
//! then every large one. Inside a round it is the builder's own order, the
//! basic lands that go in almost every deck first and then the list as it
//! opens (`Sort::Name`). There is no cheap "newest" or "most used" to be had:
//! a [`Source`] comes from the compiled registry, which holds neither a
//! printing's date nor a count of decks.
//!
//! A card's other printings are left out: they are picked one card at a time
//! in the artwork dialog, and there are many times as many of them.

use super::Reach;
use crate::images::{ArtSize, Face, art_url_at, back_url_at};
use crate::prose::sort_key;

/// One card of the pool, as the fill needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Source<'a> {
    /// Its name, which sets the order.
    pub name: &'a str,
    /// The printing a row that picks nothing is drawn as.
    pub scryfall_id: &'a str,
    /// Whether that printing has a second picture.
    pub back: bool,
    /// Whether it is a basic land.
    pub basic: bool,
}

/// One pass over the pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Round {
    /// `small`: what a row and the table draw.
    Thumbnails,
    /// `normal`: what a preview draws.
    Large,
}

impl Round {
    /// Its place in a plan, from 0.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Thumbnails => 0,
            Self::Large => 1,
        }
    }
}

/// One picture to have on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    /// The URL it is fetched from.
    pub url: String,
    /// Which round it belongs to.
    pub round: Round,
    /// Whether it stands for its card in the count: a card's front does, and
    /// its back and the printed card back do not.
    pub card: bool,
}

/// Every picture for `sources` under `base`, in fetching order. The app's base
/// is always the CDN: a picture is filed by what it shows, so a draw under any
/// other base reads it all the same.
///
/// A card whose printing yields no URL is left out, as a draw leaves it out:
/// the id is not a Scryfall id, and asking would be a certain 404.
#[must_use]
pub fn plan(base: &str, sources: &[Source<'_>], reach: Reach) -> Vec<Picture> {
    let mut order: Vec<(bool, String, &Source<'_>)> = sources
        .iter()
        .map(|source| (!source.basic, sort_key(source.name), source))
        .collect();
    order.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    let mut pictures = vec![Picture {
        url: back_url_at(base, ArtSize::Normal),
        round: Round::Thumbnails,
        card: false,
    }];
    let mut round = |round: Round, size: ArtSize| {
        for (_, _, source) in &order {
            let picture = |face, card| {
                art_url_at(base, source.scryfall_id, face, size).map(|url| Picture {
                    url,
                    round,
                    card,
                })
            };
            pictures.extend(picture(Face::Front, true));
            if source.back {
                pictures.extend(picture(Face::Back, false));
            }
        }
    };
    round(Round::Thumbnails, ArtSize::Small);
    if reach == Reach::Everything {
        round(Round::Large, ArtSize::Normal);
    }
    pictures
}
