//! Gateway-hosted Tower expeditions; only the authority changes progression.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::echoes::{Read, State, Work, api, domain};
use domain::v2::{inventory::Slot, run::Command};
use std::collections::VecDeque;
use uuid::Uuid;

mod choices;
mod draw;
mod gestures;
pub(crate) mod keys;
#[cfg(not(target_arch = "wasm32"))]
pub(super) mod native;
pub(super) use draw::{draw, entry};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Page {
    Overview,
    #[default]
    Shop,
    Equipment,
    Deck,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Open,
    Back,
    Overview,
    Refresh,
    Read(Read),
    Starter(Uuid, u8),
    Accept,
    Cancel,
    Retry,
    Play(Uuid),
    Resume,
    Page(Page),
    GoShop,
    Offer(u32),
    Purchase(u32),
    QuickEquip(u32),
    Transfer(u32, Slot),
    Upgrade(u32, u32),
    InventoryPage(usize),
    InventoryOrder(baylee_client_core::echoes::workshop::Order),
    InventoryGroups(bool),
    Place(Slot),
    Ingredient(u32),
    Material(u32),
    Buy,
    Pin(u32),
    Reroll,
    Select(u32),
    Move,
    Sell,
    Fuse,
    Mythic(u8),
    Abandon,
    Card(baylee_core::ids::CardIndex, bool),
    ZoneCard(
        baylee_core::ids::CardIndex,
        baylee_client_core::echoes::draft::DeckZone,
        bool,
    ),
    Commander(baylee_core::ids::CardIndex),
    DeckZone(baylee_client_core::echoes::draft::DeckZone),
    DeckPage(usize),
    ClosePreview,
    InspectCard(baylee_core::ids::CardIndex),
    Tiers(domain::v2::catalog::Family),
    SaveDeck,
    ResetDeck,
    Reward(baylee_core::ids::CardIndex),
    ClaimReward,
    Freeze,
    Withdraw,
}

pub(super) enum Confirmation {
    Withdrawal,
    Enrollment {
        cohort: Uuid,
        starter: u8,
        consent: u16,
    },
    Choice {
        run: Uuid,
        revision: u64,
        command: Command,
    },
}

#[derive(Default)]
struct Bag {
    page: usize,
    order: baylee_client_core::echoes::workshop::Order,
    grouped: bool,
}

#[derive(Default)]
struct Deck {
    zone: baylee_client_core::echoes::draft::DeckZone,
    page: usize,
}

#[derive(Default)]
pub(crate) struct Ui {
    model: State,
    pub(super) opened: bool,
    page: Page,
    pub(super) confirmation: Option<Confirmation>,
    choices: choices::Draft,
    bag: Bag,
    deck: Deck,
    preview: Option<baylee_core::ids::CardIndex>,
    tiers: Option<domain::v2::catalog::Family>,
    reads: VecDeque<Read>,
    loaded: bool,
    withdrawn: bool,
    playing: Option<(Uuid, Uuid)>,
    /// An authenticated gateway seat may arrive before the first Central worker.
    returning: Option<Uuid>,
    #[cfg(not(target_arch = "wasm32"))]
    session: Option<central::Handle>,
    #[cfg(not(target_arch = "wasm32"))]
    worker: native::Worker,
}

pub(super) fn install(app: &mut App) {
    crate::itemart::install(app);
    crate::towerui::install(app);
    gestures::install(app);
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.add_systems(
            Update,
            native::tick
                .after(poll)
                .before(ui)
                .run_if(in_state(DuelPhase::Closed)),
        );
        app.add_systems(
            OnEnter(DuelPhase::Closed),
            native::returned.after(super::systems::came_back),
        );
    }
    #[cfg(target_arch = "wasm32")]
    let _ = app;
}

pub(super) fn current(state: &LobbyState) -> bool {
    matches!(state.lobby.screen(), Screen::Seated(seat)
        if state.echoes.playing.is_some_and(|(_, game)| game.to_string() == seat.game_id))
}

pub(super) fn press(action: Action, cx: super::press::Cx<'_, '_, '_, '_, '_>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if action == Action::Open {
            cx.state.hub = Hub::Challenges;
            cx.state.ensure_central_lobby(cx.mailbox);
        }
        if matches!(action, Action::Play(_) | Action::Resume) && cx.state.lobby.awaiting().is_some()
        {
            cx.state
                .lobby
                .tell_refusal(Phrase::ChallengeLeaveTable, &[]);
            return;
        }
        native::ask(cx.state, action);
        if matches!(
            action,
            Action::Open
                | Action::Back
                | Action::Overview
                | Action::Page(_)
                | Action::GoShop
                | Action::Read(Read::Run(_))
                | Action::Offer(_)
                | Action::InspectCard(_)
        ) {
            cx.scrolled.set(List::Table, 0.0);
            cx.scrolled.set(List::EchoesInspector, 0.0);
        }
        if matches!(action, Action::Select(_))
            && cx
                .state
                .echoes
                .model
                .run()
                .is_some_and(|run| run.shop.generator != domain::v2::generation::RUN)
        {
            cx.scrolled.set(List::Table, 0.0);
            cx.scrolled.set(List::EchoesInspector, 0.0);
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (action, cx);
}

impl Ui {
    pub(super) fn close(&mut self) {
        self.opened = false;
        self.returning = None;
    }

    fn refresh(&mut self) {
        self.reads.clear();
        self.reads.extend([Read::Cohorts(None), Read::Runs(None)]);
        if let Some(run) = self.model.run() {
            self.reads.push_back(Read::Run(run.id));
        }
    }

    fn follow_run(&mut self) {
        let Some(run) = self.model.run() else {
            return;
        };
        if self.model.publication().is_none() {
            self.reads.push_back(Read::Cohort(run.cohort));
        }
        match run.phase {
            api::Phase::Shop => self.reads.push_back(Read::Hosts(None)),
            api::Phase::Reserved | api::Phase::Playing => self.reads.push_back(Read::Locate),
            _ => {}
        }
        self.choices.bind(run);
    }

    fn command(&mut self, command: Command) -> Option<Work> {
        if let Some(run) = self.model.run()
            && matches!(run.phase, api::Phase::Shop | api::Phase::Reward)
            && !self.model.uncertain()
        {
            if run.shop.generator == domain::v2::generation::RUN
                && !matches!(
                    command,
                    Command::Abandon {} | Command::Sell { .. } | Command::Fuse { .. }
                )
            {
                return self.model.choose(Uuid::now_v7(), command);
            }
            self.confirmation = Some(Confirmation::Choice {
                run: run.id,
                revision: run.revision,
                command,
            });
        }
        None
    }
}
