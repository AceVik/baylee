//! Small art loads only once a row enters its scroller's visible rectangle.
use super::{LobbyState, preview::HoverCard};
use baylee_client_core::deckbuilder::Zone;
use bevy::prelude::*;
use bevy::ui::{CalculatedClip, px};
use std::collections::{BTreeMap, VecDeque};

#[derive(Component)]
pub(crate) struct Thumbnail(String);
#[derive(Component)]
pub(crate) struct Quantity(pub usize);
#[derive(Default)]
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
        entity.insert(Thumbnail(url.replace("/normal/", "/small/")));
    }
    entity.id()
}

#[allow(clippy::type_complexity)] // Bevy query: visible image placeholders
pub(super) fn load(
    mut commands: Commands,
    assets: Option<Res<AssetServer>>,
    mut cache: Local<Cache>,
    rows: Query<
        (
            Entity,
            &Thumbnail,
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
        commands.entity(entity).insert(ImageNode::new(handle));
    }
}

pub(super) fn quantities(state: Res<LobbyState>, mut labels: Query<(&Quantity, &mut Text)>) {
    if !state.is_changed() {
        return;
    }
    let deck = state.lobby.builder();
    for (quantity, mut text) in &mut labels {
        let value = format!(
            "{} / {}",
            deck.count_of(quantity.0, Zone::Main),
            deck.count_of(quantity.0, Zone::Side)
        );
        if text.0 != value {
            text.0 = value;
        }
    }
}
