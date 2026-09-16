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
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_view::{CounterKind, PlayerView};

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
    /// A card came off this seat's library and into its hand.
    ///
    /// Off the *library*, which is the whole of what tells a draw from a
    /// bounce: a card returned to hand from a graveyard arrives in exactly
    /// the same field of the same view, and only the library count moving
    /// with it says which of the two happened. [`Tally`] is where that is
    /// read.
    CardDrawn,
    /// A creature already on the battlefield gained +1/+1 counters.
    ///
    /// *Already there.* One that arrives with counters on it is a creature
    /// arriving, not counters being placed, and the two are different moments
    /// however alike the two views look.
    CreatureGrew,
    /// A creature already on the battlefield gained −1/−1 counters.
    ///
    /// Two variants and not one with a direction, for the reason there are
    /// four life cues and not two: they are two different sounds, and a
    /// `bool` in the middle of a match arm is a thing to get backwards.
    CreatureShrank,
}

impl Cue {
    /// Every cue, in no particular order but in *all* of them.
    ///
    /// The sink synthesises one sound per entry at startup, so a variant
    /// missing here is a variant that is silent at runtime and loud in no
    /// test. `every_cue_is_in_all` holds the two together by counting the
    /// arms of [`Cue::name`], which the compiler already forces to be
    /// exhaustive.
    pub const ALL: [Self; 12] = [
        Self::MyLifeLost,
        Self::MyLifeGained,
        Self::TheirLifeLost,
        Self::TheirLifeGained,
        Self::YourMove,
        Self::Refused,
        Self::GameWon,
        Self::GameLost,
        Self::GameDrawn,
        Self::CardDrawn,
        Self::CreatureGrew,
        Self::CreatureShrank,
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
            Self::CardDrawn => "CardDrawn",
            Self::CreatureGrew => "CreatureGrew",
            Self::CreatureShrank => "CreatureShrank",
        }
    }

    /// The largest count this cue can carry, past which it says "a lot".
    ///
    /// **One** for every cue but three, and that is the line rather than a
    /// limit: a life total moving by twelve and by one are the same event to
    /// the ear, and the number is already on the bar. Drawing three cards is
    /// not one event — it is three, arriving close enough together that the
    /// sink plays them as a burst — and so is a resolution that puts counters
    /// on three creatures. Those are the three the owner asked for by name,
    /// and the reason [`Beat`] exists at all.
    ///
    /// The two counted ceilings are different and both are the ear's number
    /// rather than the game's. **Seven** for a draw, which is the count this
    /// game has a word for — a hand, and what a *Wheel of Fortune* or a
    /// *Windfall* deals — and about where counting a run of single taps gives
    /// out anyway. **Five** for counters, because each of those is a two-note
    /// gesture rather than one tap and a run of gestures is counted less far.
    ///
    /// It is a number rather than a `bool` because the *sink* needs exactly
    /// this: it synthesises one buffer per count, so a cue on the wrong side
    /// of the line is either silent above one or six buffers nobody plays.
    #[must_use]
    pub const fn most(self) -> u8 {
        match self {
            Self::CardDrawn => 7,
            Self::CreatureGrew | Self::CreatureShrank => 5,
            _ => 1,
        }
    }

    /// Every count this cue is ever played at, which is `1..=most`.
    #[must_use]
    pub fn counts(self) -> std::ops::RangeInclusive<u8> {
        1..=self.most()
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
/// tuning to be done here — the sounds are balanced against each other
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

/// A cue, and how many of it.
///
/// The one thing this module's first draft said it would not do, done — and
/// the reason it is a wrapper rather than a field on [`Cue`] is that the
/// original argument still holds. A `Cue` stays a flat variant with no
/// payload, because every reader wants a *name*: a sink matches on it,
/// `/state` prints it, a future sound pack is keyed by it. The count is not
/// part of what the moment is called; it is how many of that moment landed
/// together, which is a property of the frame and lives here.
///
/// [`Cue::most`] says how far a given cue can count. For everything else
/// `count` is 1 and means nothing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Beat {
    /// What happened.
    pub cue: Cue,
    /// How many of it, from 1 to [`Cue::most`].
    pub count: u8,
}

