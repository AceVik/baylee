//! The hover preview: one entity behind an epoch counter, deliberately
//! outside the retained node tree.
//!
//! Rebuilding two hundred rows per pointer move would make the pool list
//! unusable, so the preview is built and dropped on its own.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

// ------------------------------------------------------------ hover preview

/// A row that has a card behind it, and what that card looks like.
///
/// The URL is worked out when the row is spawned rather than when it is
/// hovered: the row already knows which printing it is showing, and a hover
/// that had to go looking would be doing it on the pointer's schedule.
#[derive(Component, Clone, PartialEq)]
pub struct HoverCard {
    /// The card's art, if there is a printing to fetch.
    pub url: Option<String>,
    /// The back face's art, for a card that is printed on both sides.
    ///
    /// `None` for a card with one picture, and that is the whole check: a URL
    /// for the back can be built for any printing and Scryfall answers 404 for
    /// the ones that have no back, so what decides is `has_back_image` on the
    /// pool row — read off the printing, never off the registry's compiled
    /// face count, which said yes to nine Adventures and two Splits (#115).
    pub back_url: Option<String>,
    /// How the printing is finished, so a foil previews as one.
    pub finish: FinishTreatment,
}

/// The card the pointer is over, and where the pointer was.
#[derive(Resource, Default)]
pub(super) struct Hovered {
    /// What to draw, or `None` when the pointer is over nothing.
    card: Option<HoverCard>,
    source: Option<Entity>,
    /// Where to draw it, in logical pixels.
    at: Vec2,
    /// Bumped whenever either changes, so the preview knows to redraw
    /// without comparing an image handle.
    epoch: u64,
}

/// The preview node itself.
#[derive(Component)]
pub(super) struct CardPreview {
    canvas: Vec2,
    /// The epoch this node was drawn for.
    epoch: u64,
}

/// Tracks which row the pointer is over.
pub(super) fn hovers(
    mut overs: MessageReader<Pointer<Over>>,
    mut outs: MessageReader<Pointer<Out>>,
    cards: Query<&HoverCard>,
    parents: Query<&ChildOf>,
    mut hovered: ResMut<Hovered>,
    state: Res<LobbyState>,
) {
    if state.confirmation.is_some()
        || state.lobby.builder().picker().is_some()
        || state.lobby.library().page.is_some()
    {
        overs.clear();
        outs.clear();
        hovered.source = None;
        if hovered.card.take().is_some() {
            hovered.epoch = hovered.epoch.wrapping_add(1);
        }
        return;
    }
    let mut next = hovered.card.clone();
    let mut source = hovered.source;
    if source.is_some_and(|entity| cards.get(entity).is_err()) {
        next = None;
        source = None;
    }
    let mut at = hovered.at;
    for out in outs.read() {
        if lineage_card(out.entity, &cards, &parents)
            .is_some_and(|(entity, _)| Some(entity) == source)
        {
            next = None;
            source = None;
        }
    }
    for over in overs.read() {
        if let Some((entity, card)) = lineage_card(over.entity, &cards, &parents) {
            source = Some(entity);
            next = Some(card.clone());
            at = over.pointer_location.position;
        }
    }
    let moved = source != hovered.source;
    hovered.source = source;
    if next != hovered.card || moved {
        hovered.card = next;
        hovered.at = at;
        hovered.epoch = hovered.epoch.wrapping_add(1);
    }
}

/// The nearest [`HoverCard`] at or above an entity.
fn lineage_card<'a>(
    entity: Entity,
    cards: &'a Query<&HoverCard>,
    parents: &Query<&ChildOf>,
) -> Option<(Entity, &'a HoverCard)> {
    let mut current = Some(entity);
    while let Some(e) = current {
        if let Ok(found) = cards.get(e) {
            return Some((e, found));
        }
        current = parents.get(e).ok().map(ChildOf::parent);
    }
    None
}

