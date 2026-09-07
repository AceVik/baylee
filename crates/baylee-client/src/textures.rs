//! Card texture cache.
//!
//! Bevy's asset server does the fetching and, on native, the on-disk caching.
//! What it does not do is decide when to let go, and on a phone that is the
//! decision that matters: a board of three hundred permanents will exhaust a
//! browser tab long before it exhausts the network.
//!
//! So the policy lives in [`baylee_client_core::images::TextureBudget`] — which
//! is pure arithmetic and unit-tested — and this module is the thin part that
//! turns its answers into handle drops.

use baylee_client_core::images::{ArtSize, ImageKey, TextureBudget, resolve};
use baylee_view::GameStatic;
use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// The web budget must stay below the desktop one, whatever either is tuned
/// to: a browser tab is killed for memory long before a native process is.
/// Checked at compile time so a careless tuning pass cannot invert it.
const _: () = assert!(
    baylee_client_core::images::MOBILE_BUDGET_BYTES
        < baylee_client_core::images::DESKTOP_BUDGET_BYTES
);

/// How much decoded texture the client may hold.
///
/// The web and mobile figure is deliberately conservative: a browser tab that
/// is killed for memory takes the match with it, and a player would much rather
/// see a card pop in a frame late than lose a game.
#[must_use]
pub fn default_budget_bytes() -> usize {
    if cfg!(target_arch = "wasm32") {
        baylee_client_core::images::MOBILE_BUDGET_BYTES
    } else {
        baylee_client_core::images::DESKTOP_BUDGET_BYTES
    }
}

/// Creates the texture cache at startup.
pub fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    config: Res<crate::DuelConfig>,
) {
    let cache = CardTextures::new(&mut images, config.texture_budget);
    commands.insert_resource(cache);
}

/// Why a printing's art will not arrive.
///
/// The distinction exists because exactly one of the two is temporary, and
/// treating them alike cost the player a permanently blank card: a seat *earns*
/// print entries as it sees cards (`GameStatic.prints` is a list of holes that
/// fill in), so a card asked for before its entry arrived is unresolvable now
/// and perfectly resolvable a moment later.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Failure {
    /// The print table had no entry for this printing, or the entry carried no
    /// URL at this size. Cleared by [`CardTextures::forget_unresolved`] when a
    /// new print table arrives.
    Unresolved,
    /// The fetch itself failed, and how many times it has been tried.
    ///
    /// This was once a permanent answer, on the reasoning that a URL which
    /// answered with nothing would answer the same way next time. That is
    /// measurably false: a run of the offline table recorded two of these,
    /// and curling all 194 printings of both decks against the CDN returned
    /// 200 for every one of them. The failures were transient — and a
    /// transient failure that is never retried is a card drawn blank for the
    /// rest of the game, which is exactly the bug this enum was added to fix.
    ///
    /// So it is retried, [`LOAD_TRIES`] times, and only then believed. The
    /// count is what keeps a genuine 404 from becoming a loop.
    Load(u8),
}

/// How many times a failed fetch is tried again before the client accepts that
/// the card has no art.
///
/// Three attempts spaced [`RETRY_AFTER`] apart covers a hiccup at either end
/// without costing a missing printing more than three requests in a game.
const LOAD_TRIES: u8 = 3;

/// How long a failed fetch is left alone before being tried again.
///
/// Long enough that a network that is down stays cheap, short enough that a
/// player who saw a card come up blank sees it fill in rather than reading it
/// as the way the client looks.
const RETRY_AFTER: f32 = 4.0;

/// Card art held by the client.
#[derive(Resource)]
pub struct CardTextures {
    budget: TextureBudget,
    handles: HashMap<ImageKey, Handle<Image>>,
    /// Drawn whenever the real art is missing, unknown, or still loading.
    card_back: Handle<Image>,
    /// Requests issued this frame, for diagnostics and tests.
    issued: usize,
    /// Printings whose art will not arrive, and why.
    ///
    /// A printing with no artwork at the requested size, a 404, or a client
    /// with no network all end here, and every one of them is a card the
    /// player would otherwise see as a blank rectangle. The constructed face
    /// takes over for exactly these.
    ///
    /// The reason is kept because one of the two is temporary — see
    /// [`Failure`], and [`Self::forget_unresolved`] for the way out. This set
    /// had no way out at all, which is how an opponent's land could stay a
    /// blank rectangle for the rest of the game.
    failed: HashMap<ImageKey, Failure>,
    /// Printings whose bytes are actually on the GPU.
    ///
    /// Holding a `Handle<Image>` is not the same as having the image, and the
    /// difference is visible: `art` is an `Option<Handle<Image>>` in an
    /// `AsBindGroup`, so `None` binds the fallback texture but a `Some` whose
    /// bytes have not arrived makes the whole material fail to prepare — and a
    /// card with no prepared material is not drawn at all. Until a key is in
    /// here it is, for drawing purposes, a card with no art.
    arrived: HashSet<ImageKey>,
    /// Bumped whenever `arrived` or `failed` changes — see [`Self::epoch`].
    epoch: u64,
}

