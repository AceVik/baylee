//! Profile pictures from Baylee Central, in every disc the client draws for
//! a person (`shellkit::avatar`), and moving where the picture moves.
//!
//! Desktop only, like `baylee-avif`. What is decided headless is
//! `baylee_client_core::portrait`'s; this module carries it out:
//!
//! 1. **Keys.** Once a second, [`ask_for_keys`] works out the near tags (the
//!    account's own, its contacts and requests, its groups' members) and,
//!    when the [`KeyBook`] says one is due, sends `SocialRequest::Pictures`
//!    on the account worker. A push from Central's socket (contacts, chat,
//!    presence) marks the book stale. The lookup stays off the chat bridge:
//!    its refusal or silence touches no chat poll, backoff or session. Tags
//!    of a game gateway's own (a recent player, a ranking's row) are not
//!    asked: a gateway's tags need not be Central's, and a stranger costs
//!    Central's lookup budget.
//! 2. **Bytes.** For every account wearing a picture whose disc is drawn,
//!    the worker (`fetch`) fetches the still from the picture origin
//!    ([`picture_origin`]) or the disk cache, and decodes it bounded. A
//!    moving picture's sequence is fetched and decoded (in a child process,
//!    `helper`) only while one of its discs can be seen and motion is
//!    allowed, once per key; its frames are kept within the memory bound.
//! 3. **Drawing.** Each picture is one `Image` (`KEPT_SIDE` square), laid
//!    into [`Portraits`] for every tag that wears it; `wear_portraits` puts
//!    it in the discs. [`play`] writes the image's pixels only when the frame
//!    shown changes; under `reduce_motion`, with no disc of it seen, or with
//!    the window hidden, the still stands. It never asks for a frame: it
//!    rides the frames the pacer draws anyway (`quality`), at least 30 a
//!    second while the window is seen.
//!
//! A refused, missing or broken picture leaves the monogram, without a word
//! in the log above `debug`. A sign-out (or another account) forgets every
//! key and picture; only the raw bytes stay on disk, by key, within their
//! bound (`docs/privacy.md`).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Mutex, mpsc};

use baylee_account::Error;
use baylee_account::social::{SocialRequest, SocialResponse};
use baylee_client_core::portrait::{
    Answer, Ask, CLIP_CAP_BYTES, Change, File, Held, KEPT_SIDE, KeyBook, PictureKey, Timing,
    clips_to_drop, key_hex, plays, tag_of, tag_text,
};
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::lobby::LobbyState;
use crate::shellkit::avatar::{Avatar, Portrait, Portraits};
use crate::social::SocialHost;

mod fetch;
pub mod helper;
#[cfg(test)]
mod tests;

pub use helper::helper_if_asked;

/// How often the keys and the fetches are looked at, in seconds.
const LOOK_EVERY: f64 = 1.0;

/// How long a picture that got no answer waits before it is asked again,
/// in milliseconds.
const RETRY_MS: u64 = 60_000;

/// The origin pictures are read from, for an account whose Central is at
/// `endpoint`: Central's website (`/api/v1/pictures/…` is the website's
/// route, Central's `docs/profile-pictures.md`), live or, in a development
/// build against a loopback Central, the loopback website the front door
/// links to. `None` where neither is known: no pictures.
///
/// This is the one line to change should Central serve the files on its
/// account origin instead.
#[must_use]
pub(crate) fn picture_origin(endpoint: &str) -> Option<String> {
    use crate::lobby::{CENTRAL_WEBSITE_URL, central_website_url};
    if endpoint == baylee_account::Endpoint::live().origin() {
        return Some(CENTRAL_WEBSITE_URL.to_owned());
    }
    let site = central_website_url();
    (site != CENTRAL_WEBSITE_URL && fetch::origin_ok(&site)).then_some(site)
}

/// Where a picture's still or frames stand.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Load {
    /// Not asked yet.
    #[default]
    Idle,
    /// With the worker.
    Asked,
    /// Got no answer; ask again from then (Unix ms).
    Retry(u64),
    /// In.
    Done,
    /// Refused or broken: not again this session.
    Never,
}

