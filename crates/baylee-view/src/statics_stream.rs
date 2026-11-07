//! The opening payload after the first time: only the printings that are new.
//!
//! A [`GameStatic`] goes whole on arrival and when its roster moves. What moves
//! in a game is its print table, a few printings at a time as a seat is shown
//! cards, and sending all of it each time was 39 to 47 % of a game's bytes once
//! views went as patches (`docs/perf-baseline.md`). The rule is the view's: what
//! is sent is computed from the seat's finished payload, so a seat is told only
//! what its own payload says, and a reader that cannot place a frame asks for a
//! whole one, once ([`crate::ViewReader`]).

use serde::{Deserialize, Serialize};

use crate::game_static::{GameStatic, PrintEntry};
use baylee_core::wire::{self, Unreadable};

/// Printings to add to a [`GameStatic`] a reader holds.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MorePrints {
    /// How many printings the reader holds before these: a frame it did not get
    /// shows here, where a table with a hole would only show as a blank card.
    pub held: u32,
    /// The printings, by the index a `PrintRef` names.
    pub prints: Vec<(u32, PrintEntry)>,
}

impl GameStatic {
    /// How many printings this seat has been shown.
    fn shown(&self) -> usize {
        self.prints.iter().flatten().count()
    }

    /// What makes `self` into `new`, when printings joining is all that moved.
    fn more_prints(&self, new: &Self) -> Option<MorePrints> {
        // Named one by one, so that a field added here has to say which it is.
        let Self {
            view_version,
            game_id,
            your_seat,
            seats,
            prints: _,
            decision_secs,
            reconnect_secs,
        } = self;
        let same = *view_version == new.view_version
            && *game_id == new.game_id
            && *your_seat == new.your_seat
            && *seats == new.seats
            && *decision_secs == new.decision_secs
            && *reconnect_secs == new.reconnect_secs
            && self.prints.len() == new.prints.len();
        if !same {
            return None;
        }
        let mut prints = Vec::new();
        for (at, (old, new)) in self.prints.iter().zip(&new.prints).enumerate() {
            match (old, new) {
                (None, Some(entry)) => prints.push((u32::try_from(at).ok()?, entry.clone())),
                (old, new) if old == new => {}
                _ => return None,
            }
        }
        Some(MorePrints {
            held: u32::try_from(self.shown()).ok()?,
            prints,
        })
    }

    /// Adds `more`, or leaves `self` as it was and says why not.
    fn take(&mut self, more: MorePrints) -> Result<(), StaticLost> {
        if usize::try_from(more.held).ok() != Some(self.shown()) {
            return Err(StaticLost::Gap);
        }
        let free = |at: u32| {
            usize::try_from(at)
                .ok()
                .and_then(|at| self.prints.get(at))
                .is_some_and(Option::is_none)
        };
        if !more.prints.iter().all(|(at, _)| free(*at)) {
            return Err(StaticLost::BadPrint);
        }
        for (at, entry) in more.prints {
            if let Some(slot) = usize::try_from(at)
                .ok()
                .and_then(|at| self.prints.get_mut(at))
            {
                *slot = Some(entry);
            }
        }
        Ok(())
    }
}

/// The two payloads of a `GameStaticMsg`, of which one is empty.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct StaticFrame {
    /// A whole [`GameStatic`].
    pub statics: Vec<u8>,
    /// [`MorePrints`].
    pub more: Vec<u8>,
}

/// What a host remembers of one seat: the last payload it told it.
#[derive(Clone, Debug, Default)]
pub struct StaticSender {
    last: Option<Box<GameStatic>>,
    /// Every frame goes whole ([`Self::tell_whole`]).
    whole: bool,
}

impl StaticSender {
    /// From here on every frame carries the whole payload: for what keeps no
    /// stream, a replay whose moments are read in any order.
    pub const fn tell_whole(&mut self) {
        self.whole = true;
    }

    /// The frame that tells the seat `statics` whole, whatever it holds.
    pub fn snapshot(&mut self, statics: &GameStatic) -> StaticFrame {
        self.last = Some(Box::new(statics.clone()));
        StaticFrame {
            statics: wire::encode(statics),
            more: Vec::new(),
        }
    }

    /// The frame that tells the seat `statics`: the printings it lacks, or the
    /// whole payload when more than that moved or it holds none.
    ///
    /// A debug build proves every frame by adding what it says and comparing.
    pub fn next(&mut self, statics: &GameStatic) -> StaticFrame {
        let more = self
            .last
            .as_ref()
            .filter(|_| !self.whole)
            .and_then(|last| last.more_prints(statics).map(|more| (last, more)));
        let Some((last, more)) = more else {
            return self.snapshot(statics);
        };
        if cfg!(debug_assertions) {
            let mut proof = (**last).clone();
            let joined = proof.take(more.clone());
            debug_assert_eq!(joined, Ok(()), "printings join");
            debug_assert_eq!(&proof, statics, "printings leave the payload");
        }
        let frame = StaticFrame {
            statics: Vec::new(),
            more: wire::encode(&more),
        };
        self.last = Some(Box::new(statics.clone()));
        frame
    }
}