impl CardTextures {
    /// Builds the cache and its placeholder texture.
    pub fn new(images: &mut Assets<Image>, budget_bytes: usize) -> Self {
        Self {
            budget: TextureBudget::new(budget_bytes),
            handles: HashMap::new(),
            card_back: images.add(solid_texture([26, 30, 38, 255])),
            issued: 0,
            failed: HashMap::new(),
            arrived: HashSet::new(),
            epoch: 0,
        }
    }

    /// Whether this printing's art is known not to be coming.
    #[must_use]
    pub fn has_failed(&self, key: ImageKey) -> bool {
        self.failed.contains_key(&key)
    }

    /// Records a printing whose art will not arrive, and why.
    ///
    /// The load-state sweep is the normal caller; a test needs it too, because
    /// there is no way to fail a load without a network.
    pub fn mark_failed(&mut self, key: ImageKey, why: Failure) {
        if self.failed.insert(key, why).is_none() {
            self.epoch += 1;
        }
    }

    /// The failed fetches worth another attempt, with the count each is on.
    ///
    /// Separate from the system that calls it so the policy — which failures
    /// come back, and how many times — is answered without a window.
    #[must_use]
    pub fn due_for_retry(&self) -> Vec<(ImageKey, u8)> {
        self.failed
            .iter()
            .filter_map(|(key, why)| match why {
                Failure::Load(tried) if *tried < LOAD_TRIES => Some((*key, *tried)),
                _ => None,
            })
            .collect()
    }

    /// Issues another attempt at one failed fetch.
    ///
    /// Dropping the handle is the part that matters: [`Self::get`] returns
    /// early for a key it already holds, so nothing would ask again while the
    /// dead handle is still in the map. The key stays in `failed` until the art
    /// actually lands, so the card goes on drawing its face in the meantime
    /// rather than flickering once per sweep.
    pub fn retry(&mut self, key: ImageKey) {
        let tried = match self.failed.get(&key) {
            Some(Failure::Load(n)) => *n,
            _ => return,
        };
        self.failed.insert(key, Failure::Load(tried + 1));
        self.handles.remove(&key);
    }

    /// Forgets every printing that failed only because the print table had no
    /// entry for it yet.
    ///
    /// Called when a new print table arrives, which is the one event that can
    /// change the answer: a seat is entitled to its own deck's printings and
    /// earns the rest by seeing the cards, so the gamehost re-sends the payload
    /// as entries are earned. Without this the first ask poisoned the key
    /// permanently — the card the entry finally described went on drawing as a
    /// blank rectangle, and the constructed face it fell back to was the only
    /// reason anyone could tell what it was.
    ///
    /// A load failure is kept here, because a new print table says nothing
    /// about a fetch that already failed — that one has its own way back, on a
    /// timer rather than on an event. See [`retry_failed_loads`].
    pub fn forget_unresolved(&mut self) {
        let before = self.failed.len();
        self.failed.retain(|_, why| matches!(why, Failure::Load(_)));
        if self.failed.len() != before {
            self.epoch += 1;
        }
    }

    /// Whether this printing's art is on the GPU and can be drawn.
    ///
    /// False for a key nobody has asked for yet, false while the load is in
    /// flight, false forever for one that failed, and false again if the
    /// budget evicted it — every case in which asking for the art would draw
    /// nothing.
    #[must_use]
    pub fn has_arrived(&self, key: ImageKey) -> bool {
        self.arrived.contains(&key)
    }

    /// Records a printing whose art is now on the GPU.
    ///
    /// The load-state sweep is the normal caller; a test needs it too, because
    /// there is no way to finish a load without a network.
    pub fn mark_arrived(&mut self, key: ImageKey) {
        if self.arrived.insert(key) {
            self.epoch += 1;
        }
    }

