//! The banner over the table while another player's connection is lost
//! (`docs/protocol.md` §"Leaving, and losing the connection").
//!
//! [`baylee_view::PlayerView::lost`] says who is lost and how much of the
//! table's wait is left, relative to the view, as
//! [`crate::decisionclock`] reads its clocks: the client counts down from
//! the number and takes the next view as the correction. The banner says
//! "… lost the connection – waiting 2:41", ticking once a second; once the
//! wait is out (or the roster says the house holds the chair) "The house AI
//! plays for …"; nothing once they are back.
//!
//! [`LostSeats::banner`] is a small `Copy` value that changes only when the
//! shown second does, so a renderer compares it and writes text only then.

use crate::decisionclock::mmss;
use crate::i18n::{Lang, Phrase, seat_name};
use baylee_core::ids::PlayerId;
use baylee_view::{GameStatic, LostSeat};

/// What the banner says, as a value: equal from frame to frame until the
/// shown second (or the case) changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Banner {
    /// The table waits for `seat`, `secs` left (rounded up).
    Waiting {
        /// Whose connection is lost.
        seat: PlayerId,
        /// Whole seconds left of the wait.
        secs: u32,
    },
    /// The game is paused for `seat`.
    Paused {
        /// Whose connection is lost.
        seat: PlayerId,
    },
    /// The house plays `seat`'s chair.
    House {
        /// Whose chair.
        seat: PlayerId,
    },
}

impl Banner {
    /// The sentence, in `lang`, naming the seat as the roster does.
    #[must_use]
    pub fn line(self, lang: Lang, statics: Option<&GameStatic>) -> String {
        match self {
            Self::Waiting { seat, secs } => {
                Phrase::LostWaiting.fill(lang, &[&seat_name(lang, statics, seat), &mmss(secs)])
            }
            Self::Paused { seat } => {
                Phrase::LostPaused.fill(lang, &[&seat_name(lang, statics, seat)])
            }
            Self::House { seat } => {
                Phrase::LostHousePlays.fill(lang, &[&seat_name(lang, statics, seat)])
            }
        }
    }
}

/// Every lost seat and what is left of its wait, counted down locally.
#[derive(Clone, Debug, Default)]
pub struct LostSeats {
    /// `(seat, seconds left or None while paused)`, in seat order.
    left: Vec<(PlayerId, Option<f32>)>,
}

impl LostSeats {
    /// Takes what a view says.
    pub fn sync(&mut self, lost: &[LostSeat]) {
        self.left.clear();
        self.left.extend(lost.iter().map(|l| {
            (
                l.seat,
                l.remaining_ms.map(|ms| f64::from(ms) as f32 / 1_000.0),
            )
        }));
    }

    /// `dt` seconds passed: every wait counts down, floored at zero.
    pub fn advance(&mut self, dt: f32) {
        for (_, left) in &mut self.left {
            if let Some(left) = left {
                *left = (*left - dt).max(0.0);
            }
        }
    }

    /// What the banner says to `me`, or `None` when nobody is lost.
    ///
    /// The first lost seat, in seat order; a wait that has run out reads as
    /// the house playing (the next view says so too, once the engine's
    /// clock has fired). With nobody lost, the first other chair the roster
    /// says the house holds (`away`): the house plays for them until they
    /// are back.
    #[must_use]
    pub fn banner(&self, statics: Option<&GameStatic>, me: PlayerId) -> Option<Banner> {
        if let Some(&(seat, left)) = self.left.iter().find(|(seat, _)| *seat != me) {
            return Some(match left {
                None => Banner::Paused { seat },
                Some(left) if left <= 0.0 => Banner::House { seat },
                // Rounded up, as a decision clock is: `0:01` is the last
                // second shown, never `0:00` while still waiting.
                Some(left) => Banner::Waiting {
                    seat,
                    secs: left.ceil() as u32,
                },
            });
        }
        statics?
            .seats
            .iter()
            .find(|seat| seat.player != me && seat.away)
            .map(|seat| Banner::House { seat: seat.player })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roster() -> GameStatic {
        let mut statics = crate::test_support::statics(0);
        statics.seats.push(baylee_view::SeatIdentity {
            player: PlayerId::new(1),
            display_name: "Mia".to_owned(),
            is_ai: false,
            away: false,
            team: None,
        });
        statics
    }

    fn lost(remaining_ms: Option<u32>) -> [LostSeat; 1] {
        [LostSeat {
            seat: PlayerId::new(1),
            remaining_ms,
        }]
    }

    /// The countdown over time: it ticks once a second, the value stays
    /// equal within a second (so nothing is rewritten), and at the end the
    /// house plays.
    #[test]
    fn the_countdown_ticks_once_a_second_then_the_house_plays() {
        let statics = roster();
        let me = PlayerId::new(0);
        let mut seats = LostSeats::default();
        seats.sync(&lost(Some(161_000)));
        let first = seats.banner(Some(&statics), me).expect("Mia is lost");
        assert_eq!(
            first.line(Lang::De, Some(&statics)),
            "Mia hat die Verbindung verloren – warte 2:41"
        );
        assert_eq!(
            first.line(Lang::En, Some(&statics)),
            "Mia lost the connection – waiting 2:41"
        );
        seats.advance(0.4);
        assert_eq!(
            seats.banner(Some(&statics), me),
            Some(first),
            "inside a second nothing changes"
        );
        seats.advance(0.7);
        assert_eq!(
            seats.banner(Some(&statics), me),
            Some(Banner::Waiting {
                seat: PlayerId::new(1),
                secs: 160
            })
        );
        seats.advance(159.5);
        assert_eq!(
            seats
                .banner(Some(&statics), me)
                .map(|b| b.line(Lang::En, Some(&statics))),
            Some("Mia lost the connection – waiting 0:01".to_owned())
        );
        seats.advance(1.0);
        let house = seats.banner(Some(&statics), me).expect("still shown");
        assert_eq!(
            house,
            Banner::House {
                seat: PlayerId::new(1)
            }
        );
        assert_eq!(
            house.line(Lang::De, Some(&statics)),
            "Die Haus-KI spielt für Mia"
        );
        assert_eq!(
            house.line(Lang::En, Some(&statics)),
            "The house AI plays for Mia"
        );
    }

    /// The house holding the chair (the roster's `away`) keeps the banner;
    /// a player back, or a seat that is me, shows nothing; a paused loss
    /// says so.
    #[test]
    fn the_banner_follows_the_chair() {
        let mut statics = roster();
        let me = PlayerId::new(0);
        let mut seats = LostSeats::default();
        assert_eq!(seats.banner(Some(&statics), me), None, "nobody is lost");
        statics.seats[1].away = true;
        seats.sync(&[]);
        assert_eq!(
            seats.banner(Some(&statics), me),
            Some(Banner::House {
                seat: PlayerId::new(1)
            })
        );
        statics.seats[1].away = false;
        assert_eq!(seats.banner(Some(&statics), me), None, "back: gone");
        seats.sync(&lost(None));
        assert_eq!(
            seats
                .banner(Some(&statics), me)
                .map(|b| b.line(Lang::En, Some(&statics))),
            Some("Mia lost the connection – the game is paused until they are back".to_owned())
        );
        assert_eq!(
            seats.banner(Some(&statics), PlayerId::new(1)),
            None,
            "a seat is never told about itself"
        );
    }
}
