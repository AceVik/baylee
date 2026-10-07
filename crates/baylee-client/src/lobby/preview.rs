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
    /// The pool card's registry index, for its text face (#259). The face is
    /// built when the row is hovered, not for every row the list spawns.
    pub index: Option<u32>,
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
    /// What the pointer is over now, which is not always what is previewed:
    /// on its way to a preview it crosses other rows (#289).
    under: Under,
    /// Where the pointer last was on the row being previewed: the corner of
    /// the way to its preview.
    apex: Option<Vec2>,
}

/// What the pointer is over, as the picking messages last said.
#[derive(Default, Clone, PartialEq)]
enum Under {
    /// Nothing that previews.
    #[default]
    Nothing,
    /// A row with a card behind it, and where the pointer came onto it.
    Row(Entity, HoverCard, Vec2),
    /// The preview itself.
    Preview,
}

/// The preview node itself.
#[derive(Component)]
pub(super) struct CardPreview {
    canvas: Vec2,
    /// The epoch this node was drawn for.
    epoch: u64,
    /// Where it stands, in logical pixels.
    rect: Rect,
    /// Whether the pointer may go onto it (#289): a text face, whose rules
    /// text may run over and is scrolled by a wheel over it. A picture has
    /// nothing to scroll, and stays out of the pointer's way.
    takes_pointer: bool,
}

/// Tracks which row the pointer is over, and keeps a text face's preview
/// while the pointer travels onto it (#289).
///
/// The preview stands beside the pointer, so on the way to it the pointer
/// crosses the rows above or below its own. Those are passed over while the
/// pointer is inside the triangle between where it last was on its row and
/// the preview's near edge, the way a menu lets a pointer cut across to its
/// submenu: heading for the preview, it keeps the card; turning away, the
/// row under it takes over at once. Straight down a list is never inside
/// that triangle, so reading down the pool is as quick as it was.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
pub(super) fn hovers(
    mut overs: MessageReader<Pointer<Over>>,
    mut outs: MessageReader<Pointer<Out>>,
    cards: Query<&HoverCard>,
    parents: Query<&ChildOf>,
    previews: Query<&CardPreview>,
    windows: Query<&Window>,
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
        hovered.under = Under::Nothing;
        hovered.apex = None;
        if hovered.card.take().is_some() {
            hovered.epoch = hovered.epoch.wrapping_add(1);
        }
        return;
    }
    let is_preview = |entity: Entity| lineage(entity, &parents).any(|e| previews.contains(e));
    let mut under = hovered.under.clone();
    if let Under::Row(entity, ..) = under
        && cards.get(entity).is_err()
    {
        under = Under::Nothing;
    }
    for out in outs.read() {
        let left = match &under {
            Under::Row(entity, ..) => lineage_card(out.entity, &cards, &parents)
                .is_some_and(|(found, _)| found == *entity),
            Under::Preview => is_preview(out.entity),
            Under::Nothing => false,
        };
        if left {
            under = Under::Nothing;
        }
    }
    for over in overs.read() {
        if is_preview(over.entity) {
            under = Under::Preview;
        } else if let Some((entity, card)) = lineage_card(over.entity, &cards, &parents) {
            under = Under::Row(entity, card.clone(), over.pointer_location.position);
        }
    }

    let cursor = windows.iter().next().and_then(Window::cursor_position);
    // The preview drawn for the card shown now, if the pointer may go to it.
    let reach = previews
        .iter()
        .find(|p| p.takes_pointer && p.epoch == hovered.epoch && hovered.card.is_some())
        .map(|p| p.rect);
    let (mut next, mut source, mut at) = (hovered.card.clone(), hovered.source, hovered.at);
    if source.is_some_and(|entity| cards.get(entity).is_err()) {
        next = None;
        source = None;
    }
    match &under {
        // On the preview, or back on its own row: the card stays.
        Under::Preview => {}
        Under::Row(entity, ..) if Some(*entity) == source => hovered.apex = cursor,
        _ if travelling(hovered.apex, cursor, reach) => {}
        Under::Row(entity, card, entered) => {
            next = Some(card.clone());
            source = Some(*entity);
            at = *entered;
            hovered.apex = cursor.or(Some(*entered));
        }
        Under::Nothing => {
            next = None;
            source = None;
            hovered.apex = None;
        }
    }
    hovered.under = under;
    let moved = source != hovered.source;
    hovered.source = source;
    if next != hovered.card || moved {
        hovered.card = next;
        hovered.at = at;
        hovered.epoch = hovered.epoch.wrapping_add(1);
    }
}

