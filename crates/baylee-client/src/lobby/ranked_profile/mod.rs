//! A player's public competitive record and the owner's confirmed trophy collection.
use super::{LobbyState, central};
use crate::DuelPhase;
use crate::rank_art::RankArt;
use crate::shellkit::avatar::Avatar;
use crate::shellkit::focus::{Activated, Stop, TabOrder};
use crate::ui_button::Clicks;
use baylee_account::ranked::{Format, Id, Request, Standing, Track};
use bevy::prelude::*;
use bevy::ui::CalculatedClip;
use std::collections::BTreeMap;
use std::sync::{Mutex, mpsc};
mod awards;
mod draw;
mod model;
#[cfg(test)]
mod tests;
use awards::Collection;
use model::Profile;
pub(crate) use model::valid_handle;

pub(crate) const ORDER: TabOrder = TabOrder {
    name: "ranked-profile",
    stops: &["close", "refresh"],
    modal: true,
};

/// Open a published record by its full Central handle or leaderboard account ID.
#[derive(Component, Clone)]
pub(crate) struct OpenProfile(pub String);

impl OpenProfile {
    pub(crate) fn account(id: Id) -> Self {
        Self(model::account_key(id))
    }
}

/// The localized anonymous portrait wears the frame belonging to its public ID.
#[derive(Component)]
struct PortraitKey(String);

#[derive(Component)]
pub(crate) struct Guest;

/// A board row already owns the authoritative rank and frame for its format.
#[derive(Component)]
pub(crate) struct BoardPortrait;

type NeedsProfile = (Without<Guest>, Without<BoardPortrait>);

/// Concise per-format standings on the social card that opens over every linked handle.
#[derive(Component)]
pub(crate) struct CompactRanks(pub String);

#[derive(Message)]
pub(crate) struct RefreshOwn;

#[derive(Component, Clone, Copy)]
enum Action {
    Close,
    Refresh,
}

#[derive(Resource, Default)]
pub(crate) struct Page {
    handle: Option<String>,
    refresh: bool,
    just_closed: bool,
}

impl Page {
    pub(crate) fn open(&mut self, handle: &str) {
        if model::valid_handle(handle) || model::account_id(handle).is_some() {
            self.handle = Some(handle.to_owned());
            self.refresh = true;
        }
    }

    pub(crate) fn visible(&self) -> bool {
        self.handle.is_some()
    }

    /// The authenticated owner can also inspect their private medals before
    /// choosing a public name and discriminator.
    pub(super) fn open_own(&mut self, lobby: &LobbyState) {
        let Some(central) = lobby.central.as_ref() else {
            return;
        };
        let Some(worker) = central.handle.as_ref().filter(|h| h.is_active()) else {
            return;
        };
        if let Some(key) = central
            .profile_handle
            .clone()
            .or_else(|| worker.public_id.map(model::account_key))
        {
            self.handle = Some(key);
            self.refresh = true;
        }
    }
    pub(crate) fn holds_keys(&self) -> bool {
        self.visible() || self.just_closed
    }
}

#[derive(Default)]
struct Entry {
    profile: Option<Profile>,
    retry_at: f64,
    failed: bool,
    version: u64,
}

type ReadResult = Result<Profile, baylee_account::Error>;
type Inbox<T> = Mutex<mpsc::Receiver<T>>;

// The receiver has just one owner. A stopped worker releases its sender,
// turning a stalled read into a recoverable failure without blocking a frame.
fn delivered<T>(rx: &mut Inbox<T>, disconnected: impl FnOnce() -> T) -> Option<T> {
    match rx
        .get_mut()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .try_recv()
    {
        Ok(reply) => Some(reply),
        Err(mpsc::TryRecvError::Empty) => None,
        Err(mpsc::TryRecvError::Disconnected) => Some(disconnected()),
    }
}

#[derive(Resource, Default)]
pub(crate) struct Book {
    known: BTreeMap<String, Entry>,
    reading: Option<(String, Inbox<ReadResult>)>,
    repeat_read: bool,
    next_poll: f64,
    revision: u64,
    mine: Option<String>,
    owner: Option<central::Handle>,
    owner_key: Option<String>,
    private: Collection,
    owner_read: Option<Inbox<Collection>>,
    private_fresh_at: f64,
}

impl Book {
    pub(crate) fn own_player(&self, format: Format) -> Option<&Standing> {
        self.own_profile()?
            .formats
            .iter()
            .find(|s| s.format == format && s.track == Track::Player)
    }

    fn own_profile(&self) -> Option<&Profile> {
        self.known.get(self.mine.as_ref()?)?.profile.as_ref()
    }