impl Beat {
    /// One of something.
    #[must_use]
    pub const fn once(cue: Cue) -> Self {
        Self { cue, count: 1 }
    }

    /// `count` of something, clamped into the range [`Cue::most`] allows.
    ///
    /// Clamped *here* and nowhere else, so a reader may count honestly and
    /// hand over whatever it found: a *Windfall* for twelve is seven, and the
    /// arithmetic that decided twelve does not have to know the ceiling.
    #[must_use]
    pub const fn of(cue: Cue, count: u8) -> Self {
        let most = cue.most();
        Self {
            cue,
            count: if count < 1 {
                1
            } else if count > most {
                most
            } else {
                count
            },
        }
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
    queue: Vec<Beat>,
    /// The last beat drained, for `/state` and for a test that wants to read
    /// what the client heard rather than listen for it.
    ///
    /// The whole [`Beat`] and not just its cue, because the count is the new
    /// thing worth proving: "three cards were drawn and the sink was told
    /// three" is a read, where "it sounded like three" is somebody listening
    /// at the right moment.
    last: Option<Beat>,
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
        self.push_many(cue, 1);
    }

    /// Adds a cue `count` of which happened at once.
    ///
    /// A cue already decided on this frame is **raised** to the larger count
    /// rather than added to. It is the same rule `push` has always had, read
    /// one level up: two readings of one frame are two readings of the same
    /// thing, and the one that saw more saw all of it. Summing would make a
    /// second reader who found nothing new turn three cards into six.
    ///
    /// The count is clamped by [`Beat::of`]; see [`Cue::most`] for the
    /// ceilings and why they are the ear's numbers.
    pub fn push_many(&mut self, cue: Cue, count: u8) {
        let beat = Beat::of(cue, count);
        if let Some(already) = self.queue.iter_mut().find(|b| b.cue == cue) {
            already.count = already.count.max(beat.count);
        } else {
            self.queue.push(beat);
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
        self.queue.retain(|beat| beat.cue != cue);
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

    /// Takes what one view moved, as [`Tally`] read it.
    ///
    /// Three counted cues out of one reading, and the counts go straight in:
    /// this is the only place in the module where "how many" survives as far
    /// as the queue, because these are the only three moments that *have* a
    /// how many. A flow with nothing in it pushes nothing, which is what
    /// every view that merely re-states the table looks like.
    pub fn note_flow(&mut self, flow: &Flow) {
        for (cue, count) in [
            (Cue::CardDrawn, flow.drawn),
            (Cue::CreatureGrew, flow.grew),
            (Cue::CreatureShrank, flow.shrank),
        ] {
            if count > 0 {
                self.push_many(cue, count);
            }
        }
    }

    /// Hands over everything decided since the last drain.
    ///
    /// The last of them is remembered, which is the whole of `/state`'s
    /// `last_cue` and the reason a sound can be *proven* by a read rather
    /// than by somebody listening at the right moment.
    pub fn take(&mut self) -> Vec<Beat> {
        if let Some(last) = self.queue.last() {
            self.last = Some(*last);
        }
        std::mem::take(&mut self.queue)
    }

    /// The last beat handed over, if there has been one.
    #[must_use]
    pub fn last(&self) -> Option<Beat> {
        self.last
    }

    /// What is waiting to be heard, without taking it.
    #[must_use]
    pub fn pending(&self) -> &[Beat] {
        &self.queue
    }
}

/// What one view moved that the ear is owed.
///
/// Three counts and nothing else, because three is all there is to say: how
/// many cards were drawn, how many creatures grew, how many shrank.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Flow {
    /// Cards that came off this seat's library and into its hand.
    pub drawn: u8,
    /// Creatures that gained +1/+1 counters while already on the battlefield.
    pub grew: u8,
    /// Creatures that gained −1/−1 counters, likewise.
    pub shrank: u8,
}

impl Flow {
    /// Whether this view said anything at all.
    #[must_use]
    pub const fn is_quiet(&self) -> bool {
        self.drawn == 0 && self.grew == 0 && self.shrank == 0
    }
}

/// The reader that turns two views into a [`Flow`].
///
/// The same shape as [`crate::lifeflash::Ledger`] and for the same reason:
/// the engine sends a whole [`baylee_view::PlayerView`] per change and no
/// events, so "what happened" is the difference between two of them and
/// there is nowhere else to compute it. A frame later the previous view is
/// gone.
///
/// # Why there is no clock in it, where the life ledger has one
///
/// A draw of three cards arrives in **one** view, not three.
/// `gamehost::Session::pump` runs the engine until a seat that answers over a
/// socket has a question, and only *then* builds a view for every such seat —
/// so everything between two questions, however much of it there is, is one
/// difference. `lifeflash::MERGE` exists because combat damage puts a
/// question between its hits and genuinely does send several views; a
/// resolving *Divination* puts no question anywhere and cannot.
///
/// If that ever changed, the failure is mild and worth naming: the ear would
/// get two beats of one where it now gets one beat of two, which is an
/// honest rendering of two events. It would not go silent.
///
/// # The first view is silent
///
/// The ledger's rule, carried over. An opening hand of seven is not seven
/// cards being drawn where a player can hear it, and a client that joined a
/// game in progress has not just watched the whole board arrive.
#[derive(Clone, Default, Debug)]
pub struct Tally {
    /// The cards in this seat's hand as of the last view read.
    hand: Vec<ObjectId>,
    /// How many cards were left in this seat's library.
    library: u32,
    /// Every battlefield object and its power/toughness counters, as
    /// `(object, plus, minus)`, sorted by object so a lookup is a search.
    counters: Vec<(ObjectId, u16, u16)>,
    /// Whether anything has been read yet. Not `hand.is_empty()`: a seat
    /// with no cards in hand is an ordinary state of a game.
    seeded: bool,
}

impl Tally {
    /// A reader that has seen nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads a view against the last one, and remembers it.
    ///
    /// # A draw is a library, not a hand
    ///
    /// Counting the ids that are new to the hand answers the wrong question:
    /// a bounced creature, a card put back by a *Brainstorm* and a commander
    /// declining CR 903.9b all arrive there the same way. What separates them
    /// is the **library count**, and the answer is the smaller of the two —
    /// so milling five and drawing one is one card, a bounce with no draw is
    /// none, and drawing one while bouncing one is one. It is honest in the
    /// direction that matters: it can undercount a turn that does two things
    /// at once, and it cannot invent a draw that did not happen.
    ///
    /// # Counters are creatures, not counters
    ///
    /// The count is how many **objects** gained power/toughness counters, not
    /// how many counters were placed. Three creatures each taking one +1/+1
    /// is three; one creature taking three is one, because that is one card
    /// resolving on one permanent and the plate under it says the rest. Only
    /// objects that were in the *previous* view are counted, so a creature
    /// entering with counters on it is a creature arriving.
    pub fn read(&mut self, view: &PlayerView) -> Flow {
        let hand: Vec<ObjectId> = view.hand.iter().map(|card| card.id).collect();
        let library = view
            .seats
            .iter()
            .find(|seat| seat.player == view.seat)
            .map_or(0, |seat| seat.library_count);
        let mut counters: Vec<(ObjectId, u16, u16)> = view
            .battlefield
            .iter()
            .map(|object| {
                let of = |want: CounterKind| {
                    object
                        .counters
                        .iter()
                        .find(|entry| entry.kind == want)
                        .map_or(0, |entry| entry.count)
                };
                (
                    object.id,
                    of(CounterKind::PLUS_ONE),
                    of(CounterKind::MINUS_ONE),
                )
            })
            .collect();
        counters.sort_unstable_by_key(|(id, _, _)| *id);

        let flow = if self.seeded {
            let arrived = hand.iter().filter(|id| !self.hand.contains(id)).count();
            let off_the_top = self.library.saturating_sub(library) as usize;
            let (mut grew, mut shrank) = (0_usize, 0_usize);
            for &(id, plus, minus) in &counters {
                let Ok(at) = self.counters.binary_search_by_key(&id, |(id, _, _)| *id) else {
                    continue;
                };
                let (_, was_plus, was_minus) = self.counters[at];
                grew += usize::from(plus > was_plus);
                shrank += usize::from(minus > was_minus);
            }
            Flow {
                drawn: small(arrived.min(off_the_top)),
                grew: small(grew),
                shrank: small(shrank),
            }
        } else {
            Flow::default()
        };

        self.hand = hand;
        self.library = library;
        self.counters = counters;
        self.seeded = true;
        flow
    }
}

/// A count, brought into a byte without wrapping.
///
/// Saturating and not clamped to a cue's ceiling: the ceiling is
/// [`Beat::of`]'s job and belongs in one place, and a [`Flow`] is allowed to
/// say twelve cards were drawn even though nothing will play twelve.
fn small(n: usize) -> u8 {
    u8::try_from(n).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifeflash::{Ledger, MERGE};
    use baylee_core::ids::PlayerId;

    fn who(n: u8) -> PlayerId {
        PlayerId::new(n)
    }

    /// Drains the queue and keeps only the names.
    ///
    /// Most of what this module decides has no amount in it, and a test that
    /// spelled `Beat { cue, count: 1 }` at every assertion would be saying
    /// "one" forty times about cues that can never be anything else. The
    /// counted three are asserted on as whole [`Beat`]s, where the count is
    /// the point.
    fn heard(cues: &mut Cues) -> Vec<Cue> {
        cues.take().into_iter().map(|beat| beat.cue).collect()
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
        assert!(heard(&mut cues).is_empty());
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
        assert_eq!(heard(&mut cues), vec![Cue::MyLifeLost]);
    }

    /// The counter-test: past the merge window it is a second thing
    /// happening, so the flash starts again and so does the sound.
    #[test]
    fn a_second_hit_after_the_window_is_heard_again() {
        let mut ledger = Ledger::new();
        let mut cues = Cues::new();
        ledger.read(&table(&[20, 20]));
        cues.note_life(&ledger.read(&table(&[18, 20])), who(0));
        assert_eq!(heard(&mut cues), vec![Cue::MyLifeLost]);
        ledger.tick(MERGE * 2.0);
        cues.note_life(&ledger.read(&table(&[16, 20])), who(0));
        assert_eq!(heard(&mut cues), vec![Cue::MyLifeLost]);
    }

    /// My life and somebody else's are different sounds; both directions are
    /// different again. All four, in two exchanges.
    #[test]
    fn four_ways_a_life_total_can_move_are_four_sounds() {
        let mut ledger = Ledger::new();
        let mut cues = Cues::new();
        ledger.read(&table(&[20, 20]));
        cues.note_life(&ledger.read(&table(&[17, 23])), who(0));
        assert_eq!(
            heard(&mut cues),
            vec![Cue::MyLifeLost, Cue::TheirLifeGained]
        );
        ledger.tick(MERGE * 2.0);
        cues.note_life(&ledger.read(&table(&[20, 20])), who(0));
        assert_eq!(
            heard(&mut cues),
            vec![Cue::MyLifeGained, Cue::TheirLifeLost]
        );
    }

    /// A sweeper that hits three opponents is one sound, not three.
    #[test]
    fn a_sweeper_is_one_sound_and_not_one_per_seat() {
        let mut ledger = Ledger::new();
        let mut cues = Cues::new();
        ledger.read(&table(&[20, 20, 20, 20]));
        cues.note_life(&ledger.read(&table(&[20, 17, 17, 17])), who(0));
        assert_eq!(heard(&mut cues), vec![Cue::TheirLifeLost]);
    }

    /// The flank, twice: the seat being re-sent its own question says
    /// nothing, and the question coming back after an opponent's turn does.
    #[test]
    fn a_seat_is_only_told_once_that_it_is_being_waited_for() {
        let mut cues = Cues::new();
        cues.note_question(true);
        assert_eq!(heard(&mut cues), vec![Cue::YourMove]);
        cues.note_question(true);
        assert!(heard(&mut cues).is_empty(), "the same question, re-sent");
        cues.note_question(false);
        assert!(heard(&mut cues).is_empty(), "somebody else's question");
        cues.note_question(true);
        assert_eq!(heard(&mut cues), vec![Cue::YourMove], "asked again");
    }

    /// A question the standing orders answer inside the frame it arrived in
    /// is a question the player never saw.
    #[test]
    fn a_question_the_client_answers_itself_is_never_heard() {
        let mut cues = Cues::new();
        cues.note_question(true);
        cues.retract(Cue::YourMove);
        assert!(heard(&mut cues).is_empty());
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
        assert_eq!(heard(&mut cues), vec![Cue::YourMove]);
        cues.retract(Cue::YourMove);
        assert_eq!(cues.last(), Some(Beat::once(Cue::YourMove)));
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
            heard(&mut cues),
            vec![Cue::MyLifeLost, Cue::Refused, Cue::GameLost]
        );
        assert_eq!(cues.last(), Some(Beat::once(Cue::GameLost)));
    }

