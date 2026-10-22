//! Drawing a card on the shell's kit: its ground, title row, meter,
//! close mark and actions.

use super::super::{Fetch, Progress, UpdateButton};
use super::look::{Action, CLOSE, Card, Look};
use super::{
    CardSize, ProgressFill, ProgressLine, ProgressPercent, ROW_GAP, TOAST_BORDER, TOAST_GAP,
    ToastActions, ToastPlacement, ToastScroll, UpdateToast, fill_width, meter,
};
use crate::hud::{UiFonts, icon_tf, palette, tf, tf_bold};
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::role::Role;
use crate::shellkit::{px_fixed, tokens};
use baylee_client_core::i18n::{Lang, Phrase};
use bevy::color::Mix as _;
use bevy::prelude::*;
use bevy::ui::{percent, px};

/// The accent edge on the card's left, as the lobby's toasts wear it.
const ACCENT_EDGE: f32 = 3.0;
/// The card's width at the default text step: the download's line and the
/// ready card's two actions each on one row, in both languages, and clear
/// of the front door's language strip at 1280 px, whose German row starts
/// at 362 (live check of 22.10.2026; at 380 the ready card covered
/// "English").
const CARD_WIDTH: f32 = 336.0;

/// The card's elevation over the painting: one soft shadow.
fn card_shadow() -> BoxShadow {
    BoxShadow(vec![ShadowStyle {
        color: Color::srgba(0.0, 0.0, 0.0, 0.45),
        x_offset: px_fixed(0.0),
        y_offset: px_fixed(6.0),
        spread_radius: px_fixed(0.0),
        blur_radius: px_fixed(18.0),
    }])
}

/// Figures that keep their width as they count: lining and tabular.
fn counting(fonts: &UiFonts, size: f32) -> TextFont {
    let mut font = tf(fonts, size);
    font.font_features = bevy::text::FontFeatures::builder()
        .enable(bevy::text::FontFeatureTag::LINING_FIGURES)
        .enable(bevy::text::FontFeatureTag::TABULAR_FIGURES)
        .build();
    font
}