/// Why a [`StaticReader`] cannot place a frame: it has to ask for a whole one.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum StaticLost {
    /// The bytes did not read.
    Unreadable(Unreadable),
    /// Printings came with no payload to add them to.
    NoBase,
    /// Printings came for a table that is not the one they were written on.
    Gap,
    /// A printing past the table, or over one the reader holds.
    BadPrint,
}

impl std::fmt::Display for StaticLost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable(why) => why.fmt(f),
            Self::NoBase => f.write_str("printings came before any game setup"),
            Self::Gap => f.write_str("printings came for another print table"),
            Self::BadPrint => f.write_str("a printing past the table or over another"),
        }
    }
}

impl std::error::Error for StaticLost {}

/// What a reader of one seat holds: the payload its frames left.
#[derive(Clone, Debug, Default)]
pub struct StaticReader {
    statics: Option<GameStatic>,
    /// A whole payload has been asked for and has not come.
    waiting: bool,
}

impl StaticReader {
    /// Reads a `GameStaticMsg`'s two payloads, and says what the seat's
    /// [`GameStatic`] is now. `Ok(None)` is printings passed over while a whole
    /// payload is on its way.
    ///
    /// # Errors
    /// [`StaticLost`]: the caller asks for a whole payload, once. Until one
    /// arrives this answers `Ok(None)`.
    pub fn read(&mut self, statics: &[u8], more: &[u8]) -> Result<Option<&GameStatic>, StaticLost> {
        if more.is_empty() {
            return match wire::decode::<GameStatic>(statics) {
                Ok(whole) => {
                    self.waiting = false;
                    Ok(Some(&*self.statics.insert(whole)))
                }
                Err(why) => self.lost(StaticLost::Unreadable(why)),
            };
        }
        if self.waiting {
            return Ok(None);
        }
        let more = match wire::decode::<MorePrints>(more) {
            Ok(more) => more,
            Err(why) => return self.lost(StaticLost::Unreadable(why)),
        };
        let Some(held) = &mut self.statics else {
            return self.lost(StaticLost::NoBase);
        };
        match held.take(more) {
            Ok(()) => Ok(self.statics.as_ref()),
            Err(why) => self.lost(why),
        }
    }

    /// The payload the last frame left.
    #[must_use]
    pub const fn statics(&self) -> Option<&GameStatic> {
        self.statics.as_ref()
    }

