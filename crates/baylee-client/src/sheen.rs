//! When a card catches the light.
//!
//! The card shaders used to sweep a bright band across every card in the hand,
//! in the preview and in the stack panel, continuously, on a six-second loop.
//! It looked expensive and it said nothing: a mark every card wears at all
//! times is not a mark. Measured on a still table with nothing hovered, a
//! 24×16 frame diff put *all* of the change in the hand zone — mean 18 to 30
//! per cell against a table at zero — which is also the answer to "the cards
//! flicker at rest".
//!
//! So the sweep happens **once**, on the three occasions where a card is new
//! to the player: it is drawn, it is played, or its preview is opened. This
//! module is the part that decides that. The band itself is `sweep_amount` in
//! `shaders/card_common.wgsl`, shared by the table shader and its UI twin so
//! that a card picked up off the table keeps the light it caught.
//!
//! # Why a resource and not a spawn hook
//!
//! The HUD tree is rebuilt on every hover change, so a sweep started when an
//! *entity* appears would replay on every pointer move — which is the
//! everywhere-at-once the owner asked to be rid of, through a different door.
//! A sweep therefore belongs to a `(card, surface)` pair and to the moment
//! that pair came into being, and it is remembered here across rebuilds: a
//! rebuild in the middle of a sweep re-bakes the same start and the band
//! carries on where it was.
//!
//! # Why it rides the material key
//!
//! The start is baked into the material and the shader does the arithmetic,
//! so nothing touches a uniform while a card is sweeping — the constraint
//! `cardmat`'s own header states, and the reason a GL backend could draw any
//! of this.
//! A sweeping look is nonetheless transient, and the two material stores
//! answer that differently because they are asked differently. `sync_scene`
//! runs every frame, so a look it did not cache would be a fresh material and
//! a fresh handle sixty times a second — exactly the uniform traffic that
//! header forbids; it caches the sweeping look like any other and drops the
//! entry again through [`Sheen::live`] once the band has crossed. The UI tree
//! is rebuilt only when `HudRevision` changes, and each node owns its handle
//! until it despawns, so `UiCardMaterials` builds a sweeping look outside its
//! cache and nothing accumulates.

use baylee_client_core::zones::{Move, Passage};
use baylee_core::ids::ObjectId;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;

/// One card's sheen, as the material key carries it.
///
/// Times are quantised to the millisecond so the key can be `Hash` and `Eq` —
/// which also means two cards that started on the same frame share one
/// material, and an opening hand of seven is seven materials rather than one
/// only because each is given its own duration.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Sweep {
    /// When it began, in milliseconds on the app's clock.
    at_ms: u32,
    /// How long it takes, in milliseconds. Never zero.
    ms: u32,
    /// Which door the card came through, when it came through one of the
    /// five the player is owed a picture of.
    ///
    /// `None` is the plain arrival — drawn, cast, previewed — and is what
    /// this module started life drawing and every card still draws most of
    /// the time. A door replaces the band with a figure and a colour of its
    /// own, and is in the key like everything else here: a creature dying and
    /// a creature being exiled are two materials, which is the point.
    door: Option<Passage>,
}

impl Sweep {
    /// When it began, in seconds, for the shader's `globals.time`.
    #[must_use]
    pub fn at(self) -> f32 {
        self.at_ms as f32 / 1000.0
    }

    /// One over its duration, in seconds — what the shader multiplies by.
    #[must_use]
    pub fn rate(self) -> f32 {
        1000.0 / self.ms as f32
    }

    /// Whether it is over at `now` seconds.
    ///
    /// A clock *behind* the start counts as over too. The clock wraps every
    /// hour (`Time::wrap_period`), and a sweep started a moment before the
    /// wrap would otherwise sit at a negative phase for the next hour — the
    /// shader draws nothing there, so the band would never cross and the
    /// entry would never be retired.
    #[must_use]
    fn done(self, now: f32) -> bool {
        now < self.at() || now > self.at() + self.ms as f32 / 1000.0
    }

