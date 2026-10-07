//! Every arrangement of the table (DESIGN-v8, the owner's of 07.10.2026):
//! *"maybe implement all options, and build something at the top screen
//! edge with which one can switch it? Then everyone can set it up for
//! themselves and also try out what works best when."*
//!
//! An arrangement is an arm of [`TableLayout::arranged`](super::TableLayout::arranged)
//! plus a camera policy. It reads the roster and nothing else — never a
//! view, never an object — so hidden information is as unrepresentable here
//! as everywhere else in this crate (`arranged_reads_only_the_roster`).
//!
//! Two kinds, and the kind is the one thing the client asks of an arm
//! besides its geometry ([`Arrangement::moves_cards`]): a **camera** arm
//! answers the seat of interest by moving the camera (the ring visits), a
//! **layout** arm by re-solving the slots (cards glide, the camera stays
//! home).

use crate::i18n::Phrase;

mod pair;
mod upright;

/// How a board square to my chair is turned (the owner, 07.10.2026: *a
/// non-teammate's board is rendered rotated 180° — exactly as the
/// opponent's board is drawn in the 1v1 duel*): mine and my teammates' as
/// mine (`0`), every other seat's as a duel's opponent half (`π`, its
/// creature row toward the middle and me, its cards facing their owner).
/// The relation is the roster's teams; with none, everyone else is an
/// opponent. Read by every arrangement that squares a board to my chair
/// (the upright ring, the Turntable's side mats, the pods); the ring and
/// the arc turn each board to face the middle, the duel's own rule, and
/// their across seat is the duel's opponent exactly.
#[must_use]
pub(crate) fn facing_for(seats: &[super::Seat], index: usize) -> f32 {
    let mine = seats.first().and_then(|s| s.team);
    let ally = index == 0 || (mine.is_some() && seats.get(index).and_then(|s| s.team) == mine);
    if ally { 0.0 } else { core::f32::consts::PI }
}
pub(super) use pair::spotlight;
pub(super) use upright::upright;

/// Whether two upright places stand a pile strip apart (the rule the
/// upright ring is grown by), for the layout's tests.
#[cfg(test)]
pub(super) fn upright_apart(a: &super::SeatSlot, b: &super::SeatSlot) -> bool {
    upright::apart(a, b, super::PILE_STRIP)
}
use crate::tableview::TableFrame;

/// How the seats are placed at the table, and with that which home and visit
/// shots the camera takes.
///
/// One decision point: [`TableLayout::arranged`](super::TableLayout::arranged)
/// places the seats for an arrangement, and the client's camera poses
/// (`CameraRig::home_shot` and `CameraRig::visit` in `baylee-client`) match
/// on it. The device's choice lives in `ClientSettings::table` beside the
/// ring's lean and the visit camera. The dial, the sound and picking read
/// only the layout and the rig, never this.
///
/// An arrangement this build does not know — a newer client's, in a shared
/// settings file — reads as [`Arrangement::Ring`], and never refuses the
/// file it is in (`Deserialize` is written by hand for that: serde's
/// `other` wants the catch-all last, and the ring is the first row).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Arrangement {
    /// Every seat a duel's width on one ring, clockwise in turn order, mine
    /// at the near edge ([`TableLayout::seated`](super::TableLayout::seated)).
    /// A *camera* arrangement: the seat of interest is visited by the
    /// camera, no card moves.
    #[default]
    Ring,
    /// The ring's places, every board upright to me (MULTIPLAYER §D); a
    /// visit zooms onto the pod.
    UprightRing,
    /// Me and the seat of interest as a duel, the others as small side mats
    /// on the flanks (MULTIPLAYER §A); a visit turns the table.
    Turntable,
    /// The others on one arc above me, the camera sliding along it from stop
    /// to stop (MULTIPLAYER §B).
    ArcRail,
    /// Every opponent an upright pod in a grid above mine, seen from above
    /// (MULTIPLAYER §C, as a 3D grid).
    Pods,
    /// Me and the seat of interest as a duel, every other seat parked off
    /// the felt and read from its strip chip (MULTIPLAYER §E, lite).
    Spotlight,
    /// Turntable up to four seats, Spotlight from five or on a phone: a
    /// rule, decided per table ([`Arrangement::resolve`]).
    TurntableRows,
    /// Spotlight's pair, the parked seats as peeks in columns at the arena's
    /// edges (DESIGN-v6).
    FocusRing,
}

impl<'de> serde::Deserialize<'de> for Arrangement {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Ok(Self::ALL
            .into_iter()
            .find(|a| a.tag() == name)
            .unwrap_or_default())
    }
}

impl Arrangement {
    /// Every arrangement, in the menu's order (DESIGN-v8 §1's numbering:
    /// the digit that chooses a row is its place here, plus one).
    pub const ALL: [Self; 8] = [
        Self::Ring,
        Self::UprightRing,
        Self::Turntable,
        Self::ArcRail,
        Self::Pods,
        Self::Spotlight,
        Self::TurntableRows,
        Self::FocusRing,
    ];

