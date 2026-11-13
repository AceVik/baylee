//! A table a real engine dealt, for asking the house what it does with a
//! board: seat 0 holds a hand and permanents chosen by name, seat 1 has its
//! permanents, and both libraries are one basic land. Shared by the tests of
//! the house's casts (`ai_full_casts`) and aims (`ai_target_sides`).

use baylee_ai::{AIProfile, HeuristicAgent};
use baylee_core::ids::{ObjectId, PlayerId, PrintRef};
use baylee_core::preset::{DeckEntry, GamePreset, SeatController};
use baylee_engine::choice::{Pending, PlayerAction, timeout_answer};
use baylee_engine::engine::Engine;
use baylee_engine::turn::Step;
use baylee_engine::zone::ZoneLocation;
use baylee_gamehost::{RegistryLookup, SeatContext, owed_payment, player_view};
use baylee_view::PlayerView;

pub(crate) const ME: PlayerId = PlayerId::new(0);

pub(crate) fn named(name: &str) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("{name} is in the pool")),
        print: PrintRef::new(0),
    }
}

/// Seat 0 starts, holding `hand` with `mine` on the table; seat 1 has
/// `theirs`. Both libraries are `library`.
pub(crate) fn preset(library: &str, hand: &[&str], mine: &[&str], theirs: &[&str]) -> GamePreset {
    let land = named(library);
    let mut preset =
        baylee_cards::decks::probe_preset(41, land.card).expect("a probe table for the land");
    preset.house_rules.starting_player = Some(0);
    for seat in &mut preset.seats {
        seat.controller = SeatController::Open;
        seat.deck = (0..40).map(|_| land).collect();
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_hand = Some(hand.iter().copied().map(named).collect());
    preset.seats[0].starting_battlefield = mine.iter().copied().map(named).collect();
    preset.seats[1].starting_battlefield = theirs.iter().copied().map(named).collect();
    preset
}

pub(crate) struct Table {
    pub(crate) engine: Engine<RegistryLookup>,
}

impl Table {
    pub(crate) fn open(library: &str, hand: &[&str], mine: &[&str], theirs: &[&str]) -> Self {
        let engine = Engine::new(&preset(library, hand, mine, theirs), RegistryLookup)
            .expect("the duel starts");
        let mut table = Self { engine };
        table.play_to_my_main(1);
        table
    }

    /// Answers everything with its timeout answer until seat 0 has priority
    /// in its own precombat main phase of turn `turn` or later, the stack
    /// empty.
    pub(crate) fn play_to_my_main(&mut self, turn: u32) {
        self.play_to_my(Step::Main, turn);
    }

    /// [`Self::play_to_my_main`] for seat 0's step `step`.
    pub(crate) fn play_to_my(&mut self, step: Step, turn: u32) {
        for _ in 0..400 {
            let pending = self.engine.pending().clone();
            let state = self.engine.state();
            if matches!(pending, Pending::Priority { player, .. } if player == ME)
                && state.turn.active == ME
                && state.turn.step == step
                && state.turn.number >= turn
                && state.zones.stack_is_empty()
            {
                return;
            }
            let player = pending.asked().expect("somebody is asked");
            let answer = timeout_answer(&pending).expect("a default answer");
            self.engine.apply(player, answer).expect("a legal default");
        }
        panic!("never reached seat 0's {step:?} of turn {turn}");
    }

    pub(crate) fn view(&self) -> PlayerView {
        let pending = self.engine.pending().clone();
        player_view(
            self.engine.state(),
            ME,
            0,
            Some(&pending),
            &SeatContext {
                awaiting: baylee_gamehost::view::awaiting_for(&self.engine, ME),
                controls: &baylee_gamehost::view::player_controls(&self.engine),
                decision_player: pending.asked(),
                controlled_players: self.engine.controlled_players(ME),
                deciding: baylee_gamehost::view::deciding(&self.engine),
                owed: owed_payment(&self.engine),
                ..SeatContext::default()
            },
            &[],
        )
    }

    pub(crate) fn apply(&mut self, action: PlayerAction) {
        let player = self.engine.pending().asked().expect("somebody is asked");
        let shown = format!("{action:?}");
        self.engine
            .apply(player, action)
            .unwrap_or_else(|e| panic!("{shown} refused: {e:?}"));
    }

    /// Seat 0's permanents called `name`.
    pub(crate) fn mine(&self, name: &str) -> Vec<ObjectId> {
        self.view()
            .battlefield
            .iter()
            .filter(|o| o.controller == ME && o.name.as_ref() == name)
            .map(|o| o.id)
            .collect()
    }

    /// Seat 0's card called `name` wherever its view shows it.
    pub(crate) fn card(&self, name: &str) -> ObjectId {
        let view = self.view();
        view.hand
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.id)
            .or_else(|| {
                view.graveyards
                    .iter()
                    .chain(&view.exile)
                    .flatten()
                    .find(|o| o.owner == ME && o.name.as_ref() == name)
                    .map(|o| o.id)
            })
            .unwrap_or_else(|| panic!("{name} is in no zone seat 0 sees"))
    }

    pub(crate) fn on_the_stack(&self, card: ObjectId) -> bool {
        self.engine
            .state()
            .zones
            .list(ZoneLocation::Stack)
            .contains(&card)
    }

    /// `profile` answers every question seat 0 is asked in this main phase
    /// until `card` is on the stack (`true`) or it passes with the stack
    /// empty (`false`); seat 1 answers with its timeout answers.
    pub(crate) fn house_casts(&mut self, profile: AIProfile, card: ObjectId) -> bool {
        let agent = HeuristicAgent::new(profile);
        for _ in 0..60 {
            if self.on_the_stack(card) {
                return true;
            }
            let pending = self.engine.pending().clone();
            let player = pending.asked().expect("somebody is asked");
            if player != ME {
                return false;
            }
            let view = self.view();
            let context = self.engine.decision_context();
            let action = agent.act_with_context(&view, &pending, &context);
            if matches!(pending, Pending::Priority { .. })
                && action == PlayerAction::PassPriority
                && self.engine.state().zones.stack_is_empty()
            {
                return false;
            }
            self.apply(action);
        }
        panic!("the house neither cast nor passed");
    }

    /// Seat 0's life.
    pub(crate) fn life(&self) -> i32 {
        self.view().seats[0].life
    }
}

/// Answers seat 0's questions with `answer` and everyone's others with their
/// timeout answers until the stack is empty and seat 0 has priority.
pub(crate) fn resolve(
    table: &mut Table,
    answer: &dyn Fn(&Table, &Pending) -> Option<PlayerAction>,
) {
    for _ in 0..60 {
        if table.engine.state().zones.stack_is_empty()
            && matches!(table.engine.pending(), Pending::Priority { player, .. } if *player == ME)
        {
            return;
        }
        let pending = table.engine.pending().clone();
        let action = answer(table, &pending)
            .or_else(|| timeout_answer(&pending))
            .expect("a default answer");
        table.apply(action);
    }
    panic!("the stack never emptied");
}
