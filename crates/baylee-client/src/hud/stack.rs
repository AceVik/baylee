//! The stack, drawn as cards.
//!
//! Each entry is the spell's own picture — or, for an ability, the picture
//! of the permanent it came from — followed by a row of everything it
//! targets, each drawn as its own smaller card.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;

/// The card a stack entry is drawn at.
const STACK_CARD_W: f32 = 66.0;
/// Height of that card.
const STACK_CARD_H: f32 = STACK_CARD_W * 88.0 / 63.0;
/// The smaller card a *target* is drawn at, so the two never read as peers:
/// the thing on the stack is the sentence, its targets are its objects.
const STACK_TARGET_W: f32 = 38.0;
/// Height of a target thumbnail.
const STACK_TARGET_H: f32 = STACK_TARGET_W * 88.0 / 63.0;
/// Panel width. Wide enough for a card, an arrow and three targets.
const STACK_PANEL_W: f32 = 296.0;

/// The stack, next-to-resolve at the top; pinned left of the phase rail.
///
/// Drawn as cards rather than as a list of names, because the stack is the one
/// place in the game where "what is about to happen, and to what" has to be
/// read in a hurry — and a player who has to match two names against a board
/// of twelve permanents is doing the engine's bookkeeping by hand. Each entry
/// is the spell (or the permanent whose ability it is), an arrow, and a
/// picture of everything it points at.
#[allow(clippy::too_many_arguments)] // a panel, a view, and the material store
pub(super) fn spawn_stack_panel(
    commands: &mut Commands,
    lang: Lang,
    board: &baylee_client_core::BoardModel,
    view: &PlayerView,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    mut cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(RAIL_W + 12.0),
                top: px(TAB_H + 12.0),
                width: px(STACK_PANEL_W),
                max_height: percent(62),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                padding: UiRect::all(px(10)),
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            ZIndex(1),
            overlay_shadow(),
            Pickable::IGNORE,
        ))
        .id();

    let title = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(6),
                align_items: AlignItems::Center,
                ..default()
            },
            children![
                (
                    Text::new(Phrase::StackTitle.text(lang)),
                    tf(fonts, 13.0),
                    TextColor(palette::MUTED),
                ),
                (
                    Node {
                        padding: UiRect::axes(px(6.0), px(1.0)),
                        border_radius: BorderRadius::all(px(7)),
                        ..default()
                    },
                    BackgroundColor(palette::PANEL_LIT),
                    children![(
                        Text::new(board.stack.len().to_string()),
                        tf(fonts, 12.0),
                        TextColor(palette::INK),
                    )],
                ),
            ],
        ))
        .id();
    commands.entity(panel).add_child(title);

    for item in &board.stack {
        let entry = spawn_stack_entry(
            commands,
            lang,
            item,
            view,
            statics,
            textures,
            assets,
            fonts,
            faces,
            cards.as_deref_mut(),
        );
        commands.entity(panel).add_child(entry);
    }
    panel
}

/// One row of the stack panel: the object, what it is, and what it points at.
#[allow(clippy::too_many_arguments)] // the same slot arguments, one level down
fn spawn_stack_entry(
    commands: &mut Commands,
    lang: Lang,
    item: &baylee_client_core::board::StackItem,
    view: &PlayerView,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    mut cards: Option<&mut UiCards<'_>>,
) -> Entity {
    // The next thing to resolve is lit and railed; everything under it is
    // flat. Depth is the only ordering a player has to trust here, so it is
    // the only thing the row draws differently.
    let next = item.depth == 0;
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(8),
                padding: UiRect::all(px(6)),
                border: UiRect::left(px(3)),
                align_items: AlignItems::FlexStart,
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(if next {
                palette::PANEL_LIT
            } else {
                Color::NONE
            }),
            BorderColor::all(if next { palette::ACCENT } else { Color::NONE }),
            Pickable::IGNORE,
        ))
        .id();

    let art = spawn_stack_card(
        commands,
        item.art,
        stack_face(item, view, faces, textures),
        STACK_CARD_W,
        STACK_CARD_H,
        statics,
        textures,
        assets,
        fonts,
        cards.as_deref_mut(),
    );
    commands.entity(row).add_child(art);

    let body = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                flex_grow: 1.0,
                min_width: px(0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(body);

    let name = commands
        .spawn((
            Text::new(item.name.clone()),
            tf(fonts, 13.0),
            TextColor(if next { palette::ACCENT } else { palette::INK }),
        ))
        .id();
    commands.entity(body).add_child(name);

    // What kind of thing this is, and whose. An ability names its source even
    // when the source has left — the picture above may be missing, the
    // sentence must not be.
    let kind = match item.kind {
        baylee_client_core::board::StackKind::Spell => Phrase::StackSpell.text(lang).to_string(),
        baylee_client_core::board::StackKind::Ability { source } => {
            view.object(source).map_or_else(
                || Phrase::StackAbilityBare.text(lang).to_string(),
                |o| Phrase::StackAbility.fill(lang, &[&o.name]),
            )
        }
    };
    let subtitle = commands
        .spawn((
            Text::new(format!("{kind} — {}", statics.seat_name(item.controller))),
            tf(fonts, 10.0),
            TextColor(palette::MUTED),
        ))
        .id();
    commands.entity(body).add_child(subtitle);

    if !item.targets.is_empty() {
        let targets = spawn_stack_targets(
            commands, item, next, statics, textures, assets, fonts, cards,
        );
        commands.entity(body).add_child(targets);
    }

    row
}

