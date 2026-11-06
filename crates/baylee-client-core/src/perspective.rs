//! Whose chair the bottom of the table is shown from while this seat
//! controls another player (CR 723.5: Mindslaver's turn, Word of Command's
//! instruction, CR 723.2).
//!
//! A seat that controls players makes their decisions with their cards and
//! mana, and makes its own as well (CR 723.8). The hand, the pool and the
//! name the question is asked in are one player's at a time; this decides
//! whose, as a pure state machine the client reads on two edges:
//!
//! - **A view arrives** ([`Perspective::read`]): the seats this one may be
//!   shown from are its own and each player the view carries a controlled
//!   hand for ([`seats`]). A shown player no longer among them (the control
//!   ended mid-perspective) falls back to this seat's own.
//! - **A decision arrives** ([`Perspective::decision`]): a question installed
//!   where none stood, so a re-sent question is not one. With auto-follow on,
//!   the perspective goes to the player the decision is for: the controlled
//!   player's question shows their chair, this seat's own shows its own.
//!   A seat chosen by hand ([`Perspective::choose`], [`Perspective::cycle`])
//!   holds until then.
//!
//! Nothing here can show more than the view carries: a perspective is one
//! of [`seats`], and a player's hand is read only from where the view put it
//! ([`crate::decision::hand_of`]). The question itself is never hidden by the
//! perspective; it is named for the player it is asked for.

use baylee_core::ids::PlayerId;
use baylee_view::PlayerView;

/// The seats this one may be shown from: its own first, then every player
/// it controls now (the view's `controlled_hands`, CR 723.4), in seat order.
#[must_use]
pub fn seats(view: &PlayerView) -> Vec<PlayerId> {
    let mut controlled: Vec<PlayerId> = view
        .controlled_hands
        .iter()
        .map(|hand| hand.player)
        .filter(|player| *player != view.seat)
        .collect();
    controlled.sort_unstable_by_key(|player| player.get());
    controlled.dedup();
    std::iter::once(view.seat).chain(controlled).collect()
}

/// Whether this seat controls `player` now (CR 723.5), by the view's word.
#[must_use]
pub fn controls(view: &PlayerView, player: PlayerId) -> bool {
    player != view.seat && view.controlled_hands.iter().any(|h| h.player == player)
}

/// Who is deciding for whom right now, as this seat's view says it. Control
/// of a player is public (CR 723.1 names no hidden part of it), so every seat
/// is told: the question in hand first (`awaiting` answers it, and it is
/// asked of `decision_player`), else the view's `controls`, which hold for
/// the whole controlled turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlLine {
    /// This seat is making `player`'s decision (CR 723.5).
    DecidingFor {
        /// The controlled player.
        player: PlayerId,
    },
    /// `controller` is making this seat's decision. `turn` when it is this
    /// seat's own turn (Mindslaver's, CR 723.1); otherwise a limited control
    /// (Word of Command, CR 723.2).
    DecidedBy {
        /// The player in control.
        controller: PlayerId,
        /// Whether it is this seat's turn.
        turn: bool,
    },
    /// `controller` is making `player`'s decision, neither of them this seat.
    Deciding {
        /// The player in control.
        controller: PlayerId,
        /// The controlled player.
        player: PlayerId,
    },
}

impl ControlLine {
    /// What the view says, or `None` while nobody decides for anybody, and
    /// while this seat is answering its own question inside a turn it
    /// controls (CR 723.8): that decision is its own.
    #[must_use]
    pub fn of(view: &PlayerView) -> Option<Self> {
        if let (Some(controller), Some(player)) = (view.awaiting, view.decision_player) {
            if controller != player {
                return Some(Self::between(view, controller, player));
            }
            if controller == view.seat {
                return None;
            }
        }
        let control = view
            .controls
            .iter()
            .find(|c| c.controller == view.seat || c.player == view.seat)
            .or_else(|| view.controls.first())?;
        Some(Self::between(view, control.controller, control.player))
    }

    fn between(view: &PlayerView, controller: PlayerId, player: PlayerId) -> Self {
        if controller == view.seat {
            Self::DecidingFor { player }
        } else if player == view.seat {
            Self::DecidedBy {
                controller,
                turn: view.active == view.seat,
            }
        } else {
            Self::Deciding { controller, player }
        }
    }

    /// The card whose effect gave this control, where the view names one:
    /// the banner's "(Mindslaver)".
    #[must_use]
    pub fn source(self, view: &PlayerView) -> Option<baylee_core::ids::CardIndex> {
        let (player, controller) = self.pair(view.seat);
        view.controls
            .iter()
            .find(|c| c.player == player && c.controller == controller)
            .and_then(|c| c.source)
    }