    /// The door this sweep is drawing, if it is drawing one.
    #[must_use]
    pub fn door(self) -> Option<Passage> {
        self.door
    }

    /// A departure's sweep, built where the card is leaving rather than by
    /// [`Sheen`].
    ///
    /// A card on its way off the table is out of the board model and out of
    /// `SceneIndex::cards`, so nothing will ever look its material up again —
    /// which is exactly why it is not in [`Sheen::running`] either. It is
    /// given the exit's own length rather than a place in the burst counter, for
    /// the same reason: it belongs to the exit it rides on, not to a run of
    /// arrivals.
    ///
    /// [`Sheen::running`]: Sheen
    #[must_use]
    pub fn leaving(now: f32, door: Passage, seconds: f32) -> Self {
        Self {
            at_ms: (now * 1000.0) as u32,
            ms: ((seconds * 1000.0) as u32).max(1),
            door: Some(door),
        }
    }
}

/// Which drawing of a card a sheen belongs to.
///
/// The same card is drawn in up to three places at once, and a sheen is about
/// *this drawing having appeared* rather than about the card. Hovering a land
/// that has been on the table all game opens a preview that is new even
/// though the permanent is not, which is exactly the case one key per card
/// would get wrong.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Surface {
    /// A card in the hand zone: it was drawn.
    Hand,
    /// A permanent on the felt: it was played.
    Table,
    /// The hover preview: it was opened.
    Preview,
}

/// How long the first sweep of a burst takes.
const BASE: f32 = 0.95;

/// What each further card in the same burst multiplies that by.
///
/// The owner's words are "a little bit quicker for each new card appearing",
/// and a burst is usually an opening hand: seven cards at 0.86ⁿ runs 0.95 s
/// down to 0.38 s, which reads as a hand being dealt rather than as seven
/// separate events.
const QUICKER: f32 = 0.86;

/// How short a sweep is allowed to get.
const FLOOR: f32 = 0.34;

/// How long a gap ends a burst.
///
/// Two cards drawn in the same frame are one burst; a card played a turn
/// later starts again at [`BASE`], because the counter is about a run of
/// arrivals and not about how long the game has gone on.
const GAP: f32 = 0.60;

/// Which cards are catching the light, and which have already caught it.
#[derive(Resource, Default)]
pub struct Sheen {
    /// The sweeps still running, by the drawing they belong to.
    running: HashMap<(ObjectId, Surface), Sweep>,
    /// What was in the hand and on the battlefield last time this looked, so
    /// "drawn" and "played" are transitions rather than states.
    ///
    /// Membership rather than zone: a card is in exactly one of these at a
    /// time, and a card leaving one to enter the other is the ordinary way a
    /// spell is cast.
    in_hand: HashSet<ObjectId>,
    on_table: HashSet<ObjectId>,
    /// What the preview was showing.
    previewing: Option<ObjectId>,
    /// How many sweeps have started in the run this one belongs to, and when
    /// the last one did.
    burst: u32,
    last: f32,
    /// The clock the last look read, so a caller holding a [`Sweep`] can ask
    /// whether it is still crossing without a `Time` of its own.
    now: f32,
}

impl Sheen {
    /// The sheen a card's drawing is wearing, for the material key.
    #[must_use]
    pub fn of(&self, card: ObjectId, surface: Surface) -> Option<Sweep> {
        self.running.get(&(card, surface)).copied()
    }

    /// Whether a sweep is still crossing, on the clock the last look read.
    ///
    /// `table::sync_scene` asks it about the keys in its material cache: a
    /// look with a finished sweep in it is one nothing will ever ask for
    /// again, so the entry can go. Answering from the stored clock rather
    /// than from `Time` is what keeps the caller from having to agree with
    /// this module about *which* clock.
    #[must_use]
    pub fn live(&self, sweep: Sweep) -> bool {
        !sweep.done(self.now)
    }

