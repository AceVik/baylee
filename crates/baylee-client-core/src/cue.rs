//! What the table would say out loud.
//!
//! Every sound this client will ever make is a *cue* — a named moment, not a
//! file and not a frequency. The split is the one the rest of the client
//! already uses and is worth stating once more here, because sound is the
//! first output that tempts you to skip it: **deciding** that something is
//! worth hearing is a reading of the game and belongs where a test can run
//! it; **making a noise** needs a device, a sample rate and a user gesture,
//! and belongs in the shell.
//!
//! So this module ends at a queue. Nothing here opens an audio stream,
//! chooses a waveform or names an asset, and
//! [`baylee_client_core`](crate) links no audio crate at all. What the sink
//! does with a drained cue is the shell's business, and today it is a
//! documented silence — see `docs/client.md`, "The sounds are decided before
//! anything can play them".
//!
//! # Why a cue and not a sample name
//!
//! A sample name is an answer to a question nobody has asked yet: where the
//! sounds come from is undecided (`docs/legal.md` has four clauses and none
//! of them is about audio), and a client that spelled `"life_lost.ogg"` into
//! its decision layer would have decided it by accident. A cue survives every
//! answer — shipped files, generated tones, or a player's own pack.
//!
//! # One event, one cue
//!
//! The rule that shapes everything below: **what the eye is shown and what
//! the ear is told are the same event**, answered by the same arithmetic. A
//! triple block sends three views a frame apart and
//! [`crate::lifeflash`] merges them into one `−7`; the ear is told once, off
//! [`Change::started`], and not three times. Two opponents losing life on one
//! view are two numbers on two bars and *one* sound, because two copies of a
//! sound on one frame is not two sounds, it is one sound played louder.
//!
//! And a cue is a **flank**, never a state. Nothing here is asked "is it my
//! turn"; it is told "it has become my turn", once.

use crate::interaction::Outcome;
use crate::lifeflash::Change;
use baylee_core::ids::PlayerId;

/// A moment worth hearing.
///
/// Flat variants rather than a payload, because every reader wants a name: a
/// sink matches on it, `/state` prints it so a change can be proven by a read
/// instead of by listening, and a future sound pack is keyed by it. "My life"
/// and "their life" are four variants and not two with a flag for the same
/// reason — they are four different sounds, and a `bool` in the middle of a
/// match arm is a thing to get backwards.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cue {
    /// This seat lost life.
    MyLifeLost,
    /// This seat gained life.
    MyLifeGained,
    /// Somebody else lost life.
    TheirLifeLost,
    /// Somebody else gained life.
    TheirLifeGained,
    /// The table has begun waiting for this seat.
    YourMove,
    /// The engine refused what was sent.
    Refused,
    /// The game is over and this seat won it.
    GameWon,
    /// The game is over and this seat did not.
    GameLost,
    /// The game is over and nobody won.
    GameDrawn,
}

impl Cue {
    /// Every cue, in no particular order but in *all* of them.
    ///
    /// The sink synthesises one sound per entry at startup, so a variant
    /// missing here is a variant that is silent at runtime and loud in no
    /// test. `every_cue_is_in_all` holds the two together by counting the
    /// arms of [`Cue::name`], which the compiler already forces to be
    /// exhaustive.
    pub const ALL: [Self; 9] = [
        Self::MyLifeLost,
        Self::MyLifeGained,
        Self::TheirLifeLost,
        Self::TheirLifeGained,
        Self::YourMove,
        Self::Refused,
        Self::GameWon,
        Self::GameLost,
        Self::GameDrawn,
    ];