    // ------------------------------------------------------------ the tally

    /// A table with `hand` cards in this seat's hand and `library` left.
    ///
    /// Ids run `1..=hand`, so growing the hand by one and shrinking the
    /// library by one is what a draw looks like from here — which is exactly
    /// what the reader is asked to tell from a bounce.
    fn seat_with(hand: u32, library: u32) -> PlayerView {
        let cards: Vec<(&str, u32, u32)> = (1..=hand).map(|slot| ("a card", 1, slot)).collect();
        let mut view = crate::test_support::ViewBuilder::new(2)
            .with_hand(cards)
            .build();
        for seat in &mut view.seats {
            seat.library_count = library;
        }
        view
    }

    /// Puts `plus` +1/+1 and `minus` −1/−1 counters on creature `slot`.
    fn creature(slot: u32, plus: u16, minus: u16) -> baylee_view::PublicObject {
        let mut object = crate::test_support::token(slot, 0, "a creature", 2, 2);
        for (kind, count) in [
            (CounterKind::PLUS_ONE, plus),
            (CounterKind::MINUS_ONE, minus),
        ] {
            if count > 0 {
                object
                    .counters
                    .push(baylee_view::CounterEntry { kind, count });
            }
        }
        object
    }

    /// A table whose battlefield is exactly these creatures.
    fn board(creatures: Vec<baylee_view::PublicObject>) -> PlayerView {
        crate::test_support::ViewBuilder::new(2)
            .with_battlefield(0, creatures)
            .build()
    }

