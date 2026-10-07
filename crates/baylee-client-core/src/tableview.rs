//! How this device frames the table, and where the camera goes on a visit
//! (DESIGN-v7 §1–§2; the owner's decisions D20 and D21 of 07.10.2026, made
//! settings: "implement all options and make them configurable").
//!
//! # Why the device and not the account
//!
//! Kept in the client's per-device settings (`ClientSettings::table`) beside
//! the graphics knobs, not in the account's `Preferences`: a shot is a fact
//! about the screen it is taken for. The lean that reads well on a 1708-px
//! laptop is not the one a phone held sideways wants (the Phone frame uses
//! its own), and a player who chose the gentler angle on a small monitor
//! should not have it follow them to a large one. `reduce_motion` is the
//! account's because it is about the player; these are about the window.
//!
//! # The visit
//!
//! A press on another seat's button (or `F` / `Shift+F`) moves the camera,
//! never a card: the ring stays as it is and the camera orbits to stand
//! behind that seat, so its board is upright above the hand. This module
//! holds the parts of that a test can ask without a window — which seat `F`
//! goes to next, what the frame is, and when the camera comes home by itself.

pub use crate::layout::Arrangement;
use baylee_core::ids::PlayerId;
use serde::{Deserialize, Serialize};

/// How steeply the camera looks at a table of three seats or more (D20).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RingLean {
    /// A wide duel's angle, 0.62: every seat's board grows by about two
    /// fifths, the far seats shrink against mine (the visit equalises them).
    /// The owner asked for more angle three times; this is the default.
    #[default]
    Steep,
    /// The angle the rings had before v7, 0.36, with only the tighter air
    /// and the ledge gains: every board drawn within a tenth of the same
    /// width.
    Gentle,
}

impl RingLean {
    /// The lean as the camera rig stores it: a tangent off vertical.
    #[must_use]
    pub const fn tangent(self) -> f32 {
        match self {
            Self::Steep => 0.62,
            Self::Gentle => 0.36,
        }
    }

    /// Both choices, in the order a settings row shows them.
    pub const ALL: [Self; 2] = [Self::Steep, Self::Gentle];
}

/// Where the camera stands when it visits a seat (D21, and the owner's of
/// 07.10.2026: *"The current view fits for teammates; if it is not a
/// teammate it should be rotated 180 degrees."*).
///
/// Stored by name. **`"behind"` is read as [`VisitCamera::Auto`]**: it was
/// v7's default, a table file writes every field, so a file holding it
/// cannot say whether a player chose it — and v7 had been on `main` for
/// hours when the owner moved the default. A player who chooses *Behind*
/// from now on is written `"behind_seat"` and read back as such; the other
/// two v7 names keep their meaning.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VisitCamera {
    /// A teammate's board seen from behind it (as if sitting beside them),
    /// an opponent's from across (a duel opponent's board): the default.
    /// With no teams, every other seat is an opponent. The relation comes
    /// from the roster's teams, never from anything hidden.
    #[default]
    Auto,
    /// Behind the visited seat, its board upright above my hand; my own near
    /// lane stays in frame while it costs the visited board little (three
    /// and four seats), the dial instead from five seats up.
    Behind,
    /// Behind the visited seat with the dial always in frame: the visited
    /// board larger, my own never in view.
    BehindDial,
    /// Across from the visited seat: its board faces it like a duel
    /// opponent's, and the seat across from it lies above my hand.
    Across,
}

impl VisitCamera {
    /// Every choice, in the order a settings row shows them.
    pub const ALL: [Self; 4] = [Self::Auto, Self::Behind, Self::BehindDial, Self::Across];

    /// The name a settings file holds.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Behind => "behind_seat",
            Self::BehindDial => "behind_dial",
            Self::Across => "across",
        }
    }

    /// What the camera does for a visit to a seat that is (`teammate`) or
    /// is not on my team: *Automatic* chooses behind or across, every other
    /// choice is itself.
    #[must_use]
    pub const fn resolve(self, teammate: bool) -> Self {
        match self {
            Self::Auto if teammate => Self::Behind,
            Self::Auto => Self::Across,
            other => other,
        }
    }
}

impl Serialize for VisitCamera {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.tag())
    }
}