impl Load {
    fn wanted(self, now: u64) -> bool {
        match self {
            Self::Idle => true,
            Self::Retry(at) => now >= at,
            Self::Asked | Self::Done | Self::Never => false,
        }
    }
}

/// A moving picture's decoded frames.
struct Clip {
    frames: Vec<Vec<u8>>,
    timing: Timing,
    bytes: usize,
}

/// One picture (by key) as the client holds it.
#[derive(Default)]
struct Picture {
    still: Load,
    moving: Load,
    /// The image every disc of it draws, once the still is in.
    image: Option<Handle<Image>>,
    /// The still's pixels, laid back when the picture stops.
    pixels: Vec<u8>,
    clip: Option<Clip>,
    /// The frame the image shows: `None` the still, else a clip's frame.
    shown: Option<usize>,
    /// When it began to play (seconds of real time).
    since: Option<f64>,
    /// When one of its discs was last seen, in Unix ms.
    seen: u64,
}

/// The key lookup out on the account worker.
struct Lookup {
    ask: Ask,
    receive: Mutex<mpsc::Receiver<(Answer, Option<String>)>>,
}

/// Everything the pictures' systems hold.
#[derive(Resource, Default)]
pub(crate) struct PictureWork {
    book: KeyBook,
    /// The signed-in account the book is for.
    account: Option<u32>,
    /// Where pictures come from, as the account worker said.
    origin: Option<String>,
    lookup: Option<Lookup>,
    courier: Option<fetch::Courier>,
    pictures: HashMap<PictureKey, Picture>,
    seen_social: u64,
    seen_presence: u64,
    next_look: f64,
    /// The near accounts by tag, each with the handle Central knows it by:
    /// only a disc of exactly that handle wears its picture.
    handles: BTreeMap<u32, String>,
    /// Keys no one wears any more, whose files the worker is still to remove.
    forgets: Vec<PictureKey>,
    /// How many lookups were sent, for `/state`.
    asked: u32,
    /// The development harness laid the pictures in (`/seed`): no lookup
    /// and no sign-out takes them away.
    #[cfg(feature = "dev-control")]
    seeded: bool,
}

/// One picture the development harness lays in (`/seed`).
#[cfg(feature = "dev-control")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Seeded {
    /// A made-up key whose files are written to the cache directory first:
    /// a still, or a still and a sequence.
    Fixture {
        /// Whether it moves.
        moving: bool,
    },
    /// A real key, whose files the shipped path fetches from the
    /// development Central's website: everything but Central's answer.
    Key {
        /// The key.
        key: PictureKey,
        /// Whether it moves.
        moving: bool,
    },
}

/// Lays fixed pictures in for the development harness's `/seed`, so the
/// shipped path (disk or download, decode, the child process for frames, the
/// image, the discs) draws them without Central's key lookup. Afterwards no
/// lookup and no sign-out takes them away. `still` is a 256 px PNG,
/// `sequence` an AVIF image sequence.
///
/// # Errors
/// No cache directory, or a real key without a loopback website to fetch it
/// from.
#[cfg(feature = "dev-control")]
pub(crate) fn seed(
    work: &mut PictureWork,
    pictures: &[(u32, Seeded)],
    still: &[u8],
    sequence: &[u8],
) -> Result<usize, String> {
    let dir = fetch::cache_dir().ok_or("no cache directory")?;
    let shelf = fetch::Shelf::open(dir);
    if pictures
        .iter()
        .any(|(_, seeded)| matches!(seeded, Seeded::Key { .. }))
    {
        let site = crate::lobby::central_website_url();
        if site == crate::lobby::CENTRAL_WEBSITE_URL || !fetch::origin_ok(&site) {
            return Err("no loopback Central website to fetch a key from".into());
        }
        work.origin = Some(site);
    }
    work.seeded = true;
    work.account.get_or_insert(0);
    // Fixtures are on disk; a loopback port nothing listens on is never asked.
    work.origin
        .get_or_insert_with(|| "http://127.0.0.1:9".to_owned());
    for &(tag, seeded) in pictures {
        let worn = match seeded {
            Seeded::Fixture { moving } => {
                let mut key = [0xd5; 16];
                key[..4].copy_from_slice(&tag.to_be_bytes());
                key[4] = u8::from(moving);
                shelf.write(&key, File::Still, still);
                if moving {
                    shelf.write(&key, File::Moving, sequence);
                }
                baylee_client_core::portrait::Worn { key, moving }
            }
            Seeded::Key { key, moving } => baylee_client_core::portrait::Worn { key, moving },
        };
        work.book.wear(tag, Some(worn));
        work.pictures.remove(&worn.key);
    }
    Ok(pictures.len())
}

