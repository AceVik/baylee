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

/// Card art held by the client.
#[derive(Resource)]
pub struct CardTextures {
    budget: TextureBudget,
    handles: HashMap<ImageKey, Handle<Image>>,
    /// Drawn whenever the real art is missing, unknown, or still loading.
    card_back: Handle<Image>,
    /// Requests issued this frame, for diagnostics and tests.
    issued: usize,
    /// Printings whose art will not arrive.
    ///
    /// A printing with no artwork at the requested size, a 404, or a client
    /// with no network all end here, and every one of them is a card the
    /// player would otherwise see as a blank rectangle. The constructed face
    /// takes over for exactly these.
    failed: HashSet<ImageKey>,
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
            failed: HashSet::new(),
            arrived: HashSet::new(),
            epoch: 0,
        }
    }

    /// Whether this printing's art is known not to be coming.
    #[must_use]
    pub fn has_failed(&self, key: ImageKey) -> bool {
        self.failed.contains(&key)
    }

    /// Records a printing whose art will not arrive.
    ///
    /// The load-state sweep is the normal caller; a test needs it too, because
    /// there is no way to fail a load without a network.
    pub fn mark_failed(&mut self, key: ImageKey) {
        if self.failed.insert(key) {
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
            // load state will ever report it; it is a permanent failure the
            // moment it is asked for.
            if self.failed.insert(key) {
                self.epoch += 1;
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
            Some(bevy::asset::LoadState::Failed(_)) if !textures.failed.contains(key) => {
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
        textures.failed.insert(key);
        textures.epoch += 1;
    }
    for key in newly_arrived {
        textures.arrived.insert(key);
        textures.epoch += 1;
    }
}

/// Background image warming: up to 15 loads in flight, priority-ordered
/// (hand → command zones → battlefield → the rest of the print table,
/// deterministically shuffled). Anything the renderer asks for directly
/// jumps the queue by loading immediately — this only fills ahead.
#[derive(Resource, Default)]
pub struct Preload {
    started: bool,
    in_flight: Vec<Handle<Image>>,
    queue: std::collections::VecDeque<ImageKey>,
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
    if !preload.started {
        let (Some(statics), Some(view)) = (duel.statics.as_ref(), duel.view.as_ref()) else {
            return;
        };
        preload.started = true;
        let mut queue = std::collections::VecDeque::new();
        // P1: the local hand, every command zone, the whole battlefield —
        // everything a player sees in the first minute.
        for h in &view.hand {
            queue.push_back(ImageKey::new(h.card.print, h.card.face, ArtSize::Small));
        }
        for cmds in &view.command {
            for o in cmds {
                if let Some(c) = o.card {
                    queue.push_back(ImageKey::new(c.print, c.face, ArtSize::Small));
                }
            }
        }
        for o in &view.battlefield {
            if let Some(c) = o.card {
                queue.push_back(ImageKey::new(c.print, c.face, ArtSize::Small));
            }
        }
        let seen: bevy::platform::collections::HashSet<ImageKey> = queue.iter().copied().collect();
        // P2: the rest of the print table, deterministically shuffled
        // (xorshift*, fixed seed — same order on every client).
        let mut rest: Vec<ImageKey> = (0..statics.prints.len())
            .map(|i| baylee_core::ids::PrintRef::new(i as u16))
            // A hole in the print table is a card this seat has not been
            // shown. Preloading it would be fetching the art of a card the
            // player is not entitled to know is in the game at all.
            .filter(|print| statics.print(*print).is_some())
            .map(|print| ImageKey::new(print, 0, ArtSize::Small))
            .filter(|k| !seen.contains(k))
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
            queue.push_back(key);
        }
        preload.queue = queue;
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

        textures.mark_failed(lost);
        assert!(
            textures.epoch() > arrived,
            "a load giving up is a redraw too — that card switches to its face"
        );
    }
}