    pub(crate) fn own_loaded(&self) -> bool {
        self.own_profile().is_some()
    }
}

const CAPACITY: usize = 512;
const LIFETIME: f64 = 120.0;
const RETRY: f64 = 30.0;

pub(super) fn install(app: &mut App) {
    app.init_resource::<Book>()
        .init_resource::<Page>()
        .init_resource::<NewDiscs>()
        .add_observer(|added: On<Add<Avatar>>, mut discs: ResMut<NewDiscs>| {
            discs.0.push(added.entity)
        })
        .add_message::<RefreshOwn>()
        .add_systems(PreUpdate, |mut page: ResMut<Page>| {
            if page.just_closed {
                page.just_closed = false;
            }
        })
        .add_systems(Update, refresh_on_receipt.before(read))
        .add_systems(
            Update,
            (act, read, draw::draw)
                .chain()
                .run_if(resource_exists::<crate::hud::UiFonts>),
        )
        .add_systems(OnEnter(DuelPhase::Closed), returned)
        .add_systems(OnEnter(DuelPhase::Opening), |mut page: ResMut<Page>| {
            if page.visible() {
                page.handle = None;
            }
        })
        .add_systems(
            PostUpdate,
            (wear_frames, compact_ranks).before(bevy::ui::UiSystems::Prepare),
        );
}

fn returned(mut book: ResMut<Book>) {
    if book
        .reading
        .as_ref()
        .is_some_and(|(h, _)| Some(h) == book.mine.as_ref())
    {
        book.repeat_read = true;
    }
    if let Some(handle) = book.mine.clone()
        && let Some(entry) = book.known.get_mut(&handle)
    {
        entry.retry_at = 0.0;
    }
    book.next_poll = 0.0;
}

fn refresh_on_receipt(mut receipts: MessageReader<RefreshOwn>, book: ResMut<Book>) {
    if receipts.read().count() > 0 {
        returned(book);
    }
}

fn act(
    mut opens: Clicks<OpenProfile>,
    mut clicks: Clicks<Action>,
    mut keys: MessageReader<Activated>,
    actions: Query<&Action>,
    openers: Query<&OpenProfile>,
    codes: Option<Res<ButtonInput<KeyCode>>>,
    mut page: ResMut<Page>,
) {
    let keyed: Vec<_> = keys.read().filter(|a| a.by_key).map(|a| a.entity).collect();
    for open in opens
        .read()
        .into_iter()
        .chain(keyed.iter().filter_map(|e| openers.get(*e).ok().cloned()))
    {
        page.open(&open.0);
    }
    for action in clicks
        .read()
        .into_iter()
        .chain(keyed.iter().filter_map(|e| actions.get(*e).ok().copied()))
    {
        match action {
            Action::Close => {
                page.handle = None;
                page.just_closed = true;
            }
            Action::Refresh => page.refresh = true,
        }
    }
    if page.visible() && codes.is_some_and(|c| c.just_pressed(KeyCode::Escape)) {
        page.handle = None;
        page.just_closed = true;
    }
}

fn session(book: &mut Book, state: &LobbyState, page: &mut Page) {
    let current = state
        .central
        .as_ref()
        .and_then(|c| c.handle.as_ref())
        .filter(|h| h.is_active());
    let same = match (&book.owner, current) {
        (None, None) => true,
        (Some(a), Some(b)) => a.same_session(b),
        _ => false,
    };
    if !same {
        // Outstanding private jobs only retain the old mailbox.
        let revision = book.revision.wrapping_add(1);
        *book = Book {
            owner: current.cloned(),
            owner_key: current.and_then(|h| h.public_id).map(model::account_key),
            revision,
            ..Book::default()
        };
        page.handle = None;
    }
    let mine = state
        .central
        .as_ref()
        .and_then(|s| s.profile_handle.as_deref())
        .filter(|_| book.owner.is_some())
        .or(book.owner_key.as_deref());
    if book.mine.as_deref() != mine {
        book.mine = mine.map(str::to_owned);
    }
}

type VisibleDiscs<'w, 's> = Query<
    'w,
    's,
    (
        &'static Avatar,
        &'static InheritedVisibility,
        &'static ComputedNode,
        &'static UiGlobalTransform,
        Option<&'static CalculatedClip>,
    ),
    NeedsProfile,
>;

fn read(
    time: Res<Time>,
    state: Res<LobbyState>,
    mut page: ResMut<Page>,
    mut book: ResMut<Book>,
    discs: VisibleDiscs,
) {
    let before = book.revision;
    tick(
        time.elapsed_secs_f64(),
        &state,
        page.bypass_change_detection(),
        book.bypass_change_detection(),
        &discs,
    );
    if before != book.revision {
        book.set_changed();
    }
}