    /// The clock the last look read.
    ///
    /// A departing card is dressed where it leaves rather than here, and it
    /// has to be dressed on the clock the shader compares against — which is
    /// `Time::elapsed_secs_wrapped`, read once a frame in `watch_for_arrivals`
    /// and not the unwrapped one any other caller would reach for.
    #[must_use]
    pub fn now(&self) -> f32 {
        self.now
    }

    /// Starts whatever has just arrived, and forgets what has finished.
    ///
    /// Called once a frame with the board as it now is. Everything it decides
    /// is a set difference, which is what makes it idempotent: called twice
    /// on the same board it starts nothing the second time, and that is what
    /// keeps a HUD rebuild from replaying a sweep.
    pub fn observe(
        &mut self,
        now: f32,
        hand: impl Iterator<Item = ObjectId>,
        table: impl Iterator<Item = ObjectId>,
        previewing: Option<ObjectId>,
        reduce_motion: bool,
    ) {
        let hand: HashSet<ObjectId> = hand.collect();
        let table: HashSet<ObjectId> = table.collect();
        self.now = now;
        self.running.retain(|_, sweep| !sweep.done(now));

        if reduce_motion {
            // Nothing sweeps, and nothing is remembered as having swept: a
            // player who turns the setting back on should see the next card
            // that arrives, not a backlog.
            self.running.clear();
        } else {
            // Sorted, so a burst deals its cards in a stable order and the
            // durations do not depend on how a `HashSet` happened to hash.
            // Without this the seven cards of an opening hand would take
            // seven durations in an order that changed between runs, which
            // is a difference a screenshot can see.
            let mut arrived: Vec<(ObjectId, Surface)> = hand
                .difference(&self.in_hand)
                .map(|id| (*id, Surface::Hand))
                .chain(
                    table
                        .difference(&self.on_table)
                        .map(|id| (*id, Surface::Table)),
                )
                .collect();
            arrived.sort_unstable_by_key(|(id, _)| (id.slot(), id.generation()));
            // A preview is opened rather than arriving, so it is its own
            // event and comes first: it is the one the player is looking
            // straight at.
            if let Some(shown) = previewing
                && previewing != self.previewing
            {
                arrived.insert(0, (shown, Surface::Preview));
            }
            for (id, surface) in arrived {
                let sweep = self.begin(now);
                self.running.insert((id, surface), sweep);
            }
        }

        self.in_hand = hand;
        self.on_table = table;
        self.previewing = previewing;
    }

    /// Tells the sweeps that have just started which door their card came in
    /// through.
    ///
    /// A second call rather than a sixth argument to [`observe`], and the
    /// reason is which questions the two answer. `observe` reads *membership*
    /// — a card is in the hand now and was not before — which is all a plain
    /// arrival needs and is available every frame. A door needs both ends of
    /// a move, which only [`baylee_client_core::zones::Tracker`] knows and
    /// only on the frame a new view arrives. Folding them together would put
    /// a list of moves into every one of this module's tests to say nothing
    /// about most of them.
    ///
    /// Only the two doors *back on to* the battlefield are stamped here. The
    /// three that leave it have no sweep in `running` to stamp: the card is
    /// out of the board model by the time the move is read, so its departure
    /// is minted where the exit pose is — see [`Sweep::leaving`].
    ///
    /// Stamping twice is harmless and is relied on: a frame that draws
    /// nothing leaves the moves undrawn, and the next one reads them again.
    ///
    /// [`observe`]: Sheen::observe
    pub fn usher(&mut self, moves: &[Move]) {
        for step in moves {
            let Some(door) = step.passage() else {
                continue;
            };
            if door.is_departure() {
                continue;
            }
            if let Some(sweep) = self.running.get_mut(&(step.object, Surface::Table)) {
                sweep.door = Some(door);
            }
        }
    }

