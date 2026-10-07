//! Cards another seat revealed, held up for this seat to see.
//!
//! A reveal shows a card to every player (CR 701.20a), and the host already
//! tells every seat which one: the log names it `Known` to all of them
//! (`baylee-gamehost`'s `log.rs`). What the client lacked was a way to show
//! it. The zone browser draws `PlayerView::looking_at`, which the host fills
//! only for the seat its question is about, so a Mystical Tutor's searcher
//! saw the card large and everyone else got one line in a log they may not
//! have open (owner, 07.10.2026).
//!
//! [`Reveals`] is the half of the fix that needs no renderer: it reads the
//! log as it arrives and keeps the reveals this seat has not been shown yet,
//! oldest first. The renderer draws [`Reveals::current`] and asks
//! [`Reveals::tick`] for its clock; it decides nothing about which cards.
//!
//! # Which lines are news
//!
//! Only lines this socket hears happen. The host tells a socket the whole log
//! again whenever it attaches, reattaches or is rebuilt, and every such
//! telling starts at line 0 ([`LogTail::from`]); a live socket's tails start
//! where the last one ended. So a tail from 0 is history, and so is every
//! further chunk of the same telling, which repeats its view's `seq`
//! (`LOG_TAIL_CAP` splits a long one). A reveal a reconnect tells again is
//! one the player was shown, or one that happened while they were away and
//! is in the log; it never stands up a second time.
//!
//! # Which cards
//!
//! Only what the line names `Known`, with a card or a registry token: a
//! `Hidden` card is "a card" to this seat and a `FaceDown` one may not be
//! looked at (CR 708.5), so neither is shown, and a line naming nothing else
//! shows nothing at all. The seat's own reveals are left out (it chose the
//! card and saw it), and so is any card its own `looking_at` already shows
//! it: that card is on the sheet a question opened, and a second picture of
//! it would stand over the answer.
//!
//! # Never in the way
//!
//! Nothing here touches the browser, the interaction or the outbox, so a
//! reveal cannot hold a question up; the renderer stands it under a dialog
//! that is answering one. It goes when its time is up, or earlier when the
//! player puts it away ([`Reveals::dismiss`]). It does not go when this seat
//! answers something: the autopilot and standing orders answer for it, and an
//! opponent's tutor on their own turn would then close on the pass that
//! follows it, before anybody had seen it.

use std::collections::VecDeque;

use baylee_core::ids::{ObjectId, PlayerId};
use baylee_view::{CardIdentity, LogEntry, LogEvent, LogObject, LogTail, PlayerView};

use crate::gamelog::LogBook;

/// How long a reveal of one card stands, in seconds.
pub const SHOW_SECS: f64 = 7.0;

/// How much longer it stands for every card past the first.
pub const PER_CARD_SECS: f64 = 1.5;

/// The longest any one reveal stands, however many cards it shows.
pub const LONGEST_SECS: f64 = 15.0;

/// How many reveals may wait behind the one standing. Past this the oldest
/// waiting one is let go: a loop that reveals a card a step would otherwise
/// queue a minute of pictures, and every one of them is still in the log.
pub const WAITING_CAP: usize = 6;

/// One card a reveal shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ShownCard {
    /// The object, as this seat's view names it.
    pub id: ObjectId,
    /// The card and printing the line named. `None` for a token.
    pub card: Option<CardIdentity>,
    /// The registry token, for a token.
    pub token: Option<u16>,
}

/// One seat's reveal, as it is held up.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Reveal {
    /// Who revealed the cards.
    pub player: PlayerId,
    /// The cards, in the order the log names them. Never empty.
    pub cards: Vec<ShownCard>,
    /// Counts every reveal this game held up, from 1, so a renderer can tell
    /// one reveal from the next that shows the same card.
    pub number: u64,
}

impl Reveal {
    /// How long it stands, in seconds: [`SHOW_SECS`], and
    /// [`PER_CARD_SECS`] more for each card past the first, at most
    /// [`LONGEST_SECS`].
    #[must_use]
    pub fn lasts(&self) -> f64 {
        #[allow(clippy::cast_precision_loss)] // a reveal is a handful of cards
        let extra = self.cards.len().saturating_sub(1) as f64;
        (SHOW_SECS + PER_CARD_SECS * extra).min(LONGEST_SECS)
    }
}

/// The reveals this seat is being shown, oldest first.
#[derive(Clone, Default, Debug)]
pub struct Reveals {
    queue: VecDeque<Reveal>,
    /// When the front one stood up, on the caller's clock. `None` until the
    /// first [`Self::tick`] after it reached the front.
    since: Option<f64>,
    /// The `seq` of the view that came with the last telling from line 0:
    /// every chunk of that telling is history.
    retold: Option<u64>,
    numbered: u64,
}