fn tick(now: f64, state: &LobbyState, page: &mut Page, book: &mut Book, discs: &VisibleDiscs) {
    session(book, state, page);
    if page.refresh {
        if let Some(entry) = page.handle.as_ref().and_then(|h| book.known.get_mut(h)) {
            entry.retry_at = 0.0;
        }
        book.private_fresh_at = 0.0;
        book.next_poll = 0.0;
        page.refresh = false;
    }
    receive(book, now);
    if page.handle.is_some() && page.handle == book.mine && !state.central_terms_pending() {
        read_private(book, now);
    }
    if book.reading.is_some() || now < book.next_poll {
        return;
    }
    book.next_poll = now + 0.5;
    let due = |h: &str| book.known.get(h).is_none_or(|e| now >= e.retry_at);
    let owner_id = book.owner.as_ref().and_then(|owner| owner.public_id);
    let is_owner = |key: &str| book.mine.as_deref() == Some(key) && owner_id.is_some();
    let priority = page
        .handle
        .iter()
        .chain(book.mine.iter())
        .map(String::as_str)
        .find(|h| {
            due(h) && (is_owner(h) || model::valid_handle(h) || model::account_id(h).is_some())
        });
    let wanted = priority
        .or_else(|| {
            discs
                .iter()
                .filter_map(|(d, visible, node, transform, clip)| {
                    if !visible.get()
                        || node.size.min_element() <= 0.0
                        || !due(&d.handle)
                        || !model::valid_handle(&d.handle)
                    {
                        return None;
                    }
                    let bounds = Rect::from_center_size(transform.translation, node.size);
                    if clip
                        .and_then(crate::shellkit::clip_bounds)
                        .is_some_and(|c| bounds.intersect(c).is_empty())
                    {
                        return None;
                    }
                    Some(d.handle.as_str())
                })
                .min_by(|a, b| {
                    let deadline =
                        |h: &str| book.known.get(h).map_or(f64::NEG_INFINITY, |e| e.retry_at);
                    deadline(a).total_cmp(&deadline(b))
                })
        })
        .map(str::to_owned);
    let Some(handle) = wanted else { return };
    // A header may contain a private sign-in name if the account has no display
    // name. Its public lookup always uses the account ID from the live session.
    let target = if is_owner(&handle) {
        owner_id.map(model::account_key)
    } else {
        None
    }
    .unwrap_or_else(|| handle.clone());
    if !book.known.contains_key(&handle) && book.known.len() >= CAPACITY {
        // Prefer outdated records. Never evict the open subject or current owner.
        let victim = book
            .known
            .iter()
            .filter(|(h, _)| Some(*h) != page.handle.as_ref() && Some(*h) != book.mine.as_ref())
            .min_by(|a, b| a.1.retry_at.total_cmp(&b.1.retry_at))
            .map(|(h, _)| h.clone());
        if let Some(h) = victim {
            book.known.remove(&h);
            book.revision = book.revision.wrapping_add(1);
        }
    }
    let (worker, mailbox) = mpsc::sync_channel(1);
    let _thread = std::thread::Builder::new()
        .name("ranked-public-read".into())
        .spawn(move || {
            let profile = central::endpoint()
                .and_then(|ep| {
                    baylee_account::ranked_read(
                        &ep,
                        Request::Profile {
                            target: model::target(&target)
                                .ok_or(baylee_account::Error::Rejected)?,
                            season: None,
                        },
                    )
                })
                .and_then(|res| {
                    Profile::accept(&target, res).ok_or(baylee_account::Error::Rejected)
                });
            let _ = worker.send(profile);
        });
    book.reading = Some((handle, Mutex::new(mailbox)));
}

fn receive(book: &mut Book, now: f64) {
    let response = book
        .reading
        .as_mut()
        .and_then(|(_, inbox)| delivered(inbox, || Err(baylee_account::Error::Unavailable)));
    if let Some(profile) = response {
        let (key, _) = book.reading.take().expect("reply has a key");
        let entry = book.known.entry(key).or_default();
        let failed = profile.is_err();
        let revoked = matches!(profile, Err(baylee_account::Error::Rejected));
        if entry.failed != failed
            || revoked && entry.profile.is_some()
            || profile
                .as_ref()
                .is_ok_and(|p| Some(p) != entry.profile.as_ref())
        {
            book.revision = book.revision.wrapping_add(1);
            entry.version = book.revision;
        }
        if let Ok(profile) = profile {
            entry.profile = Some(profile);
        } else if revoked {
            entry.profile = None;
        }
        entry.failed = failed;
        entry.retry_at = now + if failed { RETRY } else { LIFETIME };
        if std::mem::take(&mut book.repeat_read) {
            entry.retry_at = 0.0;
        }
    }
    let private = book.owner_read.as_mut().and_then(|inbox| {
        delivered(inbox, || Collection {
            failed: true,
            ..Collection::default()
        })
    });
    if let Some(collection) = private {
        book.owner_read = None;
        let before = book.private.clone();
        book.private.merge(collection);
        if before != book.private {
            book.revision = book.revision.wrapping_add(1);
        }
    }
}

