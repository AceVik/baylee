//! Gateway-hosted Tower expeditions; only the authority changes progression.
#[allow(clippy::wildcard_imports)]
use super::*;
#[cfg(any(not(target_arch = "wasm32"), test))]
use baylee_client_core::echoes::Work;
use baylee_client_core::echoes::{Read, State, api, domain};
use domain::v2::{inventory::Slot, run::Command};
#[cfg(any(not(target_arch = "wasm32"), test))]
use std::collections::VecDeque;
use uuid::Uuid;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod builder;
mod choices;
mod collection;
mod draw;
mod feedback;
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
    Collection,
    Ranking,
}

// Drag-and-drop and right-click quick-equip gestures run in `gestures` on desktop.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Open,
    Back,
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
    #[cfg(test)]
    ZoneCard(
        baylee_core::ids::CardIndex,
        baylee_client_core::echoes::draft::DeckZone,
        bool,
    ),
    ClosePreview,
    InspectCard(baylee_core::ids::CardIndex),
    InspectStarter(Uuid, u8),
    PurchaseEquipped(u32),
    Tiers(domain::v2::catalog::Family),
    SaveDeck,
    ResetDeck,
    SellCard(baylee_core::ids::CardIndex),
    ToggleSpareCards,
    Reward(baylee_core::ids::CardIndex),
    ClaimReward,
    ToggleRewardGold,
    ToggleRewardDeck,
    Freeze,
    Withdraw,
    Badge(baylee_client_core::echoes::awards_api::BadgeKind),
    Listing(bool),
}

// Constructed by `native` and `Ui::command`; browser Echoes is coming soon.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(super) enum Confirmation {
    Listing(u64),
    /// Enrolment shares the player's builds as echoes: the explanation of that
    /// revision is shown and confirmed before the start request carries it.
    Enrol {
        cohort: Uuid,
        starter: u8,
        consent: u16,
    },
    Withdrawal,
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
#[allow(clippy::struct_excessive_bools)] // Independent panel expansion toggles across shop & reward views.
pub(crate) struct Ui {
    #[cfg(not(target_arch = "wasm32"))]
    editor: Option<builder::Session>,
    model: State,
    pub(super) opened: bool,
    page: Page,
    pub(super) confirmation: Option<Confirmation>,
    /// One shop press made while a read held the worker, for this run, sent once
    /// the read ends. While it waits the shop draws its controls held.
    queued: Option<(Uuid, Command)>,
    choices: choices::Draft,
    bag: Bag,
    spare_cards_open: bool,
    reward_gold_open: bool,
    reward_deck_open: bool,
    pub(super) preview: Option<baylee_core::ids::CardIndex>,
    pub(super) starter_preview: Option<(Uuid, u8)>,
    tiers: Option<domain::v2::catalog::Family>,
    collection: collection::Collection,
    ranking: Option<(Uuid, api::ranking::Lane)>,
    #[cfg(any(not(target_arch = "wasm32"), test))]
    reads: VecDeque<Read>,
    loaded: bool,
    withdrawn: bool,
    playing: Option<(Uuid, Uuid)>,
    /// An authenticated gateway seat may arrive before the first Central worker.
    returning: Option<Uuid>,
    #[cfg(not(target_arch = "wasm32"))]
    sounds: Vec<crate::sound::tower::TowerSound>,
    #[cfg(not(target_arch = "wasm32"))]
    session: Option<central::Handle>,
    #[cfg(not(target_arch = "wasm32"))]
    worker: native::Worker,
}