    /// Its name, as the pill and the menu say it.
    #[must_use]
    pub const fn name(self) -> Phrase {
        match self {
            Self::Ring => Phrase::ArrRing,
            Self::UprightRing => Phrase::ArrUprightRing,
            Self::Turntable => Phrase::ArrTurntable,
            Self::ArcRail => Phrase::ArrArcRail,
            Self::Pods => Phrase::ArrPods,
            Self::Spotlight => Phrase::ArrSpotlight,
            Self::TurntableRows => Phrase::ArrTurntableRows,
            Self::FocusRing => Phrase::ArrFocusRing,
        }
    }

    /// What it does, in the one line a menu row has for it.
    #[must_use]
    pub const fn blurb(self) -> Phrase {
        match self {
            Self::Ring => Phrase::ArrRingBlurb,
            Self::UprightRing => Phrase::ArrUprightRingBlurb,
            Self::Turntable => Phrase::ArrTurntableBlurb,
            Self::ArcRail => Phrase::ArrArcRailBlurb,
            Self::Pods => Phrase::ArrPodsBlurb,
            Self::Spotlight => Phrase::ArrSpotlightBlurb,
            Self::TurntableRows => Phrase::ArrTurntableRowsBlurb,
            Self::FocusRing => Phrase::ArrFocusRingBlurb,
        }
    }

