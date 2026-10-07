//! Small art loads only once a row enters its scroller's visible rectangle.
use super::{LobbyState, preview::HoverCard};
use baylee_client_core::deckbuilder::Zone;
use bevy::prelude::*;
use bevy::ui::{CalculatedClip, px};
use std::collections::{BTreeMap, VecDeque};

#[derive(Component)]
pub(crate) struct Thumbnail(String, baylee_client_core::images::FinishTreatment);
#[derive(Component)]
pub(crate) struct Quantity(pub usize, pub Zone);
#[derive(Resource, Default)]
pub(crate) struct Cache {
    images: BTreeMap<String, Handle<Image>>,
    order: VecDeque<String>,
}

pub(crate) fn spawn(commands: &mut Commands, card: &HoverCard) -> Entity {
    let mut entity = commands.spawn((
        Node {
            width: px(38),
            height: px(53),
            flex_shrink: 0.0,
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(crate::hud::palette::PANEL),
        Pickable::IGNORE,
    ));
    if let Some(url) = &card.url {
        entity.insert(Thumbnail(url.replace("/normal/", "/small/"), card.finish));
    }
    let id = entity.id();
    if let Some(url) = &card.url
        && card.finish == baylee_client_core::images::FinishTreatment::Plain
    {
        let url = url.replace("/normal/", "/small/");
        // Recycled rows get their cached image before the next layout/render,
        // rather than spending a frame as an empty placeholder.
        commands.queue(move |world: &mut World| {
            let handle = world
                .get_resource::<Cache>()
                .and_then(|cache| cache.images.get(&url))
                .cloned();
            if let Some(handle) = handle
                && let Ok(mut entity) = world.get_entity_mut(id)
            {
                entity.insert(ImageNode::new(handle));
            }
        });
    }
    id
}

#[allow(clippy::type_complexity)] // Bevy query: visible image placeholders
pub(super) fn load(
    mut commands: Commands,
    assets: Option<Res<AssetServer>>,
    mut cache: ResMut<Cache>,
    materials: Option<ResMut<crate::cardmat::UiCardMaterials>>,
    store: Option<ResMut<Assets<crate::cardmat::CardUiMaterial>>>,
    rows: Query<
        (
            Entity,
            &Thumbnail,
            &ComputedNode,
            &UiGlobalTransform,
            Option<&CalculatedClip>,
        ),
        (
            Without<ImageNode>,
            Without<MaterialNode<crate::cardmat::CardUiMaterial>>,
        ),
    >,
) {
    let Some(assets) = assets else {
        return;
    };
    let mut materials = materials;
    let mut store = store;
    let mut budget = 4;
    for (entity, thumb, node, transform, clip) in &rows {
        if node.size.min_element() <= 0.0 {
            continue;
        }
        let bounds = Rect::from_center_size(transform.translation, node.size);
        if clip.is_some_and(|clip| bounds.intersect(clip.clip).is_empty()) {
            continue;
        }
        let handle = if let Some(handle) = cache.images.get(&thumb.0) {
            handle.clone()
        } else {
            if budget == 0 {
                continue;
            }
            budget -= 1;
            let handle = assets.load(thumb.0.clone());
            if cache.images.len() >= 256
                && let Some(old) = cache.order.pop_front()
            {
                cache.images.remove(&old);
            }
            cache.order.push_back(thumb.0.clone());
            cache.images.insert(thumb.0.clone(), handle.clone());
            handle
        };
        if thumb.1 != baylee_client_core::images::FinishTreatment::Plain
            && let (Some(materials), Some(store)) = (materials.as_deref_mut(), store.as_deref_mut())
        {
            let material = materials.preview(&thumb.0, thumb.1, handle, store);
            commands.entity(entity).insert(MaterialNode(material));
        } else {
            commands.entity(entity).insert(ImageNode::new(handle));
        }
    }
}

pub(super) fn quantities(state: Res<LobbyState>, mut labels: Query<(&Quantity, &mut Text)>) {
    if !state.is_changed() {
        return;
    }
    let deck = state.lobby.builder();
    for (quantity, mut text) in &mut labels {
        let value = deck.count_of(quantity.0, quantity.1).to_string();
        if text.0 != value {
            text.0 = value;
        }
    }
}

// ------------------------------------------------------------ the art band

/// A deck's picture (the shell design, §2.4 Tile, §6): Scryfall's
/// `art_crop`, whole at its own aspect, credited beside it by the caller.
/// Nothing is ever drawn on it (#274): the node is the image and nothing
/// else.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArtBand(pub String);

/// The art bands' own cache: up to [`ART_CACHED`] pictures, oldest out
/// first, and which of them have been on screen once (those come back at
/// once; a first showing fades in).
#[derive(Resource, Default)]
pub(crate) struct ArtCache {
    images: BTreeMap<String, Handle<Image>>,
    order: VecDeque<String>,
    seen: std::collections::BTreeSet<String>,
    /// Addresses asked for ahead of the screen that shows them.
    queued: VecDeque<String>,
}

/// How many pictures the art cache holds (§10 #5).
pub(crate) const ART_CACHED: usize = 1024;

/// How many new pictures are asked for per frame, so a shelf of forty decks
/// does not ask forty at once.
const ART_PER_FRAME: usize = 4;

/// How long a first showing takes to fade in.
const ART_FADE_SECS: f32 = 0.25;

/// A picture fading in: its alpha so far.
#[derive(Component)]
pub(crate) struct ArtFade(f32);

impl ArtCache {
    fn remember(&mut self, url: &str, handle: Handle<Image>) {
        if self.images.len() >= ART_CACHED
            && let Some(old) = self.order.pop_front()
        {
            self.images.remove(&old);
            self.seen.remove(&old);
        }
        self.order.push_back(url.to_string());
        self.images.insert(url.to_string(), handle);
    }

    /// Asks for these pictures ahead of the screen that shows them (the
    /// deck list landed: Play's hero and the shelf are a press away).
    pub(crate) fn prefetch<'a>(&mut self, urls: impl IntoIterator<Item = &'a str>) {
        for url in urls {
            if !self.images.contains_key(url) && !self.queued.iter().any(|u| u == url) {
                self.queued.push_back(url.to_string());
            }
        }
    }

    /// Whether a picture has been asked for (a test's question).
    #[cfg(test)]
    pub(crate) fn holds(&self, url: &str) -> bool {
        self.images.contains_key(url)
    }
}