/// Installs the pictures' systems. A headless app has no image assets (or
/// clock), and there every disc keeps its monogram.
pub(crate) fn install(app: &mut App) {
    app.init_resource::<PictureWork>().add_systems(
        Update,
        (ask_for_keys, lay_in, play)
            .chain()
            .run_if(resource_exists::<Portraits>)
            .run_if(resource_exists::<Assets<Image>>)
            .run_if(resource_exists::<Time<Real>>),
    );
}

/// The near accounts: the account itself (`me`, its handle), its contacts
/// and requests, and its groups' members (Central charges no lookup for
/// them), by tag, each with the handle Central knows it by.
fn near_handles(me: Option<&str>, host: Option<&SocialHost>) -> BTreeMap<u32, String> {
    let mut near: BTreeMap<u32, String> = me
        .and_then(|me| Some((tag_of(me)?, me.to_owned())))
        .into_iter()
        .collect();
    if let Some(machine) = host.and_then(SocialHost::machine) {
        let contacts = machine.contacts();
        let people = contacts
            .friends
            .iter()
            .map(|friend| &friend.person)
            .chain(&contacts.incoming)
            .chain(&contacts.outgoing)
            .chain(
                contacts
                    .groups
                    .iter()
                    .flat_map(|group| group.members.iter().map(|member| &member.person)),
            );
        near.extend(
            people.filter_map(|person| Some((tag_of(&person.tag)?, person.handle.clone()))),
        );
    }
    near
}

/// Sends `ask` on the account worker; the answer comes back with the picture
/// origin the account's Central gives.
fn send(handle: &crate::lobby::central::Handle, ask: &Ask) -> Option<Lookup> {
    let tags = ask.tags.clone();
    let (answer, receive) = mpsc::sync_channel(1);
    handle
        .submit(move |account| {
            let origin = picture_origin(account.endpoint_origin());
            let said = match account.social(SocialRequest::Pictures { tags }) {
                Ok(SocialResponse::Pictures { pictures }) => Answer::Pictures(pictures),
                Err(Error::Busy) => Answer::Busy,
                Err(Error::Unavailable) => Answer::Unavailable,
                Ok(_) | Err(_) => Answer::Refused,
            };
            let _ = answer.send((said, origin));
        })
        .then(|| Lookup {
            ask: ask.clone(),
            receive: Mutex::new(receive),
        })
}

/// Learns which picture each near account wears (module docs, 1).
fn ask_for_keys(
    time: Res<Time<Real>>,
    lobby: Option<Res<LobbyState>>,
    host: Option<Res<SocialHost>>,
    mut work: ResMut<PictureWork>,
    mut portraits: ResMut<Portraits>,
    mut images: ResMut<Assets<Image>>,
) {
    let clock = time.elapsed_secs_f64();
    if work.lookup.is_none() && clock < work.next_look {
        return;
    }
    let now = crate::social::now_ms();
    let work = &mut *work;
    let changes = take_answer(work, now);
    if !changes.is_empty() {
        apply(work, &changes, &mut portraits.reborrow(), &mut images);
    }
    if clock < work.next_look {
        return;
    }
    work.next_look = clock + LOOK_EVERY;
    let handle = lobby.as_ref().and_then(|lobby| lobby.central_handle());
    let own = lobby
        .as_ref()
        .and_then(|lobby| lobby.central.as_ref())
        .and_then(|central| central.profile_handle.as_deref());
    let near = near_handles(own, host.as_deref());
    known_as(work, near, &mut portraits.reborrow());
    let tags: BTreeSet<u32> = work.handles.keys().copied().collect();
    look(
        work,
        handle,
        own.and_then(tag_of),
        &tags,
        now,
        &mut portraits.reborrow(),
    );
}

