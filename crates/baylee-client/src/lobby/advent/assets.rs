//! Bounded, immutable, privately scoped render textures. Each cold request
//! obtains fresh account/seat authorization; scopes never substitute for one another.
#[cfg(test)]
mod tests;
use super::{
    AccountHandle, AssetMeta, AssetScope, Book, DecodeCasual, Error, ForgetMatch, LoadReward,
    LoadScoped,
};
use bevy::prelude::*;
use std::{
    collections::VecDeque,
    io::Cursor,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

struct Task {
    scope: AssetScope,
    meta: AssetMeta,
    bytes: Option<Vec<u8>>,
}
struct Pending {
    scope: AssetScope,
    meta: AssetMeta,
    cancelled: Arc<AtomicBool>,
    rx: Mutex<mpsc::Receiver<Result<Image, Error>>>,
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}
struct Ready {
    scope: AssetScope,
    meta: AssetMeta,
    image: Handle<Image>,
}
#[derive(Default)]
pub(super) struct Library {
    queue: VecDeque<Task>,
    pending: Option<Pending>,
    ready: VecDeque<Ready>,
    errors: VecDeque<(AssetScope, Error)>,
}

impl Library {
    pub(super) fn error(&self, scope: AssetScope) -> Option<Error> {
        self.errors
            .iter()
            .find_map(|(s, e)| (*s == scope).then_some(*e))
    }
    fn failed(&mut self, scope: AssetScope, error: Error) {
        self.errors.retain(|(s, _)| *s != scope);
        if self.errors.len() == 20 {
            self.errors.pop_front();
        }
        self.errors.push_back((scope, error));
    }
    pub(super) fn image(&self, scope: &AssetScope, meta: &AssetMeta) -> Option<&Handle<Image>> {
        self.ready
            .iter()
            .find(|r| &r.scope == scope && &r.meta == meta)
            .map(|r| &r.image)
    }
    pub(super) fn want(&mut self, scope: AssetScope, meta: AssetMeta) {
        self.queue(scope, meta, None);
    }
    fn queue(&mut self, scope: AssetScope, meta: AssetMeta, bytes: Option<Vec<u8>>) {
        if self.image(&scope, &meta).is_some()
            || self
                .pending
                .as_ref()
                .is_some_and(|p| p.scope == scope && p.meta == meta)
            || self
                .queue
                .iter()
                .any(|p| p.scope == scope && p.meta == meta)
            || self.queue.len() >= 20
        {
            return;
        }
        self.errors.retain(|(s, _)| *s != scope);
        if self.pending.as_ref().is_some_and(|p| p.scope == scope) {
            self.pending = None;
        }
        self.queue.retain(|p| p.scope != scope);
        self.ready.retain(|r| r.scope != scope);
        self.queue.push_back(Task { scope, meta, bytes });
    }
    fn retain(&mut self, keep: impl Fn(&AssetScope) -> bool) {
        self.errors.retain(|(s, _)| keep(s));
        self.ready.retain(|r| keep(&r.scope));
        self.queue.retain(|q| keep(&q.scope));
        if self.pending.as_ref().is_some_and(|p| !keep(&p.scope)) {
            self.pending = None;
        }
    }
    fn start(&mut self, account: Option<&AccountHandle>) -> bool {
        if self.pending.is_some() {
            return false;
        }
        let Some(task) = self.queue.pop_front() else {
            return false;
        };
        let (tx, rx) = mpsc::sync_channel(1);
        let scope = task.scope;
        let meta = task.meta;
        let account = account.cloned();
        let cancelled = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&cancelled);
        if std::thread::Builder::new()
            .name("advent-image".into())
            .spawn(move || {
                let bytes = match (task.bytes, account) {
                    (Some(bytes), _) => Ok(bytes),
                    (None, Some(a)) => {
                        crate::lobby::advent_table::load_asset(&a, scope, &meta, &stop)
                    }
                    (None, None) => Err(Error::Unavailable),
                };
                if !stop.load(Ordering::Acquire) {
                    let _ = tx.send(bytes.and_then(|bytes| decode(&bytes, meta)));
                }
            })
            .is_err()
        {
            self.failed(scope, Error::Unavailable);
            return true;
        }
        self.pending = Some(Pending {
            scope,
            meta,
            cancelled,
            rx: Mutex::new(rx),
        });
        false
    }
}