    /// Forgets everything, when a duel closes.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// The next sweep in the current burst.
    fn begin(&mut self, now: f32) -> Sweep {
        // Absolute, because the clock wraps: an hour into a session `now`
        // restarts at zero and every later comparison would say "no gap".
        if (now - self.last).abs() > GAP {
            self.burst = 0;
        }
        let seconds = (BASE * QUICKER.powi(self.burst.min(16) as i32)).max(FLOOR);
        self.burst += 1;
        self.last = now;
        Sweep {
            at_ms: (now * 1000.0) as u32,
            ms: (seconds * 1000.0) as u32,
            // The burst is arrivals, and an arrival with a door of its own
            // is stamped by `usher` a moment later, from the moves.
            door: None,
        }
    }
}

/// Notices what has arrived, once a frame, before anything draws it.
///
/// Ordered ahead of `table::sync_scene` and `hud::sync_overlay` in
/// `DuelSet::Present`, so a card placed on the felt this frame is placed with
/// its sheen already decided. The other order works too and is a frame worse:
/// the sweep would start on the frame *after* the card appeared, which on a
/// third-of-a-second sweep is a visible fraction of it.
///
/// The clock is `Time::elapsed_secs_wrapped`, and the `_wrapped` is the whole
/// point: `bevy_render`'s `GlobalsUniform` is written from exactly that call,
/// so it is the clock the shader compares the sweep's start against. Reading
/// the unwrapped one here agrees with it for an hour and then never again —
/// `Time::wrap_period` is one hour, after which `globals.time` restarts at
/// zero while a start baked from `elapsed_secs` keeps counting.
pub fn watch_for_arrivals(
    time: Res<Time>,
    duel: Res<crate::Duel>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut sheen: ResMut<Sheen>,
    mut watch: ResMut<crate::table::ZoneWatch>,
) {
    let Some(board) = duel.board.as_ref() else {
        // No board: between duels, or before the first view. Whatever the
        // last one left behind is not this one's.
        if !sheen.running.is_empty() || !sheen.in_hand.is_empty() {
            sheen.clear();
        }
        return;
    };
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    // Every permanent on every seat's board, not only this one's: an
    // opponent's creature resolving is exactly as new to the player as their
    // own, and is the arrival they most need to see.
    let table = board.pods.iter().flat_map(|pod| {
        pod.lanes
            .iter()
            .flat_map(|lane| lane.groups.iter().map(|group| group.representative))
    });
    sheen.observe(
        time.elapsed_secs_wrapped(),
        board.hand.iter().map(|card| card.id),
        table,
        duel.hovered,
        still,
    );
    // And which door the ones that came from somewhere in particular came
    // through. Read after `observe`, because it stamps the sweeps `observe`
    // has just started; `sync_scene` is the one that empties the list, and it
    // runs after this.
    if let Some(view) = duel.view.as_ref() {
        watch.observe(view);
    }
    let doors: Vec<Move> = watch.undrawn().to_vec();
    sheen.usher(&doors);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u32) -> ObjectId {
        ObjectId::new(n, 0)
    }

    /// The fault the whole module exists for: a card that has been in the
    /// hand for ten frames must not sweep on the eleventh, however often the
    /// HUD is rebuilt around it.
    #[test]
    fn a_card_sweeps_when_it_arrives_and_never_again() {
        let mut sheen = Sheen::default();
        sheen.observe(1.0, [id(1)].into_iter(), std::iter::empty(), None, false);
        let first = sheen.of(id(1), Surface::Hand).expect("a drawn card sweeps");
        for step in 1..6 {
            let now = 0.05f32.mul_add(step as f32, 1.0);
            sheen.observe(now, [id(1)].into_iter(), std::iter::empty(), None, false);
            assert_eq!(
                sheen.of(id(1), Surface::Hand),
                Some(first),
                "the sweep restarted on a rebuild at {now}"
            );
        }
        // And it is gone once it has run, rather than lingering in the map.
        sheen.observe(9.0, [id(1)].into_iter(), std::iter::empty(), None, false);
        assert_eq!(sheen.of(id(1), Surface::Hand), None);
    }

    /// Playing a card is one arrival, not two: it leaves the hand and reaches
    /// the table, and the table is where the player is looking.
    #[test]
    fn a_card_played_sweeps_on_the_table_and_not_in_the_hand() {
        let mut sheen = Sheen::default();
        sheen.observe(1.0, [id(7)].into_iter(), std::iter::empty(), None, false);
        // Past the first sweep, so what follows cannot be mistaken for it.
        sheen.observe(4.0, [id(7)].into_iter(), std::iter::empty(), None, false);
        assert_eq!(sheen.of(id(7), Surface::Hand), None);
        sheen.observe(4.1, std::iter::empty(), [id(7)].into_iter(), None, false);
        assert!(sheen.of(id(7), Surface::Table).is_some());
        assert_eq!(sheen.of(id(7), Surface::Hand), None);
    }

    /// A preview is opened, and opening it again is a second event — the
    /// permanent under the pointer may have been on the table all game.
    #[test]
    fn opening_a_preview_sweeps_even_for_a_card_nobody_would_call_new() {
        let mut sheen = Sheen::default();
        sheen.observe(1.0, std::iter::empty(), [id(3)].into_iter(), None, false);
        sheen.observe(5.0, std::iter::empty(), [id(3)].into_iter(), None, false);
        assert_eq!(sheen.of(id(3), Surface::Table), None, "the arrival is over");

        sheen.observe(
            5.1,
            std::iter::empty(),
            [id(3)].into_iter(),
            Some(id(3)),
            false,
        );
        let opened = sheen.of(id(3), Surface::Preview).expect("a preview sweeps");
        // Held open: still the same sweep, not a new one every frame.
        sheen.observe(
            5.2,
            std::iter::empty(),
            [id(3)].into_iter(),
            Some(id(3)),
            false,
        );
        assert_eq!(sheen.of(id(3), Surface::Preview), Some(opened));
    }

    /// "A little bit quicker for each new card appearing" — and then back to
    /// the top once the run is over.
    #[test]
    fn a_burst_quickens_and_a_gap_starts_again() {
        let mut sheen = Sheen::default();
        let hand: Vec<ObjectId> = (1..=7).map(id).collect();
        sheen.observe(1.0, hand.iter().copied(), std::iter::empty(), None, false);
        let mut rates: Vec<f32> = hand
            .iter()
            .map(|c| sheen.of(*c, Surface::Hand).expect("dealt").rate())
            .collect();
        rates.sort_by(f32::total_cmp);
        assert_eq!(rates.len(), 7);
        assert!(
            rates.first() < rates.last(),
            "seven cards dealt at one speed: {rates:?}"
        );

        // A card played much later is not part of that run.
        sheen.observe(
            20.0,
            hand.iter().copied(),
            [id(50)].into_iter(),
            None,
            false,
        );
        let alone = sheen.of(id(50), Surface::Table).expect("played");
        assert!(
            (alone.rate() - 1.0 / BASE).abs() < 1e-3,
            "a card on its own swept at {} rather than the base rate",
            alone.rate()
        );
    }

    /// `reduce_motion` is a promise that nothing moves, and a one-shot sweep
    /// is still a thing that moves.
    #[test]
    fn a_still_client_never_sweeps() {
        let mut sheen = Sheen::default();
        sheen.observe(1.0, [id(1)].into_iter(), std::iter::empty(), None, true);
        assert_eq!(sheen.of(id(1), Surface::Hand), None);
        // And what was already running stops rather than finishing.
        let mut moving = Sheen::default();
        moving.observe(1.0, [id(1)].into_iter(), std::iter::empty(), None, false);
        assert!(moving.of(id(1), Surface::Hand).is_some());
        moving.observe(1.1, [id(1)].into_iter(), std::iter::empty(), None, true);
        assert_eq!(moving.of(id(1), Surface::Hand), None);
    }

    fn arriving_from(place: baylee_client_core::zones::Place) -> Move {
        Move {
            object: id(1),
            from: Some(place),
            to: Some(baylee_client_core::zones::Place::Battlefield),
        }
    }

    /// The two doors back on to the battlefield: a card the player last saw
    /// in a pile is not merely arriving, and the sweep is where the client
    /// says so.
    #[test]
    fn a_card_coming_back_from_a_pile_sweeps_through_that_pile_s_door() {
        use baylee_client_core::zones::{Passage, Place};
        let seat = baylee_core::ids::PlayerId::new(0);
        for (from, want) in [
            (Place::Exile(seat), Passage::Flickered),
            (Place::Graveyard(seat), Passage::Returned),
        ] {
            let mut sheen = Sheen::default();
            sheen.observe(1.0, std::iter::empty(), [id(1)].into_iter(), None, false);
            sheen.usher(&[arriving_from(from)]);
            let sweep = sheen
                .of(id(1), Surface::Table)
                .unwrap_or_else(|| panic!("a card arriving from {from:?} did not sweep"));
            assert_eq!(sweep.door(), Some(want), "the wrong door from {from:?}");
        }
    }

    /// The counter-test, and the one that keeps the mark meaning something:
    /// the commonest arrival in the game — a spell resolving off the stack —
    /// gets the plain band and no door at all.
    #[test]
    fn a_spell_resolving_on_to_the_table_comes_through_no_door() {
        use baylee_client_core::zones::Place;
        let mut sheen = Sheen::default();
        sheen.observe(1.0, std::iter::empty(), [id(1)].into_iter(), None, false);
        sheen.usher(&[arriving_from(Place::Stack)]);
        let sweep = sheen.of(id(1), Surface::Table).expect("it still sweeps");
        assert_eq!(sweep.door(), None, "an ordinary arrival was given a door");
    }

    /// A departure has no sweep here to stamp — the card is out of the board
    /// model by the time the move is read — so `usher` must leave it alone
    /// rather than dressing whatever else happens to answer to that id.
    #[test]
    fn a_card_leaving_the_table_is_not_ushered_in() {
        use baylee_client_core::zones::Place;
        let mut sheen = Sheen::default();
        // It arrived a moment ago and is still sweeping when it dies.
        sheen.observe(1.0, std::iter::empty(), [id(1)].into_iter(), None, false);
        let arrived = sheen
            .of(id(1), Surface::Table)
            .expect("it swept on arrival");
        sheen.usher(&[Move {
            object: id(1),
            from: Some(Place::Battlefield),
            to: Some(Place::Graveyard(baylee_core::ids::PlayerId::new(0))),
        }]);
        assert_eq!(
            sheen.of(id(1), Surface::Table),
            Some(arrived),
            "the arrival sweep was overwritten by the exit"
        );
    }

    /// Stamping twice is relied on: a frame that draws nothing leaves the
    /// moves undrawn and the next one reads them again.
    #[test]
    fn ushering_the_same_move_twice_changes_nothing() {
        use baylee_client_core::zones::Place;
        let mut sheen = Sheen::default();
        sheen.observe(1.0, std::iter::empty(), [id(1)].into_iter(), None, false);
        sheen.usher(&[arriving_from(Place::Graveyard(
            baylee_core::ids::PlayerId::new(0),
        ))]);
        let once = sheen.of(id(1), Surface::Table);
        sheen.usher(&[arriving_from(Place::Graveyard(
            baylee_core::ids::PlayerId::new(0),
        ))]);
        assert_eq!(sheen.of(id(1), Surface::Table), once);
    }
}