    /// How many times art has arrived, failed or been evicted.
    ///
    /// The HUD is a *retained* tree: [`hud::sync_overlay`] rebuilds it only
    /// when something in `HudRevision` changed, and a load finishing is not a
    /// new snapshot. On the table that does not matter — `sync_scene` decides
    /// per frame and the material flips to the art the frame after it lands —
    /// but a hand card would keep whatever it was built with until the
    /// opponent did something. So the gate compares this, and a counter rather
    /// than a `Changed` flag because [`Self::get`] takes `&mut self` and the
    /// table calls it every frame.
    ///
    /// Eviction counts as well, and that is the one case with a cost: a board
    /// big enough to thrash the budget bumps this every frame and rebuilds the
    /// overlay with it. It is still the right answer — a HUD card holds a
    /// *clone* of the handle, so an eviction frees nothing until the tree that
    /// holds it is rebuilt — but if a phone ever shows it, the fix is to read
    /// the counter at the end of `sync_overlay` rather than at its start.
    ///
    /// [`hud::sync_overlay`]: crate::hud::sync_overlay
    #[must_use]
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// The placeholder texture.
    #[must_use]
    pub fn card_back(&self) -> Handle<Image> {
        self.card_back.clone()
    }

    /// Bytes currently accounted for.
    #[must_use]
    pub fn used_bytes(&self) -> usize {
        self.budget.used()
    }

    /// How many textures are resident.
    #[must_use]
    pub fn resident(&self) -> usize {
        self.handles.len()
    }

    /// The texture for a key, starting a load if this is the first request.
    ///
    /// An unresolvable printing yields the card back. A load still in flight
    /// yields its handle, which is *not* drawable yet — ask [`Self::has_arrived`]
    /// before building a material out of it, or draw the constructed face
    /// instead, which is what the board does.
    pub fn get(
        &mut self,
        key: ImageKey,
        statics: &GameStatic,
        assets: &AssetServer,
    ) -> Handle<Image> {
        if let Some(handle) = self.handles.get(&key) {
            self.budget.touch(key);
            return handle.clone();
        }
        let Some(request) = resolve(statics, key) else {
            // An unresolvable printing never even becomes a request, so no
            // load state will ever report it — which is why it has to be
            // recorded here, and why this is the one failure that says so out
            // loud. The other path logs; this one did not, and a print table
            // that had not caught up yet was therefore indistinguishable from
            // a card whose art does not exist.
            if self.failed.insert(key, Failure::Unresolved).is_none() {
                self.epoch += 1;
                bevy::log::debug!(
                    ?key,
                    known = statics.print(key.print).is_some(),
                    "card art unresolvable; the print table has no URL for it yet"
                );
            }
            return self.card_back.clone();
        };
        let handle: Handle<Image> = assets.load(request.url);
        self.handles.insert(key, handle.clone());
        self.issued += 1;
        for evicted in self.budget.insert(key) {
            self.handles.remove(&evicted);
            // Dropping the handle drops the image, so the art is no longer
            // there to draw. Leaving the key in `arrived` would tell the board
            // to build a material around a texture that has to be fetched
            // again, which is the invisible card this set exists to prevent.
            if self.arrived.remove(&evicted) {
                self.epoch += 1;
            }
        }
        handle
    }

    /// Marks everything currently on screen as used, then evicts what the
    /// budget can no longer justify holding.
    ///
    /// Called once per frame with the board's own answer to what it needs, so
    /// the cache never has to guess at visibility.
    pub fn retain_visible(&mut self, visible: &[ImageKey]) {
        self.budget.touch_all(visible.iter().copied());
    }

    /// Requests are counted so a test can assert that a redraw of an unchanged
    /// board issues none.
    #[must_use]
    pub const fn issued(&self) -> usize {
        self.issued
    }

    /// Which size a card should be fetched at for a given role.
    ///
    /// The single most important memory decision in the client: the board is
    /// drawn small, and only the card a player is actually reading is fetched
    /// at a legible resolution.
    #[must_use]
    pub const fn size_for(focused: bool) -> ArtSize {
        if focused {
            ArtSize::Normal
        } else {
            ArtSize::Small
        }
    }
}