/// Whether a pointer at `cursor` is on its way from `apex` to the preview
/// standing at `reach`: inside the triangle between the two (a few pixels
/// wider, for a hand that is not a ruler), or on the preview already.
fn travelling(apex: Option<Vec2>, cursor: Option<Vec2>, reach: Option<Rect>) -> bool {
    /// How much wider than the preview the triangle's far side is.
    const SLACK: f32 = 12.0;
    let (Some(apex), Some(cursor), Some(rect)) = (apex, cursor, reach) else {
        return false;
    };
    if rect.contains(cursor) {
        return true;
    }
    let edge = if rect.min.x >= apex.x {
        rect.min.x
    } else {
        rect.max.x
    };
    let top = Vec2::new(edge, rect.min.y - SLACK);
    let foot = Vec2::new(edge, rect.max.y + SLACK);
    // The same side of all three edges, by the sign of each cross product.
    let side = |a: Vec2, b: Vec2| (b - a).perp_dot(cursor - a);
    let signs = [side(apex, top), side(top, foot), side(foot, apex)];
    signs.iter().all(|s| *s >= 0.0) || signs.iter().all(|s| *s <= 0.0)
}

/// `entity` and every ancestor above it, nearest first.
fn lineage<'a>(entity: Entity, parents: &'a Query<&ChildOf>) -> impl Iterator<Item = Entity> + 'a {
    std::iter::successors(Some(entity), move |e| {
        parents.get(*e).ok().map(ChildOf::parent)
    })
}

/// Scrolls a preview's rules text under a wheel over the preview (#289):
/// the wheel scrolls what is under the pointer, and once the pointer is on
/// a text face's preview, that is its text.
pub(super) fn scroll_the_preview(
    mut wheels: MessageReader<Pointer<Scroll>>,
    parents: Query<&ChildOf>,
    previews: Query<(), With<CardPreview>>,
    mut boxes: Query<(Entity, &mut ScrollPosition, &ComputedNode), With<crate::face::FaceTextBox>>,
) {
    for wheel in wheels.read() {
        let Some(preview) = lineage(wheel.entity, &parents).find(|e| previews.contains(*e)) else {
            continue;
        };
        let travel = match wheel.unit {
            bevy::input::mouse::MouseScrollUnit::Line => wheel.y * super::scrolling::WHEEL_LINE,
            bevy::input::mouse::MouseScrollUnit::Pixel => wheel.y,
        };
        for (text_box, mut position, computed) in &mut boxes {
            if lineage(text_box, &parents).any(|e| e == preview) {
                position.y = crate::hud::scrolled(
                    position.y,
                    -travel,
                    computed.size().y,
                    computed.content_size().y,
                    computed.inverse_scale_factor(),
                );
            }
        }
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

use baylee_client_core::card_face::CardFace;

/// What drawing a pool card as its text face reads (#259).
///
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct Reading<'w> {
    settings: Option<Res<'w, crate::settings::ClientSettings>>,
    state: Option<Res<'w, LobbyState>>,
    fonts: Option<Res<'w, UiFonts>>,
    font_assets: Option<Res<'w, Assets<Font>>>,
}

impl Reading<'_> {
    /// The text face `card` previews as, and the fonts to set it in: where
    /// there is no picture to show, and where the player reads text rather
    /// than art (the setting the table reads too). Built here, on the hover,
    /// and not for every row the list spawns.
    fn face(&self, card: &HoverCard) -> Option<(CardFace, &UiFonts)> {
        let reads_text = self.settings.as_deref().is_some_and(|s| s.prefer_text_view);
        if card.url.is_some() && !reads_text {
            return None;
        }
        let builder = self.state.as_deref()?.lobby.builder();
        let pool = builder.pool();
        let mut face = crate::face::of_pool(pool.iter().find(|c| Some(c.index) == card.index)?);
        // The foot's Set · Artist (WP6), where the builder has been told about
        // the printing the row shows: its art's id is the URL's last part.
        face.credit = card.index.and_then(|index| {
            let id = card
                .url
                .as_deref()
                .and_then(|url| url.rsplit('/').next())
                .map_or("", |file| file.trim_end_matches(".jpg"));
            builder.printing_credit(index, id)
        });
        Some((face, self.fonts.as_deref()?))
    }

    /// Spawns a text face as the front side of the preview `frame`, in the
    /// printing's finish.
    fn spawn(
        &self,
        commands: &mut Commands,
        cards: &mut UiCards,
        frame: Entity,
        finish: FinishTreatment,
        (face, fonts): &(CardFace, &UiFonts),
        width: f32,
    ) {
        let lang = self.state.as_deref().map_or(Lang::En, |s| s.lobby.lang());
        let widths =
            crate::face::Widths::of(self.font_assets.as_deref().and_then(|a| a.get(&fonts.text)));
        // No plate: a card in the pool is on no battlefield, so its face
        // writes its own numbers.
        let laid =
            crate::face::UiFace::lay(face, lang, width, crate::face::Detail::Full, &widths, 0);
        let look = crate::cardmat::CardLook::back(finish)
            .faced(crate::face::table_color(face.colors), laid.word);
        let node = commands
            .spawn((
                MaterialNode(cards.get(look, None)),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(0),
                    width: percent(100),
                    height: percent(100),
                    border_radius: BorderRadius::all(px(12)),
                    ..default()
                },
                crate::flip::Side::Front,
                Visibility::Inherited,
                Pickable::IGNORE,
            ))
            .id();
        crate::face::spawn_ui(commands, node, lang, face, &laid, fonts);
        commands.entity(frame).add_child(node);
    }
}

