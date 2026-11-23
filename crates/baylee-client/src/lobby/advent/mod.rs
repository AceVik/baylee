//! Central-authorized seasonal doors, a private gallery and scoped play assets.
use super::{LobbyState, central::Handle as AccountHandle};
use baylee_account::{
    Error,
    advent::{
        self as api, AssetMeta, AssetScope, DoorState, Id,
        collection::Item,
        puzzle::{Line, Move, Puzzle},
    },
};
use bevy::prelude::*;
use std::{
    collections::VecDeque,
    sync::{Mutex, mpsc},
    time::{Duration, Instant},
};
mod assets;
mod calendar;
pub(in crate::lobby) mod collection;
mod draw;
mod puzzle;
mod seasonal;
pub(crate) use draw::ORDER;
pub(super) use draw::entry;
#[cfg(feature = "dev-control")]
pub(crate) use seasonal::ShaderProbe;

/// An owned item looked up for a preview.
#[derive(Message)]
pub(crate) struct LoadReward(pub Id);
/// Explicit, previously authorized scoped metadata requested by the table renderer.
#[derive(Message)]
pub(crate) struct LoadScoped {
    pub scope: AssetScope,
    pub asset: AssetMeta,
}
/// A previously authenticated private casual-table transfer.
#[derive(Message)]
pub(crate) struct DecodeCasual {
    pub scope: AssetScope,
    pub asset: AssetMeta,
    pub bytes: Vec<u8>,
}
#[derive(Message)]
pub(crate) struct ForgetMatch(pub Id);
#[derive(Message)]
pub(crate) struct RefreshInventory;
#[derive(Message)]
pub(crate) struct ShowReward(pub Id);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Closed,
    Calendar,
    Door(u8),
    Gallery,
    Item(Id),
}
#[derive(Clone, Copy, Component)]
#[require(crate::ui_button::UiButton)]
enum Action {
    Open,
    Close,
    Calendar,
    Door(u8),
    Gallery,
    Item(Id),
    Choose(Move),
    Hint(u8),
    Reset,
    Submit,
    Claim,
    Deck,
    Prev,
    Next,
    Retry,
}

