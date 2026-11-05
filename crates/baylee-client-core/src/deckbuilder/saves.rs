//! The line a deck's saves stand in on their way to a gateway.
//!
//! Every change saves the whole deck, and the transport runs each request on
//! a thread of its own, so two saves out at once can land in either order and
//! leave the gateway holding the older list with nothing to say so. So one
//! save is out at a time, and the next waits here. A later edit replaces the
//! save that waits rather than queueing behind it: a run of edits is the save
//! out and one more, however long the run.
//!
//! The line outlives a builder visit, because leaving is when the last save
//! goes out, and it is one line for every deck: a deck left with a save out
//! and the next one opened stay in order without asking.

use super::Draft;
use std::collections::VecDeque;

/// One save of one deck.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeckSave {
    /// The builder visit it was made in ([`super::DeckBuilder::visit`]).
    pub visit: u64,
    /// What it saves. Its `editing` says where: `None` creates the deck.
    pub draft: Draft,
    /// The format it saves under, which the draft holds only once chosen.
    pub format: String,
    /// Whether it keeps the state it replaces in the deck's history.
    pub versioned: bool,
}

/// The save that went out, and the epoch of the gateway it went to.
#[derive(Debug)]
struct Out {
    save: DeckSave,
    epoch: u64,
    /// The player deleted the deck this save is creating.
    thrown_away: bool,
}

/// What an answer leaves to do.
#[derive(Debug, PartialEq, Eq)]
pub struct Answered {
    /// The save it answers.
    pub sent: DeckSave,
    /// The save to send now, already out as far as the line goes.
    pub next: Option<DeckSave>,
    /// The deck this save created was deleted while it was out, and is to
    /// go as soon as it has an id.
    pub thrown_away: bool,
}

/// One save out, and the rest waiting their turn.
#[derive(Debug, Default)]
pub struct SaveLine {
    out: Option<Out>,
    waiting: VecDeque<DeckSave>,
}

impl SaveLine {
    /// Takes a save, answering it if it is to be sent now. `epoch` names the
    /// gateway: a save out to one since left will never be answered, so it
    /// and those behind it are forgotten, and their drafts stay on the device.
    pub fn offer(&mut self, save: DeckSave, epoch: u64) -> Option<DeckSave> {
        if self.out.as_ref().is_some_and(|out| out.epoch != epoch) {
            *self = Self::default();
        }
        let Some(out) = &self.out else {
            self.send(save.clone(), epoch);
            return Some(save);
        };
        if let Some(waiting) = self.waiting.iter_mut().find(|w| w.visit == save.visit) {
            *waiting = save;
        } else if out.save.visit != save.visit || out.save.draft != save.draft {
            self.waiting.push_back(save);
        }
        None
    }

    /// The save out has landed: what waited for its deck is told what the
    /// answer knows, and the next save goes out. `deck_id` is the deck it
    /// created; `archived` that the gateway kept the state it replaced.
    pub fn landed(&mut self, deck_id: Option<&str>, archived: bool) -> Option<Answered> {
        let out = self.out.take()?;
        for waiting in self
            .waiting
            .iter_mut()
            .filter(|w| w.visit == out.save.visit)
        {
            if let Some(id) = deck_id {
                waiting.draft.editing = Some(id.to_string());
            }
            waiting.versioned &= !(archived || deck_id.is_some());
        }
        Some(self.answer(out))
    }

    /// The save out failed. What waits still goes: it is a later edit, and
    /// the gateway may take it.
    pub fn failed(&mut self) -> Option<Answered> {
        let out = self.out.take()?;
        Some(self.answer(out))
    }

    /// Takes back what waits for `visit`: the player discarded it. What is
    /// out cannot be called back.
    pub fn withdraw(&mut self, visit: u64) {
        self.waiting.retain(|waiting| waiting.visit != visit);
    }

    /// The deck `visit` is creating was deleted. A `POST` out cannot be
    /// called back, so its answer is told to delete what it made.
    pub fn throw_away(&mut self, visit: u64) {
        self.withdraw(visit);
        if let Some(out) = self.out.as_mut().filter(|out| out.save.visit == visit) {
            out.thrown_away = out.save.draft.editing.is_none();
        }
    }

    fn send(&mut self, save: DeckSave, epoch: u64) {
        self.out = Some(Out {
            save,
            epoch,
            thrown_away: false,
        });
    }

    fn answer(&mut self, out: Out) -> Answered {
        let next = self.waiting.pop_front();
        if let Some(next) = &next {
            self.send(next.clone(), out.epoch);
        }
        Answered {
            sent: out.save,
            next,
            thrown_away: out.thrown_away,
        }
    }