fn read_private(book: &mut Book, now: f64) {
    if now < book.private_fresh_at || book.owner_read.is_some() {
        return;
    }
    let Some(owner) = book.owner.as_ref() else {
        return;
    };
    let (worker, inbox) = mpsc::sync_channel(1);
    if owner.submit(move |account| {
        let awards = Collection::read(account);
        let _ = worker.send(awards);
    }) {
        book.owner_read = Some(Mutex::new(inbox));
        book.private_fresh_at = now + LIFETIME;
    } else {
        book.private.failed = true;
        book.private_fresh_at = now + RETRY;
    }
}

#[derive(Component)]
struct Worn(RankArt, Entity);

#[derive(Resource, Default)]
struct NewDiscs(Vec<Entity>);

fn compact_ranks(
    book: Res<Book>,
    state: Res<LobbyState>,
    mut seen: Local<Option<(u64, baylee_client_core::i18n::Lang)>>,
    mut labels: Query<(Ref<CompactRanks>, &mut Text, &mut Node)>,
) {
    let lang = state.lobby.lang();
    let key = (book.revision, lang);
    let changed = *seen != Some(key);
    *seen = Some(key);
    for (label, mut text, mut node) in &mut labels {
        if !changed && !label.is_added() {
            continue;
        }
        let profile = book.known.get(&label.0).and_then(|e| e.profile.as_ref());
        let lines = profile
            .map(|p| {
                Format::ALL
                    .map(|format| {
                        let standing = p
                            .formats
                            .iter()
                            .find(|s| s.track == Track::Player && s.format == format);
                        let name = standing.filter(|s| s.placement_games >= 10).map_or_else(
                            || {
                                baylee_client_core::i18n::Phrase::RankedPlacements.fill(
                                    lang,
                                    &[&standing.map_or(0, |s| s.placement_games).to_string()],
                                )
                            },
                            |s| model::rank_name(s.rank, lang),
                        );
                        format!("{} · {}", model::format_name(format, lang), name)
                    })
                    .join("\n")
            })
            .unwrap_or_default();
        let display = if lines.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
        if node.display != display {
            node.display = display;
        }
        if text.0 != lines {
            text.0 = lines;
        }
    }
}

fn wear_frames(
    mut commands: Commands,
    book: Res<Book>,
    mut seen: Local<u64>,
    discs: Query<(Entity, &Avatar, Option<&PortraitKey>, Option<&Worn>), NeedsProfile>,
    mut fresh: ResMut<NewDiscs>,
) {
    let changed = *seen != book.revision;
    *seen = book.revision;
    if !changed && fresh.0.is_empty() {
        return;
    }
    let mut wear =
        |(entity, avatar, key, old): (Entity, &Avatar, Option<&PortraitKey>, Option<&Worn>)| {
            let want = book
                .known
                .get(key.map_or(&avatar.handle, |k| &k.0))
                .and_then(|e| e.profile.as_ref())
                .and_then(|p| {
                    p.best()
                        .map_or_else(|| RankArt::frame("unranked", None, None), model::frame)
                });
            if want == old.map(|w| w.0) {
                return;
            }
            commands
                .entity(avatar.plate)
                .try_insert(UiTransform::from_scale(Vec2::splat(
                    want.map_or(1.0, RankArt::portrait_scale),
                )));
            if let Some(w) = old {
                commands.entity(w.1).try_despawn();
                commands.entity(entity).remove::<Worn>();
            }
            if let Some(art) = want {
                let frame = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: percent(100),
                            height: percent(100),
                            ..default()
                        },
                        art,
                        ZIndex(1),
                        Pickable::IGNORE,
                    ))
                    .id();
                commands
                    .entity(entity)
                    .add_child(frame)
                    .insert(Worn(art, frame));
            }
        };
    if changed {
        for disc in &discs {
            wear(disc);
        }
    } else {
        for id in &fresh.0 {
            if let Ok(disc) = discs.get(*id) {
                wear(disc);
            }
        }
    }
    fresh.0.clear();
}