pub(super) fn install(app: &mut App) {
    crate::towerprints::install(app);
    crate::itemart::install(app);
    crate::towerui::install(app);
    gestures::install(app);
    feedback::install(app);
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.add_systems(
            Update,
            (native::tick, collection::poll)
                .chain()
                .after(poll)
                .after(super::scrollbars::remember)
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
        let confirming = cx.state.echoes.confirmation.is_some();
        native::ask(cx.state, action);
        if (!confirming && cx.state.echoes.confirmation.is_some())
            || matches!(
                action,
                Action::Open
                    | Action::Back
                    | Action::Page(_)
                    | Action::GoShop
                    | Action::Read(Read::Run(_) | Read::Ranking(..))
            )
        {
            cx.scrolled.set(List::Table, 0.0);
            cx.scrolled.set(List::EchoesInspector, 0.0);
        }
        if matches!(
            action,
            Action::Offer(_) | Action::InspectCard(_) | Action::InspectStarter(..)
        ) {
            cx.scrolled.set(List::EchoesInspector, 0.0);
        }
        if matches!(action, Action::Select(_))
            && cx
                .state
                .echoes
                .model
                .run()
                .is_some_and(|run| !domain::v2::generation::is_run(run.shop.generator))
        {
            cx.scrolled.set(List::Table, 0.0);
            cx.scrolled.set(List::EchoesInspector, 0.0);
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (action, cx);
}

impl Ui {
    pub(crate) const fn game(&self) -> Option<Uuid> {
        match self.playing {
            Some((_, game)) => Some(game),
            None => None,
        }
    }
    /// Retry an unacknowledged refit even when later local edits are incomplete.
    // `self.editor` is desktop-only; method form keeps callers uniform across targets.
    #[cfg_attr(target_arch = "wasm32", allow(clippy::unused_self))]
    pub(crate) fn refit_uncertain(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        return self.editor.is_some() && self.model.uncertain();
        #[cfg(target_arch = "wasm32")]
        false
    }

    pub(super) fn rewarding(&self) -> bool {
        self.opened
            && self.page == Page::Shop
            && self
                .model
                .run()
                .is_some_and(|run| run.phase == api::Phase::Reward)
    }

    pub(in crate::lobby) fn inspecting(&self) -> bool {
        if !self.opened || matches!(self.page, Page::Collection | Page::Ranking) {
            return false;
        }
        let on_landing = self.model.run().is_none() || self.page == Page::Overview;
        let starter_drawn = on_landing
            && self.starter_preview.is_some_and(|(cohort, index)| {
                self.model
                    .cohorts()
                    .iter()
                    .find(|entry| entry.id == cohort)
                    .is_some_and(|entry| usize::from(index) < entry.starters.len())
            });
        let deck_drawn = self.page == Page::Deck
            && self.model.publication().is_some()
            && self.model.run().is_some_and(|run| {
                !matches!(
                    run.phase,
                    api::Phase::Reward
                        | api::Phase::Conquered
                        | api::Phase::Exhausted
                        | api::Phase::Abandoned
                        | api::Phase::Interrupted
                ) && !domain::v2::generation::is_run(run.shop.generator)
            });
        starter_drawn || (self.preview.is_some() && (on_landing || deck_drawn))
    }

    pub(super) fn close(&mut self) {
        self.opened = false;
        self.preview = None;
        self.starter_preview = None;
        self.spare_cards_open = false;
        self.reward_gold_open = false;
        self.reward_deck_open = false;
        self.returning = None;
        // Leaving the Tower withdraws a press still waiting for its read.
        self.queued = None;
        if self.model.run().is_some_and(|run| {
            matches!(
                run.phase,
                api::Phase::Conquered
                    | api::Phase::Exhausted
                    | api::Phase::Abandoned
                    | api::Phase::Interrupted
            )
        }) {
            self.page = Page::Overview;
        }
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    fn refresh(&mut self) {
        self.reads.clear();
        if self.page == Page::Ranking
            && let Some((cohort, lane)) = self.ranking
        {
            if !self.loaded {
                self.reads.push_back(Read::Cohorts(None));
            }
            self.reads.push_back(Read::Ranking(cohort, lane));
            return;
        }
        self.reads.extend([Read::Cohorts(None), Read::Runs(None)]);
        if let Some(run) = self.model.run() {
            self.reads.push_back(Read::Run(run.id));
        }
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    fn follow_run(&mut self) {
        let Some(run) = self.model.run() else {
            return;
        };
        if self.model.publication().is_none() {
            self.reads.push_back(Read::Cohort(run.cohort));
        }
        match run.phase {
            api::Phase::Shop if self.model.hosts().is_empty() => {
                self.reads.push_back(Read::Hosts(None));
            }
            api::Phase::Reserved | api::Phase::Playing if self.model.location().is_none() => {
                self.reads.push_back(Read::Locate);
            }
            _ => {}
        }
        self.choices.bind(run);
    }

    /// A mutation is out, or a press already waits for a read to end: further
    /// state-changing controls are drawn (and answered) as unavailable.
    pub(super) fn holding(&self) -> bool {
        self.model.mutating() || self.queued.is_some()
    }

    /// Hand the press that waited for a read to the now idle worker.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn release(&mut self) -> Option<Work> {
        if self.model.busy() {
            return None;
        }
        let (run, command) = self.queued.take()?;
        let current = self.model.run()?;
        let cohort = self.model.publication()?;
        (current.id == run && baylee_client_core::echoes::draft::allowed(current, cohort, &command))
            .then(|| self.command(command))
            .flatten()
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub(super) fn command(&mut self, command: Command) -> Option<Work> {
        if let Some(run) = self.model.run()
            && (matches!(run.phase, api::Phase::Shop | api::Phase::Reward)
                || (matches!(command, Command::Abandon {})
                    && matches!(run.phase, api::Phase::Reserved | api::Phase::Quarantined)))
            && !self.model.uncertain()
        {
            if domain::v2::generation::is_run(run.shop.generator)
                && !matches!(
                    command,
                    Command::Abandon {} | Command::Sell { .. } | Command::Fuse { .. }
                )
            {
                // A passive read holds the one worker: keep the press and send
                // it when the read is done, instead of dropping it silently.
                if self.model.busy() {
                    self.queued = Some((run.id, command));
                    return None;
                }
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