    /// The name its settings file and `/state.arrangement` spell it with.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Ring => "ring",
            Self::UprightRing => "upright_ring",
            Self::Turntable => "turntable",
            Self::ArcRail => "arc_rail",
            Self::Pods => "pods",
            Self::Spotlight => "spotlight",
            Self::TurntableRows => "turntable_rows",
            Self::FocusRing => "focus_ring",
        }
    }

    /// Where it stands in [`Self::ALL`]: the menu row, and the digit less one.
    #[must_use]
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|a| *a == self).unwrap_or(0)
    }

    /// Whether this build places the seats for it. An arrangement not yet
    /// built is shown in the menu, greyed, with the package that brings it
    /// ([`Self::package`]): the owner sees the whole set from the first day.
    #[must_use]
    pub const fn built(self) -> bool {
        matches!(self, Self::Ring | Self::UprightRing | Self::Spotlight)
    }

    /// The work package that builds it (DESIGN-v8 §4).
    #[must_use]
    pub const fn package(self) -> &'static str {
        match self {
            Self::Ring => "WA1",
            Self::UprightRing => "WA2",
            Self::Spotlight => "WA3",
            Self::Turntable => "WA4",
            Self::TurntableRows => "WA5",
            Self::Pods => "WA6",
            Self::ArcRail => "WA7",
            Self::FocusRing => "WA8",
        }
    }

    /// Whether the menu tags it *experimentell*: a measurement its package
    /// promised is still outstanding (DESIGN-v8 §7.1).
    #[must_use]
    pub const fn experimental(self) -> bool {
        !matches!(self, Self::Ring)
    }

    /// Whether the seat of interest moves **cards** (a layout arrangement:
    /// the slots are re-solved round it and the camera stays home) rather
    /// than the camera (the ring's visit).
    #[must_use]
    pub const fn moves_cards(self) -> bool {
        matches!(
            self,
            Self::Turntable | Self::Spotlight | Self::TurntableRows | Self::FocusRing
        )
    }

    /// Whether it is offered at a table of `seats` on a window of `frame`
    /// (DESIGN-v8 §2.5), and if not, the sentence a greyed menu row and
    /// `/state.arrangement.reason` say instead.
    ///
    /// # Errors
    /// The reason it is not offered.
    pub fn offered(self, seats: usize, frame: TableFrame) -> Result<(), Phrase> {
        use TableFrame::{Compact, Narrow, Phone, Vast, Wide};
        if seats <= 2 {
            return if self == Self::Ring {
                Ok(())
            } else {
                Err(Phrase::ArrNotInADuel)
            };
        }
        if !self.built() {
            return Err(Phrase::ArrComing);
        }
        let wide = matches!(frame, Wide | Vast);
        match self {
            Self::Ring
            | Self::UprightRing
            | Self::ArcRail
            | Self::Spotlight
            | Self::TurntableRows => Ok(()),
            Self::Turntable if seats <= 4 || wide => Ok(()),
            Self::Turntable => Err(Phrase::ArrNotHereSeats),
            Self::Pods => match frame {
                Phone => Err(Phrase::ArrNotOnAPhone),
                Wide | Vast => Ok(()),
                Narrow if seats <= 5 => Ok(()),
                Compact if seats <= 3 => Ok(()),
                Narrow | Compact => Err(Phrase::ArrNotHereSeats),
            },
            Self::FocusRing => match frame {
                Phone => Err(Phrase::ArrNotOnAPhone),
                Compact => Err(Phrase::ArrNotHereWindow),
                Narrow | Wide | Vast => Ok(()),
            },
        }
    }

    /// What it places the seats as at this table: itself, except the
    /// Turntable with rows, which is a rule (DESIGN-v8 §1 row 7) — Turntable
    /// up to four seats on a window that is not a phone's, Spotlight from
    /// five or on a phone. Decided from the seat count and the frame alone,
    /// so it never flips while a game goes on.
    #[must_use]
    pub fn resolve(self, seats: usize, frame: TableFrame) -> Self {
        match self {
            Self::TurntableRows => {
                if seats <= 4 && frame != TableFrame::Phone {
                    Self::Turntable
                } else {
                    Self::Spotlight
                }
            }
            other => other,
        }
    }

    /// The arrangement a table of `seats` on `frame` is drawn with when this
    /// one is chosen: itself if offered (resolved), else the ring, which is
    /// offered everywhere.
    #[must_use]
    pub fn effective(self, seats: usize, frame: TableFrame) -> Self {
        if self.offered(seats, frame).is_ok() {
            self.resolve(seats, frame)
        } else {
            Self::Ring
        }
    }

    /// The arrangements offered at this table, in the menu's order.
    #[must_use]
    pub fn offered_at(seats: usize, frame: TableFrame) -> Vec<Self> {
        Self::ALL
            .into_iter()
            .filter(|a| a.offered(seats, frame).is_ok())
            .collect()
    }

    /// The next offered arrangement after this one (`Shift+P`), past the
    /// last the first. Itself when it is the only one offered.
    #[must_use]
    pub fn next_offered(self, seats: usize, frame: TableFrame) -> Self {
        let offered = Self::offered_at(seats, frame);
        let at = offered.iter().position(|a| *a == self);
        match at {
            Some(i) => offered[(i + 1) % offered.len()],
            None => offered.first().copied().unwrap_or(Self::Ring),
        }
    }

    /// The placeholder pictogram: a letter in a disc, A for the first row to
    /// H for the eighth, until the eight line drawings exist (DESIGN-v8
    /// §7.6).
    #[must_use]
    pub fn letter(self) -> char {
        char::from(b'A' + u8::try_from(self.index()).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A duel has one arrangement, the ring.
    #[test]
    fn a_duel_offers_only_the_ring() {
        for frame in TableFrame::ALL {
            assert_eq!(Arrangement::offered_at(2, frame), vec![Arrangement::Ring]);
            for arm in Arrangement::ALL {
                assert_eq!(arm.effective(2, frame), Arrangement::Ring);
            }
        }
    }

    /// The ring is offered on every window at every seat count.
    #[test]
    fn the_ring_is_offered_everywhere() {
        for frame in TableFrame::ALL {
            for seats in 2..=8 {
                assert!(Arrangement::Ring.offered(seats, frame).is_ok());
            }
        }
    }

    /// An arrangement's name in a settings file this build has never seen
    /// reads as the ring, and never refuses the file.
    #[test]
    fn an_unknown_arrangement_reads_as_the_ring() {
        let read: Arrangement = serde_json::from_str("\"zukunft\"").expect("reads");
        assert_eq!(read, Arrangement::Ring);
        for arm in Arrangement::ALL {
            let json = serde_json::to_string(&arm).expect("writes");
            assert_eq!(json, format!("\"{}\"", arm.tag()));
            let back: Arrangement = serde_json::from_str(&json).expect("reads");
            assert_eq!(back, arm);
        }
    }

    /// `Shift+P` walks the offered arrangements and comes round.
    #[test]
    fn the_next_arrangement_walks_the_offered_ones_and_comes_round() {
        let frame = TableFrame::Wide;
        let offered = Arrangement::offered_at(4, frame);
        let mut at = Arrangement::Ring;
        for want in offered.iter().cycle().skip(1).take(offered.len()) {
            at = at.next_offered(4, frame);
            assert_eq!(at, *want);
        }
        assert_eq!(at, Arrangement::Ring, "past the last is the first");
        assert_eq!(
            Arrangement::Ring.next_offered(2, frame),
            Arrangement::Ring,
            "one offered: a no-op"
        );
    }

    /// The hybrid is a rule of the seat count and the frame.
    #[test]
    fn the_turntable_with_rows_resolves_by_seats_and_frame() {
        let rows = Arrangement::TurntableRows;
        assert_eq!(rows.resolve(4, TableFrame::Wide), Arrangement::Turntable);
        assert_eq!(rows.resolve(3, TableFrame::Compact), Arrangement::Turntable);
        assert_eq!(rows.resolve(5, TableFrame::Wide), Arrangement::Spotlight);
        assert_eq!(rows.resolve(4, TableFrame::Phone), Arrangement::Spotlight);
        assert_eq!(
            Arrangement::Pods.resolve(4, TableFrame::Wide),
            Arrangement::Pods
        );
    }

    /// The letters run A to H in the menu's order.
    #[test]
    fn the_placeholder_letters_run_in_the_menu_s_order() {
        let letters: String = Arrangement::ALL.iter().map(|a| a.letter()).collect();
        assert_eq!(letters, "ABCDEFGH");
    }
}