impl<'de> Deserialize<'de> for VisitCamera {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Ok(match name.as_str() {
            // v7's default, written whether chosen or not: the new default.
            "behind" => Self::Auto,
            other => Self::ALL
                .into_iter()
                .find(|v| v.tag() == other)
                .unwrap_or_default(),
        })
    }
}

/// This device's table framing (`ClientSettings::table`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TableView {
    /// How the seats are placed, unless [`Self::arrangement_by_seats`] names
    /// one for the table's seat count (DESIGN-v8 §2.6).
    #[serde(deserialize_with = "crate::graphics::lenient")]
    pub arrangement: Arrangement,
    /// The arrangement remembered for a seat count, three to eight: the
    /// menu's *remember for this seat count*. Sparse; a count with none takes
    /// [`Self::arrangement`].
    pub arrangement_by_seats: BySeats,
    /// *Tisch folgt dem Zug* (DESIGN-v8 §1.1, D25): the active player's side
    /// becomes the seat of interest at the start of its turn. Off.
    #[serde(deserialize_with = "crate::graphics::lenient")]
    pub follow: bool,
    /// The ring's lean.
    #[serde(deserialize_with = "crate::graphics::lenient")]
    pub lean: RingLean,
    /// Where a visit stands.
    #[serde(deserialize_with = "crate::graphics::lenient")]
    pub visit: VisitCamera,
}

impl TableView {
    /// The arrangement chosen for a table of `seats`: the one remembered for
    /// that count, else the default (DESIGN-v8 §2.6's reading order).
    #[must_use]
    pub fn chosen(&self, seats: usize) -> Arrangement {
        self.arrangement_by_seats
            .get(seats)
            .unwrap_or(self.arrangement)
    }
}

/// The arrangement remembered per seat count, three to eight (DESIGN-v8
/// §2.6, D24): a fixed array so the table settings stay `Copy`, written as a
/// sparse map (`{"4":"turntable"}`) so a file reads as the design says.
///
/// A count outside three to eight, or a value that is not an arrangement
/// name at all, is dropped on reading rather than refusing the file; an
/// arrangement name this build does not know reads as the ring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BySeats([Option<Arrangement>; 6]);

impl BySeats {
    /// The fewest seats a count is remembered for.
    pub const FIRST: usize = 3;
    /// The most.
    pub const LAST: usize = 8;

    /// The arrangement remembered for `seats`, if any.
    #[must_use]
    pub fn get(&self, seats: usize) -> Option<Arrangement> {
        seats
            .checked_sub(Self::FIRST)
            .and_then(|i| self.0.get(i).copied().flatten())
    }

    /// Remembers `arrangement` for `seats` (`None` forgets it). A count
    /// outside three to eight is not remembered.
    pub fn set(&mut self, seats: usize, arrangement: Option<Arrangement>) {
        if let Some(cell) = seats
            .checked_sub(Self::FIRST)
            .and_then(|i| self.0.get_mut(i))
        {
            *cell = arrangement;
        }
    }

    /// Every remembered count and its arrangement, fewest seats first.
    pub fn iter(&self) -> impl Iterator<Item = (usize, Arrangement)> + '_ {
        self.0
            .iter()
            .enumerate()
            .filter_map(|(i, a)| a.map(|a| (i + Self::FIRST, a)))
    }
}

impl Serialize for BySeats {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.iter().count()))?;
        for (seats, arrangement) in self.iter() {
            map.serialize_entry(&seats.to_string(), &arrangement)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for BySeats {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Read whole first, so a malformed value is dropped without leaving
        // the reader halfway through it.
        let raw = serde_json::Value::deserialize(deserializer)?;
        let mut out = Self::default();
        let serde_json::Value::Object(raw) = raw else {
            return Ok(out);
        };
        for (key, value) in raw {
            let (Ok(seats), Ok(arrangement)) = (
                key.parse::<usize>(),
                serde_json::from_value::<Arrangement>(value),
            ) else {
                continue;
            };
            out.set(seats, Some(arrangement));
        }
        Ok(out)
    }
}

/// The window as the arrangements read it: the shell's five size classes
/// (ux-b6 DESIGN-v5 §2.7) by the **raw** window, as [`WindowClass`] reads
/// them for the camera. Which arrangements are offered depends on it
/// ([`Arrangement::offered`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableFrame {
    /// Raw logical height under 500: a phone held sideways.
    Phone,
    /// Under 760 wide.
    Compact,
    /// 760 to 1180 wide.
    Narrow,
    /// 1180 to 2560 wide.
    Wide,
    /// 2560 and wider.
    Vast,
}