/// An art band's node: `width` × `height` logical pixels, the picture laid
/// in once it is loaded. A cached picture already shown once is in place
/// before the first layout, so a rebuilt tile never blinks.
pub(crate) fn art_band(commands: &mut Commands, url: &str, width: Val, height: Val) -> Entity {
    let id = commands
        .spawn((
            Node {
                width,
                height,
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px(6)),
                overflow: Overflow::clip(),
                ..default()
            },
            ArtBand(url.to_string()),
            crate::shellkit::Role::Art,
            Pickable::IGNORE,
        ))
        .id();
    let url = url.to_string();
    commands.queue(move |world: &mut World| {
        let handle = world.get_resource::<ArtCache>().and_then(|cache| {
            cache
                .seen
                .contains(&url)
                .then(|| cache.images.get(&url).cloned())
                .flatten()
        });
        if let Some(handle) = handle
            && let Ok(mut entity) = world.get_entity_mut(id)
        {
            entity.insert(ImageNode::new(handle));
        }
    });
    id
}

/// Lays each visible art band's picture in, from the cache or by asking for
/// it, and works through the prefetch queue.
#[allow(clippy::type_complexity)] // Bevy query: visible bands without a picture
pub(super) fn load_art(
    mut commands: Commands,
    assets: Option<Res<AssetServer>>,
    images: Option<Res<Assets<Image>>>,
    mut cache: ResMut<ArtCache>,
    prefs: Res<crate::prefs::Prefs>,
    bands: Query<
        (
            Entity,
            &ArtBand,
            &ComputedNode,
            &UiGlobalTransform,
            Option<&CalculatedClip>,
        ),
        Without<ImageNode>,
    >,
) {
    let Some(assets) = assets else {
        return;
    };
    let mut budget = ART_PER_FRAME;
    for (entity, band, node, place, clip) in &bands {
        if node.size.min_element() <= 0.0 {
            continue;
        }
        let bounds = Rect::from_center_size(place.translation, node.size);
        if clip.is_some_and(|clip| bounds.intersect(clip.clip).is_empty()) {
            continue;
        }
        let handle = if let Some(handle) = cache.images.get(&band.0) {
            handle.clone()
        } else {
            if budget == 0 {
                continue;
            }
            budget -= 1;
            let handle = assets.load(band.0.clone());
            cache.remember(&band.0, handle.clone());
            handle
        };
        // Laid in only once decoded, so the fade starts with the picture.
        if !images.as_ref().is_some_and(|i| i.contains(&handle)) {
            continue;
        }
        let first = !cache.seen.contains(&band.0);
        cache.seen.insert(band.0.clone());
        if first && !prefs.all().reduce_motion {
            commands.entity(entity).insert((
                ImageNode::new(handle).with_color(Color::WHITE.with_alpha(0.0)),
                ArtFade(0.0),
            ));
        } else {
            commands.entity(entity).insert(ImageNode::new(handle));
        }
    }
    while budget > 0
        && let Some(url) = cache.queued.pop_front()
    {
        if cache.images.contains_key(&url) {
            continue;
        }
        budget -= 1;
        let handle = assets.load(url.clone());
        cache.remember(&url, handle);
    }
}