    /// Whether a save is out to the gateway of `epoch`.
    #[must_use]
    pub fn busy(&self, epoch: u64) -> bool {
        self.out.as_ref().is_some_and(|out| out.epoch == epoch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn save(visit: u64, name: &str) -> DeckSave {
        DeckSave {
            visit,
            draft: Draft {
                name: name.into(),
                ..Draft::default()
            },
            format: String::new(),
            versioned: true,
        }
    }

    #[test]
    fn one_save_is_out_and_the_next_waits() {
        let mut line = SaveLine::default();
        assert_eq!(line.offer(save(1, "a"), 0), Some(save(1, "a")));
        assert_eq!(line.offer(save(1, "b"), 0), None);
        assert!(line.busy(0));

        let answered = line.landed(None, false).expect("a save was out");
        assert_eq!(answered.sent, save(1, "a"));
        assert_eq!(answered.next, Some(save(1, "b")));
        assert!(line.busy(0), "the next one is out now");

        let answered = line.landed(None, false).expect("a save was out");
        assert_eq!(answered.next, None);
        assert!(!line.busy(0));
    }

    #[test]
    fn a_later_edit_replaces_the_save_that_waits() {
        let mut line = SaveLine::default();
        line.offer(save(1, "a"), 0);
        for name in ["b", "c", "d"] {
            assert_eq!(line.offer(save(1, name), 0), None);
        }
        let answered = line.landed(None, false).expect("a save was out");
        assert_eq!(answered.next, Some(save(1, "d")), "only the last");
        assert_eq!(line.landed(None, false).expect("out").next, None);
    }

    #[test]
    fn what_is_already_out_is_not_sent_twice() {
        let mut line = SaveLine::default();
        line.offer(save(1, "a"), 0);
        assert_eq!(line.offer(save(1, "a"), 0), None);
        assert_eq!(line.landed(None, false).expect("out").next, None);
    }

    #[test]
    fn an_answer_tells_what_waited_for_the_same_deck() {
        let mut line = SaveLine::default();
        line.offer(save(1, "a"), 0);
        line.offer(save(1, "b"), 0);
        line.offer(save(2, "other"), 0);

        let next = line.landed(Some("d9"), false).expect("out").next;
        let next = next.expect("the second save");
        assert_eq!(
            next.draft.editing.as_deref(),
            Some("d9"),
            "not a second POST"
        );
        assert!(!next.versioned, "a deck made here has nothing behind it");

        let other = line.landed(None, false).expect("out").next.expect("waits");
        assert_eq!(other, save(2, "other"), "another deck's save is its own");
    }

    #[test]
    fn a_kept_state_lets_what_waited_go_in_place() {
        let mut line = SaveLine::default();
        line.offer(save(1, "a"), 0);
        line.offer(save(1, "b"), 0);
        let next = line.landed(None, true).expect("out").next.expect("waits");
        assert!(!next.versioned);
    }

    #[test]
    fn nothing_kept_leaves_what_waited_keeping_history() {
        let mut line = SaveLine::default();
        line.offer(save(1, "a"), 0);
        line.offer(save(1, "b"), 0);
        let next = line.landed(None, false).expect("out").next.expect("waits");
        assert!(next.versioned);
    }

    #[test]
    fn a_failed_save_lets_the_next_go() {
        let mut line = SaveLine::default();
        line.offer(save(1, "a"), 0);
        line.offer(save(1, "b"), 0);
        let answered = line.failed().expect("out");
        assert_eq!(answered.sent, save(1, "a"));
        assert_eq!(answered.next, Some(save(1, "b")));
        assert!(answered.next.is_some_and(|next| next.versioned));
    }

    #[test]
    fn a_gateway_left_is_not_waited_for() {
        let mut line = SaveLine::default();
        line.offer(save(1, "a"), 0);
        line.offer(save(1, "b"), 0);
        assert!(!line.busy(1));
        assert_eq!(line.offer(save(2, "c"), 1), Some(save(2, "c")));
        assert_eq!(line.landed(None, false).expect("out").next, None);
    }

    #[test]
    fn withdrawing_takes_back_one_decks_waiting_save() {
        let mut line = SaveLine::default();
        line.offer(save(1, "a"), 0);
        line.offer(save(1, "b"), 0);
        line.offer(save(2, "other"), 0);
        line.withdraw(1);
        let next = line.landed(None, false).expect("out").next;
        assert_eq!(next, Some(save(2, "other")));
    }

    #[test]
    fn a_deck_deleted_under_its_post_is_told_on_the_answer() {
        let mut line = SaveLine::default();
        line.offer(save(1, "a"), 0);
        line.offer(save(1, "b"), 0);
        line.throw_away(1);
        let answered = line.landed(Some("d9"), false).expect("out");
        assert!(answered.thrown_away);
        assert_eq!(answered.next, None, "what waited went with it");
    }

    #[test]
    fn a_stored_deck_needs_no_answer_to_be_deleted() {
        let mut line = SaveLine::default();
        let mut stored = save(1, "a");
        stored.draft.editing = Some("d1".into());
        line.offer(stored, 0);
        line.throw_away(1);
        assert!(!line.landed(None, false).expect("out").thrown_away);
    }

    #[test]
    fn an_answer_with_nothing_out_is_nothing() {
        let mut line = SaveLine::default();
        assert_eq!(line.landed(None, false), None);
        assert_eq!(line.failed(), None);
    }
}