/// Notes which loads have finished and which have failed, so the renderer
/// knows which cards have art to draw.
///
/// Bevy reports both only through the asset server, and nothing asked it
/// before: a 404 left a card as an untextured rectangle for the rest of the
/// game, and a load merely in flight left one as nothing at all. Checking once
/// per frame is cheap — the map holds at most a board's worth of handles — and
/// it is the only signal that separates the three states a request can be in.
pub fn note_load_states(mut textures: ResMut<CardTextures>, assets: Res<AssetServer>) {
    let mut newly_failed: Vec<ImageKey> = Vec::new();
    let mut newly_arrived: Vec<ImageKey> = Vec::new();
    for (key, handle) in &textures.handles {
        match assets.get_load_state(handle) {
            Some(bevy::asset::LoadState::Failed(_)) if !textures.failed.contains_key(key) => {
                newly_failed.push(*key);
            }
            Some(bevy::asset::LoadState::Loaded) if !textures.arrived.contains(key) => {
                newly_arrived.push(*key);
            }
            _ => {}
        }
    }
    for key in newly_failed {
        bevy::log::debug!(
            ?key,
            "card art failed to load; falling back to the card face"
        );
        textures.failed.insert(key, Failure::Load(1));
        textures.epoch += 1;
    }
    for key in newly_arrived {
        textures.arrived.insert(key);
        // A retry that landed. Without this the art would be on the GPU and
        // the card would go on drawing its constructed face, because
        // `has_failed` is the question the board asks first.
        textures.failed.remove(&key);
        textures.epoch += 1;
    }
}

/// Tries failed fetches again, a few times, slowly.
///
/// The measurement that made this necessary: an offline table recorded two
/// load failures while all 194 printings of both decks answered `200` to a
/// plain `curl`. A fetch can simply not land, and until this existed that card
/// was blank for the rest of the game — the client had recorded an opinion
/// about the network and never revisited it.
///
/// Dropping the handle is what actually re-fetches: [`CardTextures::get`]
/// returns early for a key it already holds, so the key has to stop being held
/// before anything will ask for it again. The entry stays in `failed`
/// meanwhile, so the card keeps drawing its face until the art really arrives
/// — `note_load_states` is what clears it, on success.
pub fn retry_failed_loads(
    mut textures: ResMut<CardTextures>,
    time: Res<Time>,
    mut next_sweep: Local<f32>,
) {
    let now = time.elapsed_secs();
    if now < *next_sweep {
        return;
    }
    *next_sweep = now + RETRY_AFTER;
    for (key, tried) in textures.due_for_retry() {
        textures.retry(key);
        bevy::log::debug!(?key, attempt = tried + 1, "retrying a card image");
    }
}

/// Background image warming: up to 15 loads in flight, priority-ordered
/// (hand → command zones → battlefield → the rest of the print table,
/// deterministically shuffled). Anything the renderer asks for directly
/// jumps the queue by loading immediately — this only fills ahead.
#[derive(Resource, Default)]
pub struct Preload {
    /// Every key ever queued, so the sweep can run again without asking twice.
    queued: HashSet<ImageKey>,
    /// How many print entries this seat held when the print table was last
    /// swept.
    ///
    /// A seat is entitled to its own deck's printings and earns the rest by
    /// seeing the cards, so this grows during a game. The sweep used to run
    /// exactly once — on the first frame a view existed, when the battlefield
    /// is empty and every printing but the player's own deck is still a hole —
    /// and every card earned after that was never warmed at all.
    swept: usize,
    in_flight: Vec<Handle<Image>>,
    queue: std::collections::VecDeque<ImageKey>,
}

impl Preload {
    /// Queues a key, unless it has been queued before.
    fn want(&mut self, key: ImageKey) {
        if self.queued.insert(key) {
            self.queue.push_back(key);
        }
    }
}

/// How many image loads may be in flight at once.
const PRELOAD_PARALLEL: usize = 15;

