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

/// Where the camera stands when it visits a seat (D21).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisitCamera {
    /// Behind the visited seat, its board upright above my hand; my own near
    /// lane stays in frame while it costs the visited board little (three
    /// and four seats), the dial instead from five seats up. Recommended.
    #[default]
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
    pub const ALL: [Self; 3] = [Self::Behind, Self::BehindDial, Self::Across];
}

/// This device's table framing (`ClientSettings::table`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TableView {
    /// How the seats are placed (one arrangement for now: the ring).
    #[serde(deserialize_with = "crate::graphics::lenient")]
    pub arrangement: Arrangement,
    /// The ring's lean.
    #[serde(deserialize_with = "crate::graphics::lenient")]
    pub lean: RingLean,
    /// Where a visit stands.
    #[serde(deserialize_with = "crate::graphics::lenient")]
    pub visit: VisitCamera,
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
    #[must_use]
    pub fn of(choice: VisitCamera, lane_eye: f32, dial_eye: f32, phone: bool) -> Self {
        if phone {
            return Self::Pod;
        }
        match choice {
            VisitCamera::Across => Self::Across,
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

    /// The defaults are the recommended options, and a file naming a choice
    /// this build does not know keeps the rest of the table settings.
    #[test]
    fn the_defaults_are_the_recommendations_and_unknown_choices_fall_back() {
        let view = TableView::default();
        assert_eq!(view.lean, RingLean::Steep);
        assert_eq!(view.visit, VisitCamera::Behind);
        assert!((RingLean::Steep.tangent() - 0.62).abs() < f32::EPSILON);
        let read: TableView =
            serde_json::from_str(r#"{"lean":"sideways","visit":"across"}"#).expect("reads");
        assert_eq!(read.lean, RingLean::Steep);
        assert_eq!(read.visit, VisitCamera::Across);
        let read: TableView = serde_json::from_str("{}").expect("reads");
        assert_eq!(read, TableView::default());
    }
}
