//! A player's public competitive record and the owner's confirmed trophy collection.
use super::{LobbyState, central};
use crate::DuelPhase;
use crate::rank_art::RankArt;
use crate::shellkit::avatar::Avatar;
use crate::shellkit::focus::{Activated, Stop, TabOrder};
use crate::ui_button::Clicks;
use baylee_account::ranked::{Format, ProfileTarget, Request, Standing, Track};
use bevy::prelude::*;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
mod awards;
mod draw;
mod model;
#[cfg(test)]
mod tests;
use awards::Collection;
use model::Profile;

pub(crate) const ORDER: TabOrder = TabOrder {
    name: "ranked-profile",
    stops: &["close", "refresh"],
    modal: true,
};

/// Opening a public record always uses a full, exact Central handle.
#[derive(Component, Clone)]
pub(crate) struct OpenProfile(pub String);

#[derive(Component)]
pub(crate) struct Guest;

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
        if model::valid_handle(handle) {
            self.handle = Some(handle.to_owned());
            self.refresh = true;
        }
    }

    pub(crate) fn visible(&self) -> bool {
        self.handle.is_some()
    }

    /// The authenticated owner can also inspect their private medals before
    /// choosing a public name and discriminator.
    pub(super) fn open_own(&mut self, handle: &str) {
        self.handle = Some(handle.to_owned());
        self.refresh = true;
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
type PublicReply = Arc<Mutex<Option<ReadResult>>>;

#[derive(Resource, Default)]
pub(crate) struct Book {
    known: BTreeMap<String, Entry>,
    reading: Option<(String, PublicReply)>,
    repeat_read: bool,
    next_poll: f64,
    revision: u64,
    mine: Option<String>,
    owner: Option<central::Handle>,
    private: Collection,
    owner_read: Option<Arc<Mutex<Option<Collection>>>>,
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
    let current = state.central_handle();
    let same = match (&book.owner, &current) {
        (None, None) => true,
        (Some(a), Some(b)) => a.same_session(b),
        _ => false,
    };
    if !same {
        // Outstanding private jobs only retain the old mailbox.
        let revision = book.revision.wrapping_add(1);
        *book = Book {
            owner: current,
            revision,
            ..Book::default()
        };
        page.handle = None;
    }
    let mine = state
        .central
        .as_ref()
        .and_then(|s| s.profile_handle.as_deref())
        .filter(|_| book.owner.is_some());
    if book.mine.as_deref() != mine {
        book.mine = mine.map(str::to_owned);
    }
}

fn read(
    time: Res<Time>,
    state: Res<LobbyState>,
    mut page: ResMut<Page>,
    mut book: ResMut<Book>,
    discs: Query<&Avatar, Without<Guest>>,
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

fn tick(
    now: f64,
    state: &LobbyState,
    page: &mut Page,
    book: &mut Book,
    discs: &Query<&Avatar, Without<Guest>>,
) {
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
    let wanted = page
        .handle
        .iter()
        .chain(book.mine.iter())
        .map(String::as_str)
        .chain(discs.iter().map(|d| d.handle.as_str()))
        .find(|h| model::valid_handle(h) && book.known.get(*h).is_none_or(|e| now >= e.retry_at))
        .map(str::to_owned);
    let Some(handle) = wanted else { return };
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
        }
    }
    let mailbox: PublicReply = Arc::default();
    let worker = mailbox.clone();
    let target = handle.clone();
    let result = std::thread::Builder::new()
        .name("ranked-public-read".into())
        .spawn(move || {
            let profile = central::endpoint()
                .and_then(|ep| {
                    baylee_account::ranked_read(
                        &ep,
                        Request::Profile {
                            target: ProfileTarget::Handle(target.clone()),
                            season: None,
                        },
                    )
                })
                .and_then(|res| {
                    Profile::accept(&target, res).ok_or(baylee_account::Error::Rejected)
                });
            *worker
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(profile);
        });
    if result.is_err() {
        *mailbox
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            Some(Err(baylee_account::Error::Unavailable));
    }
    book.reading = Some((handle, mailbox));
}

fn receive(book: &mut Book, now: f64) {
    let response = book
        .reading
        .as_ref()
        .and_then(|(_, inbox)| inbox.lock().ok()?.take());
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
    let private = book
        .owner_read
        .as_ref()
        .and_then(|inbox| inbox.lock().ok()?.take());
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
    let inbox: Arc<Mutex<Option<Collection>>> = Arc::default();
    let worker = inbox.clone();
    if owner.submit(move |account| {
        let awards = Collection::read(account);
        *worker
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(awards);
    }) {
        book.owner_read = Some(inbox);
        book.private_fresh_at = now + LIFETIME;
    } else {
        book.private.failed = true;
        book.private_fresh_at = now + RETRY;
    }
}

#[derive(Component)]
struct Worn(RankArt, Entity);

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
                model::FORMATS
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
    discs: Query<(Entity, Ref<Avatar>, Option<&Worn>), Without<Guest>>,
) {
    let changed = *seen != book.revision;
    *seen = book.revision;
    for (entity, avatar, old) in &discs {
        if !changed && !avatar.is_added() {
            continue;
        }
        let want = book
            .known
            .get(&avatar.handle)
            .and_then(|e| e.profile.as_ref())
            .and_then(|p| {
                p.best()
                    .map_or_else(|| RankArt::frame("unranked", None, None), model::frame)
            });
        if want == old.map(|w| w.0) {
            continue;
        }
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
    }
}