impl TableFrame {
    /// Every class, smallest first.
    pub const ALL: [Self; 5] = [
        Self::Phone,
        Self::Compact,
        Self::Narrow,
        Self::Wide,
        Self::Vast,
    ];

    /// The class of a window `width` × `height` logical pixels.
    #[must_use]
    pub fn of(width: f32, height: f32) -> Self {
        if height < 500.0 {
            Self::Phone
        } else if width < 760.0 {
            Self::Compact
        } else if width < 1180.0 {
            Self::Narrow
        } else if width < 2560.0 {
            Self::Wide
        } else {
            Self::Vast
        }
    }

    /// The name `/state` prints.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Phone => "phone",
            Self::Compact => "compact",
            Self::Narrow => "narrow",
            Self::Wide => "wide",
            Self::Vast => "vast",
        }
    }
}

/// How much further off than the dial shot a lane shot may stand: a board
/// drawn at four fifths of the size is the price §2.2 accepts for my lane in
/// view.
pub const LANE_COST: f32 = 1.25;

/// The window as the table's camera reads it: the shell's size classes
/// (ux-b6 DESIGN-v5 §2.7) by the **raw** window, never by the shell's text
/// step — a text step must not move a card (`shellkit::lint`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowClass {
    /// Raw logical height under 500: a phone held sideways.
    Phone,
    /// Under 1180 wide (the shell's Compact and Narrow).
    Narrow,
    /// 1180 and wider (Wide and Vast).
    Wide,
}

impl WindowClass {
    /// The class of a window `width` × `height` logical pixels.
    #[must_use]
    pub fn of(width: f32, height: f32) -> Self {
        if height < 500.0 {
            Self::Phone
        } else if width < 1180.0 {
            Self::Narrow
        } else {
            Self::Wide
        }
    }
}

/// What a visit's shot takes in besides the visited board, for
/// `/state.camera.frame` and for the camera's fit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisitFrame {
    /// The visited pod and my own lane nearest the centre (3–4 seats).
    Lane,
    /// The visited pod and the dial (5+ seats, or always by choice).
    Dial,
    /// The visited pod alone (a phone).
    Pod,
    /// Seen from across: the visited pod at the far side, the dial between.
    Across,
}

impl VisitFrame {
    /// Which frame a visit takes, by the device's choice, how far off the eye
    /// would stand for each of the two frames, and whether the window is a
    /// phone's.
    ///
    /// The rule behind `Lane` (DESIGN-v7 §2.2): my near lane stays in the
    /// frame **while it costs the visited board less than a fifth** of its
    /// size. A board is drawn in inverse proportion to the eye's distance, so
    /// that is a lane shot standing no more than [`LANE_COST`] times as far
    /// off as the dial shot. Asked of the two fits and not of the seat count:
    /// a three-seat ring is round and wide, and there my lane would cost two
    /// fifths, where at four seats it costs a tenth.
    ///
    /// `choice` is the resolved one ([`VisitCamera::resolve`]); an
    /// unresolved *Automatic* is read as across, its answer for an opponent.
    #[must_use]
    pub fn of(choice: VisitCamera, lane_eye: f32, dial_eye: f32, phone: bool) -> Self {
        if phone {
            return Self::Pod;
        }
        match choice {
            VisitCamera::Across | VisitCamera::Auto => Self::Across,
            VisitCamera::Behind if lane_eye <= dial_eye * LANE_COST => Self::Lane,
            VisitCamera::Behind | VisitCamera::BehindDial => Self::Dial,
        }
    }

    /// The name `/state.camera.frame` prints.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Lane => "lane",
            Self::Dial => "dial",
            Self::Pod => "pod",
            Self::Across => "across",
        }
    }
}

/// The seat `F` (`forward`) or `Shift+F` visits next.
///
/// `ring` is the table's seats in ring order, mine first (the layout's slot
/// order: clockwise in turn order from the near edge). From home, `F` goes
/// to the first seat after mine and `Shift+F` to the last; past either end
/// is home (`None`), so the key never dead-ends. A seat no longer at the
/// table counts as home.
#[must_use]
pub fn step_seat(
    ring: &[PlayerId],
    me: PlayerId,
    visiting: Option<PlayerId>,
    forward: bool,
) -> Option<PlayerId> {
    let others: Vec<PlayerId> = ring.iter().copied().filter(|p| *p != me).collect();
    if others.is_empty() {
        return None;
    }
    let at = visiting.and_then(|v| others.iter().position(|p| *p == v));
    match (at, forward) {
        (None, true) => others.first().copied(),
        (None, false) => others.last().copied(),
        (Some(i), true) => others.get(i + 1).copied(),
        (Some(i), false) => i.checked_sub(1).and_then(|j| others.get(j).copied()),
    }
}