    /// The rule the whole module opens with, once more: the first view is not
    /// an opening hand of seven being drawn where anybody could hear it.
    #[test]
    fn an_opening_hand_is_not_seven_draws() {
        let mut tally = Tally::new();
        let mut cues = Cues::new();
        cues.note_flow(&tally.read(&seat_with(7, 53)));
        assert!(heard(&mut cues).is_empty());
    }

    /// Three cards off the top is one cue that says three.
    ///
    /// And it is **one view**, not three — see [`Tally`] for why a
    /// *Divination* resolving cannot send more than one. This is the owner's
    /// request turned into a number: "so that when several cards are drawn,
    /// you hear that too".
    #[test]
    fn drawing_three_is_heard_as_three() {
        let mut tally = Tally::new();
        let mut cues = Cues::new();
        tally.read(&seat_with(4, 53));
        cues.note_flow(&tally.read(&seat_with(7, 50)));
        assert_eq!(cues.take(), vec![Beat::of(Cue::CardDrawn, 3)]);
    }

    /// …and one card is one, which is every turn of every game.
    #[test]
    fn drawing_for_the_turn_is_heard_as_one() {
        let mut tally = Tally::new();
        let mut cues = Cues::new();
        tally.read(&seat_with(4, 53));
        cues.note_flow(&tally.read(&seat_with(5, 52)));
        assert_eq!(cues.take(), vec![Beat::once(Cue::CardDrawn)]);
    }