#[derive(Resource)]
pub(crate) struct Book {
    owner: Option<AccountHandle>,
    calendar: Option<Calendar>,
    refresh: Instant,
    screen: Screen,
    just_closed: bool,
    puzzle: Option<Challenge>,
    revision: u64,
    inventory: Vec<Item>,
    inventory_sync: Vec<Item>,
    inventory_revision: u64,
    inventory_refresh_due: bool,
    inventory_loaded: bool,
    inventory_error: Option<Error>,
    inventory_retry: Option<Instant>,
    recovery: Vec<(u16, u8)>,
    inventory_page: usize,
    work: VecDeque<api::Request>,
    pending: Option<ResponseJob>,
    error: Option<Error>,
    media: assets::Library,
}
struct ResponseJob {
    request: api::Request,
    rx: Mutex<mpsc::Receiver<Result<api::Response, Error>>>,
}
struct Calendar {
    year: u16,
    opens: i64,
    closes: i64,
    as_of: i64,
    observed: Instant,
    doors: Vec<api::Door>,
    hero: Option<AssetMeta>,
}
struct Challenge {
    door: u8,
    puzzle: Puzzle,
    line: Line,
    hints: [Option<api::Text>; 2],
    submitted: Option<bool>,
}
impl Default for Book {
    fn default() -> Self {
        Self {
            owner: None,
            calendar: None,
            refresh: Instant::now(),
            screen: Screen::Closed,
            just_closed: false,
            puzzle: None,
            revision: 0,
            inventory: Vec::new(),
            inventory_sync: Vec::new(),
            inventory_revision: 0,
            inventory_refresh_due: false,
            inventory_loaded: false,
            inventory_error: None,
            inventory_retry: None,
            recovery: Vec::new(),
            inventory_page: 0,
            work: VecDeque::new(),
            pending: None,
            error: None,
            media: assets::Library::default(),
        }
    }
}
impl Book {
    pub(crate) fn belongs_to(&self, handle: &AccountHandle) -> bool {
        handle.is_active()
            && self
                .owner
                .as_ref()
                .is_some_and(|owner| owner.same_session(handle))
    }
    pub(crate) fn inventory(&self) -> &[Item] {
        &self.inventory
    }
    pub(super) fn inventory_revision(&self) -> u64 {
        self.inventory_revision
    }
    pub(crate) fn inventory_loaded(&self) -> bool {
        self.inventory_loaded
    }
    pub(crate) fn inventory_error(&self) -> Option<Error> {
        self.inventory_error
    }
    pub(crate) fn image_error(&self, item: Id) -> Option<Error> {
        self.media.error(AssetScope::Owned { item })
    }
    pub(crate) fn image(&self, item: Id) -> Option<&Handle<Image>> {
        let meta = self.inventory.iter().find(|r| r.id == item)?.asset;
        self.scoped_image(&AssetScope::Owned { item }, &meta)
    }
    pub(crate) fn scoped_image(
        &self,
        scope: &AssetScope,
        asset: &AssetMeta,
    ) -> Option<&Handle<Image>> {
        self.media.image(scope, asset)
    }
    pub(crate) fn visible(&self) -> bool {
        self.screen != Screen::Closed
    }
    pub(crate) fn holds_keys(&self) -> bool {
        self.visible() || self.just_closed
    }
    fn touch(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
    fn push(&mut self, request: api::Request) {
        if self.work.len() < 12 {
            self.work.push_back(request);
        }
    }
    fn year(&self) -> u16 {
        self.calendar.as_ref().map_or(0, |c| c.year)
    }
    fn now(calendar: &Calendar) -> i64 {
        calendar.as_of.saturating_add(
            i64::try_from(calendar.observed.elapsed().as_secs()).unwrap_or(i64::MAX),
        )
    }
    fn released(&self) -> bool {
        self.calendar
            .as_ref()
            .is_some_and(|c| (c.opens..c.closes).contains(&Self::now(c)))
    }
    fn unsolved(&self) -> bool {
        self.released()
            && self.calendar.as_ref().is_some_and(|c| {
                c.doors
                    .iter()
                    .any(|d| matches!(d.state, DoorState::Open | DoorState::Solved))
            })
    }
    fn door(&self, number: u8) -> Option<&api::Door> {
        self.calendar
            .as_ref()?
            .doors
            .iter()
            .find(|d| d.door == number)
    }
    fn request_inventory(&mut self) {
        if self
            .pending
            .as_ref()
            .is_some_and(|p| matches!(p.request, api::Request::Collection { .. }))
            || self
                .work
                .iter()
                .any(|r| matches!(r, api::Request::Collection { .. }))
        {
            return;
        }
        self.push(api::Request::Collection { after: None });
    }

    fn invalidate_inventory(&mut self) {
        // A rank may settle while an older collection page is in flight.
        self.inventory_refresh_due = self
            .pending
            .as_ref()
            .is_some_and(|job| matches!(job.request, api::Request::Collection { .. }))
            || self
                .work
                .iter()
                .any(|r| matches!(r, api::Request::Collection { after: Some(_) }));
        self.request_inventory();
    }
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<Book>()
        .add_message::<LoadReward>()
        .add_message::<LoadScoped>()
        .add_message::<DecodeCasual>()
        .add_message::<ForgetMatch>()
        .add_message::<RefreshInventory>()
        .add_message::<ShowReward>();
    seasonal::install(app);
    app.add_systems(Update, (tick, assets::poll).chain().after(super::poll));
    app.add_systems(
        Update,
        (draw::actions, draw::render, draw::entries)
            .chain()
            .after(assets::poll)
            .run_if(resource_exists::<crate::hud::UiFonts>)
            .run_if(in_state(crate::DuelPhase::Closed)),
    );
}

fn tick(
    mut book: ResMut<Book>,
    lobby: Res<LobbyState>,
    phase: Option<Res<State<crate::DuelPhase>>>,
    mut refresh: MessageReader<RefreshInventory>,
    mut show: MessageReader<ShowReward>,
    entries: Query<(), With<draw::Entry>>,
) {
    let handle = lobby.central_handle().filter(|h| {
        h.is_active() && lobby.has_central_account() && !super::central::terms_pending(&lobby)
    });
    let change = match (&book.owner, &handle) {
        (Some(a), Some(b)) => !a.same_session(b),
        (None, None) => false,
        _ => true,
    };
    if change {
        *book = Book {
            owner: handle,
            ..Default::default()
        };
    }
    let Some(owner) = book.owner.clone() else {
        return;
    };
    let starting = book.revision;
    let b = book.bypass_change_detection();
    if refresh.read().count() > 0 {
        b.invalidate_inventory();
    }
    if b.inventory_retry.is_some_and(|when| Instant::now() >= when) {
        b.inventory_retry = None;
        b.request_inventory();
    }
    for event in show.read() {
        b.screen = Screen::Item(event.0);
        b.request_inventory();
        b.touch();
    }
    if b.visible()
        && phase
            .as_ref()
            .is_some_and(|p| *p.get() != crate::DuelPhase::Closed)
    {
        b.screen = Screen::Closed;
        b.touch();
    }
    if let Some(job) = b.pending.as_mut() {
        let response = match job.rx.get_mut().ok().map(|rx| rx.try_recv()) {
            Some(Ok(reply)) => Some(reply),
            Some(Err(mpsc::TryRecvError::Empty)) => None,
            _ => Some(Err(Error::Unavailable)),
        };
        if let Some(response) = response
            && let Some(job) = b.pending.take()
        {
            b.accept(job.request, response);
        }
    }
    if (!entries.is_empty() || b.visible()) && Instant::now() >= b.refresh {
        b.refresh = Instant::now() + Duration::from_secs(60);
        b.push(api::Request::CurrentCalendar {});
        if !b.inventory_loaded && b.inventory_retry.is_none() {
            b.request_inventory();
        }
    }
    if b.pending.is_none()
        && let Some(request) = b.work.pop_front()
    {
        let (tx, rx) = mpsc::sync_channel(1);
        let call = request.clone();
        if owner.submit(move |account| {
            let _ = tx.send(account.advent(call));
        }) {
            b.pending = Some(ResponseJob {
                request,
                rx: Mutex::new(rx),
            });
        } else {
            b.accept(request, Err(Error::Busy));
        }
    }
    if b.revision != starting {
        book.set_changed();
    }
}