/// A line of the card's text.
fn line(commands: &mut Commands, kit: Kit, text: String, ink: Color) -> Entity {
    commands
        .spawn((
            Text::new(text),
            TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
            Node {
                min_width: px_fixed(0.0),
                flex_shrink: 0.0,
                ..default()
            },
            tf(kit.fonts, kit.m.small),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id()
}

/// The title row: the mark in the accent and the title, which names the
/// version. Room is left on its right for the close mark.
fn title_row(commands: &mut Commands, kit: Kit, look: &Look, pad: f32) -> Entity {
    let row = commands
        .spawn((
            Node {
                width: percent(100),
                min_width: px_fixed(0.0),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(8.0),
                padding: UiRect::right(px(if look.close {
                    (kit.m.hit - pad).max(0.0)
                } else {
                    0.0
                })),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mark = commands
        .spawn((
            Text::new(look.mark.to_string()),
            icon_tf(kit.fonts, kit.m.text * 0.95),
            TextColor(look.accent),
            Pickable::IGNORE,
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(look.title.clone()),
            TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
            // The rest of the row, whatever the text measures: a wrapping
            // text's own basis in a row is its narrowest, one character.
            Node {
                min_width: px_fixed(0.0),
                flex_basis: px_fixed(0.0),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                ..default()
            },
            tf_bold(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_children(&[mark, title]);
    row
}

/// The stage with its share, the slim bar, and the line under it.
fn meter_part(
    commands: &mut Commands,
    kit: Kit,
    lang: Lang,
    stage: Fetch,
    progress: Option<&Progress>,
) -> Entity {
    let (fill, share, under) = meter(stage, progress, lang);
    let part = commands
        .spawn((
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(5.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let head = commands
        .spawn((
            Node {
                width: percent(100),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Baseline,
                column_gap: kit.m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let said = match stage {
        Fetch::Downloading => Phrase::UpdateDownloading,
        Fetch::Verifying => Phrase::UpdateVerifying,
        Fetch::Unpacking => Phrase::UpdateUnpacking,
    };
    let stage_words = line(commands, kit, said.text(lang).to_owned(), tokens::INK);
    // The row's rest, as the title's: a wrapping text's own basis in a row
    // is one character wide.
    commands
        .entity(stage_words)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.flex_basis = px_fixed(0.0);
            node.flex_grow = 1.0;
            node.flex_shrink = 1.0;
        });
    let share = commands
        .spawn((
            ProgressPercent,
            Text::new(share),
            counting(kit.fonts, kit.m.small),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(head).add_children(&[stage_words, share]);
    let track = commands
        .spawn((
            Node {
                width: percent(100),
                height: kit.m.px(4.0),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::TRACK_OFF),
            Pickable::IGNORE,
        ))
        .id();
    let filled = commands
        .spawn((
            ProgressFill,
            Node {
                width: fill_width(fill),
                height: percent(100),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::ACCENT),
            BoxShadow(vec![ShadowStyle {
                color: tokens::ACCENT.with_alpha(0.45),
                x_offset: px_fixed(0.0),
                y_offset: px_fixed(0.0),
                spread_radius: px_fixed(0.0),
                blur_radius: px_fixed(6.0),
            }]),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(track).add_child(filled);
    let under = commands
        .spawn((
            ProgressLine,
            Text::new(under),
            TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
            Node {
                min_width: px_fixed(0.0),
                ..default()
            },
            counting(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(part).add_children(&[head, track, under]);
    part
}

/// The close mark at the card's top right: a hit area as large as the
/// kit's, laid over the padding, so it hides the card without taking a row.
fn close_mark(commands: &mut Commands, kit: Kit) -> Entity {
    let face = commands
        .spawn((
            Node {
                width: kit.m.px(24.0),
                height: kit.m.px(24.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            crate::ambience::Feel::rising_to(Color::NONE, tokens::HOVER),
            Pickable::IGNORE,
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new(CLOSE.to_string()),
            icon_tf(kit.fonts, kit.m.small * 0.9),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_child(glyph);
    let hit = commands
        .spawn((
            Role::Hit,
            UpdateButton::Hide,
            Node {
                position_type: PositionType::Absolute,
                top: px_fixed(0.0),
                right: px_fixed(0.0),
                width: px(kit.m.hit),
                height: px(kit.m.hit),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .id();
    commands.entity(hit).add_child(face);
    hit
}

/// The card's actions in a row that wraps, the release notes (a ghost) to
/// the right of the rest.
fn action_row(commands: &mut Commands, kit: Kit, actions: Vec<Action>) -> Entity {
    let row = commands
        .spawn((
            Node {
                width: percent(100),
                min_width: px_fixed(0.0),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(8.0),
                row_gap: kit.m.px(6.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for action in actions {
        // The release notes as a ghost go to the right of the choices; a
        // ghost that is a choice of its own ("Keep it") stays with them.
        let aside = action.weight == Weight::Ghost && matches!(action.does, UpdateButton::Open(_));
        let button = controls::button(
            commands,
            kit,
            &action.label,
            action.weight,
            Live::Yes,
            None,
            action.does,
        );
        commands
            .entity(button)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.min_width = px_fixed(0.0);
                node.max_width = percent(100);
                if aside {
                    node.margin.left = Val::Auto;
                }
            });
        commands.entity(row).add_child(button);
    }
    row
}

pub(super) fn spawn_card(commands: &mut Commands, kit: Kit, lang: Lang, card: &Card) -> Entity {
    let look = Look::of(card, lang);
    let width = kit.m.scaled(CARD_WIDTH);
    let pad = kit.m.pad.min(kit.m.scaled(14.0));
    // Opaque, as the kit's sheets are: a card of words never lets the
    // painting through (the shell design, §2.2). The accent only tints
    // its top, and both stops are opaque, so the tint cannot thin it.
    let base = tokens::OPAQUE;
    let tinted = base.mix(&look.accent, 0.14);
    let edge = look.accent.with_alpha(0.35);
    let toast = commands
        .spawn((
            UpdateToast,
            ToastPlacement(false),
            CardSize { width, pad },
            Role::Toast,
            Node {
                position_type: PositionType::Absolute,
                left: px_fixed(TOAST_GAP),
                bottom: px_fixed(TOAST_GAP),
                width: px(width),
                min_width: px_fixed(0.0),
                min_height: px_fixed(0.0),
                overflow: Overflow::clip(),
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(ROW_GAP),
                padding: UiRect::all(px(pad)),
                border: UiRect::new(
                    px_fixed(ACCENT_EDGE),
                    px_fixed(TOAST_BORDER),
                    px_fixed(TOAST_BORDER),
                    px_fixed(TOAST_BORDER),
                ),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(base),
            BackgroundGradient::from(LinearGradient::to_bottom(vec![tinted.into(), base.into()])),
            BorderColor {
                top: edge,
                right: edge,
                bottom: edge,
                left: look.accent.with_alpha(0.9),
            },
            card_shadow(),
            GlobalZIndex(tokens::z::TOAST),
        ))
        .id();
    let body = commands
        .spawn((
            ToastScroll,
            ScrollPosition::default(),
            Node {
                width: percent(100),
                min_width: px_fixed(0.0),
                min_height: px_fixed(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(ROW_GAP),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(toast).add_child(body);
    let title = title_row(commands, kit, &look, pad);
    commands.entity(body).add_child(title);
    if let Some(stage) = look.fetch {
        let part = meter_part(commands, kit, lang, stage, card.progress);
        commands.entity(body).add_child(part);
    }
    for text in &look.lines {
        let said = line(commands, kit, text.clone(), palette::MUTED);
        commands.entity(body).add_child(said);
    }
    let close = look.close;
    if !look.actions.is_empty() {
        let actions = commands
            .spawn((
                ToastActions,
                ToastScroll,
                ScrollPosition::default(),
                Node {
                    width: percent(100),
                    min_width: px_fixed(0.0),
                    min_height: px_fixed(0.0),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
            ))
            .id();
        let row = action_row(commands, kit, look.actions);
        commands.entity(actions).add_child(row);
        commands.entity(toast).add_child(actions);
    }
    if close {
        let mark = close_mark(commands, kit);
        commands.entity(toast).add_child(mark);
    }
    toast
}