/// Takes the near accounts' handles as Central knows them now, and lays a
/// renamed account's picture in under its new handle.
fn known_as(
    work: &mut PictureWork,
    near: BTreeMap<u32, String>,
    portraits: &mut Mut<'_, Portraits>,
) {
    if work.handles == near {
        return;
    }
    for (tag, handle) in &near {
        let renamed = portraits
            .by_tag(&tag_text(*tag))
            .filter(|portrait| portrait.handle != *handle)
            .cloned();
        if let Some(portrait) = renamed {
            portraits.wear(Portrait {
                handle: handle.clone(),
                ..portrait
            });
        }
    }
    work.handles = near;
}

/// What the disc of `handle` wears: only an account near this one, under
/// the handle Central knows it by.
fn worn_by(work: &PictureWork, handle: &str) -> Option<baylee_client_core::portrait::Worn> {
    let tag = tag_of(handle)?;
    if work.handles.get(&tag)? != handle {
        return None;
    }
    work.book.worn(tag)
}

/// Takes the key lookup's answer, if it came, into the book, and says
/// what changed (for [`apply`]).
fn take_answer(work: &mut PictureWork, now: u64) -> Vec<Change> {
    let Some(lookup) = work.lookup.as_ref() else {
        return Vec::new();
    };
    let arrived = lookup
        .receive
        .lock()
        .ok()
        .and_then(|receive| match receive.try_recv() {
            Ok(said) => Some(said),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some((Answer::Unavailable, None)),
        });
    if let Some((answer, origin)) = arrived
        && let Some(lookup) = work.lookup.take()
    {
        if origin.is_some() {
            work.origin = origin;
        }
        return work.book.answer(now, &lookup.ask, answer);
    }
    Vec::new()
}

/// One look at the keys at `now`: the account signed in (`handle`, `me`),
/// whether a push said something changed, and the lookup the book wants.
fn look(
    work: &mut PictureWork,
    handle: Option<crate::lobby::central::Handle>,
    me: Option<u32>,
    near: &BTreeSet<u32>,
    now: u64,
    portraits: &mut Mut<'_, Portraits>,
) {
    #[cfg(feature = "dev-control")]
    if work.seeded {
        return;
    }
    let Some(handle) = handle.filter(|_| me.is_some()) else {
        if work.account.is_some() || !work.pictures.is_empty() {
            forget(work, portraits);
        }
        return;
    };
    if work.account != me {
        forget(work, portraits);
        work.account = me;
        work.seen_social = handle.social_seq();
        work.seen_presence = handle.presence_seq();
    }
    let (social, presence) = (handle.social_seq(), handle.presence_seq());
    if social != work.seen_social || presence != work.seen_presence {
        work.seen_social = social;
        work.seen_presence = presence;
        work.book.stale();
    }
    if work.lookup.is_some() {
        return;
    }
    if let Some(ask) = work.book.next(now, near, &BTreeSet::new()) {
        work.asked = work.asked.saturating_add(1);
        match send(&handle, &ask) {
            Some(lookup) => work.lookup = Some(lookup),
            None => {
                work.book.answer(now, &ask, Answer::Unavailable);
            }
        }
    }
}

/// Lets go of every key and picture: a sign-out, or another account.
fn forget(work: &mut PictureWork, portraits: &mut Mut<'_, Portraits>) {
    work.book.forget();
    work.lookup = None;
    work.account = None;
    work.origin = None;
    work.handles.clear();
    work.pictures.clear();
    if !portraits.is_empty() {
        let tags: Vec<String> = portraits.all().map(|(tag, _)| tag.to_owned()).collect();
        for tag in tags {
            portraits.forget(&tag);
        }
    }
}