/// What a press on a seat's button does to the camera.
///
/// My own button, or the button of the seat being visited, brings it home;
/// any other visits that seat.
#[must_use]
pub fn press(
    me: Option<PlayerId>,
    visiting: Option<PlayerId>,
    pressed: PlayerId,
) -> Option<PlayerId> {
    if me == Some(pressed) || visiting == Some(pressed) {
        None
    } else {
        Some(pressed)
    }
}

/// Whether the camera comes home by itself on this edge (DESIGN-v7 §2.4).
///
/// Two edges and no others: **my turn begins** (the active seat becomes me),
/// and **a combat question about my creatures opens for me** (attackers or
/// blockers addressed to me, as it opens). Never my priority, a target or a
/// yes–no — the owner visits to target that seat — and never another seat's
/// turn: there is no auto-follow.
#[must_use]
pub fn comes_home(edge: &HomeEdge) -> bool {
    let my_turn_began = edge.active_now == Some(edge.me) && edge.active_before != Some(edge.me);
    my_turn_began || edge.combat_question_opened
}

/// What *Tisch folgt dem Zug* does on a turn's edge (DESIGN-v8 §1.1, D25).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Follow {
    /// Nothing: the switch is off, the turn did not change hands, or it is
    /// my own turn (my board is home).
    Stay,
    /// Show the active player's seat now.
    Show(PlayerId),
    /// Show it once the question open for me is answered, or the pointer
    /// has left the board it rests on: never move the table under a
    /// decision or a reading hand.
    Defer(PlayerId),
}

/// The follow switch's rule, at the start of a turn: off, nothing; my own
/// turn, nothing (my turn beginning brings the table home by
/// [`comes_home`]); another player's, show that seat — deferred while a
/// question is open for me or the pointer rests on the board shown now.
/// The manual seat of interest holds until the next turn starts, because
/// this only answers a turn changing hands.
#[must_use]
pub fn follow(edge: &FollowEdge) -> Follow {
    if !edge.on {
        return Follow::Stay;
    }
    let Some(active) = edge.active_now else {
        return Follow::Stay;
    };
    if edge.active_before == Some(active) || active == edge.me {
        return Follow::Stay;
    }
    if edge.question_for_me || edge.pointer_on_interest {
        Follow::Defer(active)
    } else {
        Follow::Show(active)
    }
}

/// The facts [`follow`] reads.
#[derive(Clone, Copy, Debug)]
pub struct FollowEdge {
    /// The switch.
    pub on: bool,
    /// This client's seat.
    pub me: PlayerId,
    /// Whose turn it was.
    pub active_before: Option<PlayerId>,
    /// Whose turn it is.
    pub active_now: Option<PlayerId>,
    /// Whether a question is open for me.
    pub question_for_me: bool,
    /// Whether the pointer rests on the board of the seat shown now.
    pub pointer_on_interest: bool,
}

/// The two edges [`comes_home`] reads.
#[derive(Clone, Copy, Debug)]
pub struct HomeEdge {
    /// This client's seat.
    pub me: PlayerId,
    /// Whose turn it was in the previous view.
    pub active_before: Option<PlayerId>,
    /// Whose turn it is now.
    pub active_now: Option<PlayerId>,
    /// Whether a `ChooseAttackers`/`ChooseBlockers` addressed to me just
    /// arrived, where the previous question was not one.
    pub combat_question_opened: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(n: u8) -> PlayerId {
        PlayerId::new(n)
    }

