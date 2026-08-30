//! Card image resolution and cache accounting — policy only, no I/O.
//!
//! # Why this is a separate, I/O-free layer
//!
//! Card art is by far the largest thing a client holds. A commander table with
//! eight seats can easily show 300 permanents; at full card resolution that is
//! several hundred megabytes of decoded texture, which no browser tab and no
//! phone will survive. The rules that keep it bounded — which resolution a
//! card gets, when a texture may be dropped — are decisions, not plumbing, so
//! they live here where they can be tested without a network or a GPU.
//!
//! The renderer asks this module *what* it needs and *what it may drop*; the
//! transport layer does the fetching.

use baylee_core::ids::PrintRef;
use baylee_view::{Finish, GameStatic, PrintEntry};
use std::collections::HashMap;

/// Which rendition of a card image is wanted.
///
/// The distinction is the client's main memory lever: board cards are drawn at
/// a few hundred pixels and never need more.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ArtSize {
    /// 146×204 — board, hand, and grouped stacks. The default everywhere.
    Small,
    /// 488×680 — the focused card, the stack, and hover previews.
    Normal,
    /// 626×457 crop of the illustration only, for wide summary banners.
    ArtCrop,
}

impl ArtSize {
    /// The path segment Scryfall serves this size under.
    #[must_use]
    pub const fn path_segment(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Normal => "normal",
            Self::ArtCrop => "art_crop",
        }
    }

    /// Pixel dimensions Scryfall guarantees for this size.
    #[must_use]
    pub const fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Small => (146, 204),
            Self::Normal => (488, 680),
            Self::ArtCrop => (626, 457),
        }
    }

    /// Decoded RGBA footprint in bytes.
    ///
    /// Budgeting against the *decoded* size rather than the download size is
    /// the whole point: a 40 KB JPEG still costs 1.3 MB of VRAM once uploaded.
    #[must_use]
    pub const fn decoded_bytes(self) -> usize {
        let (w, h) = self.dimensions();
        (w as usize) * (h as usize) * 4
    }
}

/// Which face of a double-faced card.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Face {
    /// The front face; also the only face of a single-faced card.
    Front,
    /// The back face of a transforming or modal double-faced card.
    Back,
}

impl Face {
    /// Maps the engine's face index onto a Scryfall face.
    #[must_use]
    pub const fn from_index(index: u8) -> Self {
        if index == 0 { Self::Front } else { Self::Back }
    }

    /// The path segment Scryfall serves this face under.
    #[must_use]
    pub const fn path_segment(self) -> &'static str {
        match self {
            Self::Front => "front",
            Self::Back => "back",
        }
    }
}

/// A compact, copyable cache key.
///
/// Deliberately not string-keyed: a table holds thousands of these and a
/// `String` per key would cost more than some of the images. The print table
/// in [`GameStatic`] turns it back into a URL when one is actually needed.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ImageKey {
    /// Index into the game's print table.
    pub print: PrintRef,
    /// Which face.
    pub face: Face,
    /// Which rendition.
    pub size: ArtSize,
}

impl ImageKey {
    /// Builds a key for a card face at a size.
    #[must_use]
    pub const fn new(print: PrintRef, face_index: u8, size: ArtSize) -> Self {
        Self {
            print,
            face: Face::from_index(face_index),
            size,
        }
    }

    /// The same card at a different size — used when a card gains focus and
    /// its low-resolution texture must be upgraded.
    #[must_use]
    pub const fn at(self, size: ArtSize) -> Self {
        Self { size, ..self }
    }
}

/// How a printing should be tinted or overlaid once its art is drawn.
///
/// Finish is a *treatment*, not a different image: Scryfall serves one file per
/// printing, and foiling is a shader pass on top. Modelling it separately stops
/// the cache from storing the same bytes twice for a foil and a non-foil copy.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum FinishTreatment {
    /// No overlay.
    #[default]
    Plain,
    /// Animated rainbow sheen.
    Foil,
    /// Etched foil: a duller, engraved sheen.
    Etched,
}