/// Carries an answer's changes into [`Portraits`]: a tag that lost its
/// picture shows its monogram, one with a new key waits for its still, and a
/// key no one wears any more is let go, on disk too.
fn apply(
    work: &mut PictureWork,
    changes: &[Change],
    portraits: &mut Mut<'_, Portraits>,
    images: &mut Assets<Image>,
) {
    for change in changes {
        let tag = tag_text(change.tag);
        match change.now {
            Some(worn) => {
                let image = work
                    .pictures
                    .get(&worn.key)
                    .and_then(|picture| picture.image.clone());
                match (image, work.handles.get(&change.tag)) {
                    (Some(image), Some(handle)) => portraits.wear(Portrait {
                        handle: handle.clone(),
                        image,
                        moving: worn.moving,
                    }),
                    _ if portraits.by_tag(&tag).is_some() => portraits.forget(&tag),
                    _ => {}
                }
            }
            None => {
                if portraits.by_tag(&tag).is_some() {
                    portraits.forget(&tag);
                }
            }
        }
        if let Some(was) = change.was
            && change.now.map(|now| now.key) != Some(was.key)
            && !work.book.all().any(|(_, worn)| worn.key == was.key)
        {
            if let Some(picture) = work.pictures.remove(&was.key)
                && let Some(image) = picture.image
            {
                images.remove(&image);
            }
            work.forgets.push(was.key);
        }
    }
}

/// An image of `rgba` at `KEPT_SIDE`, which the main world keeps so a moving
/// picture's frames can be written into it.
fn image_of(rgba: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: KEPT_SIDE,
            height: KEPT_SIDE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::linear();
    image
}

/// Whether reduced motion is asked for.
fn reduced(prefs: Option<&crate::prefs::Prefs>) -> bool {
    prefs.is_some_and(|prefs| prefs.all().reduce_motion)
}

/// Takes what the worker finished, and hands it what the discs need (module
/// docs, 2).
#[allow(clippy::too_many_arguments)] // the worker's two ends and the discs
fn lay_in(
    time: Res<Time<Real>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    watch: Option<Res<crate::quality::Watch>>,
    discs: Query<(&Avatar, &InheritedVisibility)>,
    mut work: ResMut<PictureWork>,
    mut portraits: ResMut<Portraits>,
    mut images: ResMut<Assets<Image>>,
    mut looked: Local<f64>,
) {
    let clock = time.elapsed_secs_f64();
    let done = work
        .courier
        .as_ref()
        .map_or_else(Vec::new, fetch::Courier::take);
    if done.is_empty() && work.forgets.is_empty() && (clock < *looked || work.account.is_none()) {
        return;
    }
    let now = crate::social::now_ms();
    let work = &mut *work;
    for job in done {
        arrived(work, job, now, &mut portraits.reborrow(), &mut images);
    }
    forget_files(work);
    if clock < *looked || work.account.is_none() {
        return;
    }
    *looked = clock + LOOK_EVERY / 2.0;
    let Some(origin) = work.origin.clone() else {
        return;
    };
    let motion = !reduced(prefs.as_deref()) && !watch.is_some_and(|watch| watch.hidden());
    let mut wanted: Vec<fetch::Job> = Vec::new();
    for (avatar, visible) in &discs {
        let Some(worn) = worn_by(work, &avatar.handle) else {
            continue;
        };
        let picture = work.pictures.entry(worn.key).or_default();
        if picture.still.wanted(now) {
            picture.still = Load::Asked;
            wanted.push(fetch::Job::Still {
                origin: origin.clone(),
                key: worn.key,
            });
        } else if worn.moving
            && motion
            && visible.get()
            && picture.image.is_some()
            && picture.clip.is_none()
            && picture.moving.wanted(now)
        {
            picture.moving = Load::Asked;
            wanted.push(fetch::Job::Moving {
                origin: origin.clone(),
                key: worn.key,
            });
        }
    }
    if wanted.is_empty() {
        return;
    }
    if work.courier.is_none() {
        work.courier = fetch::Courier::start(fetch::cache_dir());
    }
    for job in wanted {
        let which = match &job {
            fetch::Job::Still { key, .. } => Some((*key, File::Still)),
            fetch::Job::Moving { key, .. } => Some((*key, File::Moving)),
            fetch::Job::Forget { .. } => None,
        };
        let sent = work
            .courier
            .as_ref()
            .is_some_and(|courier| courier.ask(job));
        // The queue is full: looked at again on the next round.
        if !sent
            && let Some((key, file)) = which
            && let Some(picture) = work.pictures.get_mut(&key)
        {
            match file {
                File::Still => picture.still = Load::Idle,
                File::Moving => picture.moving = Load::Idle,
            }
        }
    }
}