/// Draws the hovered card beside the pointer.
///
/// Its own entity, spawned and despawned on its own: rebuilding the whole
/// builder on every hover would mean tearing down two hundred rows to show
/// one picture.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
pub(super) fn preview(
    mut commands: Commands,
    hovered: Res<Hovered>,
    existing: Query<(Entity, &CardPreview)>,
    windows: Query<&Window>,
    assets: Option<Res<AssetServer>>,
    ui_materials: Option<ResMut<UiCardMaterials>>,
    material_assets: Option<ResMut<Assets<CardUiMaterial>>>,
    reading: Reading,
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
    let text = reading.face(card);
    if text.is_none() && card.url.is_none() {
        return;
    }
    let (Some(mut cache), Some(mut store)) = (ui_materials, material_assets) else {
        return;
    };
    let mut cards = UiCards {
        cache: &mut cache,
        assets: &mut store,
    };

    let rect = place(hovered.at, canvas);
    let (left, top, width, height) = (rect.min.x, rect.min.y, rect.width(), rect.height());

    // The frame that turns: it holds the position and the scale, and each
    // face fills it. Two nodes rather than one swapped material, so the back
    // can carry the mirroring that cancels the parent's negative scale.
    let frame = commands
        .spawn((
            CardPreview {
                canvas,
                epoch: hovered.epoch,
                rect,
                takes_pointer: text.is_some(),
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
            // A picture is passed over by the pointer. A text face takes it
            // (#289), so its rules text can be scrolled; it stands beside
            // the pointer and never over the row it previews, so it never
            // eats the click that would add the card.
            if text.is_some() {
                Pickable::default()
            } else {
                Pickable::IGNORE
            },
        ))
        .id();

    if let Some(text) = &text {
        reading.spawn(&mut commands, &mut cards, frame, card.finish, text, width);
    }

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
    if text.is_none()
        && let Some(url) = &card.url
    {
        face(url, crate::flip::Side::Front);
    }
    let back = card.back_url.clone().unwrap_or_else(|| {
        baylee_client_core::images::back_url(baylee_client_core::images::ArtSize::Normal)
    });
    face(&back, crate::flip::Side::Back);
}

/// Where the preview of a row the pointer came onto at `at` stands, in a
/// window of `canvas` logical pixels.
///
/// Big enough to read the art, small enough to leave the list visible;
/// beside the pointer, flipped to the other side when there is no room, and
/// clamped so a row near the bottom does not push it off screen.
fn place(at: Vec2, canvas: Vec2) -> Rect {
    let (w, h) = (canvas.x, canvas.y);
    let height = (h * 0.65).clamp(280.0, 520.0).min((h - 32.0).max(100.0));
    let width = height * baylee_client_core::layout::CARD_ASPECT;
    let left = if at.x + width + 32.0 < w {
        at.x + 24.0
    } else {
        (at.x - width - 24.0).max(8.0)
    };
    let left = left.min((w - width - 8.0).max(8.0));
    let top = (at.y - height / 2.0).clamp(8.0, (h - height - 8.0).max(8.0));
    Rect::new(left, top, left + width, top + height)
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
        index: Some(card.index),
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
        index: Some(card.index),
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

    /// The builder's hover, with a window whose pointer the test moves.
    fn hover_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<LobbyState>()
            .init_resource::<Hovered>()
            .add_message::<Pointer<Over>>()
            .add_message::<Pointer<Out>>()
            .add_systems(Update, hovers);
        let window = app.world_mut().spawn(Window::default()).id();
        (app, window)
    }

    /// Moves the pointer to `at`, says it left `from` and came onto `onto`
    /// (either may be nothing), and runs a frame.
    fn step(
        app: &mut App,
        window: Entity,
        at: Vec2,
        from: Option<Entity>,
        onto: Option<Entity>,
    ) -> u64 {
        use bevy::picking::pointer::{Location, PointerId};
        let location = Location {
            target: bevy::camera::NormalizedRenderTarget::Window(
                bevy::window::WindowRef::Entity(window)
                    .normalize(None)
                    .unwrap(),
            ),
            position: at,
        };
        let hit = bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(Some(at));
        if let Some(from) = from {
            app.world_mut().write_message(Pointer::new(
                PointerId::Mouse,
                location.clone(),
                Out { hit: hit.clone() },
                from,
            ));
        }
        if let Some(onto) = onto {
            app.world_mut().write_message(Pointer::new(
                PointerId::Mouse,
                location,
                Over { hit },
                onto,
            ));
        }
        app.update();
        app.world().resource::<Hovered>().epoch
    }

    /// Stands a preview for the card hovered now, as `preview` would.
    fn stand_preview(app: &mut App, rect: Rect, takes_pointer: bool) -> Entity {
        let epoch = app.world().resource::<Hovered>().epoch;
        app.world_mut()
            .spawn(CardPreview {
                canvas: Vec2::new(1280.0, 800.0),
                epoch,
                rect,
                takes_pointer,
            })
            .id()
    }

    fn source(app: &App) -> Option<Entity> {
        app.world().resource::<Hovered>().source
    }

    /// #289: a text face's preview keeps its card while the pointer cuts
    /// across the rows between it and the preview, and on the preview; off
    /// the preview, onto nothing, it goes. The preview stands where it was
    /// drawn throughout: its card's epoch never moves.
    #[test]
    fn a_text_preview_keeps_its_card_while_the_pointer_travels_onto_it() {
        let (mut app, window) = hover_app();
        let art = hover_of_card(&row(false, false));
        let first = app.world_mut().spawn(art.clone()).id();
        let second = app.world_mut().spawn(art).id();
        let start = Vec2::new(100.0, 100.0);
        let epoch = step(&mut app, window, start, None, Some(first));
        assert_eq!(source(&app), Some(first));
        let preview = stand_preview(&mut app, Rect::new(124.0, -50.0, 424.0, 350.0), true);

        // Diagonally towards it, over the next row: still the first card.
        let crossing = Vec2::new(110.0, 130.0);
        assert_eq!(
            step(&mut app, window, crossing, Some(first), Some(second)),
            epoch
        );
        assert_eq!(source(&app), Some(first), "the row crossed on the way");
        // On the preview.
        let on = Vec2::new(150.0, 130.0);
        assert_eq!(
            step(&mut app, window, on, Some(second), Some(preview)),
            epoch
        );
        assert_eq!(source(&app), Some(first), "on the preview");
        // Off it, onto nothing and away from it: gone.
        step(
            &mut app,
            window,
            Vec2::new(500.0, 600.0),
            Some(preview),
            None,
        );
        assert_eq!(source(&app), None);
        assert!(app.world().resource::<Hovered>().card.is_none());
    }

    /// The other side of the same rule: straight down the list is never on
    /// the way to the preview, so the next row takes over at once; and a
    /// pointer that turns away while over a crossed row lands on that row.
    #[test]
    fn a_pointer_that_is_not_heading_for_the_preview_moves_on_at_once() {
        let (mut app, window) = hover_app();
        let art = hover_of_card(&row(false, false));
        let first = app.world_mut().spawn(art.clone()).id();
        let second = app.world_mut().spawn(art).id();
        step(&mut app, window, Vec2::new(100.0, 100.0), None, Some(first));
        stand_preview(&mut app, Rect::new(124.0, -50.0, 424.0, 350.0), true);
        step(
            &mut app,
            window,
            Vec2::new(100.0, 130.0),
            Some(first),
            Some(second),
        );
        assert_eq!(source(&app), Some(second), "straight down");

        let (mut app, window) = hover_app();
        let art = hover_of_card(&row(false, false));
        let first = app.world_mut().spawn(art.clone()).id();
        let second = app.world_mut().spawn(art).id();
        step(&mut app, window, Vec2::new(100.0, 100.0), None, Some(first));
        stand_preview(&mut app, Rect::new(124.0, -50.0, 424.0, 350.0), true);
        step(
            &mut app,
            window,
            Vec2::new(110.0, 130.0),
            Some(first),
            Some(second),
        );
        assert_eq!(source(&app), Some(first), "heading for it");
        // Turned back left, still over the crossed row: no new message, and
        // the row under the pointer takes over.
        step(&mut app, window, Vec2::new(80.0, 132.0), None, None);
        assert_eq!(source(&app), Some(second), "turned away");
    }

    /// A picture's preview takes no pointer, so nothing is kept for it.
    #[test]
    fn a_picture_preview_keeps_nothing_on_the_way() {
        let (mut app, window) = hover_app();
        let art = hover_of_card(&row(false, false));
        let first = app.world_mut().spawn(art.clone()).id();
        let second = app.world_mut().spawn(art).id();
        step(&mut app, window, Vec2::new(100.0, 100.0), None, Some(first));
        stand_preview(&mut app, Rect::new(124.0, -50.0, 424.0, 350.0), false);
        step(
            &mut app,
            window,
            Vec2::new(110.0, 130.0),
            Some(first),
            Some(second),
        );
        assert_eq!(source(&app), Some(second));
    }

    /// A wheel over the preview scrolls its own rules text (#289), and no
    /// other text box.
    #[test]
    fn a_wheel_over_a_preview_scrolls_its_text() {
        use bevy::picking::pointer::{Location, PointerId};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<Pointer<Scroll>>()
            .add_systems(Update, scroll_the_preview);
        let text_box = || {
            (
                crate::face::FaceTextBox,
                ScrollPosition::default(),
                ComputedNode {
                    size: Vec2::new(200.0, 100.0),
                    content_size: Vec2::new(200.0, 400.0),
                    ..default()
                },
            )
        };
        let preview = app
            .world_mut()
            .spawn(CardPreview {
                canvas: Vec2::ZERO,
                epoch: 0,
                rect: Rect::default(),
                takes_pointer: true,
            })
            .id();
        let face = app.world_mut().spawn(ChildOf(preview)).id();
        let inside = app.world_mut().spawn((text_box(), ChildOf(face))).id();
        let elsewhere = app.world_mut().spawn(text_box()).id();
        let window = app.world_mut().spawn(Window::default()).id();
        app.world_mut().write_message(Pointer::new(
            PointerId::Mouse,
            Location {
                target: bevy::camera::NormalizedRenderTarget::Window(
                    bevy::window::WindowRef::Entity(window)
                        .normalize(None)
                        .unwrap(),
                ),
                position: Vec2::ZERO,
            },
            Scroll {
                unit: bevy::input::mouse::MouseScrollUnit::Line,
                x: 0.0,
                y: -1.0,
                hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                phase: bevy::input::touch::TouchPhase::Moved,
            },
            preview,
        ));
        app.update();
        let at = |e: Entity| app.world().get::<ScrollPosition>(e).unwrap().y;
        assert!(at(inside) > 0.0, "the preview's text moved");
        assert!(at(elsewhere).abs() < f32::EPSILON, "and nothing else");
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

    /// A pool row the registry compiles a face for: a one-mana green
    /// creature with rules text.
    fn birds() -> PoolCard {
        PoolCard {
            index: baylee_cards::decks::by_name("Birds of Paradise")
                .expect("in the pool")
                .get(),
            name: "Birds of Paradise".to_string(),
            english_name: "Birds of Paradise".to_string(),
            colors: "G".to_string(),
            ..row(false, false)
        }
    }

    /// What the preview draws for `card` over a pool holding [`birds`]:
    /// whether a rules text box stands in it, and each front side's
    /// material, with whether it carries a picture.
    fn drawn(card: HoverCard, reads_text: bool) -> (bool, Vec<(bool, crate::cardmat::CardParams)>) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<Font>()
            .init_asset::<CardUiMaterial>()
            .init_resource::<UiCardMaterials>()
            .init_resource::<LobbyState>()
            .insert_resource(crate::settings::ClientSettings {
                prefer_text_view: reads_text,
                ..default()
            })
            .insert_resource(UiFonts {
                text: Handle::default(),
                medium: Handle::default(),
                bold: Handle::default(),
                italic: Handle::default(),
                medium_italic: Handle::default(),
                serif: Handle::default(),
                serif_italic: Handle::default(),
                icons: Handle::default(),
                mana: Handle::default(),
            })
            .insert_resource(Hovered {
                card: Some(card),
                at: Vec2::new(100.0, 300.0),
                ..default()
            })
            .add_systems(Update, preview);
        app.world_mut()
            .resource_mut::<LobbyState>()
            .lobby
            .builder_mut()
            .set_pool(vec![birds()], true);
        app.world_mut().spawn(Window::default());
        app.update();
        let world = app.world_mut();
        let text_box = world
            .query_filtered::<(), With<crate::face::FaceTextBox>>()
            .iter(world)
            .count()
            > 0;
        // A text face takes the pointer (#289), and a picture never does.
        let (preview, pickable) = world
            .query::<(&CardPreview, &Pickable)>()
            .single(world)
            .expect("one preview");
        assert_eq!(preview.takes_pointer, text_box);
        assert_eq!(pickable.should_block_lower, text_box);
        assert_eq!(pickable.is_hoverable, text_box);
        let fronts: Vec<_> = world
            .query::<(&crate::flip::Side, &MaterialNode<CardUiMaterial>)>()
            .iter(world)
            .filter(|(side, _)| **side == crate::flip::Side::Front)
            .map(|(_, node)| node.0.clone())
            .collect();
        let materials = world.resource::<Assets<CardUiMaterial>>();
        let fronts = fronts
            .iter()
            .map(|handle| {
                let material = materials.get(handle).expect("a material");
                (material.art.is_some(), material.params)
            })
            .collect();
        (text_box, fronts)
    }

    /// A card with no printing to fetch previews as its text face, where it
    /// used to preview as nothing at all (#259).
    #[test]
    fn a_card_with_no_picture_previews_as_its_text_face() {
        let card = HoverCard {
            url: None,
            ..hover_of_card(&birds())
        };
        let (text, fronts) = drawn(card, false);
        assert!(text, "the rules text stands in the face");
        let [(art, params)] = fronts[..] else {
            panic!("one front, not {}", fronts.len())
        };
        assert!(!art, "no picture under the face");
        assert_ne!(params.face, 0, "drawn as a text face, not a flat tint");
    }

    /// The table's preference reaches the builder, both ways: a player who
    /// reads text sees the text face where there is a picture, and nothing
    /// under it; one who does not sees the picture and no text.
    #[test]
    fn a_player_who_reads_text_previews_the_text_face_over_the_picture() {
        let (text, fronts) = drawn(hover_of_card(&birds()), true);
        assert!(text, "the text face is drawn");
        assert!(
            fronts.iter().all(|(art, _)| !art),
            "and the picture is not drawn under it"
        );
        let (text, fronts) = drawn(hover_of_card(&birds()), false);
        assert!(!text, "the picture alone");
        assert!(matches!(fronts[..], [(true, _)]), "{} fronts", fronts.len());
    }

    /// A foil printing read as text is still a foil.
    #[test]
    fn a_text_face_keeps_the_printing_s_finish() {
        let card = HoverCard {
            finish: FinishTreatment::Foil,
            ..hover_of_card(&birds())
        };
        let (_, fronts) = drawn(card, true);
        let [(_, params)] = fronts[..] else {
            panic!("one front, not {}", fronts.len())
        };
        assert_ne!(params.face, 0);
        assert_eq!(
            params.finish,
            crate::cardmat::finish_code(FinishTreatment::Foil)
        );
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