/// Draws the hovered card beside the pointer.
///
/// Its own entity, spawned and despawned on its own: rebuilding the whole
/// builder on every hover would mean tearing down two hundred rows to show
/// one picture.
pub(super) fn preview(
    mut commands: Commands,
    hovered: Res<Hovered>,
    existing: Query<(Entity, &CardPreview)>,
    windows: Query<&Window>,
    assets: Option<Res<AssetServer>>,
    ui_materials: Option<ResMut<UiCardMaterials>>,
    material_assets: Option<ResMut<Assets<CardUiMaterial>>>,
) {
    let canvas = windows
        .iter()
        .next()
        .map_or(Vec2::new(1280.0, 800.0), |win| {
            Vec2::new(win.width(), win.height())
        });
    let current = existing.iter().next().map(|(_, p)| (p.epoch, p.canvas));
    if current == Some((hovered.epoch, canvas)) {
        return;
    }
    for (entity, _) in existing {
        commands.entity(entity).despawn();
    }
    let (Some(card), Some(assets)) = (hovered.card.as_ref(), assets) else {
        return;
    };
    let Some(url) = card.url.clone() else {
        return;
    };
    let (Some(mut cache), Some(mut store)) = (ui_materials, material_assets) else {
        return;
    };
    let mut cards = UiCards {
        cache: &mut cache,
        assets: &mut store,
    };

    // Big enough to read the art, small enough to leave the list visible.

    let (w, h) = (canvas.x, canvas.y);
    let height = (h * 0.65).clamp(280.0, 520.0).min((h - 32.0).max(100.0));
    let width = height * baylee_client_core::layout::CARD_ASPECT;
    // Beside the pointer, flipped to the other side when there is no room
    // and clamped so a row near the bottom does not push it off screen.
    let left = if hovered.at.x + width + 32.0 < w {
        hovered.at.x + 24.0
    } else {
        (hovered.at.x - width - 24.0).max(8.0)
    };
    let left = left.min((w - width - 8.0).max(8.0));
    let top = (hovered.at.y - height / 2.0).clamp(8.0, (h - height - 8.0).max(8.0));

    // The frame that turns: it holds the position and the scale, and each
    // face fills it. Two nodes rather than one swapped material, so the back
    // can carry the mirroring that cancels the parent's negative scale.
    let frame = commands
        .spawn((
            CardPreview {
                canvas,
                epoch: hovered.epoch,
            },
            crate::flip::Flip::default(),
            Node {
                position_type: PositionType::Absolute,
                left: px(left),
                top: px(top),
                width: px(width),
                height: px(height),
                ..default()
            },
            BoxShadow::new(
                Color::srgba(0.0, 0.0, 0.0, 0.65),
                px(0),
                px(12),
                px(3),
                px(28),
            ),
            GlobalZIndex(600),
            // A preview must never eat the click that would add the card.
            Pickable::IGNORE,
        ))
        .id();

    let mut face = |url: &str, side: crate::flip::Side| {
        let material = cards.preview(url, card.finish, assets.load(url.to_string()));
        let node = commands
            .spawn((
                MaterialNode(material),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(0),
                    width: percent(100),
                    height: percent(100),
                    border_radius: BorderRadius::all(px(12)),
                    ..default()
                },
                side,
                // Hidden until the turn passes the quarter, where the card is
                // edge-on and the swap cannot be seen.
                if side == crate::flip::Side::Back {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(frame).add_child(node);
    };
    face(&url, crate::flip::Side::Front);
    let back = card.back_url.clone().unwrap_or_else(|| {
        baylee_client_core::images::back_url(baylee_client_core::images::ArtSize::Normal)
    });
    face(&back, crate::flip::Side::Back);
}

/// Takes the preview down when the builder does.
pub(super) fn despawn_preview(mut commands: Commands, previews: Query<Entity, With<CardPreview>>) {
    for entity in previews {
        commands.entity(entity).despawn();
    }
}

/// The art a pool row previews: the printing the registry names.
pub(crate) fn hover_of_card(card: &baylee_client_core::deckbuilder::PoolCard) -> HoverCard {
    let entry = baylee_view::PrintEntry {
        scryfall_id: card.scryfall_id.clone(),
        lang: "en".to_string(),
        finish: baylee_view::Finish::Normal,
    };
    HoverCard {
        url: baylee_client_core::images::image_url(
            &entry,
            baylee_client_core::images::Face::Front,
            baylee_client_core::images::ArtSize::Normal,
        ),
        back_url: card
            .has_back_image
            .then(|| {
                baylee_client_core::images::image_url(
                    &entry,
                    baylee_client_core::images::Face::Back,
                    baylee_client_core::images::ArtSize::Normal,
                )
            })
            .flatten(),
        finish: FinishTreatment::Plain,
    }
}

/// The art a deck row previews: the printing that row actually names.
pub(crate) fn hover_of_entry(
    card: &baylee_client_core::deckbuilder::PoolCard,
    print: &baylee_core::deckrow::PrintChoice,
) -> HoverCard {
    let finish = print.finish_or_default();
    let entry = baylee_view::PrintEntry {
        // A row that named an exact printing previews that one; one that only
        // narrowed by set has no id to fetch with, so it falls back to the
        // art the pool row shows.
        scryfall_id: print
            .scryfall_id
            .clone()
            .unwrap_or_else(|| card.scryfall_id.clone()),
        lang: print.lang_or_default().to_string(),
        finish: match finish {
            Finish::Foil => baylee_view::Finish::Foil,
            Finish::Etched => baylee_view::Finish::Etched,
            Finish::Holographic => baylee_view::Finish::Holographic,
            Finish::Glitter => baylee_view::Finish::Glitter,
            Finish::Galaxy => baylee_view::Finish::Galaxy,
            Finish::Normal => baylee_view::Finish::Normal,
        },
    };
    HoverCard {
        url: baylee_client_core::images::image_url(
            &entry,
            baylee_client_core::images::Face::Front,
            baylee_client_core::images::ArtSize::Normal,
        ),
        back_url: card
            .has_back_image
            .then(|| {
                baylee_client_core::images::image_url(
                    &entry,
                    baylee_client_core::images::Face::Back,
                    baylee_client_core::images::ArtSize::Normal,
                )
            })
            .flatten(),
        finish: crate::buildui::treatment(finish),
    }
}

/// The starter deck's rows, in the `"N Card Name"` form `POST /decks` takes.
#[cfg(test)]
pub(super) fn starter_rows() -> Vec<String> {
    use baylee_core::acceptance::Zone;

    baylee_core::acceptance::parse_decks(&crate::host::acceptance_text())
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.deck == STARTER && row.zone == Zone::Main)
        .map(|row| format!("{} {}", row.count, row.name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::deckbuilder::PoolCard;

    fn pointer_event<E: std::fmt::Debug + Clone + Reflect>(
        entity: Entity,
        event: E,
        x: f32,
    ) -> Pointer<E> {
        use bevy::picking::pointer::{Location, PointerId};
        Pointer::new(
            PointerId::Mouse,
            Location {
                target: bevy::camera::NormalizedRenderTarget::Window(
                    bevy::window::WindowRef::Entity(Entity::PLACEHOLDER)
                        .normalize(None)
                        .unwrap(),
                ),
                position: Vec2::new(x, 100.0),
            },
            event,
            entity,
        )
    }

    #[test]
    fn identical_art_moves_to_the_new_row_and_ignores_unrelated_out_events() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<LobbyState>()
            .init_resource::<Hovered>()
            .add_message::<Pointer<Over>>()
            .add_message::<Pointer<Out>>()
            .add_systems(Update, hovers);
        let art = hover_of_card(&row(false, false));
        let first = app.world_mut().spawn(art.clone()).id();
        let second = app.world_mut().spawn(art).id();
        let hit = bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
        app.world_mut()
            .write_message(pointer_event(first, Over { hit: hit.clone() }, 100.0));
        app.update();
        let epoch = app.world().resource::<Hovered>().epoch;
        // A nested icon must still resolve to the row, however many layout wrappers it has.
        let mut leaf = second;
        for _ in 0..12 {
            let child = app.world_mut().spawn_empty().id();
            app.world_mut().entity_mut(leaf).add_child(child);
            leaf = child;
        }
        app.world_mut()
            .write_message(pointer_event(leaf, Over { hit: hit.clone() }, 600.0));
        app.update();
        let hovered = app.world().resource::<Hovered>();
        assert_eq!(hovered.source, Some(second));
        assert!(hovered.epoch > epoch);
        assert_eq!(hovered.at, Vec2::new(600.0, 100.0));
        app.world_mut()
            .write_message(pointer_event(first, Out { hit }, 100.0));
        app.update();
        assert_eq!(app.world().resource::<Hovered>().source, Some(second));
        assert!(app.world().resource::<Hovered>().card.is_some());
    }

    #[test]
    fn resizing_the_window_repositions_a_stationary_preview() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<CardUiMaterial>()
            .init_resource::<UiCardMaterials>()
            .insert_resource(Hovered {
                card: Some(hover_of_card(&row(false, false))),
                at: Vec2::new(1100.0, 600.0),
                ..default()
            })
            .add_systems(Update, preview);
        let window = app.world_mut().spawn(Window::default()).id();
        app.update();
        let first = app
            .world_mut()
            .query_filtered::<Entity, With<CardPreview>>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set(640.0, 480.0);
        app.update();
        assert!(app.world().get_entity(first).is_err());
        let (_, node) = app
            .world_mut()
            .query::<(&CardPreview, &Node)>()
            .single(app.world())
            .unwrap();
        let (Val::Px(left), Val::Px(width)) = (node.left, node.width) else {
            panic!("pixel placement")
        };
        assert!(left + width <= 640.0);
    }

    /// A pool row with one printing id and whichever sides flags a test wants.
    fn row(has_back_image: bool, double_faced: bool) -> PoolCard {
        PoolCard {
            // A well-formed Scryfall id, or `image_url` refuses before the
            // question this test is asking is ever reached.
            scryfall_id: "2f40613b-1bde-4939-86ad-6bd40f9db0d6".to_string(),
            has_back_image,
            double_faced,
            ..PoolCard::default()
        }
    }

    /// The preview offers a back only for a card that has one.
    ///
    /// Both directions, because the front URL is built either way and a
    /// preview that offered a flip on everything would look identical here
    /// to one that offered it on nothing.
    #[test]
    fn only_a_card_with_a_second_picture_is_offered_a_back() {
        assert!(
            hover_of_card(&row(true, true)).back_url.is_some(),
            "a double-faced card has a back to preview"
        );
        assert!(
            hover_of_card(&row(false, false)).back_url.is_none(),
            "an ordinary card has none"
        );
        assert!(
            hover_of_card(&row(true, true)).url.is_some(),
            "the front is built either way, so the back is what this measures"
        );
    }

    /// The **rules** answer does not reach this preview, and must not.
    ///
    /// A meld card is double-faced under CR 712.1 and Scryfall serves no
    /// second picture for it, so a preview keyed on `double_faced` would
    /// build a URL that answers 404 — which is exactly the failure #115 was
    /// about, one field standing in for two questions. Injected from the
    /// other side too: a printing with a back that the rules do not call
    /// double-faced still gets its back, because what the preview needs is a
    /// picture and not a classification.
    #[test]
    fn the_preview_asks_for_a_picture_and_not_for_a_classification() {
        assert!(
            hover_of_card(&row(false, true)).back_url.is_none(),
            "a meld card is double-faced with nothing at the back shelf"
        );
        assert!(
            hover_of_card(&row(true, false)).back_url.is_some(),
            "a printing with a second picture has one whatever it is called"
        );
    }
}