    /// Its own name, for `/state` and for whatever ends up playing it.
    ///
    /// Spelled out rather than derived from [`Debug`], because a `Debug`
    /// rendering is allowed to change and this string is read by a harness.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::MyLifeLost => "MyLifeLost",
            Self::MyLifeGained => "MyLifeGained",
            Self::TheirLifeLost => "TheirLifeLost",
            Self::TheirLifeGained => "TheirLifeGained",
            Self::YourMove => "YourMove",
            Self::Refused => "Refused",
            Self::GameWon => "GameWon",
            Self::GameLost => "GameLost",
            Self::GameDrawn => "GameDrawn",
        }
    }

    /// How a finished game sounds from this chair.
    ///
    /// Three sounds out of [`Outcome`]'s five sentences: the sheet has to
    /// name *which* team won and a sound must not, or a table would chime
    /// differently for team 1 and team 2 and be saying something the game
    /// does not mean. Collapsed through [`Outcome::won`] rather than by
    /// re-reading `GameResult`, so the ear and the sheet cannot come to
    /// different conclusions about who lost.
    #[must_use]
    pub fn of_outcome(outcome: Outcome) -> Self {
        match outcome.won() {
            None => Self::GameDrawn,
            Some(true) => Self::GameWon,
            Some(false) => Self::GameLost,
        }
    }

    /// Whether this is the last thing that will ever be said at this table.
    ///
    /// Exactly the three [`of_outcome`](Self::of_outcome) produces, asked as
    /// a question because a *sink* needs it: the frame a game ends on is the
    /// only frame that can carry three cues — the lethal hit is a life change
    /// here, a life change there, and the ending — and those three amplitudes
    /// together are louder than a loudspeaker can be. An ending is therefore
    /// played alone. Deciding it here rather than in the shell is the split
    /// the whole module is built on: which moment outranks which is a reading
    /// of the game, and the shell only makes the noise.
    #[must_use]
    pub fn ends_the_game(self) -> bool {
        matches!(self, Self::GameWon | Self::GameLost | Self::GameDrawn)
    }
}

/// How loud the table is, if at all.
///
/// Three steps and not a slider, for the reason the sky picker is three
/// chips: a slider is a number a player has to *tune*, and there is no
/// tuning to be done here — the nine sounds are balanced against each other
/// in [`crate::cue`]'s sink, so the only questions are "on", "quieter" and
/// "off". It is stored under its own key rather than as a `bool` pair
/// because two bools make four states and one of them is nonsense.
///
/// [`Full`](Self::Full) is the default and a missing key reads as it, which
/// is the way round it has to be: a settings blob written by a client from
/// before there was sound must not open a silent one, or the feature looks
/// broken to exactly the player who has been here longest.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Loudness {
    /// Everything, at the levels the sink was balanced at.
    #[default]
    Full,
    /// The same balance, half the amplitude.
    Half,
    /// Nothing at all. Cues are still decided, drained and reported to
    /// `/state` — only the device is never asked for anything, which keeps
    /// "is it silent" and "is it deciding" two separate questions.
    Off,
}

impl Loudness {
    /// Every step, in the order a picker offers them.
    pub const ALL: [Self; 3] = [Self::Full, Self::Half, Self::Off];

    /// The wire and storage spelling. Stable: an identifier, not a label.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Half => "half",
            Self::Off => "off",
        }
    }

    /// What every cue's own amplitude is multiplied by.
    ///
    /// Half is **0.5 of the amplitude**, which is about −6 dB and reads as
    /// "clearly quieter" rather than as "barely there"; a perceptual halving
    /// would be nearer 0.25 and is a step too far for a table whose loudest
    /// sound is already a tap.
    #[must_use]
    pub const fn gain(self) -> f32 {
        match self {
            Self::Full => 1.0,
            Self::Half => 0.5,
            Self::Off => 0.0,
        }
    }

    /// Whether anything should be played at all.
    #[must_use]
    pub const fn audible(self) -> bool {
        !matches!(self, Self::Off)
    }
}

/// The cues this frame has decided on, and the last one it ever decided.
///
/// A queue and not a callback: the client decides on cues in three different
/// systems in the first half of a frame, and a sink that fired on each would
/// have no way to know that two of them were the same moment. Everything is
/// gathered, deduplicated, and drained once.
#[derive(Clone, Default, Debug)]
pub struct Cues {
    /// Decided this frame, not yet handed over.
    queue: Vec<Cue>,
    /// The last cue drained, for `/state` and for a test that wants to read
    /// what the client heard rather than listen for it.
    last: Option<Cue>,
    /// Whether the last question the table asked was this seat's to answer.
    ///
    /// The one remembered bit in the whole module, and it is what makes
    /// [`Cue::YourMove`] a flank. It is *not* reset by a question being
    /// answered: the acting seat is re-sent its own question every time
    /// anybody at the table says anything (an opponent holding priority, a
    /// print table arriving), and a client that chimed on each of those would
    /// be a metronome.
    waiting: bool,
}