    /// The banner's sentence: "You decide for Mia" to the seat in control,
    /// "Ben controls your turn" (or, outside it, "Ben decides for you") to
    /// the controlled player, "Ben decides for Mia" to everyone else.
    #[must_use]
    pub fn text(
        self,
        lang: crate::i18n::Lang,
        statics: Option<&baylee_view::GameStatic>,
    ) -> String {
        use crate::i18n::{Phrase, seat_name};
        let name = |player| seat_name(lang, statics, player);
        match self {
            Self::DecidingFor { player } => Phrase::DecidingFor.fill(lang, &[&name(player)]),
            Self::DecidedBy {
                controller,
                turn: true,
            } => Phrase::ControlTakesYourTurn.fill(lang, &[&name(controller)]),
            Self::DecidedBy { controller, .. } => {
                Phrase::ControlDecidesForYou.fill(lang, &[&name(controller)])
            }
            Self::Deciding { controller, player } => {
                Phrase::ControlDecidesForOther.fill(lang, &[&name(controller), &name(player)])
            }
        }
    }

    /// The controlled player and the one in control, whoever this seat is.
    #[must_use]
    pub fn pair(self, seat: PlayerId) -> (PlayerId, PlayerId) {
        match self {
            Self::DecidingFor { player } => (player, seat),
            Self::DecidedBy { controller, .. } => (seat, controller),
            Self::Deciding { controller, player } => (player, controller),
        }
    }
}

/// The card behind `player`'s control as the table can see it now: the view's
/// `source` names the card, and its printing and finish are on the object. A
/// cost may have moved that (Mindslaver is sacrificed), so the zones are read,
/// the controller's own first for two copies in two printings. `None` where no
/// object shows it: nothing is guessed.
#[must_use]
pub fn control_source(view: &PlayerView, player: PlayerId) -> Option<baylee_view::CardIdentity> {
    let control = view.controls.iter().find(|c| c.player == player)?;
    let source = control.source?;
    let own = usize::from(control.controller.get());
    let of = |zones: &[Vec<baylee_view::PublicObject>]| zones.get(own).cloned();
    of(&view.graveyards)
        .into_iter()
        .chain(of(&view.exile))
        .flatten()
        .chain(view.graveyards.iter().flatten().cloned())
        .chain(view.exile.iter().flatten().cloned())
        .chain(view.battlefield.iter().cloned())
        .chain(view.stack.iter().cloned())
        .filter_map(|object| object.card)
        .find(|card| card.index == source)
}

/// Whose chair the bottom of the table shows, and how it came to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Perspective {
    /// The controlled player shown; `None` is this seat's own.
    shown: Option<PlayerId>,
    /// Whether the player chose it by hand since the last decision.
    by_hand: bool,
}

impl Perspective {
    /// The player the table is shown from.
    #[must_use]
    pub fn shown(&self, view: &PlayerView) -> PlayerId {
        self.shown.unwrap_or(view.seat)
    }

    /// The controlled player shown, or `None` while this seat shows its own.
    #[must_use]
    pub fn other(&self) -> Option<PlayerId> {
        self.shown
    }

    /// Whether the seat shown was chosen by hand since the last decision.
    #[must_use]
    pub fn by_hand(&self) -> bool {
        self.by_hand
    }

    /// A view arrived. A shown player this seat no longer controls is shown
    /// no longer: the table falls back to this seat's own (and a choice by
    /// hand with it). Answers whether the shown seat changed.
    pub fn read(&mut self, view: &PlayerView) -> bool {
        match self.shown {
            Some(player) if !controls(view, player) => {
                *self = Self::default();
                true
            }
            _ => false,
        }
    }

    /// A decision arrived for this seat, made for `deciding` (this seat
    /// itself, or a player it controls). With `follow`, the table is shown
    /// from that player, and a seat chosen by hand gives way to it. Answers
    /// whether the shown seat changed.
    pub fn decision(&mut self, view: &PlayerView, deciding: PlayerId, follow: bool) -> bool {
        self.read(view);
        if !follow {
            return false;
        }
        let next = (deciding != view.seat && controls(view, deciding)).then_some(deciding);
        let changed = next != self.shown;
        *self = Self {
            shown: next,
            by_hand: false,
        };
        changed
    }

    /// The player chose to be shown `player`'s chair: one of [`seats`], or
    /// nothing changes. Holds until the next decision. Answers whether the
    /// shown seat changed.
    pub fn choose(&mut self, view: &PlayerView, player: PlayerId) -> bool {
        if !seats(view).contains(&player) {
            return false;
        }
        let next = (player != view.seat).then_some(player);
        let changed = next != self.shown;
        *self = Self {
            shown: next,
            by_hand: true,
        };
        changed
    }

