//! Encoders 4, 7, 8 and 9 held to one contract on real tables: each answers
//! the questions PR #410 added the same way ([`mechanics`]), and the outputs
//! older datasets were built from stay byte for byte as they were
//! ([`pins`]).
//!
//! The four share one candidate builder (`encode_candidates4`, which
//! `encode_candidates7_grouped` wraps) and differ in their state encoder and
//! in the view their runtime reads, so every test here runs each of them
//! through [`Encoder`], with the view and the seat memory its runtime uses.

use baylee_core::ids::{ObjectId, PlayerId, PrintRef};
use baylee_core::preset::{AIProfile, DeckEntry, FormatId, GamePreset};
use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};
use baylee_gamehost::Session;
use baylee_view::{Phase, PlayerView};

use crate::features4::{
    EncodedCandidates4, EncodedState4, SeatKnowledge4, encode_candidates4, encode_state4,
};
use crate::features7::{encode_candidates7_grouped, encode_state7};

mod mechanics;
mod pins;
pub(super) mod scenes;

pub(super) const P0: PlayerId = PlayerId::new(0);
pub(super) const P1: PlayerId = PlayerId::new(1);

/// The two seats are each on their own side, as convert4 reads a preset
/// without teams.
const TEAMS: [Option<u8>; 2] = [None, None];

/// One member of the encoder family, with the view its runtime reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Encoder {
    /// Encoder 6, the v4 state and candidate encoder.
    V4,
    /// Encoder 7: v4 candidates grouped, the v7 state.
    V7,
    /// Encoder 8: Encoder 7 with the announced payment source.
    V8,
    /// Encoder 9: Encoder 8 with the settled X.
    V9,
}

impl Encoder {
    pub(super) const ALL: [Self; 4] = [Self::V4, Self::V7, Self::V8, Self::V9];

    /// The deciding view this encoder's runtime and converter read.
    pub(super) fn view_for(self, session: &Session, seat: PlayerId) -> (Pending, PlayerView) {
        match self {
            Self::V4 | Self::V7 => session.view_for_with_context(seat),
            Self::V8 => session.view_for_with_payment_context(seat),
            Self::V9 => session.view_for_with_announcement_context(seat),
        }
        .expect("the seat is asked")
    }

    /// The passive view a seat's memory observes between decisions.
    pub(super) fn observe(self, session: &Session, seat: PlayerId) -> PlayerView {
        match self {
            Self::V4 | Self::V7 => session.observe_view_with_context(seat),
            Self::V8 => session.observe_view_with_payment_context(seat),
            Self::V9 => session.observe_view_with_announcement_context(seat),
        }
    }

    /// Feeds `view` to a seat's memory; whether it changed.
    pub(super) fn remember(self, knowledge: &mut SeatKnowledge4, view: &PlayerView) -> bool {
        match self {
            Self::V4 | Self::V7 => knowledge.observe(view),
            Self::V8 => knowledge.observe_with_payment_context(view),
            Self::V9 => knowledge.observe_with_announcement_context(view),
        }
    }

    /// The state this encoder writes for `view`, asked `pending` or passive.
    pub(super) fn state(
        self,
        view: &PlayerView,
        pending: Option<&Pending>,
        knowledge: Option<&SeatKnowledge4>,
    ) -> EncodedState4 {
        match self {
            Self::V4 => encode_state4(view, pending, knowledge, &TEAMS),
            Self::V7 => encode_state7(view, pending, knowledge, &TEAMS),
            Self::V8 => crate::features8::encode_state8(view, pending, knowledge, &TEAMS),
            Self::V9 => crate::features9::encode_state9(view, pending, knowledge, &TEAMS),
        }
    }

