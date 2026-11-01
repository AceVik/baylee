//! Card links at the table: only identities this seat knows, drawn as art
//! when it has arrived and as an honest constructed text face otherwise.

#[allow(clippy::wildcard_imports)] // the window's own vocabulary
use super::*;
use crate::cardmat::{CardUiMaterial, UiCardMaterials};
use baylee_client_core::images::{ArtSize, ImageKey};
use baylee_view::CardIdentity;

/// The chat link hovered now, independently of the game log's own links.
#[derive(Resource, Default)]
pub(crate) struct ChatHover(Option<Request>);

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Request {
    span: Entity,
    identity: CardIdentity,
    at: Vec2,
}

/// Everything that makes the retained preview redraw: link, available art,
/// text generation, canvas, and the two render-resource readiness flags.
type ShownKey = (Option<Request>, Option<ImageKey>, u64, Vec2, bool, bool);

/// The root of the chat's own preview.
#[derive(Component)]
pub(crate) struct ChatPreview;

/// The identity a visible hand card, `looking_at` row or public object proves.
pub(super) fn in_view(
    view: &baylee_view::PlayerView,
    link: &rows::ChatCardLink,
) -> Option<CardIdentity> {
    let matches = |card: &CardIdentity| card.index.get() == link.card;
    view.hand
        .iter()
        .chain(
            view.shared_hands
                .iter()
                .chain(view.controlled_hands.iter())
                .flat_map(|hand| hand.cards.iter()),
        )
        .map(|card| card.card)
        .find(matches)
        .or_else(|| {
            view.looking_at
                .iter()
                .filter_map(|object| object.card)
                .find(matches)
        })
        .or_else(|| {
            view.battlefield
                .iter()
                .chain(view.stack.iter())
                .chain(view.graveyards.iter().flatten())
                .chain(view.exile.iter().flatten())
                .chain(view.command.iter().flatten())
                .chain(view.library_tops.iter())
                .filter_map(|object| object.card)
                .find(matches)
        })
        .map(|identity| CardIdentity {
            face: link.face,
            ..identity
        })
}

/// The identity a log line already showed, with that line's printing.
pub(super) fn in_log(
    lines: &[baylee_client_core::gamelog::LogLine],
    link: &rows::ChatCardLink,
) -> Option<CardIdentity> {
    lines
        .iter()
        .flat_map(|line| line.names.iter())
        .filter_map(|name| name.card)
        .find(|card| card.index.get() == link.card)
        .map(|identity| CardIdentity {
            face: link.face,
            ..identity
        })
}

/// Resolves a chat link only through information already given to this seat.
fn identity(
    duel: &Duel,
    texts: Option<&crate::cardtext::CardTexts>,
    lang: Lang,
    link: &rows::ChatCardLink,
) -> Option<CardIdentity> {
    let current = duel.view.as_ref().and_then(|view| in_view(view, link));
    let logged = current.is_none().then(|| {
        let lookup = logpanel::LogTexts(texts);
        let wording = logpanel::wording(duel, lang, &lookup);
        in_log(&duel.log.lines(&wording), link)
    });
    let identity = current.or(logged.flatten())?;
    Some(with_requested_print(duel.statics.as_ref(), link, identity))
}

/// Applies the link's print only when this seat's print table already knows it.
pub(super) fn with_requested_print(
    statics: Option<&baylee_view::GameStatic>,
    link: &rows::ChatCardLink,
    identity: CardIdentity,
) -> CardIdentity {
    let Some(requested) = link.print.as_ref() else {
        return identity;
    };
    let print = statics.and_then(|statics| {
        statics
            .prints
            .iter()
            .enumerate()
            .find_map(|(index, entry)| {
                let entry = entry.as_ref()?;
                let index = u16::try_from(index).ok()?;
                (entry.scryfall_id == requested.scryfall_id
                    && entry.lang.eq_ignore_ascii_case(&requested.lang))
                .then(|| baylee_core::ids::PrintRef::new(index))
            })
    });
    print.map_or(identity, |print| CardIdentity { print, ..identity })
}

/// The picture that can be drawn now, or `None` for the constructed face.
pub(super) fn stopgap(
    textures: &crate::textures::CardTextures,
    identity: CardIdentity,
    prefer_text: bool,
) -> Option<ImageKey> {
    if prefer_text {
        return None;
    }
    let normal = ImageKey::of(identity, ArtSize::Normal);
    if textures.has_arrived(normal) {
        return Some(normal);
    }
    let small = ImageKey::of(identity, ArtSize::Small);
    textures.has_arrived(small).then_some(small)
}

/// Follows a card link without inventing an identity from its candidate list.
pub(super) fn hover(
    mut overs: MessageReader<bevy::picking::events::PointerOver>,
    mut outs: MessageReader<bevy::picking::events::PointerOut>,
    links: Query<&rows::ChatCardLink>,
    duel: Option<Res<Duel>>,
    texts: Option<Res<crate::cardtext::CardTexts>>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    mut hovered: ResMut<ChatHover>,
) {
    let Some(duel) = duel else {
        overs.clear();
        outs.clear();
        hovered.0 = None;
        return;
    };
    let lang = settings.as_deref().map_or(Lang::En, |s| Lang::of(&s.lang));
    for over in overs.read() {
        let Ok(link) = links.get(over.entity) else {
            continue;
        };
        hovered.0 = identity(&duel, texts.as_deref(), lang, link).map(|identity| Request {
            span: over.entity,
            identity,
            at: over.pointer.position,
        });
    }
    for out in outs.read() {
        if hovered
            .0
            .as_ref()
            .is_some_and(|hover| hover.span == out.entity)
        {
            hovered.0 = None;
        }
    }
    if hovered
        .0
        .as_ref()
        .is_some_and(|hover| !links.contains(hover.span))
    {
        hovered.0 = None;
    }
}