impl From<Finish> for FinishTreatment {
    fn from(f: Finish) -> Self {
        match f {
            Finish::Normal => Self::Plain,
            Finish::Foil => Self::Foil,
            Finish::Etched => Self::Etched,
        }
    }
}

/// Everything the renderer needs to display one card image.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ImageRequest {
    /// Cache key.
    pub key: ImageKey,
    /// Absolute URL to fetch.
    pub url: String,
    /// Overlay treatment for this printing.
    pub treatment: FinishTreatment,
}

/// Base URL of the Scryfall image CDN.
///
/// Images are served straight from the CDN by printing id, so a client never
/// has to call the Scryfall API to render a board — which also means no rate
/// limit applies to gameplay.
const CDN: &str = "https://cards.scryfall.io";

/// Builds the CDN URL for a printing.
///
/// Returns `None` when the printing id is not a plausible Scryfall id; the
/// caller renders a card back rather than issuing a request that will 404.
#[must_use]
pub fn image_url(entry: &PrintEntry, face: Face, size: ArtSize) -> Option<String> {
    let id = entry.scryfall_id.as_str();
    // Scryfall shards by the first two characters of the id.
    let mut chars = id.chars();
    let a = chars.next()?;
    let b = chars.next()?;
    if !id.contains('-') || id.len() < 32 {
        return None;
    }
    Some(format!(
        "{CDN}/{}/{}/{a}/{b}/{id}.jpg",
        size.path_segment(),
        face.path_segment()
    ))
}

/// Resolves a key against the game's print table into a fetchable request.
#[must_use]
pub fn resolve(statics: &GameStatic, key: ImageKey) -> Option<ImageRequest> {
    let entry = statics.print(key.print)?;
    Some(ImageRequest {
        key,
        url: image_url(entry, key.face, key.size)?,
        treatment: entry.finish.into(),
    })
}

// ------------------------------------------------------------------- budget

/// Default texture budget for a desktop client.
pub const DESKTOP_BUDGET_BYTES: usize = 256 * 1024 * 1024;
/// Default texture budget for a browser or mobile client, where the whole tab
/// may be capped well below a native process.
pub const MOBILE_BUDGET_BYTES: usize = 96 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
struct Entry {
    bytes: usize,
    last_used: u64,
    pinned: bool,
}

/// A least-recently-used texture budget.
///
/// Tracks what the renderer holds and decides what to drop when the budget is
/// exceeded. It never owns pixels — the renderer owns those and acts on the
/// eviction list — which is what keeps this testable.
///
/// Eviction is deterministic: ties on last-use are broken by key, so two
/// clients in the same state make the same decision. That matters less for
/// correctness than for reproducing a memory bug from a report.
#[derive(Debug)]
pub struct TextureBudget {
    entries: HashMap<ImageKey, Entry>,
    budget: usize,
    used: usize,
    clock: u64,
}

impl TextureBudget {
    /// A budget holding at most `budget` bytes of decoded texture.
    #[must_use]
    pub fn new(budget: usize) -> Self {
        Self {
            entries: HashMap::new(),
            budget,
            used: 0,
            clock: 0,
        }
    }

    /// Bytes currently accounted for.
    #[must_use]
    pub const fn used(&self) -> usize {
        self.used
    }

    /// The configured budget.
    #[must_use]
    pub const fn budget(&self) -> usize {
        self.budget
    }

    /// How many textures are held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether a texture is already resident.
    #[must_use]
    pub fn contains(&self, key: ImageKey) -> bool {
        self.entries.contains_key(&key)
    }