impl Cues {
    /// An empty queue that has heard nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a cue, unless this frame has already decided on it.
    ///
    /// Deduplicating here and not at the sink is the "one sound played
    /// louder" rule: two opponents taking damage on one view is one event to
    /// a listener, however many numbers it puts on the table.
    pub fn push(&mut self, cue: Cue) {
        if !self.queue.contains(&cue) {
            self.queue.push(cue);
        }
    }

    /// Takes a cue back, if it has not been heard yet.
    ///
    /// The client answers some of its own questions — the standing orders and
    /// the autopilot both run in the same half-frame that installs a question
    /// — and a question the player never saw is not a question. Everything
    /// that submits an action goes through one door, so one call there
    /// withdraws the chime before the frame ends; a cue already drained is
    /// past withdrawing, and this is then a no-op, which is the right answer
    /// for a player answering a question they *did* see.
    pub fn retract(&mut self, cue: Cue) {
        self.queue.retain(|c| *c != cue);
    }

    /// Reads a view's life changes.
    ///
    /// `mine` is the chair this client is sitting in; every other seat is
    /// somebody else's, teammate included. A teammate's life is not this
    /// seat's own in any rules sense and the bar does not draw it as such
    /// ([`crate::lifeflash`] colours by direction, not by allegiance), so the
    /// sound does not either.
    pub fn note_life(&mut self, changes: &[Change], mine: PlayerId) {
        for change in changes.iter().filter(|c| c.started) {
            self.push(match (change.seat == mine, change.delta < 0) {
                (true, true) => Cue::MyLifeLost,
                (true, false) => Cue::MyLifeGained,
                (false, true) => Cue::TheirLifeLost,
                (false, false) => Cue::TheirLifeGained,
            });
        }
    }

    /// Takes the question the table is now asking.
    ///
    /// Called once per question arriving and never per frame, with whether it
    /// is addressed to this seat. Only the rising flank makes a sound; see
    /// [`Self::waiting`].
    pub fn note_question(&mut self, mine: bool) {
        if mine && !self.waiting {
            self.push(Cue::YourMove);
        }
        self.waiting = mine;
    }

    /// Takes a finished game.
    pub fn note_ending(&mut self, outcome: Outcome) {
        self.push(Cue::of_outcome(outcome));
    }

    /// Takes the engine's refusal of an action.
    pub fn note_refusal(&mut self) {
        self.push(Cue::Refused);
    }

    /// Hands over everything decided since the last drain.
    ///
    /// The last of them is remembered, which is the whole of `/state`'s
    /// `last_cue` and the reason a sound can be *proven* by a read rather
    /// than by somebody listening at the right moment.
    pub fn take(&mut self) -> Vec<Cue> {
        if let Some(last) = self.queue.last() {
            self.last = Some(*last);
        }
        std::mem::take(&mut self.queue)
    }

    /// The last cue handed over, if there has been one.
    #[must_use]
    pub fn last(&self) -> Option<Cue> {
        self.last
    }