/// Hands the worker the keys whose files are to go, as far as its queue
/// takes them; the rest wait for the next look.
fn forget_files(work: &mut PictureWork) {
    if work.forgets.is_empty() {
        return;
    }
    if work.courier.is_none() {
        work.courier = fetch::Courier::start(fetch::cache_dir());
    }
    let Some(courier) = work.courier.as_ref() else {
        return;
    };
    while let Some(key) = work.forgets.last().copied() {
        if !courier.ask(fetch::Job::Forget { key }) {
            break;
        }
        work.forgets.pop();
    }
}

/// One finished job of the worker.
fn arrived(
    work: &mut PictureWork,
    done: fetch::Done,
    now: u64,
    portraits: &mut Mut<'_, Portraits>,
    images: &mut Assets<Image>,
) {
    match done {
        fetch::Done::Still { key, rgba } => {
            let wearers: Vec<(u32, bool)> = work
                .book
                .all()
                .filter(|(_, worn)| worn.key == key)
                .map(|(tag, worn)| (tag, worn.moving))
                .collect();
            if wearers.is_empty() {
                work.pictures.remove(&key);
                return;
            }
            let picture = work.pictures.entry(key).or_default();
            let image = images.add(image_of(rgba.clone()));
            picture.pixels = rgba;
            picture.still = Load::Done;
            if let Some(old) = picture.image.replace(image.clone()) {
                images.remove(&old);
            }
            for (tag, moving) in wearers {
                if let Some(handle) = work.handles.get(&tag) {
                    portraits.wear(Portrait {
                        handle: handle.clone(),
                        image: image.clone(),
                        moving,
                    });
                }
            }
        }
        fetch::Done::Moving { key, frames } => {
            let Some(picture) = work.pictures.get_mut(&key) else {
                return;
            };
            let timing = Timing::new(&frames.durations)
                .filter(|timing| timing.frames() == frames.rgba.len());
            match timing {
                Some(timing) => {
                    let bytes = frames.rgba.iter().map(Vec::len).sum();
                    picture.clip = Some(Clip {
                        frames: frames.rgba,
                        timing,
                        bytes,
                    });
                    picture.moving = Load::Done;
                    picture.seen = now;
                    hold_clips(work, images);
                }
                None => picture.moving = Load::Never,
            }
        }
        fetch::Done::Failed { key, file, again } => {
            let Some(picture) = work.pictures.get_mut(&key) else {
                return;
            };
            let next = if again {
                Load::Retry(now + RETRY_MS)
            } else {
                Load::Never
            };
            debug!(
                picture = %key_hex(&key),
                ?file,
                again,
                "a profile picture could not be shown; its monogram stands"
            );
            match file {
                File::Still => picture.still = next,
                File::Moving => picture.moving = next,
            }
        }
    }
}

/// Lets go of the clips seen longest ago until the frames held fit their
/// bound; a clip let go shows its still and is decoded anew when seen.
fn hold_clips(work: &mut PictureWork, images: &mut Assets<Image>) {
    let held: Vec<Held> = work
        .pictures
        .iter()
        .filter_map(|(key, picture)| {
            let clip = picture.clip.as_ref()?;
            Some(Held {
                key: *key,
                bytes: clip.bytes,
                seen: picture.seen,
            })
        })
        .collect();
    let gone = clips_to_drop(&held, CLIP_CAP_BYTES);
    if gone.is_empty() {
        return;
    }
    for picture in work
        .pictures
        .iter_mut()
        .filter_map(|(key, picture)| gone.contains(key).then_some(picture))
    {
        picture.clip = None;
        picture.moving = Load::Idle;
        if picture.shown.take().is_some() {
            write_pixels(images, picture.image.as_ref(), &picture.pixels);
        }
        picture.since = None;
    }
}