    /// `F` walks the ring clockwise from me and comes home past the last;
    /// `Shift+F` walks it the other way.
    #[test]
    fn f_walks_the_ring_in_order_and_comes_home() {
        let ring = [p(2), p(3), p(0), p(1)];
        let me = p(2);
        let mut at = None;
        let mut walked = Vec::new();
        for _ in 0..4 {
            at = step_seat(&ring, me, at, true);
            walked.push(at);
        }
        assert_eq!(walked, vec![Some(p(3)), Some(p(0)), Some(p(1)), None]);
        assert_eq!(step_seat(&ring, me, None, false), Some(p(1)));
        assert_eq!(step_seat(&ring, me, Some(p(1)), false), Some(p(0)));
        assert_eq!(step_seat(&ring, me, Some(p(3)), false), None);
        assert_eq!(step_seat(&[p(0)], p(0), None, true), None, "alone: nowhere");
    }

    /// My own button and the visited seat's button are home; another visits.
    #[test]
    fn a_button_visits_or_comes_home() {
        assert_eq!(press(Some(p(0)), None, p(2)), Some(p(2)));
        assert_eq!(press(Some(p(0)), Some(p(2)), p(2)), None);
        assert_eq!(press(Some(p(0)), Some(p(2)), p(0)), None);
        assert_eq!(press(Some(p(0)), Some(p(2)), p(3)), Some(p(3)));
    }

    /// Home on my turn's start and on my combat question; not on another
    /// seat's turn, and not when my turn simply goes on.
    #[test]
    fn the_camera_comes_home_on_two_edges_only() {
        let edge = |before: u8, now: u8, combat: bool| HomeEdge {
            me: p(0),
            active_before: Some(p(before)),
            active_now: Some(p(now)),
            combat_question_opened: combat,
        };
        assert!(comes_home(&edge(3, 0, false)), "my turn began");
        assert!(!comes_home(&edge(0, 0, false)), "my turn goes on");
        assert!(!comes_home(&edge(0, 1, false)), "no auto-follow");
        assert!(comes_home(&edge(1, 1, true)), "blockers for me");
    }

    /// The frame rule and its three settings.
    #[test]
    fn the_visit_frame_follows_the_choice_and_what_the_lane_costs() {
        assert_eq!(
            VisitFrame::of(VisitCamera::Behind, 12.0, 10.0, false),
            VisitFrame::Lane
        );
        assert_eq!(
            VisitFrame::of(VisitCamera::Behind, 12.5, 10.0, false),
            VisitFrame::Lane
        );
        assert_eq!(
            VisitFrame::of(VisitCamera::Behind, 12.6, 10.0, false),
            VisitFrame::Dial
        );
        assert_eq!(
            VisitFrame::of(VisitCamera::BehindDial, 10.0, 10.0, false),
            VisitFrame::Dial
        );
        assert_eq!(
            VisitFrame::of(VisitCamera::Across, 10.0, 10.0, false),
            VisitFrame::Across
        );
        assert_eq!(
            VisitFrame::of(VisitCamera::Across, 10.0, 10.0, true),
            VisitFrame::Pod
        );
    }

    /// The follow rule: nothing when off or on my own turn or within one
    /// turn; another player's turn shows that seat, or defers it under an
    /// open question or a reading pointer.
    #[test]
    fn the_table_follows_the_turn_only_when_asked_and_never_under_a_decision() {
        let edge = |on: bool, before: u8, now: u8, question: bool, pointer: bool| FollowEdge {
            on,
            me: p(0),
            active_before: Some(p(before)),
            active_now: Some(p(now)),
            question_for_me: question,
            pointer_on_interest: pointer,
        };
        assert_eq!(
            follow(&edge(false, 0, 1, false, false)),
            Follow::Stay,
            "off"
        );
        assert_eq!(follow(&edge(true, 0, 1, false, false)), Follow::Show(p(1)));
        assert_eq!(
            follow(&edge(true, 1, 1, false, false)),
            Follow::Stay,
            "same turn"
        );
        assert_eq!(
            follow(&edge(true, 3, 0, false, false)),
            Follow::Stay,
            "my turn"
        );
        assert_eq!(follow(&edge(true, 1, 2, true, false)), Follow::Defer(p(2)));
        assert_eq!(follow(&edge(true, 1, 2, false, true)), Follow::Defer(p(2)));
    }