/// Where a card of `width` stands beside the pointer without leaving the window.
fn preview_rect(at: Vec2, canvas: Vec2, width: f32) -> Rect {
    let height = width / baylee_client_core::layout::CARD_ASPECT;
    let left = if at.x + width + 32.0 <= canvas.x {
        at.x + 20.0
    } else {
        at.x - width - 20.0
    }
    .clamp(8.0, (canvas.x - width - 8.0).max(8.0));
    let top = (at.y - height / 2.0).clamp(8.0, (canvas.y - height - 8.0).max(8.0));
    Rect::new(left, top, left + width, top + height)
}

/// Draws the current chat preview, rebuilding only on a new link, face or canvas.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one preview, with its render resources
pub(super) fn sync(
    mut commands: Commands,
    hovered: Res<ChatHover>,
    duel: Option<Res<Duel>>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    texts: Option<Res<crate::cardtext::CardTexts>>,
    fonts: Res<UiFonts>,
    font_assets: Option<Res<Assets<Font>>>,
    windows: Query<&Window>,
    scale: Option<Res<UiScale>>,
    mut textures: Option<ResMut<crate::textures::CardTextures>>,
    assets: Option<Res<AssetServer>>,
    ui_materials: Option<ResMut<UiCardMaterials>>,
    material_assets: Option<ResMut<Assets<CardUiMaterial>>>,
    roots: Query<Entity, With<ChatPreview>>,
    mut shown: Local<Option<ShownKey>>,
) {
    let canvas = windows.iter().next().map_or(Vec2::new(1280.0, 720.0), |w| {
        crate::hud::scale::space(w, scale.as_deref())
    });
    let prefer_text = settings.as_deref().is_some_and(|s| s.prefer_text_view);
    let art = hovered.0.as_ref().and_then(|hover| {
        textures
            .as_deref()
            .and_then(|textures| stopgap(textures, hover.identity, prefer_text))
    });
    let generation = texts
        .as_deref()
        .map_or(0, crate::cardtext::CardTexts::generation);
    let materials_ready = ui_materials.is_some() && material_assets.is_some();
    let print_ready = assets.is_some() && duel.as_deref().is_some_and(|d| d.statics.is_some());
    let key = (
        hovered.0.clone(),
        art,
        generation,
        canvas,
        materials_ready,
        print_ready,
    );
    if shown.as_ref() == Some(&key) {
        return;
    }
    *shown = Some(key);
    for root in &roots {
        commands.entity(root).despawn();
    }
    let (Some(request), Some(duel)) = (hovered.0.as_ref(), duel) else {
        return;
    };
    let (Some(mut cache), Some(mut store)) = (ui_materials, material_assets) else {
        return;
    };
    let mut cards = crate::cardmat::UiCards {
        cache: &mut cache,
        assets: &mut store,
    };
    let picture = art.and_then(|key| {
        Some((
            key,
            textures
                .as_deref_mut()?
                .get(key, duel.statics.as_ref()?, assets.as_deref()?),
        ))
    });
    let face = if picture.is_none() {
        texts.as_deref().and_then(|texts| {
            crate::face::of_identity(request.identity.index, request.identity.face, texts)
        })
    } else {
        None
    };
    if picture.is_none() && face.is_none() {
        return;
    }
    let preview_scale = settings.as_deref().map_or(1.0, |s| s.preview_scale);
    let width = (260.0 * preview_scale)
        .clamp(140.0, 360.0)
        .min((canvas.x - 16.0).max(80.0));
    let at = request.at / scale.as_deref().map_or(1.0, |scale| scale.0);
    let rect = preview_rect(at, canvas, width);
    let root = commands
        .spawn((
            ChatPreview,
            Node {
                position_type: PositionType::Absolute,
                left: zpx(rect.min.x),
                top: zpx(rect.min.y),
                width: zpx(width),
                height: zpx(width / baylee_client_core::layout::CARD_ASPECT),
                ..default()
            },
            BoxShadow::new(
                Color::srgba(0.0, 0.0, 0.0, 0.55),
                zpx(0.0),
                zpx(10.0),
                zpx(2.0),
                zpx(24.0),
            ),
            stacked(35),
            Pickable::IGNORE,
        ))
        .id();
    if let Some((key, image)) = picture {
        let finish = duel.statics.as_ref().map_or(
            baylee_client_core::images::FinishTreatment::Plain,
            |statics| crate::cardmat::finish_of(statics, Some(key)),
        );
        let cache_key = format!("chat-preview:{key:?}");
        let material = cards.preview(&cache_key, finish, image);
        let node = commands
            .spawn((
                MaterialNode(material),
                Node {
                    width: zpx(width),
                    height: zpx(width / baylee_client_core::layout::CARD_ASPECT),
                    border_radius: BorderRadius::all(zpx(12.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(root).add_child(node);
    } else if let Some(face) = face {
        let lang = settings.as_deref().map_or(Lang::En, |s| Lang::of(&s.lang));
        let widths =
            crate::face::Widths::of(font_assets.as_deref().and_then(|a| a.get(&fonts.text)));
        crate::face::spawn_ui_card(
            &mut commands,
            &mut cards,
            root,
            lang,
            &face,
            &fonts,
            &widths,
            baylee_client_core::images::FinishTreatment::Plain,
            width,
        );
    }
}

#[cfg(test)]
mod tests;