    /// The next of [`seats`] after the one shown (or before it), round past
    /// either end. Nothing to cycle with one seat. Answers whether the shown
    /// seat changed.
    pub fn cycle(&mut self, view: &PlayerView, forward: bool) -> bool {
        self.read(view);
        let seats = seats(view);
        if seats.len() < 2 {
            return false;
        }
        let at = seats
            .iter()
            .position(|p| *p == self.shown(view))
            .unwrap_or(0);
        let next = if forward {
            (at + 1) % seats.len()
        } else {
            (at + seats.len() - 1) % seats.len()
        };
        self.choose(view, seats[next])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ViewBuilder;

    const ME: PlayerId = PlayerId::new(0);
    const P2: PlayerId = PlayerId::new(1);
    const P3: PlayerId = PlayerId::new(2);

    /// This seat controls `players`, and is asked a decision for `deciding`.
    fn view(players: &[u8], deciding: PlayerId) -> PlayerView {
        let mut view = ViewBuilder::new(4).build();
        view.controlled_hands = players
            .iter()
            .map(|p| baylee_view::SharedHand {
                player: PlayerId::new(*p),
                public: false,
                cards: Vec::new(),
            })
            .collect();
        view.awaiting = Some(ME);
        view.decision_player = Some(deciding);
        view
    }

    #[test]
    fn the_seats_are_this_one_then_every_controlled_player_in_seat_order() {
        assert_eq!(seats(&view(&[], ME)), vec![ME]);
        assert_eq!(seats(&view(&[2, 1], ME)), vec![ME, P2, P3]);
        assert!(controls(&view(&[1], ME), P2));
        assert!(!controls(&view(&[1], ME), P3));
        assert!(
            !controls(&view(&[1], ME), ME),
            "this seat is its own, not controlled"
        );
    }

    /// Auto-follow: their decision shows their chair, mine shows mine.
    #[test]
    fn a_decision_shows_the_chair_it_is_for_and_mine_brings_it_back() {
        let mut p = Perspective::default();
        let theirs = view(&[1], P2);
        assert!(p.decision(&theirs, P2, true));
        assert_eq!(p.shown(&theirs), P2);
        assert!(!p.by_hand());
        assert!(
            !p.decision(&theirs, P2, true),
            "the same chair again changes nothing"
        );
        let mine = view(&[1], ME);
        assert!(
            p.decision(&mine, ME, true),
            "my blocks (CR 723.8) bring it back"
        );
        assert_eq!(p.shown(&mine), ME);
    }

    /// A seat chosen by hand holds until the next decision, which wins.
    #[test]
    fn a_choice_by_hand_holds_until_the_next_decision() {
        let mut p = Perspective::default();
        let theirs = view(&[1], P2);
        p.decision(&theirs, P2, true);
        assert!(p.choose(&theirs, ME), "back to my own chair by hand");
        assert!(p.by_hand());
        assert!(!p.read(&theirs), "a re-sent view leaves the choice alone");
        assert_eq!(p.shown(&theirs), ME);
        assert!(
            p.decision(&theirs, P2, true),
            "the next decision follows again"
        );
        assert_eq!(p.shown(&theirs), P2);
        assert!(!p.by_hand());
    }

    /// Without auto-follow nothing moves but a choice by hand.
    #[test]
    fn with_follow_off_only_the_player_moves_it() {
        let mut p = Perspective::default();
        let theirs = view(&[1], P2);
        assert!(!p.decision(&theirs, P2, false));
        assert_eq!(p.shown(&theirs), ME);
        assert!(p.choose(&theirs, P2));
        assert!(!p.decision(&view(&[1], ME), ME, false));
        assert_eq!(p.shown(&theirs), P2);
    }

    /// Several controlled seats: each is a perspective, cycled round past
    /// either end, and a seat this one does not control is never one.
    #[test]
    fn several_controlled_seats_cycle_and_only_they_can_be_shown() {
        let mut p = Perspective::default();
        let v = view(&[1, 2], ME);
        assert!(p.cycle(&v, true));
        assert_eq!(p.shown(&v), P2);
        assert!(p.cycle(&v, true));
        assert_eq!(p.shown(&v), P3);
        assert!(p.cycle(&v, true));
        assert_eq!(p.shown(&v), ME, "round past the end");
        assert!(p.cycle(&v, false));
        assert_eq!(p.shown(&v), P3, "and back past the start");
        assert!(p.decision(&v, P2, true));
        assert_eq!(
            p.shown(&v),
            P2,
            "a decision for another controlled seat goes there"
        );

        let only_one = view(&[1], P2);
        assert!(
            !p.choose(&only_one, PlayerId::new(3)),
            "a seat not controlled"
        );
        assert!(
            !Perspective::default().cycle(&view(&[], ME), true),
            "nothing to cycle"
        );
    }

    /// Every seat is told who decides for whom, from the same two fields:
    /// the controller, the controlled player on its own turn and outside it,
    /// a third seat, and nobody while each decides for itself.
    #[test]
    fn the_control_line_is_read_from_who_answers_and_whose_question_it_is() {
        let mut v = view(&[1], P2);
        assert_eq!(
            ControlLine::of(&v),
            Some(ControlLine::DecidingFor { player: P2 })
        );
        v.decision_player = Some(ME);
        assert_eq!(ControlLine::of(&v), None, "my own question");

        let mut theirs = ViewBuilder::new(3).build();
        theirs.seat = P2;
        theirs.active = P2;
        theirs.awaiting = Some(ME);
        theirs.decision_player = Some(P2);
        assert_eq!(
            ControlLine::of(&theirs),
            Some(ControlLine::DecidedBy {
                controller: ME,
                turn: true
            })
        );
        theirs.active = ME;
        assert_eq!(
            ControlLine::of(&theirs),
            Some(ControlLine::DecidedBy {
                controller: ME,
                turn: false
            }),
            "outside their turn: a limited control (CR 723.2)"
        );
        theirs.seat = P3;
        let line = ControlLine::of(&theirs).expect("public");
        assert_eq!(
            line,
            ControlLine::Deciding {
                controller: ME,
                player: P2
            }
        );
        assert_eq!(line.pair(P3), (P2, ME));
        theirs.awaiting = Some(P2);
        assert_eq!(ControlLine::of(&theirs), None);
    }

    /// `PlayerView::controls` tells the table for the whole controlled turn:
    /// a third seat between two questions, with the card behind it, and the
    /// controller's own question inside that turn stays its own (CR 723.8).
    #[test]
    fn the_public_control_holds_for_the_whole_turn() {
        let card = baylee_core::ids::CardIndex::new(7);
        let mut v = ViewBuilder::new(3).build();
        v.seat = P3;
        v.awaiting = None;
        v.decision_player = None;
        assert_eq!(ControlLine::of(&v), None, "nobody controls anybody");
        v.controls = vec![baylee_view::PlayerControl {
            player: P2,
            controller: ME,
            source: Some(card),
        }];
        let line = ControlLine::of(&v).expect("public between questions");
        assert_eq!(
            line,
            ControlLine::Deciding {
                controller: ME,
                player: P2
            }
        );
        assert_eq!(line.source(&v), Some(card));

        v.seat = ME;
        v.awaiting = Some(ME);
        v.decision_player = Some(ME);
        assert_eq!(ControlLine::of(&v), None, "its own blocks are its own");
        v.awaiting = None;
        assert_eq!(
            ControlLine::of(&v),
            Some(ControlLine::DecidingFor { player: P2 })
        );
    }

    /// Each seat reads the line in its own words, the players by name.
    #[test]
    fn the_control_line_names_both_players() {
        use crate::i18n::Lang;
        let mut statics = crate::test_support::statics(0);
        statics.seats.clear();
        for (player, name) in [(ME, "Ben"), (P2, "Mia"), (P3, "Ola")] {
            statics.seats.push(baylee_view::SeatIdentity {
                player,
                display_name: name.to_owned(),
                is_ai: false,
                away: false,
                team: None,
                guest: false,
            });
        }
        let s = Some(&statics);
        let say = |line: ControlLine, lang| line.text(lang, s);
        assert_eq!(
            say(ControlLine::DecidingFor { player: P2 }, Lang::En),
            "You decide for Mia"
        );
        let mine = ControlLine::DecidedBy {
            controller: ME,
            turn: true,
        };
        assert_eq!(say(mine, Lang::En), "Ben controls your turn");
        assert_eq!(say(mine, Lang::De), "Ben steuert deinen Zug");
        let limited = ControlLine::DecidedBy {
            controller: ME,
            turn: false,
        };
        assert_eq!(say(limited, Lang::En), "Ben decides for you");
        let other = ControlLine::Deciding {
            controller: ME,
            player: P2,
        };
        assert_eq!(say(other, Lang::En), "Ben decides for Mia");
        assert_eq!(say(other, Lang::De), "Ben entscheidet für Mia");
    }

    /// The control ends while its chair is shown: the next view brings this
    /// seat's own back, whatever chose the chair.
    #[test]
    fn control_ending_mid_perspective_falls_back_to_my_own_seat() {
        let mut p = Perspective::default();
        p.choose(&view(&[1, 2], ME), P3);
        let after = view(&[1], ME);
        assert!(p.read(&after));
        assert_eq!(p.shown(&after), ME);
        assert!(!p.by_hand());
        let mut followed = Perspective::default();
        followed.decision(&view(&[1], P2), P2, true);
        let mut ended = view(&[], P2);
        ended.awaiting = Some(P2);
        assert!(followed.read(&ended));
        assert_eq!(followed.other(), None);
        assert!(
            !followed.decision(&ended, P2, true),
            "a decision for a player no longer controlled shows my own"
        );
    }
}