fn decode(bytes: &[u8], meta: AssetMeta) -> Result<Image, Error> {
    baylee_account::check_advent_image(&meta, bytes)?;
    let expected = (u32::from(meta.width), u32::from(meta.height));
    let mut source = image::ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::WebP);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(expected.0);
    limits.max_image_height = Some(expected.1);
    limits.max_alloc = Some(128 * 1024 * 1024);
    source.limits(limits);
    let image = source.decode().map_err(|_| Error::Rejected)?;
    Ok(Image::from_dynamic(image, true, default()))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn poll(
    mut b: ResMut<Book>,
    mut owned: MessageReader<LoadReward>,
    mut scoped: MessageReader<LoadScoped>,
    mut casual: MessageMutator<DecodeCasual>,
    mut forget: MessageReader<ForgetMatch>,
    table: Option<Res<crate::event_cosmetics::MatchLooks>>,
    images: Option<ResMut<Assets<Image>>>,
) {
    let Some(mut images) = images else { return };
    let book = b.bypass_change_detection();
    let account = book.owner.clone();
    let season = book.released().then(|| book.year());
    book.media
        .retain(|s| !matches!(s,AssetScope::Calendar{year} if Some(*year)!=season));
    let current = table
        .as_deref()
        .and_then(crate::event_cosmetics::MatchLooks::table);
    for event in forget.read() {
        book.media
            .retain(|s| !matches!(s,AssetScope::Equipped{match_id,..} if *match_id==event.0));
    }
    book.media
        .retain(|s| !matches!(s,AssetScope::Equipped{match_id,..} if Some(*match_id)!=current));
    for LoadReward(item) in owned.read() {
        book.load_owned(*item);
    }
    for event in scoped.read() {
        if matches!(event.scope,AssetScope::Equipped{match_id,..} if Some(match_id)==current) {
            book.media.want(event.scope, event.asset);
        }
    }
    for event in casual.read() {
        if matches!(event.scope,AssetScope::Equipped{match_id,..} if Some(match_id)==current)
            && event.bytes.len()
                <= usize::try_from(baylee_account::advent::MAX_ART_BYTES).unwrap_or(0)
        {
            book.media.queue(
                event.scope,
                event.asset,
                Some(std::mem::take(&mut event.bytes)),
            );
        }
    }
    let result = book
        .media
        .pending
        .as_mut()
        .and_then(|p| match p.rx.get_mut().ok()?.try_recv() {
            Ok(r) => Some(r),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(Error::Unavailable)),
        });
    if let Some(result) = result
        && let Some(p) = book.media.pending.take()
    {
        match result {
            Ok(image) => {
                book.media.ready.retain(|r| r.scope != p.scope);
                book.media.ready.push_back(Ready {
                    scope: p.scope,
                    meta: p.meta,
                    image: images.add(image),
                });
                while book
                    .media
                    .ready
                    .iter()
                    .filter(|r| matches!(r.scope, AssetScope::Owned { .. }))
                    .count()
                    > 4
                {
                    if let Some(i) = book
                        .media
                        .ready
                        .iter()
                        .position(|r| matches!(r.scope, AssetScope::Owned { .. }))
                    {
                        book.media.ready.remove(i);
                    }
                }
            }
            Err(error) => book.media.failed(p.scope, error),
        }
        book.touch();
        b.set_changed();
    }
    if b.bypass_change_detection().media.start(account.as_ref()) {
        b.touch();
    }
}
