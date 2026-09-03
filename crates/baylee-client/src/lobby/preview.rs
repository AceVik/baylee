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
#[derive(Component, Clone)]
pub struct HoverCard {
    /// The card's art, if there is a printing to fetch.
    pub url: Option<String>,
    /// The back face's art, for a card that is printed on both sides.
    ///
    /// `None` for a single-faced card, and that is the whole check: a URL for
    /// the back can be built for any printing and Scryfall answers 404 for
    /// the ones that have no back, so what decides is the registry's face
    /// count, which arrives on the pool row.
    pub back_url: Option<String>,
    /// How the printing is finished, so a foil previews as one.
    pub finish: FinishTreatment,
}

/// The card the pointer is over, and where the pointer was.
#[derive(Resource, Default)]
pub(super) struct Hovered {
    /// What to draw, or `None` when the pointer is over nothing.
    card: Option<HoverCard>,
    /// Where to draw it, in logical pixels.
    at: Vec2,
    /// Bumped whenever either changes, so the preview knows to redraw
    /// without comparing an image handle.
    epoch: u64,
}

/// The preview node itself.
#[derive(Component)]
pub(super) struct CardPreview {
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
) {
    for out in outs.read() {
        if lineage_card(out.entity, &cards, &parents).is_some() {
            hovered.card = None;
            hovered.epoch = hovered.epoch.wrapping_add(1);
        }
    }
    for over in overs.read() {
        if let Some(card) = lineage_card(over.entity, &cards, &parents) {
            hovered.card = Some(card.clone());
            hovered.at = over.pointer_location.position;
            hovered.epoch = hovered.epoch.wrapping_add(1);
        }
    }
}

/// The nearest [`HoverCard`] at or above an entity.
fn lineage_card<'a>(
    entity: Entity,
    cards: &'a Query<&HoverCard>,
    parents: &Query<&ChildOf>,
) -> Option<&'a HoverCard> {
    let mut current = Some(entity);
    for _ in 0..6 {
        let e = current?;
        if let Ok(found) = cards.get(e) {
            return Some(found);
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
    let current = existing.iter().next().map(|(_, p)| p.epoch);
    if current == Some(hovered.epoch) {
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
    let height = 340.0_f32;
    let width = height * baylee_client_core::layout::CARD_ASPECT;
    let window = windows.iter().next();
    let (w, h) = window.map_or((1280.0, 800.0), |win| (win.width(), win.height()));
    // Beside the pointer, flipped to the other side when there is no room
    // and clamped so a row near the bottom does not push it off screen.
    let left = if hovered.at.x + width + 32.0 < w {
        hovered.at.x + 24.0
    } else {
        (hovered.at.x - width - 24.0).max(8.0)
    };
    let top = (hovered.at.y - height / 2.0).clamp(8.0, (h - height - 8.0).max(8.0));

    // The frame that turns: it holds the position and the scale, and each
    // face fills it. Two nodes rather than one swapped material, so the back
    // can carry the mirroring that cancels the parent's negative scale.
    let frame = commands
        .spawn((
            CardPreview {
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
    if let Some(back) = card.back_url.as_deref() {
        face(back, crate::flip::Side::Back);
    }
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
            .two_faced
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
            .two_faced
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
pub(super) fn starter_rows() -> Vec<String> {
    use baylee_core::acceptance::Zone;

    baylee_core::acceptance::parse_decks(&crate::host::acceptance_text())
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.deck == STARTER && row.zone == Zone::Main)
        .map(|row| format!("{} {}", row.count, row.name))
        .collect()
}