    fn lost(&mut self, why: StaticLost) -> Result<Option<&GameStatic>, StaticLost> {
        if self.waiting {
            return Ok(None);
        }
        self.waiting = true;
        Err(why)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_static::Finish;
    use baylee_core::ids::PlayerId;
    use proptest::prelude::*;

    fn entry(id: u8) -> PrintEntry {
        PrintEntry {
            scryfall_id: format!("print-{id}"),
            lang: "EN".to_owned(),
            finish: Finish::default(),
        }
    }

    fn table(size: usize) -> GameStatic {
        GameStatic {
            view_version: crate::VIEW_VERSION,
            game_id: "g".to_owned(),
            your_seat: PlayerId::new(0),
            seats: Vec::new(),
            prints: vec![None; size],
            decision_secs: None,
            reconnect_secs: None,
        }
    }

    fn told(sender: &mut StaticSender, reader: &mut StaticReader, statics: &GameStatic) -> usize {
        let frame = sender.next(statics);
        let got = reader.read(&frame.statics, &frame.more).expect("it reads");
        assert_eq!(got, Some(statics));
        frame.statics.len() + frame.more.len()
    }

    #[test]
    fn a_printing_shown_goes_alone_and_the_first_payload_whole() {
        let (mut sender, mut reader) = (StaticSender::default(), StaticReader::default());
        let mut statics = table(200);
        statics.prints[3] = Some(entry(3));
        let first = sender.next(&statics);
        assert!(first.more.is_empty() && !first.statics.is_empty());
        reader.read(&first.statics, &first.more).expect("whole");
        statics.prints[100] = Some(entry(100));
        let second = sender.next(&statics);
        assert!(second.statics.is_empty(), "only what is new");
        assert_eq!(
            wire::decode::<MorePrints>(&second.more),
            Ok(MorePrints {
                held: 1,
                prints: vec![(100, entry(100))],
            })
        );
        let got = reader.read(&second.statics, &second.more).expect("more");
        assert_eq!(got, Some(&statics));
    }

    #[test]
    fn a_roster_that_moved_goes_whole() {
        let (mut sender, mut reader) = (StaticSender::default(), StaticReader::default());
        let mut statics = table(4);
        told(&mut sender, &mut reader, &statics);
        statics.decision_secs = Some(30);
        statics.prints[0] = Some(entry(0));
        let frame = sender.next(&statics);
        assert!(frame.more.is_empty(), "more than printings moved");
        assert_eq!(wire::decode::<GameStatic>(&frame.statics), Ok(statics));
    }

    #[test]
    fn a_printing_taken_back_or_changed_goes_whole() {
        let (mut sender, mut reader) = (StaticSender::default(), StaticReader::default());
        let mut statics = table(4);
        statics.prints[0] = Some(entry(0));
        told(&mut sender, &mut reader, &statics);
        statics.prints[0] = Some(entry(1));
        assert!(sender.next(&statics).more.is_empty());
        statics.prints[0] = None;
        assert!(sender.next(&statics).more.is_empty());
    }

    #[test]
    fn a_replay_is_told_whole_every_time() {
        let mut sender = StaticSender::default();
        sender.tell_whole();
        let mut statics = table(4);
        sender.next(&statics);
        statics.prints[1] = Some(entry(1));
        assert!(sender.next(&statics).more.is_empty());
    }

    #[test]
    fn a_frame_missed_is_found_asked_for_once_and_mended_by_a_whole_payload() {
        let (mut sender, mut reader) = (StaticSender::default(), StaticReader::default());
        let mut statics = table(8);
        told(&mut sender, &mut reader, &statics);
        statics.prints[1] = Some(entry(1));
        let missed = sender.next(&statics);
        assert!(!missed.more.is_empty());
        statics.prints[2] = Some(entry(2));
        let next = sender.next(&statics);
        assert_eq!(reader.read(&[], &next.more), Err(StaticLost::Gap));
        statics.prints[3] = Some(entry(3));
        let after = sender.next(&statics);
        assert_eq!(reader.read(&[], &after.more), Ok(None), "asked once");
        let whole = sender.snapshot(&statics);
        let got = reader.read(&whole.statics, &whole.more).expect("whole");
        assert_eq!(got, Some(&statics));
    }

    #[test]
    fn printings_before_any_payload_are_lost_and_nothing_panics() {
        let mut reader = StaticReader::default();
        let more = wire::encode(&MorePrints {
            held: 0,
            prints: vec![(0, entry(0))],
        });
        assert_eq!(reader.read(&[], &more), Err(StaticLost::NoBase));
    }

    #[test]
    fn a_printing_past_the_table_or_over_another_changes_nothing() {
        for at in [1, 4, u32::MAX] {
            let (mut sender, mut reader) = (StaticSender::default(), StaticReader::default());
            let mut statics = table(4);
            statics.prints[1] = Some(entry(1));
            told(&mut sender, &mut reader, &statics);
            let more = wire::encode(&MorePrints {
                held: 1,
                prints: vec![(0, entry(9)), (at, entry(9))],
            });
            assert_eq!(reader.read(&[], &more), Err(StaticLost::BadPrint));
            assert_eq!(reader.statics(), Some(&statics), "all or nothing");
        }
    }

    #[test]
    fn bytes_that_are_no_printings_are_lost_and_not_a_panic() {
        let (mut sender, mut reader) = (StaticSender::default(), StaticReader::default());
        told(&mut sender, &mut reader, &table(4));
        assert!(matches!(
            reader.read(&[], &[0xff; 16]),
            Err(StaticLost::Unreadable(_))
        ));
        let mut fresh = StaticReader::default();
        assert!(fresh.read(&[], &[]).is_err(), "no payload at all");
    }

    proptest! {
        /// Printings shown in any order, in any batches, leave a reader on the
        /// payload the host holds, and never cost more than the whole one.
        #[test]
        fn any_order_of_printings_leaves_the_reader_on_the_payload(
            batches in prop::collection::vec(prop::collection::vec(0..64usize, 0..6), 1..24),
        ) {
            let (mut sender, mut reader) = (StaticSender::default(), StaticReader::default());
            let mut statics = table(64);
            told(&mut sender, &mut reader, &statics);
            for batch in batches {
                for at in batch {
                    statics.prints[at] = Some(entry(u8::try_from(at).unwrap_or(0)));
                }
                let size = told(&mut sender, &mut reader, &statics);
                prop_assert!(size <= wire::encode(&statics).len());
            }
        }
    }
}