    /// State and candidates for a decision, as `NetPlayer4::policy_encoding`
    /// builds them.
    pub(super) fn encode(
        self,
        view: &PlayerView,
        pending: &Pending,
        knowledge: Option<&SeatKnowledge4>,
    ) -> (EncodedState4, EncodedCandidates4) {
        let state = self.state(view, Some(pending), knowledge);
        let candidates = match self {
            Self::V4 => encode_candidates4(&state, view, pending),
            Self::V7 | Self::V8 | Self::V9 => encode_candidates7_grouped(&state, view, pending),
        };
        (state, candidates)
    }
}

/// A deck entry for the first pool card printed with `name`.
pub(super) fn card(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::all()
            .find(|c| c.name() == name)
            .unwrap_or_else(|| panic!("{name} is in the pool"))
            .index,
        print: PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

/// What each seat starts with: its hand and its battlefield, by name.
pub(super) struct Seat<'a> {
    pub hand: &'a [&'a str],
    pub battlefield: &'a [&'a str],
}

/// A freeform two-seat table both of whose seats the test answers for.
pub(super) struct Table {
    pub preset: GamePreset,
    pub session: Session,
}

impl Table {
    /// Libraries of 32 `library` cards, 20 life, no commanders.
    pub fn new(seed: u64, library: &str, seats: [Seat<'_>; 2]) -> Self {
        let deck = crate::housedeck::HouseDeck::named("victory").expect("the victory house deck");
        let mut preset =
            crate::selfplay::table_for(seed, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
        preset.format = FormatId::Freeform;
        preset.modifiers.clear();
        for (spec, seat) in preset.seats.iter_mut().zip(seats) {
            spec.deck = vec![card(library); 32];
            spec.commanders.clear();
            spec.sideboard.clear();
            spec.starting_life = Some(20);
            spec.starting_hand = Some(seat.hand.iter().map(|name| card(name)).collect());
            spec.starting_battlefield = seat.battlefield.iter().map(|name| card(name)).collect();
        }
        let mut session = Session::new(&preset).expect("the preset builds a game");
        assert!(session.take_over(P0));
        assert!(session.take_over(P1));
        Self { preset, session }
    }

    /// Answers every question by its default (keep, pass, no attack, no
    /// block) until `done` holds.
    pub fn answer_until(&mut self, done: impl Fn(&Session) -> bool) {
        for _ in 0..512 {
            if done(&self.session) {
                return;
            }
            let pending = self.session.pending().clone();
            let seat = pending.asked().expect("a question is open");
            let action =
                timeout_answer(&pending).unwrap_or_else(|| panic!("no default for {pending:?}"));
            self.session
                .act(seat, action)
                .expect("a default answer is legal");
        }
        panic!("the table never got there");
    }

    /// Answers by default until `seat` holds priority in its own main phase
    /// with an empty stack.
    pub fn reach_main(&mut self, seat: PlayerId) {
        self.answer_until(|session| at_rest_in_main(session, seat));
    }

    /// Taps every land of `seat`'s that makes mana by its basic land type.
    pub fn tap_lands(&mut self, seat: PlayerId) {
        while let Pending::Priority { player, legal } = self.session.pending()
            && *player == seat
            && let Some(&source) = legal.mana_abilities.first()
        {
            self.session
                .act(seat, PlayerAction::ActivateManaAbility { source })
                .expect("an offered mana ability");
        }
    }

    /// The object in `seat`'s hand printed with `name`.
    pub fn in_hand(&self, seat: PlayerId, name: &str) -> ObjectId {
        self.session
            .observe_view(seat)
            .hand
            .iter()
            .find(|card| card.name == name)
            .unwrap_or_else(|| panic!("{name} is in seat {seat:?}'s hand"))
            .id
    }
}

/// `seat` holds priority on its own turn, in a main phase, over an empty stack.
pub(super) fn at_rest_in_main(session: &Session, seat: PlayerId) -> bool {
    let view = session.observe_view(seat);
    view.active == seat
        && matches!(view.phase, Phase::FirstMain | Phase::SecondMain)
        && view.stack.is_empty()
        && matches!(session.pending(), Pending::Priority { player, .. } if *player == seat)
}