impl Reveals {
    /// Nothing to show.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads a frame's log: `tail` as it arrived, with the view `view` that
    /// came beside it, and `new`, the entries of it the book did not hold
    /// yet ([`LogBook::append`]'s count from the end of
    /// [`LogBook::entries`]). [`Self::take`] does both.
    ///
    /// Returns whether a reveal was queued.
    pub fn heard(&mut self, tail: &LogTail, view: &PlayerView, new: &[LogEntry]) -> bool {
        if tail.entries.is_empty() {
            // A question asked again carries `{from: 0, entries: []}`; it
            // tells nothing, and must not mark the frames after it history.
            return false;
        }
        if tail.from == 0 {
            self.retold = Some(view.seq);
        }
        if self.retold == Some(view.seq) {
            return false;
        }
        let mut found: Vec<Reveal> = Vec::new();
        for entry in new {
            let LogEvent::Revealed { player, cards } = &entry.event else {
                continue;
            };
            if *player == view.seat {
                continue;
            }
            let shown = cards.iter().filter_map(|object| match object {
                LogObject::Known {
                    id, card, token, ..
                } if (card.is_some() || token.is_some())
                    && !view.looking_at.iter().any(|o| o.id == *id) =>
                {
                    Some(ShownCard {
                        id: *id,
                        card: *card,
                        token: *token,
                    })
                }
                LogObject::Known { .. } | LogObject::FaceDown { .. } | LogObject::Hidden => None,
            });
            // One frame's reveals by one seat are one picture: a search that
            // reveals and an effect that reveals again are read together.
            if !found.iter().any(|r| r.player == *player) {
                found.push(Reveal {
                    player: *player,
                    cards: Vec::new(),
                    number: 0,
                });
            }
            let Some(reveal) = found.iter_mut().find(|r| r.player == *player) else {
                continue;
            };
            for card in shown {
                if !reveal.cards.iter().any(|c| c.id == card.id) {
                    reveal.cards.push(card);
                }
            }
        }
        let mut queued = false;
        for mut reveal in found.into_iter().filter(|r| !r.cards.is_empty()) {
            self.numbered += 1;
            reveal.number = self.numbered;
            self.queue.push_back(reveal);
            queued = true;
            if self.queue.len() > 1 + WAITING_CAP {
                // The oldest *waiting* one, never the one standing: taking
                // that away would be a picture vanishing under the player.
                self.queue.remove(1);
            }
        }
        queued
    }

    /// Takes a frame's tail into `book` and hears the entries it added: the
    /// one door a renderer takes the log through, so the book and this never
    /// read two different frames. Returns [`LogBook::append`]'s count.
    pub fn take(&mut self, book: &mut LogBook, tail: &LogTail, view: &PlayerView) -> usize {
        let added = book.append(tail, view);
        let entries = book.entries();
        let new = entries.get(entries.len() - added..).unwrap_or_default();
        self.heard(tail, view, new);
        added
    }

    /// The reveal standing now.
    #[must_use]
    pub fn current(&self) -> Option<&Reveal> {
        self.queue.front()
    }

    /// How many wait behind it.
    #[must_use]
    pub fn waiting(&self) -> usize {
        self.queue.len().saturating_sub(1)
    }

    /// Whether [`Self::tick`] at `now` would change anything, so a caller
    /// holding the state behind change detection can ask before it writes.
    #[must_use]
    pub fn due(&self, now: f64) -> bool {
        match (self.queue.front(), self.since) {
            (None, since) => since.is_some(),
            (Some(_), None) => true,
            (Some(front), Some(at)) => now - at >= front.lasts(),
        }
    }

    /// Runs the clock to `now`, in seconds on any clock that only moves
    /// forward: starts the front reveal's time the first time it is asked,
    /// and lets it go once its time is up. Returns whether the reveal
    /// standing changed.
    pub fn tick(&mut self, now: f64) -> bool {
        let Some(front) = self.queue.front() else {
            self.since = None;
            return false;
        };
        match self.since {
            None => {
                self.since = Some(now);
                false
            }
            Some(at) if now - at >= front.lasts() => {
                self.queue.pop_front();
                self.since = self.queue.front().map(|_| now);
                true
            }
            Some(_) => false,
        }
    }

    /// The player put the standing reveal away: the next one, if any, stands
    /// up on the next [`Self::tick`]. Returns whether there was one.
    pub fn dismiss(&mut self) -> bool {
        self.since = None;
        self.queue.pop_front().is_some()
    }
}

/// A card's height over its width (88 mm by 63 mm).
const CARD_TALL: f32 = 88.0 / 63.0;

/// How wide to draw each of `cards` cards, and how many to a row, to fit
/// them all in `room` (width, height) with `gap` between them: the widest
/// card that fits, at most `widest`, and one row where that costs nothing.
///
/// Every card is drawn, at one size: a reveal is the cards in it, and one
/// left out for room would be a card this seat was shown and did not see.
#[must_use]
pub fn fit(cards: usize, room: (f32, f32), gap: f32, widest: f32) -> (f32, usize) {
    let n = cards.max(1);
    let mut best = (0.0_f32, n);
    // From one row down to one column, keeping a later layout only when it
    // draws strictly larger cards: a tie stays the wider, flatter row.
    for across in (1..=n).rev() {
        let rows = n.div_ceil(across);
        #[allow(clippy::cast_precision_loss)] // a handful of cards
        let (across_f, rows_f) = (across as f32, rows as f32);
        let by_width = (room.0 - gap * (across_f - 1.0)) / across_f;
        let by_height = (room.1 - gap * (rows_f - 1.0)) / rows_f / CARD_TALL;
        let width = by_width.min(by_height).min(widest);
        if width > best.0 {
            best = (width, across);
        }
    }
    (best.0.max(0.0), best.1)
}

#[cfg(test)]
mod tests;