/// The arrow and everything a stack entry points at, as one wrapping row.
#[allow(clippy::too_many_arguments)] // the same slot arguments again
fn spawn_stack_targets(
    commands: &mut Commands,
    item: &baylee_client_core::board::StackItem,
    next: bool,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    mut cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                row_gap: px(4),
                margin: UiRect::top(px(2)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    let arrow = commands
        .spawn((
            Text::new("→"),
            tf(fonts, 14.0),
            TextColor(if next {
                palette::ACCENT
            } else {
                palette::MUTED
            }),
        ))
        .id();
    commands.entity(row).add_child(arrow);

    for target in &item.targets {
        let chip = spawn_stack_target(
            commands,
            target,
            statics,
            textures,
            assets,
            fonts,
            cards.as_deref_mut(),
        );
        commands.entity(row).add_child(chip);
    }
    row
}

/// The face a stack entry should show instead of its art, if any.
///
/// A spell on the stack has its own object and its own projected text; an
/// ability has neither, so it keeps its picture and its name.
fn stack_face(
    item: &baylee_client_core::board::StackItem,
    view: &PlayerView,
    faces: &FaceCtx<'_>,
    textures: &CardTextures,
) -> Option<CardFace> {
    let object = view.object(item.id)?;
    faces.object(object, textures, item.art)
}

/// A card in the stack panel: its art, its face, or a plain plate with the
/// card's back colour when the seat may not know what it is.
#[allow(clippy::too_many_arguments)] // the slot, the card, and the stores
fn spawn_stack_card(
    commands: &mut Commands,
    art: Option<ImageKey>,
    face: Option<CardFace>,
    width: f32,
    height: f32,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let slot = commands
        .spawn((
            Node {
                width: px(width),
                height: px(height),
                flex_shrink: 0.0,
                border_radius: card_radius(width),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            soft_shadow(),
            Pickable::IGNORE,
        ))
        .id();
    if let Some(key) = art {
        let image = textures.get(key, statics, assets);
        let visual = spawn_card_art(
            commands,
            image,
            face.as_ref(),
            width,
            height,
            crate::face::Detail::Compact,
            fonts,
            // Nothing on the stack is on a battlefield, so no glow: the
            // border says what the rules have made a *permanent*, and a
            // spell wearing one would be claiming something untrue.
            CardLook::art(key, finish_of(statics, Some(key)), 0),
            cards,
        );
        commands.entity(slot).add_child(visual);
    }
    slot
}

/// One target of a stack entry: its picture when it has one, a chip when it
/// does not — a player, a token this seat cannot name, a face-down permanent.
#[allow(clippy::too_many_arguments)] // as above
fn spawn_stack_target(
    commands: &mut Commands,
    target: &baylee_client_core::board::StackTarget,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    cards: Option<&mut UiCards<'_>>,
) -> Entity {
    if target.art.is_some() {
        return spawn_stack_card(
            commands,
            target.art,
            None,
            STACK_TARGET_W,
            STACK_TARGET_H,
            statics,
            textures,
            assets,
            fonts,
            cards,
        );
    }
    // A player is a name and a heart, not a rectangle pretending to be a
    // card. Anything else with no picture (a token, something face down) is
    // its name in the same chip, so the row never has a hole in it.
    let (glyph, label) = match target.player() {
        Some(player) => (Some(glyph::HEART), statics.seat_name(player).to_string()),
        None => (None, target.name.clone().unwrap_or_else(|| "?".into())),
    };
    let chip = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(6.0), px(3.0)),
                border_radius: BorderRadius::all(px(9)),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            Pickable::IGNORE,
        ))
        .id();
    if let Some(glyph) = glyph {
        let icon = commands
            .spawn((
                Text::new(glyph.to_string()),
                icon_tf(fonts, 10.0),
                TextColor(palette::DANGER),
            ))
            .id();
        commands.entity(chip).add_child(icon);
    }
    let text = commands
        .spawn((Text::new(label), tf(fonts, 11.0), TextColor(palette::INK)))
        .id();
    commands.entity(chip).add_child(text);
    chip
}
