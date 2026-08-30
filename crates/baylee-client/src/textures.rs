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
use bevy::platform::collections::HashMap;
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
}

impl CardTextures {
    /// Builds the cache and its placeholder texture.
    pub fn new(images: &mut Assets<Image>, budget_bytes: usize) -> Self {
        Self {
            budget: TextureBudget::new(budget_bytes),
            handles: HashMap::new(),
            card_back: images.add(solid_texture([26, 30, 38, 255])),
            issued: 0,
        }
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
    /// Always returns something drawable: an unresolvable printing or a load
    /// still in flight yields the card back rather than a hole in the table.
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
            return self.card_back.clone();
        };
        let handle: Handle<Image> = assets.load(request.url);
        self.handles.insert(key, handle.clone());
        self.issued += 1;
        for evicted in self.budget.insert(key) {
            self.handles.remove(&evicted);
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
}