/// Writes `pixels` into the image: the one write a frame change costs.
fn write_pixels(images: &mut Assets<Image>, image: Option<&Handle<Image>>, pixels: &[u8]) {
    let Some(image) = image else {
        return;
    };
    if let Some(mut image) = images.get_mut(image)
        && let Some(data) = image.data.as_mut()
        && data.len() == pixels.len()
    {
        data.copy_from_slice(pixels);
    }
}

/// Plays the moving pictures that are seen (module docs, 3).
fn play(
    time: Res<Time<Real>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    watch: Option<Res<crate::quality::Watch>>,
    discs: Query<(&Avatar, &InheritedVisibility)>,
    mut work: ResMut<PictureWork>,
    mut images: ResMut<Assets<Image>>,
    mut seen: Local<Vec<PictureKey>>,
) {
    if !work.pictures.values().any(|picture| picture.clip.is_some()) {
        return;
    }
    let work = &mut *work;
    let reduce = reduced(prefs.as_deref());
    let shown = !watch.is_some_and(|watch| watch.hidden());
    seen.clear();
    for (avatar, visible) in &discs {
        if visible.get()
            && let Some(worn) = worn_by(work, &avatar.handle)
            && worn.moving
            && !seen.contains(&worn.key)
        {
            seen.push(worn.key);
        }
    }
    let now = crate::social::now_ms();
    let clock = time.elapsed_secs_f64();
    for (key, picture) in &mut work.pictures {
        let Some(clip) = picture.clip.as_ref() else {
            continue;
        };
        let looked_at = seen.contains(key);
        if looked_at {
            picture.seen = now;
        }
        let want = if plays(reduce, looked_at, shown) {
            let since = *picture.since.get_or_insert(clock);
            let elapsed = ((clock - since).max(0.0) * 1000.0) as u64;
            Some(clip.timing.frame_at(elapsed))
        } else {
            picture.since = None;
            None
        };
        if want == picture.shown {
            continue;
        }
        picture.shown = want;
        match want {
            Some(frame) => write_pixels(&mut images, picture.image.as_ref(), &clip.frames[frame]),
            None => write_pixels(&mut images, picture.image.as_ref(), &picture.pixels),
        }
    }
}

/// What `/state` says of the pictures: the lookup, and every disc drawn with
/// whether it shows a picture, whether that picture moves and which frame
/// it shows (`null`: the still).
#[cfg(feature = "dev-control")]
pub(crate) fn state_json<'a>(
    work: Option<&PictureWork>,
    portraits: Option<&Portraits>,
    discs: impl Iterator<Item = (&'a Avatar, bool, bool)>,
) -> serde_json::Value {
    let Some(work) = work else {
        return serde_json::Value::Null;
    };
    let discs: Vec<serde_json::Value> = discs
        .map(|(avatar, visible, worn)| {
            let portrait = portraits.and_then(|portraits| portraits.portrait(&avatar.handle));
            let picture =
                worn_by(work, &avatar.handle).and_then(|worn| work.pictures.get(&worn.key));
            serde_json::json!({
                "handle": avatar.handle,
                "size": avatar.size,
                "visible": visible,
                "picture": worn,
                "moving": portrait.is_some_and(|portrait| portrait.moving),
                "frames": picture.and_then(|p| p.clip.as_ref()).map(|clip| clip.frames.len()),
                "frame": picture.and_then(|p| p.shown),
            })
        })
        .collect();
    serde_json::json!({
        "account": work.account.map(tag_text),
        "origin": work.origin,
        "asked": work.asked,
        "asking": work.lookup.is_some(),
        "quiet_until": work.book.quiet_until(),
        "worn": work.book.all().count(),
        "pictures": work.pictures.values().filter(|p| p.image.is_some()).count(),
        "clips": work.pictures.values().filter(|p| p.clip.is_some()).count(),
        "discs": discs,
    })
}
