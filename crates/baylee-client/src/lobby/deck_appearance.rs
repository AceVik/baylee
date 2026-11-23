//! The private library's appearance picker. An earned reward can open this sheet
//! over the calendar or the owner can change either slot from the deck builder.
use super::{
    LobbyState,
    advent::{Book, LoadReward, RefreshInventory},
    central,
};
use crate::{
    shellkit::focus::{Activated, TabOrder},
    ui_button::Clicks,
};
use baylee_account::{
    Error,
    advent::{Id, Kind},
};
use bevy::prelude::*;
use std::sync::{
    Mutex,
    mpsc::{Receiver, TryRecvError},
};
use uuid::Uuid;
mod draw;
mod worker;
use worker::{Deck, Job, Reply};

pub(crate) const ORDER: TabOrder = TabOrder {
    name: "deck-appearance",
    stops: &[
        "close", "sleeve", "mat", "default", "item", "previous", "next", "apply", "retry",
    ],
    modal: true,
};

#[derive(Message)]
pub(crate) struct EquipReward {
    pub item: Id,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct OpenEditor;

#[derive(Component, Clone, Copy)]
enum Action {
    Close,
    Deck(Uuid),
    Slot(Kind),
    Select(Option<Id>),
    Apply,
    Retry,
    RetryImage,
    Page(usize),
}

#[derive(Clone, Copy)]
enum Origin {
    Editor(Uuid),
    Reward(Id, Kind),
}

#[derive(Resource, Default)]
pub(crate) struct Page {
    origin: Option<Origin>,
    owner: Option<central::Handle>,
    decks: Vec<(Uuid, String)>,
    deck: Option<Deck>,
    reading: Option<Uuid>,
    kind: Option<Kind>,
    selected: Option<Id>,
    pending: Option<Mutex<Receiver<Result<Reply, Error>>>>,
    problem: Option<Error>,
    page: usize,
    saved: bool,
    escape_held: bool,
}

impl Page {
    pub(crate) fn holds_keys(&self) -> bool {
        self.origin.is_some() || self.escape_held
    }
    fn request(&mut self, job: Job) {
        self.problem = None;
        self.saved = false;
        if let Some(owner) = &self.owner {
            match worker::submit(owner, job) {
                Ok(reply) => self.pending = Some(Mutex::new(reply)),
                Err(e) => self.problem = Some(e),
            }
        }
    }

    fn read(&mut self, id: Uuid) {
        self.reading = Some(id);
        self.deck = None;
        self.page = 0;
        self.request(Job::Read(id));
    }

    fn apply(&mut self, book: &Book) {
        let (Some(deck), Some(kind)) = (&self.deck, self.kind) else {
            return;
        };
        let current = worker::slot(deck.look, kind);
        if current == self.selected {
            return;
        }
        let selected = self.selected.and_then(|id| {
            book.inventory()
                .iter()
                .find(|r| r.id == id && r.kind == kind)
                .map(|r| (id, r.asset))
        });
        if selected.is_none() && self.selected.is_some() {
            return;
        }
        self.request(Job::Apply {
            deck: deck.id,
            kind,
            previous: current,
            selected,
        });
    }

    fn open(&mut self, owner: central::Handle, origin: Origin) {
        *self = Self {
            owner: Some(owner),
            origin: Some(origin),
            ..Self::default()
        };
        match origin {
            Origin::Editor(id) => self.read(id),
            Origin::Reward(..) => self.request(Job::List),
        }
    }