/// Builds and drains the preload queue.
pub fn drive_preloads(
    mut preload: ResMut<Preload>,
    duel: Res<crate::Duel>,
    mut textures: ResMut<CardTextures>,
    assets: Res<AssetServer>,
) {
    if let (Some(statics), Some(view)) = (duel.statics.as_ref(), duel.view.as_ref()) {
        // P1: the local hand, every command zone, the whole battlefield —
        // everything a player is looking at *now*. Swept every frame rather
        // than once: on the first frame a view exists the battlefield is
        // empty, so a queue built there and never rebuilt holds nothing a
        // player will be looking at a minute later. `want` makes the repeat
        // free.
        for h in &view.hand {
            preload.want(ImageKey::new(h.card.print, h.card.face, ArtSize::Small));
        }
        for cmds in &view.command {
            for o in cmds {
                if let Some(c) = o.card {
                    preload.want(ImageKey::new(c.print, c.face, ArtSize::Small));
                }
            }
        }
        for o in &view.battlefield {
            if let Some(c) = o.card {
                preload.want(ImageKey::new(c.print, c.face, ArtSize::Small));
            }
        }
        // P2: the same cards at the size the hover preview reads them at.
        //
        // The preview rewrites whatever key it was handed to `ArtSize::Normal`
        // (`hud::overlay`), and it does so at the moment the pointer arrives —
        // far too late to fetch. That is why a card a player could already see
        // in their hand still flashed the constructed face when they looked at
        // it: the hand draws `Small`, and the two are different keys.
        //
        // Only the hand and the *local* command zone, which are the two zones
        // the preview can point at that the rules keep small — a hand is about
        // seven cards (CR 514.1) and a command zone at most a pair. A
        // battlefield has no such bound, and queueing eighty permanents at
        // 1.3 MB each would spend the whole mobile budget on a convenience and
        // then thrash it. Hovering a permanent still fetches on the spot; it
        // is one image, and the face it falls back to meanwhile is correct.
        for h in &view.hand {
            preload.want(ImageKey::new(h.card.print, h.card.face, ArtSize::Normal));
        }
        if let Some(cmds) = view.command.get(view.seat.get() as usize) {
            for o in cmds {
                if let Some(c) = o.card {
                    preload.want(ImageKey::new(c.print, c.face, ArtSize::Normal));
                }
            }
        }
        // P3: the rest of the print table, deterministically shuffled
        // (xorshift*, fixed seed — same order on every client). Re-swept
        // whenever this seat has earned entries it did not have last time,
        // which is the only event that can add to it.
        let known = statics.prints.iter().flatten().count();
        if known != preload.swept {
            preload.swept = known;
            let mut rest: Vec<ImageKey> = (0..statics.prints.len())
                .map(|i| baylee_core::ids::PrintRef::new(i as u16))
                // A hole in the print table is a card this seat has not been
                // shown. Preloading it would be fetching the art of a card the
                // player is not entitled to know is in the game at all.
                .filter(|print| statics.print(*print).is_some())
                .map(|print| ImageKey::new(print, 0, ArtSize::Small))
                .filter(|k| !preload.queued.contains(k))
                .collect();
            let mut s = 0x9e37_79b9_7f4a_7c15u64;
            for i in (1..rest.len()).rev() {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                let j = (s % (i as u64 + 1)) as usize;
                rest.swap(i, j);
            }
            for key in rest {
                preload.want(key);
            }
        }
    }

    // Retire finished loads.
    preload.in_flight.retain(|h| {
        !matches!(
            assets.get_load_state(h),
            Some(bevy::asset::LoadState::Loaded | bevy::asset::LoadState::Failed(_))
        )
    });

    // Keep the pipe full.
    let Some(statics) = duel.statics.as_ref() else {
        return;
    };
    while preload.in_flight.len() < PRELOAD_PARALLEL {
        let Some(key) = preload.queue.pop_front() else {
            break;
        };
        let handle = textures.get(key, statics, &assets);
        preload.in_flight.push(handle);
    }
}