    /// Marks a texture as used now, so it survives the next eviction.
    pub fn touch(&mut self, key: ImageKey) {
        self.clock += 1;
        let clock = self.clock;
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_used = clock;
        }
    }

    /// Marks every currently visible texture as used in one pass.
    pub fn touch_all(&mut self, keys: impl IntoIterator<Item = ImageKey>) {
        for key in keys {
            self.touch(key);
        }
    }

    /// Pins a texture so it is never evicted.
    ///
    /// Used for the card backs and the placeholder: evicting those would make
    /// the client flicker precisely when it is already under memory pressure.
    pub fn pin(&mut self, key: ImageKey) {
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.pinned = true;
        }
    }

    /// Records a newly decoded texture and returns everything that must be
    /// dropped to stay inside the budget, least-recently-used first.
    ///
    /// The newly inserted texture is never itself evicted — a client that just
    /// decoded an image is about to draw it.
    pub fn insert(&mut self, key: ImageKey) -> Vec<ImageKey> {
        self.clock += 1;
        let bytes = key.size.decoded_bytes();
        if let Some(existing) = self.entries.insert(
            key,
            Entry {
                bytes,
                last_used: self.clock,
                pinned: false,
            },
        ) {
            self.used -= existing.bytes;
        }
        self.used += bytes;
        self.evict_down_to_budget(key)
    }

    /// Drops a texture the renderer no longer holds.
    pub fn remove(&mut self, key: ImageKey) {
        if let Some(entry) = self.entries.remove(&key) {
            self.used -= entry.bytes;
        }
    }

    /// Everything currently held, for diagnostics.
    pub fn keys(&self) -> impl Iterator<Item = ImageKey> + '_ {
        self.entries.keys().copied()
    }

    fn evict_down_to_budget(&mut self, protect: ImageKey) -> Vec<ImageKey> {
        if self.used <= self.budget {
            return Vec::new();
        }
        let mut candidates: Vec<(u64, ImageKey)> = self
            .entries
            .iter()
            .filter(|(k, e)| **k != protect && !e.pinned)
            .map(|(k, e)| (e.last_used, *k))
            .collect();
        // Oldest first; the key breaks ties so eviction is reproducible.
        candidates.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

        let mut evicted = Vec::new();
        for (_, key) in candidates {
            if self.used <= self.budget {
                break;
            }
            if let Some(entry) = self.entries.remove(&key) {
                self.used -= entry.bytes;
                evicted.push(key);
            }
        }
        evicted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PlayerId;

    fn statics() -> GameStatic {
        GameStatic {
            view_version: baylee_view::VIEW_VERSION,
            game_id: "g".into(),
            your_seat: PlayerId::new(0),
            seats: vec![],
            prints: vec![
                PrintEntry {
                    scryfall_id: "f333ea01-124f-4125-87ab-609be40e774c".into(),
                    lang: "EN".into(),
                    finish: Finish::Normal,
                },
                PrintEntry {
                    scryfall_id: "1ed4c0bb-b710-44a1-b8bc-6bd11c27b8b8".into(),
                    lang: "DE".into(),
                    finish: Finish::Foil,
                },
                PrintEntry {
                    scryfall_id: "not-an-id".into(),
                    lang: "EN".into(),
                    finish: Finish::Normal,
                },
            ],
        }
    }

    fn key(print: u16, size: ArtSize) -> ImageKey {
        ImageKey::new(PrintRef::new(print), 0, size)
    }

    #[test]
    fn url_follows_the_scryfall_cdn_sharding_scheme() {
        let s = statics();
        let req = resolve(&s, key(0, ArtSize::Small)).expect("resolves");
        assert_eq!(
            req.url,
            "https://cards.scryfall.io/small/front/f/3/f333ea01-124f-4125-87ab-609be40e774c.jpg"
        );
        assert_eq!(req.treatment, FinishTreatment::Plain);
    }

    #[test]
    fn back_faces_and_sizes_select_different_paths() {
        let s = statics();
        let back = ImageKey::new(PrintRef::new(0), 1, ArtSize::Normal);
        let req = resolve(&s, back).expect("resolves");
        assert!(req.url.contains("/normal/back/"));
    }

    #[test]
    fn finish_travels_as_a_treatment_not_a_separate_image() {
        let s = statics();
        let foil = resolve(&s, key(1, ArtSize::Small)).expect("resolves");
        let plain = resolve(&s, key(0, ArtSize::Small)).expect("resolves");
        assert_eq!(foil.treatment, FinishTreatment::Foil);
        // Same size and face, different printings: different files.
        assert_ne!(foil.url, plain.url);
    }

    #[test]
    fn an_implausible_printing_id_yields_no_request() {
        let s = statics();
        assert!(resolve(&s, key(2, ArtSize::Small)).is_none());
        assert!(resolve(&s, key(99, ArtSize::Small)).is_none());
    }

    #[test]
    fn small_art_is_an_order_of_magnitude_cheaper_than_full_size() {
        // The reason the board never requests `Normal`.
        assert!(ArtSize::Normal.decoded_bytes() > 10 * ArtSize::Small.decoded_bytes());
        assert_eq!(ArtSize::Small.decoded_bytes(), 146 * 204 * 4);
    }

    #[test]
    fn budget_evicts_least_recently_used_first() {
        // Room for exactly three small textures.
        let mut budget = TextureBudget::new(ArtSize::Small.decoded_bytes() * 3);
        for i in 0..3 {
            assert!(budget.insert(key(i, ArtSize::Small)).is_empty());
        }
        assert_eq!(budget.len(), 3);

        // Re-use the two newest, leaving print 0 as the coldest.
        budget.touch(key(1, ArtSize::Small));
        budget.touch(key(2, ArtSize::Small));

        let evicted = budget.insert(key(3, ArtSize::Small));
        assert_eq!(evicted, vec![key(0, ArtSize::Small)]);
        assert!(!budget.contains(key(0, ArtSize::Small)));
        assert!(budget.contains(key(3, ArtSize::Small)));
        assert!(budget.used() <= budget.budget());
    }

    #[test]
    fn a_freshly_inserted_texture_is_never_evicted_to_make_room_for_itself() {
        // Budget smaller than a single normal texture: the insert cannot fit,
        // but the renderer is about to draw it, so it must survive.
        let mut budget = TextureBudget::new(1);
        let k = key(0, ArtSize::Normal);
        budget.insert(k);
        assert!(budget.contains(k));
    }

    #[test]
    fn pinned_textures_survive_pressure() {
        let mut budget = TextureBudget::new(ArtSize::Small.decoded_bytes() * 2);
        let back = key(0, ArtSize::Small);
        budget.insert(back);
        budget.pin(back);
        budget.insert(key(1, ArtSize::Small));
        // Third insert must evict, but the pinned card back is off limits.
        let evicted = budget.insert(key(2, ArtSize::Small));
        assert_eq!(evicted, vec![key(1, ArtSize::Small)]);
        assert!(budget.contains(back));
    }

    #[test]
    fn re_inserting_the_same_key_does_not_double_count() {
        let mut budget = TextureBudget::new(ArtSize::Small.decoded_bytes() * 4);
        let k = key(0, ArtSize::Small);
        budget.insert(k);
        budget.insert(k);
        assert_eq!(budget.len(), 1);
        assert_eq!(budget.used(), ArtSize::Small.decoded_bytes());
    }

    #[test]
    fn upgrading_a_card_to_focus_resolution_is_a_distinct_entry() {
        let small = key(0, ArtSize::Small);
        let normal = small.at(ArtSize::Normal);
        assert_ne!(small, normal);
        assert_eq!(small.print, normal.print);

        let mut budget = TextureBudget::new(DESKTOP_BUDGET_BYTES);
        budget.insert(small);
        budget.insert(normal);
        // Both are held: the board keeps drawing the cheap one while the
        // focused card uses the expensive one.
        assert_eq!(budget.len(), 2);
    }

    #[test]
    fn removal_frees_the_accounted_bytes() {
        let mut budget = TextureBudget::new(DESKTOP_BUDGET_BYTES);
        let k = key(0, ArtSize::Normal);
        budget.insert(k);
        assert!(budget.used() > 0);
        budget.remove(k);
        assert_eq!(budget.used(), 0);
        assert!(budget.is_empty());
    }

    #[test]
    fn a_full_eight_seat_board_fits_the_mobile_budget_at_small_size() {
        // 8 seats x 40 permanents, all distinct printings.
        let mut budget = TextureBudget::new(MOBILE_BUDGET_BYTES);
        for i in 0..320u16 {
            budget.insert(key(i, ArtSize::Small));
        }
        assert_eq!(budget.len(), 320, "no eviction should have been needed");
        assert!(budget.used() < MOBILE_BUDGET_BYTES);
    }
}