    /// A card arriving in hand from anywhere but the library is silent.
    ///
    /// The counter-test that makes the cue mean "drew" rather than "gained a
    /// card": an *Unsummon* on your own creature, a commander declining
    /// CR 903.9b's replacement and a *Regrowth* all put a card in a hand and
    /// take nothing off a library.
    #[test]
    fn a_card_bounced_back_to_hand_is_not_a_draw() {
        let mut tally = Tally::new();
        let mut cues = Cues::new();
        tally.read(&seat_with(4, 53));
        cues.note_flow(&tally.read(&seat_with(5, 53)));
        assert!(heard(&mut cues).is_empty(), "a bounce sounded like a draw");
    }

    /// A library that shrinks without the hand growing is silent too.
    ///
    /// The other half: a mill, a fetchland's shuffle, a cascade. Both halves
    /// matter because the reader takes the *smaller* of the two, and a reader
    /// that took either one alone would be wrong on one of these two tests.
    #[test]
    fn milling_five_is_not_drawing_five() {
        let mut tally = Tally::new();
        let mut cues = Cues::new();
        tally.read(&seat_with(4, 53));
        cues.note_flow(&tally.read(&seat_with(4, 48)));
        assert!(heard(&mut cues).is_empty(), "a mill sounded like a draw");
    }