/// A 1×1 texture of a solid colour, used for placeholders and table felt.
fn solid_texture(rgba: [u8; 4]) -> Image {
    Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::images::ArtSize;

    #[test]
    fn the_board_asks_for_cheap_art_and_only_focus_asks_for_readable_art() {
        assert_eq!(CardTextures::size_for(false), ArtSize::Small);
        assert_eq!(CardTextures::size_for(true), ArtSize::Normal);
    }

    /// The signal the retained HUD redraws on.
    ///
    /// `hud::sync_overlay` rebuilds only when something in `HudRevision`
    /// changed, and it compares [`CardTextures::epoch`] for exactly this: what
    /// a card should draw — its art or its constructed face — changes when a
    /// load lands or fails, and neither of those is a new snapshot. A counter
    /// that stood still would leave a cold-cache hand showing text faces until
    /// the opponent did something; one that moved for nothing would rebuild
    /// two hundred rows a frame.
    #[test]
    fn the_epoch_moves_when_what_a_card_can_draw_does_and_not_otherwise() {
        let mut images = Assets::<Image>::default();
        let mut textures = CardTextures::new(&mut images, default_budget_bytes());
        let art = ImageKey::new(baylee_core::ids::PrintRef::new(0), 0, ArtSize::Small);
        let lost = ImageKey::new(baylee_core::ids::PrintRef::new(1), 0, ArtSize::Small);

        let start = textures.epoch();
        textures.mark_arrived(art);
        let arrived = textures.epoch();
        assert!(arrived > start, "art landing is a redraw");

        textures.mark_arrived(art);
        assert_eq!(
            textures.epoch(),
            arrived,
            "the same art landing twice is not a second redraw"
        );

        textures.mark_failed(lost, Failure::Load(1));
        assert!(
            textures.epoch() > arrived,
            "a load giving up is a redraw too — that card switches to its face"
        );
    }

    /// A seat earns print entries as it sees cards, so a printing that could
    /// not be resolved a moment ago can be resolvable now. Nothing said so:
    /// `failed` had no way out, and an opponent's land whose entry arrived a
    /// frame after the permanent did stayed a blank rectangle for the rest of
    /// the game.
    ///
    /// The two failures have to part company here or the fix would be worse
    /// than the bug — retrying every 404 on every earned printing is a fetch
    /// storm, and a URL that answered with nothing will answer the same way
    /// however many print tables arrive.
    #[test]
    fn a_new_print_table_forgives_the_unresolved_and_not_the_unreachable() {
        let mut images = Assets::<Image>::default();
        let mut textures = CardTextures::new(&mut images, default_budget_bytes());
        let early = ImageKey::new(baylee_core::ids::PrintRef::new(0), 0, ArtSize::Small);
        let gone = ImageKey::new(baylee_core::ids::PrintRef::new(1), 0, ArtSize::Small);

        textures.mark_failed(early, Failure::Unresolved);
        textures.mark_failed(gone, Failure::Load(1));
        assert!(textures.has_failed(early) && textures.has_failed(gone));

        let before = textures.epoch();
        textures.forget_unresolved();
        assert!(
            !textures.has_failed(early),
            "the print table caught up, so the card gets another chance"
        );
        assert!(
            textures.has_failed(gone),
            "a 404 is still a 404 after a new print table"
        );
        assert!(
            textures.epoch() > before,
            "those cards can draw something else now, so the HUD has to rebuild"
        );

        let steady = textures.epoch();
        textures.forget_unresolved();
        assert_eq!(
            textures.epoch(),
            steady,
            "a print table that forgives nothing is not a redraw"
        );
    }

    /// The measurement this exists for: an offline table recorded two load
    /// failures in one game while all 194 printings of both decks answered
    /// `200` to a plain `curl`. A fetch can simply not land — so treating the
    /// first failure as the final answer blanks a card for the whole game,
    /// which is the bug the player actually reported.
    ///
    /// It has to stop, though. A printing whose art really is gone must cost a
    /// bounded number of requests, not one every four seconds forever.
    #[test]
    fn a_failed_fetch_is_tried_again_a_few_times_and_then_believed() {
        let mut images = Assets::<Image>::default();
        let mut textures = CardTextures::new(&mut images, default_budget_bytes());
        let flaky = ImageKey::new(baylee_core::ids::PrintRef::new(0), 0, ArtSize::Small);
        let absent = ImageKey::new(baylee_core::ids::PrintRef::new(1), 0, ArtSize::Small);

        textures.mark_failed(flaky, Failure::Load(1));
        textures.mark_failed(absent, Failure::Unresolved);

        // An unresolvable printing is not a fetch that failed, and retrying it
        // would be asking the network about a decision the print table makes.
        assert_eq!(
            textures.due_for_retry(),
            vec![(flaky, 1)],
            "only a failed fetch comes back, and only with its own count"
        );

        // Each sweep spends one attempt, and the card keeps drawing its face
        // throughout — a key that left `failed` between sweeps would flicker.
        for attempt in 1..LOAD_TRIES {
            assert_eq!(textures.due_for_retry(), vec![(flaky, attempt)]);
            textures.retry(flaky);
            assert!(
                textures.has_failed(flaky),
                "a retry in flight is still a card with no art to draw"
            );
        }
        assert!(
            textures.due_for_retry().is_empty(),
            "after {LOAD_TRIES} attempts the client believes the art is gone"
        );

        // And a retry that lands clears it, which is `note_load_states`' job —
        // done here by hand, because there is no way to finish a load without a
        // network.
        textures.mark_arrived(flaky);
        textures.failed.remove(&flaky);
        assert!(!textures.has_failed(flaky));
    }
}