/// Fades a first showing in over a quarter second.
pub(super) fn fade_art(
    mut commands: Commands,
    time: Res<Time>,
    mut fading: Query<(Entity, &mut ArtFade, &mut ImageNode)>,
) {
    for (entity, mut fade, mut image) in &mut fading {
        fade.0 = (fade.0 + time.delta_secs() / ART_FADE_SECS).min(1.0);
        image.color = Color::WHITE.with_alpha(fade.0);
        if fade.0 >= 1.0 {
            commands.entity(entity).remove::<ArtFade>();
        }
    }
}

/// Queues every deck's picture once a deck list lands (§10 #5: the shelf
/// and Play's hero are a press away from it).
pub(super) fn prefetch_art(
    state: Res<LobbyState>,
    mut cache: ResMut<ArtCache>,
    mut asked: Local<Vec<String>>,
) {
    if !state.is_changed() {
        return;
    }
    let lobby = &state.lobby;
    let urls: Vec<String> = lobby
        .decks()
        .iter()
        .filter_map(baylee_client_core::lobby::DeckSummary::art)
        .chain(
            lobby
                .library()
                .house
                .iter()
                .filter_map(baylee_client_core::lobby::library::HouseDeck::art),
        )
        .map(|art| art.url)
        .collect();
    if *asked == urls {
        return;
    }
    cache.prefetch(urls.iter().map(String::as_str));
    *asked = urls;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture shown once comes back in place before the first layout.
    #[test]
    fn a_rebuilt_band_has_its_seen_picture_before_layout() {
        let mut world = World::new();
        world.init_resource::<ArtCache>();
        let handle = Handle::<Image>::default();
        {
            let mut cache = world.resource_mut::<ArtCache>();
            cache.remember("u", handle.clone());
            cache.seen.insert("u".into());
        }
        let id = art_band(&mut world.commands(), "u", px(82), px(60));
        world.flush();
        assert_eq!(world.entity(id).get::<ImageNode>().unwrap().image, handle);
        // One never shown is not laid in yet: it fades in when it loads.
        let fresh = art_band(&mut world.commands(), "v", px(82), px(60));
        world.flush();
        assert!(world.entity(fresh).get::<ImageNode>().is_none());
    }

    /// The cache holds its bound and drops the oldest first.
    #[test]
    fn the_art_cache_drops_the_oldest_past_its_bound() {
        let mut cache = ArtCache::default();
        for i in 0..=ART_CACHED {
            cache.remember(&i.to_string(), Handle::default());
        }
        assert!(!cache.holds("0"));
        assert!(cache.holds("1"));
        assert!(cache.holds(&ART_CACHED.to_string()));
        cache.prefetch(["1", "new", "new"]);
        assert_eq!(
            cache.queued.len(),
            1,
            "held and repeated ones are not queued"
        );
    }

    #[test]
    fn a_remounted_thumbnail_has_its_cached_image_before_layout() {
        let mut world = World::new();
        world.init_resource::<Cache>();
        let handle = Handle::<Image>::default();
        world
            .resource_mut::<Cache>()
            .images
            .insert("test/small/art".into(), handle.clone());
        let id = spawn(
            &mut world.commands(),
            &HoverCard {
                url: Some("test/normal/art".into()),
                back_url: None,
                finish: default(),
                index: None,
            },
        );
        world.flush();
        assert_eq!(world.entity(id).get::<ImageNode>().unwrap().image, handle);
    }
}