    fn loaded(&mut self, deck: Deck, saved: bool, images: &mut MessageWriter<LoadReward>) {
        let (kind, selected) = match self.origin {
            Some(Origin::Reward(id, kind)) => (kind, Some(id)),
            _ => {
                let kind = self.kind.unwrap_or(Kind::Sleeve);
                (kind, worker::slot(deck.look, kind))
            }
        };
        self.kind = Some(kind);
        self.selected = selected;
        self.deck = Some(deck);
        self.saved = saved;
        if let Some(id) = selected {
            images.write(LoadReward(id));
        }
    }
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<Page>()
        .add_message::<EquipReward>()
        .add_systems(Update, (open, act, poll, draw::render).chain());
}

#[allow(clippy::too_many_arguments)] // Bevy provides keyboard, pointer, worker and inventory separately.
fn open(
    mut page: ResMut<Page>,
    lobby: Res<LobbyState>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    book: Res<Book>,
    mut requests: MessageReader<EquipReward>,
    mut clicks: Clicks<OpenEditor>,
    mut keys: MessageReader<Activated>,
    openers: Query<&OpenEditor>,
    mut refresh: MessageWriter<RefreshInventory>,
) {
    let reward = requests.read().last().map(|m| m.item);
    let editor =
        !clicks.read().is_empty() || keys.read().any(|a| a.by_key && openers.contains(a.entity));
    if !editor && reward.is_none() && page.origin.is_none() {
        return;
    }
    if phase
        .as_ref()
        .is_some_and(|s| *s.get() != crate::DuelPhase::Closed)
    {
        if page.origin.is_some() {
            *page = Page::default();
        }
        return;
    }
    if let Some(Origin::Editor(id)) = page.origin
        && (!matches!(
            lobby.lobby.screen(),
            baylee_client_core::lobby::Screen::Build
        ) || lobby
            .lobby
            .builder()
            .editing()
            .and_then(|v| Uuid::parse_str(v).ok())
            != Some(id))
    {
        *page = Page::default();
    }
    let handle = lobby.central_handle().filter(central::Handle::is_active);
    if page
        .owner
        .as_ref()
        .is_some_and(|old| handle.as_ref().is_none_or(|now| !now.same_session(old)))
    {
        *page = Page::default();
    }
    let Some(handle) = handle else {
        return;
    };
    let origin = reward
        .filter(|_| book.belongs_to(&handle))
        .and_then(|id| {
            book.inventory()
                .iter()
                .find(|r| r.id == id)
                .map(|r| Origin::Reward(id, r.kind))
        })
        .or_else(|| {
            let deck = lobby.lobby.builder();
            (editor
                && matches!(
                    lobby.lobby.screen(),
                    baylee_client_core::lobby::Screen::Build
                )
                && deck.limited().is_none())
            .then(|| deck.editing().and_then(|id| Uuid::parse_str(id).ok()))
            .flatten()
            .map(Origin::Editor)
        });
    if let Some(origin) = origin {
        page.open(handle, origin);
        if !book.inventory_loaded() {
            refresh.write(RefreshInventory);
        }
    }
}

fn poll(mut page: ResMut<Page>, mut images: MessageWriter<LoadReward>) {
    let Some(recv) = &page.pending else {
        return;
    };
    let answer = match recv.lock().map(|r| r.try_recv()) {
        Ok(Err(TryRecvError::Empty)) => return,
        Ok(Ok(r)) => r,
        _ => Err(Error::Unavailable),
    };
    page.pending = None;
    match answer {
        Ok(Reply::Decks(decks)) => page.decks = decks,
        Ok(Reply::Loaded(deck)) => page.loaded(deck, false, &mut images),
        Ok(Reply::Saved(deck)) => page.loaded(deck, true, &mut images),
        Err(e) => page.problem = Some(e),
    }
}

#[allow(clippy::too_many_arguments)] // The modal shares ownership and image loading with the inventory.
fn act(
    mut page: ResMut<Page>,
    book: Res<Book>,
    mut clicks: Clicks<Action>,
    mut keys: MessageReader<Activated>,
    actions: Query<&Action>,
    codes: Option<Res<ButtonInput<KeyCode>>>,
    mut images: MessageWriter<LoadReward>,
    mut refresh: MessageWriter<RefreshInventory>,
) {
    if page.origin.is_none() {
        keys.clear();
        if page.escape_held && !codes.as_ref().is_some_and(|k| k.pressed(KeyCode::Escape)) {
            page.escape_held = false;
        }
        return;
    }
    let presses = clicks.read().into_iter().chain(
        keys.read()
            .filter(|a| a.by_key)
            .filter_map(|a| actions.get(a.entity).ok().copied()),
    );
    for action in presses {
        if matches!(action, Action::Close) {
            *page = Page::default();
            return;
        }
        if page.pending.is_some()
            || page
                .owner
                .as_ref()
                .is_none_or(|owner| !book.belongs_to(owner))
        {
            continue;
        }
        match action {
            Action::Deck(id) if page.decks.iter().any(|(d, _)| *d == id) => page.read(id),
            Action::Slot(kind) if matches!(page.origin, Some(Origin::Editor(_))) => {
                page.kind = Some(kind);
                page.page = 0;
                page.saved = false;
                page.problem = None;
                page.selected = page.deck.as_ref().and_then(|d| worker::slot(d.look, kind));
                if let Some(id) = page.selected {
                    images.write(LoadReward(id));
                }
            }
            Action::Select(selected)
                if matches!(page.origin, Some(Origin::Editor(_)))
                    && selected.is_none_or(|id| {
                        book.inventory()
                            .iter()
                            .any(|r| r.id == id && Some(r.kind) == page.kind)
                    }) =>
            {
                page.saved = false;
                page.problem = None;
                page.selected = selected;
                if let Some(id) = selected {
                    images.write(LoadReward(id));
                }
            }
            Action::Page(n) => page.page = n,
            Action::Retry => {
                refresh.write(RefreshInventory);
                if let Some(id) = page.reading {
                    page.read(id);
                } else {
                    page.request(Job::List);
                }
            }
            Action::RetryImage => {
                if let Some(id) = page.selected {
                    images.write(LoadReward(id));
                }
            }
            Action::Apply => page.apply(&book),
            _ => {}
        }
    }
    if codes.is_some_and(|k| k.just_pressed(KeyCode::Escape)) {
        *page = Page {
            escape_held: true,
            ..Page::default()
        };
    }
}