    /// What is waiting to be heard, without taking it.
    #[must_use]
    pub fn pending(&self) -> &[Cue] {
        &self.queue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifeflash::{Ledger, MERGE};
    use baylee_core::ids::PlayerId;

    fn who(n: u8) -> PlayerId {
        PlayerId::new(n)
    }

    /// [`Cue::ALL`] is every variant and each of them once.
    ///
    /// It cannot be derived, so it can go stale — and a stale entry is a
    /// sound the sink never synthesises and nothing ever complains about.
    /// The names are the check because [`Cue::name`] is a `match` the
    /// compiler makes exhaustive: a tenth variant breaks that function, and
    /// a duplicate here shows up as a short set.
    #[test]
    fn every_cue_is_in_all() {
        let mut names: Vec<&str> = Cue::ALL.iter().map(|cue| cue.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Cue::ALL.len(), "two entries name one cue");
    }

    /// The endings are exactly what an outcome produces, and nothing else.
    ///
    /// Two predicates over the same three variants is a thing to get
    /// backwards once and never notice: a cue wrongly called an ending
    /// silences whatever shares its frame, and an ending not called one
    /// clips. So the list is built from [`Cue::of_outcome`] — the only thing
    /// that makes an ending — rather than typed out a second time.
    #[test]
    fn an_ending_is_what_an_outcome_produces() {
        let from_outcomes = [
            Cue::of_outcome(Outcome::YouWon),
            Cue::of_outcome(Outcome::YouLost),
            Cue::of_outcome(Outcome::Draw),
            Cue::of_outcome(Outcome::YourTeamWon(1)),
            Cue::of_outcome(Outcome::TheirTeamWon(2)),
        ];
        for cue in Cue::ALL {
            assert_eq!(
                cue.ends_the_game(),
                from_outcomes.contains(&cue),
                "{} disagrees about being an ending",
                cue.name()
            );
        }
    }

    fn table(life: &[i32]) -> Vec<baylee_view::SeatView> {
        life.iter()
            .enumerate()
            .map(|(i, &life)| baylee_view::SeatView {
                player: who(u8::try_from(i).expect("a small table")),
                life,
                poison: 0,
                energy: 0,
                hand_count: 7,
                library_count: 53,
                graveyard_count: 0,
                has_lost: false,
                mana_pool: baylee_view::ManaPoolView::default(),
                commanders: Vec::new(),
                commander_damage: Vec::new(),
            })
            .collect()
    }

    /// The ledger's own rule, carried through: the first view of a game is
    /// not twenty life arriving, so it is not a sound either.
    #[test]
    fn the_first_view_of_a_table_is_silent() {
        let mut ledger = Ledger::new();
        let mut cues = Cues::new();
        cues.note_life(&ledger.read(&table(&[20, 20])), who(0));
        assert!(cues.take().is_empty());
    }

    /// Three blockers dealing damage in three views are one number on the bar
    /// and one sound in the room.
    #[test]
    fn a_triple_block_is_heard_once() {
        let mut ledger = Ledger::new();
        let mut cues = Cues::new();
        ledger.read(&table(&[20, 20]));
        for life in [18, 16, 13] {
            cues.note_life(&ledger.read(&table(&[life, 20])), who(0));
            ledger.tick(MERGE / 2.0);
        }
        assert_eq!(cues.take(), vec![Cue::MyLifeLost]);
    }

    /// The counter-test: past the merge window it is a second thing
    /// happening, so the flash starts again and so does the sound.
    #[test]
    fn a_second_hit_after_the_window_is_heard_again() {
        let mut ledger = Ledger::new();
        let mut cues = Cues::new();
        ledger.read(&table(&[20, 20]));
        cues.note_life(&ledger.read(&table(&[18, 20])), who(0));
        assert_eq!(cues.take(), vec![Cue::MyLifeLost]);
        ledger.tick(MERGE * 2.0);
        cues.note_life(&ledger.read(&table(&[16, 20])), who(0));
        assert_eq!(cues.take(), vec![Cue::MyLifeLost]);
    }

    /// My life and somebody else's are different sounds; both directions are
    /// different again. All four, in two exchanges.
    #[test]
    fn four_ways_a_life_total_can_move_are_four_sounds() {
        let mut ledger = Ledger::new();
        let mut cues = Cues::new();
        ledger.read(&table(&[20, 20]));
        cues.note_life(&ledger.read(&table(&[17, 23])), who(0));
        assert_eq!(cues.take(), vec![Cue::MyLifeLost, Cue::TheirLifeGained]);
        ledger.tick(MERGE * 2.0);
        cues.note_life(&ledger.read(&table(&[20, 20])), who(0));
        assert_eq!(cues.take(), vec![Cue::MyLifeGained, Cue::TheirLifeLost]);
    }

    /// A sweeper that hits three opponents is one sound, not three.
    #[test]
    fn a_sweeper_is_one_sound_and_not_one_per_seat() {
        let mut ledger = Ledger::new();
        let mut cues = Cues::new();
        ledger.read(&table(&[20, 20, 20, 20]));
        cues.note_life(&ledger.read(&table(&[20, 17, 17, 17])), who(0));
        assert_eq!(cues.take(), vec![Cue::TheirLifeLost]);
    }

    /// The flank, twice: the seat being re-sent its own question says
    /// nothing, and the question coming back after an opponent's turn does.
    #[test]
    fn a_seat_is_only_told_once_that_it_is_being_waited_for() {
        let mut cues = Cues::new();
        cues.note_question(true);
        assert_eq!(cues.take(), vec![Cue::YourMove]);
        cues.note_question(true);
        assert!(cues.take().is_empty(), "the same question, re-sent");
        cues.note_question(false);
        assert!(cues.take().is_empty(), "somebody else's question");
        cues.note_question(true);
        assert_eq!(cues.take(), vec![Cue::YourMove], "asked again");
    }

    /// A question the standing orders answer inside the frame it arrived in
    /// is a question the player never saw.
    #[test]
    fn a_question_the_client_answers_itself_is_never_heard() {
        let mut cues = Cues::new();
        cues.note_question(true);
        cues.retract(Cue::YourMove);
        assert!(cues.take().is_empty());
        assert_eq!(
            cues.last(),
            None,
            "and nothing was heard, so nothing is last"
        );
    }

    /// …and the counter-test: taking an answer back after the cue has been
    /// handed over changes nothing, which is what a player answering a
    /// question they *did* hear looks like.
    #[test]
    fn a_cue_already_heard_cannot_be_taken_back() {
        let mut cues = Cues::new();
        cues.note_question(true);
        assert_eq!(cues.take(), vec![Cue::YourMove]);
        cues.retract(Cue::YourMove);
        assert_eq!(cues.last(), Some(Cue::YourMove));
    }

    /// Five sentences, three sounds — and the two collapses are the ones a
    /// team game needs.
    #[test]
    fn every_way_a_game_ends_has_a_sound() {
        assert_eq!(Cue::of_outcome(Outcome::YouWon), Cue::GameWon);
        assert_eq!(Cue::of_outcome(Outcome::YourTeamWon(2)), Cue::GameWon);
        assert_eq!(Cue::of_outcome(Outcome::YouLost), Cue::GameLost);
        assert_eq!(Cue::of_outcome(Outcome::TheirTeamWon(1)), Cue::GameLost);
        assert_eq!(Cue::of_outcome(Outcome::Draw), Cue::GameDrawn);
    }

    /// Which team won is not a thing a sound may say.
    #[test]
    fn two_teams_winning_sound_the_same_to_the_seats_that_won() {
        assert_eq!(
            Cue::of_outcome(Outcome::YourTeamWon(1)),
            Cue::of_outcome(Outcome::YourTeamWon(2))
        );
    }

    /// Every cue has a name, and no two share one — `/state` is read by a
    /// harness that has nothing else to tell them apart by.
    #[test]
    fn no_two_cues_answer_to_the_same_name() {
        let all = [
            Cue::MyLifeLost,
            Cue::MyLifeGained,
            Cue::TheirLifeLost,
            Cue::TheirLifeGained,
            Cue::YourMove,
            Cue::Refused,
            Cue::GameWon,
            Cue::GameLost,
            Cue::GameDrawn,
        ];
        let mut names: Vec<&str> = all.iter().map(|c| c.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), all.len());
        assert!(!names.iter().any(|n| n.is_empty()));
    }

    /// The three sources in one frame, in the order the client decides them.
    #[test]
    fn a_frame_can_decide_on_more_than_one_sound() {
        let mut ledger = Ledger::new();
        let mut cues = Cues::new();
        ledger.read(&table(&[20, 20]));
        cues.note_life(&ledger.read(&table(&[0, 20])), who(0));
        cues.note_refusal();
        cues.note_ending(Outcome::YouLost);
        assert_eq!(
            cues.take(),
            vec![Cue::MyLifeLost, Cue::Refused, Cue::GameLost]
        );
        assert_eq!(cues.last(), Some(Cue::GameLost));
    }
}