    /// A draw and a bounce on one view is one draw.
    ///
    /// The case that decides the arithmetic. Two cards arrive in hand and one
    /// came off the library, so the honest answer is one — and the failure a
    /// naive count would make is the loud one: two.
    #[test]
    fn a_draw_beside_a_bounce_is_one_card() {
        let mut tally = Tally::new();
        let mut cues = Cues::new();
        tally.read(&seat_with(4, 53));
        cues.note_flow(&tally.read(&seat_with(6, 52)));
        assert_eq!(cues.take(), vec![Beat::once(Cue::CardDrawn)]);
    }

    /// A *Windfall* is a hand's worth, not sixty.
    #[test]
    fn a_draw_past_the_ceiling_is_the_ceiling() {
        let mut tally = Tally::new();
        let mut cues = Cues::new();
        tally.read(&seat_with(0, 60));
        let flow = tally.read(&seat_with(20, 40));
        assert_eq!(flow.drawn, 20, "the reader counts honestly");
        cues.note_flow(&flow);
        assert_eq!(
            cues.take(),
            vec![Beat::of(Cue::CardDrawn, Cue::CardDrawn.most())],
            "and the queue is what clamps"
        );
    }

    /// Counters on three creatures is one cue that says three.
    ///
    /// **Objects, not counters.** A *Cathars' Crusade* trigger putting one
    /// +1/+1 on each of three creatures is three; one *Hardened Scales*
    /// making a single creature take four is one, because that is one card
    /// resolving on one permanent and the plate under it says the rest.
    #[test]
    fn counters_on_three_creatures_are_heard_as_three() {
        let mut tally = Tally::new();
        let mut cues = Cues::new();
        tally.read(&board(vec![
            creature(1, 0, 0),
            creature(2, 0, 0),
            creature(3, 0, 0),
            creature(4, 0, 0),
        ]));
        cues.note_flow(&tally.read(&board(vec![
            creature(1, 1, 0),
            creature(2, 1, 0),
            creature(3, 1, 0),
            creature(4, 0, 0),
        ])));
        assert_eq!(cues.take(), vec![Beat::of(Cue::CreatureGrew, 3)]);
    }

    /// …and four counters on one creature is one.
    #[test]
    fn four_counters_on_one_creature_are_heard_as_one() {
        let mut tally = Tally::new();
        let mut cues = Cues::new();
        tally.read(&board(vec![creature(1, 0, 0)]));
        cues.note_flow(&tally.read(&board(vec![creature(1, 4, 0)])));
        assert_eq!(cues.take(), vec![Beat::once(Cue::CreatureGrew)]);
    }

