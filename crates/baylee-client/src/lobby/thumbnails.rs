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

#[cfg(test)]
mod tests {
    use super::*;
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