    /// The defaults are the recommended options, and a file naming a choice
    /// this build does not know keeps the rest of the table settings.
    #[test]
    fn the_defaults_are_the_recommendations_and_unknown_choices_fall_back() {
        let view = TableView::default();
        assert_eq!(view.lean, RingLean::Steep);
        assert_eq!(view.visit, VisitCamera::Auto);
        assert!((RingLean::Steep.tangent() - 0.62).abs() < f32::EPSILON);
        let read: TableView =
            serde_json::from_str(r#"{"lean":"sideways","visit":"across"}"#).expect("reads");
        assert_eq!(read.lean, RingLean::Steep);
        assert_eq!(read.visit, VisitCamera::Across);
        let read: TableView = serde_json::from_str("{}").expect("reads");
        assert_eq!(read, TableView::default());
    }

    /// The arrangement settings: a default, a sparse per-count memory read
    /// first, the follow switch off; an unknown name reads as the ring and
    /// a bad entry is dropped, never the file.
    #[test]
    fn the_arrangement_settings_read_leniently_and_by_seat_count() {
        let view = TableView::default();
        assert_eq!(view.arrangement, Arrangement::Ring);
        assert!(!view.follow, "follow is off by default (D25)");
        let read: TableView = serde_json::from_str(
            r#"{"arrangement":"zukunft","arrangement_by_seats":{"4":"zukunft","5":"ring","9":"ring","x":"ring","6":7},"follow":true,"lean":"gentle"}"#,
        )
        .expect("reads");
        assert_eq!(read.arrangement, Arrangement::Ring);
        assert_eq!(read.arrangement_by_seats.get(4), Some(Arrangement::Ring));
        assert_eq!(read.arrangement_by_seats.get(5), Some(Arrangement::Ring));
        assert_eq!(read.arrangement_by_seats.get(6), None);
        assert_eq!(read.arrangement_by_seats.iter().count(), 2);
        assert!(read.follow);
        assert_eq!(read.lean, RingLean::Gentle, "the rest of the file kept");
        let mut view = TableView::default();
        view.arrangement_by_seats
            .set(4, Some(Arrangement::Spotlight));
        view.arrangement_by_seats
            .set(9, Some(Arrangement::Spotlight));
        assert_eq!(view.chosen(4), Arrangement::Spotlight);
        assert_eq!(view.chosen(5), Arrangement::Ring);
        let json = serde_json::to_string(&view).expect("writes");
        assert!(
            json.contains(r#""arrangement_by_seats":{"4":"spotlight"}"#),
            "{json}"
        );
        let back: TableView = serde_json::from_str(&json).expect("reads");
        assert_eq!(back, view);
    }

    /// *Automatic* is the default; v7's default name reads as it, every
    /// other name keeps its meaning, an explicit *Behind* round-trips under
    /// its own name, and *Automatic* answers behind for a teammate and
    /// across for an opponent.
    #[test]
    fn automatic_is_the_visit_s_default_and_v7_s_default_reads_as_it() {
        let read = |json: &str| -> TableView { serde_json::from_str(json).expect("reads") };
        assert_eq!(read(r#"{"visit":"behind"}"#).visit, VisitCamera::Auto);
        assert_eq!(
            read(r#"{"visit":"behind_dial"}"#).visit,
            VisitCamera::BehindDial
        );
        assert_eq!(read(r#"{"visit":"across"}"#).visit, VisitCamera::Across);
        assert_eq!(read(r#"{"visit":"sideways"}"#).visit, VisitCamera::Auto);
        for choice in VisitCamera::ALL {
            let view = TableView {
                visit: choice,
                ..TableView::default()
            };
            let json = serde_json::to_string(&view).expect("writes");
            assert_eq!(read(&json).visit, choice, "{json}");
        }
        assert_eq!(VisitCamera::Auto.resolve(true), VisitCamera::Behind);
        assert_eq!(VisitCamera::Auto.resolve(false), VisitCamera::Across);
        assert_eq!(
            VisitCamera::BehindDial.resolve(false),
            VisitCamera::BehindDial
        );
    }

    /// The classes change where the shell's do.
    #[test]
    fn the_table_frame_changes_where_the_shell_s_classes_do() {
        assert_eq!(TableFrame::of(1708.0, 499.0), TableFrame::Phone);
        assert_eq!(TableFrame::of(759.0, 600.0), TableFrame::Compact);
        assert_eq!(TableFrame::of(760.0, 600.0), TableFrame::Narrow);
        assert_eq!(TableFrame::of(1179.0, 800.0), TableFrame::Narrow);
        assert_eq!(TableFrame::of(1180.0, 800.0), TableFrame::Wide);
        assert_eq!(TableFrame::of(2560.0, 1440.0), TableFrame::Vast);
    }
}