    /// The two directions are two sounds, and a view can carry both.
    ///
    /// A fight against a wither creature, or a *Bloodflow Connoisseur* beside
    /// an infect blocker: one creature grows, another shrinks, on one view.
    #[test]
    fn growing_and_shrinking_are_two_sounds() {
        let mut tally = Tally::new();
        let mut cues = Cues::new();
        tally.read(&board(vec![creature(1, 0, 0), creature(2, 0, 0)]));
        cues.note_flow(&tally.read(&board(vec![creature(1, 2, 0), creature(2, 0, 1)])));
        assert_eq!(
            cues.take(),
            vec![
                Beat::once(Cue::CreatureGrew),
                Beat::once(Cue::CreatureShrank)
            ]
        );
    }

    /// A creature that arrives with counters on it is a creature arriving.
    ///
    /// The line the cue's own doc draws, and the one a diff over the whole
    /// battlefield would get wrong every time: a *Scute Mob*, a kicked
    /// *Rite of Replication*, any token made with +1/+1 counters. The view
    /// looks identical to a counter being placed — a battlefield entry with
    /// counters on it that was not there before — and only "was it there
    /// before" tells them apart.
    #[test]
    fn a_creature_that_enters_with_counters_is_not_counters_being_placed() {
        let mut tally = Tally::new();
        let mut cues = Cues::new();
        tally.read(&board(vec![creature(1, 0, 0)]));
        cues.note_flow(&tally.read(&board(vec![creature(1, 0, 0), creature(2, 5, 0)])));
        assert!(
            heard(&mut cues).is_empty(),
            "an arrival sounded like growth"
        );
    }

    /// Counters coming *off* a creature say nothing.
    ///
    /// A vanishing permanent losing time counters, a −1/−1 removed by a
    /// *Nest Invader*'s owner at end of turn: the owner asked for the sound
    /// of counters being *placed*, and a cue is a flank in one direction.
    #[test]
    fn counters_taken_away_are_silent() {
        let mut tally = Tally::new();
        let mut cues = Cues::new();
        tally.read(&board(vec![creature(1, 3, 0)]));
        cues.note_flow(&tally.read(&board(vec![creature(1, 1, 0)])));
        assert!(heard(&mut cues).is_empty());
    }

    /// A view that says nothing new makes no sound.
    ///
    /// The common case by a distance: the acting seat is re-sent its own
    /// question every time anybody at the table says anything, so most views
    /// a client sees are the previous one again.
    #[test]
    fn a_view_that_repeats_itself_is_silent() {
        let mut tally = Tally::new();
        let mut cues = Cues::new();
        tally.read(&seat_with(7, 53));
        let flow = tally.read(&seat_with(7, 53));
        assert!(flow.is_quiet());
        cues.note_flow(&flow);
        assert!(heard(&mut cues).is_empty());
    }

    /// Two readings of one frame do not add up.
    ///
    /// `push_many` raises rather than sums, which is `push`'s old rule read
    /// one level up. The failure it stops is the loud one: a second reader
    /// finding the same three cards would otherwise make it six.
    #[test]
    fn one_frame_read_twice_is_still_one_frame() {
        let mut cues = Cues::new();
        cues.push_many(Cue::CardDrawn, 3);
        cues.push_many(Cue::CardDrawn, 2);
        assert_eq!(cues.take(), vec![Beat::of(Cue::CardDrawn, 3)]);
    }

    /// Every counted cue can say every count it claims to.
    ///
    /// [`Cue::most`] is read by the sink to decide how many buffers to build,
    /// so a ceiling that does not round-trip through [`Beat::of`] is a burst
    /// that finds no sound at the top of its range.
    #[test]
    fn a_beat_can_carry_every_count_its_cue_allows() {
        for cue in Cue::ALL {
            for count in cue.counts() {
                assert_eq!(Beat::of(cue, count).count, count, "{}", cue.name());
            }
            assert_eq!(Beat::of(cue, 0).count, 1, "{} went to nothing", cue.name());
            assert_eq!(
                Beat::of(cue, u8::MAX).count,
                cue.most(),
                "{} went past its ceiling",
                cue.name()
            );
        }
    }
}
