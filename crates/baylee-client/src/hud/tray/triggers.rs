//! The zone dialog's body while a player sorts their own triggers: why the
//! suggestion stands as it does, and a row for each trigger in the order they
//! will resolve, the first at the top.
//!
//! A row says what the trigger will do in the card's own words, and its
//! picture and the hover preview show the card. It moves by dragging, by its
//! arrows, or from the keyboard: the row is taken up as any arrangement's card
//! is, and nothing here decides anything `triggerorder` did not.

use super::rows::{row_slot, spawn_thumb};
use super::{
    BrowseRow, CardTextures, FaceCtx, GridCtx, TRAY_GAP, TRAY_ROW_H, TRAY_ROW_PAD, TRAY_SIDE,
    TRAY_THUMB_W, TrayCard, UiCards, UiFonts, dialog_label, dialog_text,
};
use crate::ambience::Feel;
use crate::hud::{CardLens, Hint, LogLink, btn_radius, icon_tf, palette};
use crate::ui_button::UiButton;
use baylee_client_core::arrange::{Arrangement, Row};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::triggerorder::sheet::{self, Sheet};
use baylee_core::ids::ObjectId;
use baylee_view::{OrderedTrigger, PlayerView, StackItem};
use bevy::picking::events::PointerDragDrop;
use bevy::prelude::*;

/// A row's arrows: one step earlier or later.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct TriggerMove {
    /// The trigger.
    pub object: ObjectId,
    /// Towards the end of the list: resolving later.
    pub later: bool,
}

/// Places these twins one by one, or together again.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct TriggerSplit(pub u32);

/// One arrow's side, a finger's width at a table.
const ARROW: f32 = 26.0;

/// The sheet's whole body.
#[allow(clippy::too_many_arguments)] // the rows, the sheet, and the stores
pub(super) fn spawn_sheet(
    commands: &mut Commands,
    list: Entity,
    lang: Lang,
    sheet: &Sheet,
    arrangement: &Arrangement,
    rows: &[BrowseRow],
    grid: GridCtx<'_>,
    faces: &FaceCtx<'_>,
    textures: &mut CardTextures,
    assets: &AssetServer,
    cards: &mut Option<&mut UiCards<'_>>,
) {
    let head = spawn_head(commands, lang, grid.fonts, sheet);
    commands.entity(list).add_child(head);
    let pile = arrangement.cards(Row::Pile(0));
    let runs = sheet.runs(pile, &grid.view.trigger_order);
    // "About you" on every row says nothing the question did not.
    let told = &grid.view.trigger_order;
    let tells = told.iter().any(|t| t.about_name.is_some())
        || told
            .windows(2)
            .any(|t| t[0].about_player != t[1].about_player);
    for (place, run) in runs.iter().enumerate() {
        let handle = run.first();
        let row = rows.iter().find(|row| Some(&row.id) == handle);
        let trigger = grid
            .view
            .trigger_order
            .iter()
            .find(|t| Some(&t.handle) == handle);
        if let (Some(row), Some(trigger)) = (row, trigger) {
            // The row is lit as any of its copies is.
            let mut row = row.clone();
            let members = || rows.iter().filter(|r| run.contains(&r.id));
            row.standing.selected = members().any(|r| r.standing.selected);
            row.standing.focused = members().any(|r| r.standing.focused);
            let drawn = spawn_row(
                commands,
                lang,
                sheet,
                &row,
                trigger,
                (place, runs.len(), run.len()),
                tells,
                grid,
                faces,
                textures,
                assets,
                cards,
            );
            commands.entity(list).add_child(drawn);
        }
    }
}

/// Why the order is what it is, and what the list means.
fn spawn_head(commands: &mut Commands, lang: Lang, fonts: &UiFonts, sheet: &Sheet) -> Entity {
    let head = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(3.0),
                padding: UiRect::new(
                    px(TRAY_SIDE),
                    px(TRAY_SIDE),
                    px(TRAY_ROW_PAD),
                    px(TRAY_ROW_PAD),
                ),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let reason = dialog_text(
        commands,
        fonts,
        sheet.reason().phrase().text(lang),
        13.0,
        palette::CANDLE,
    );
    let how = dialog_text(
        commands,
        fonts,
        Phrase::TriggerOrderHow.text(lang),
        10.5,
        palette::DIALOG_SOFT,
    );
    commands.entity(head).add_children(&[reason, how]);
    head
}

/// The words of a row's trigger, in the player's language.
fn sentence(trigger: &OrderedTrigger, faces: &FaceCtx<'_>) -> Option<String> {
    let StackItem::Ability {
        text,
        rules,
        ability,
        token,
        ..
    } = trigger.item
    else {
        return None;
    };
    let blocks = crate::hud::stack::sentence::ability_sentence(text, rules, ability, token, faces)?;
    let words: Vec<&str> = blocks
        .iter()
        .map(baylee_client_core::card_face::TextBlock::text)
        .collect();
    Some(words.join(" "))
}

/// What the row's event was about, as far as this seat may read it.
fn about(
    trigger: &OrderedTrigger,
    copies: usize,
    view: &PlayerView,
    grid: GridCtx<'_>,
    lang: Lang,
) -> String {
    let player = trigger.about_player.and_then(|seat| {
        if seat == view.seat {
            return Some(Phrase::You.text(lang).to_string());
        }
        grid.statics
            .seats
            .iter()
            .find(|identity| identity.player == seat)
            .map(|identity| identity.display_name.clone())
    });
    // Copies stand for several events: one object's name would be one of them.
    let object = trigger.about_name.as_ref().filter(|_| copies == 1);
    let named = match (object, player) {
        (Some(name), Some(player)) => format!("{name} \u{b7} {player}"),
        (Some(name), None) => name.clone(),
        (None, Some(player)) => player,
        (None, None) => return String::new(),
    };
    Phrase::TriggerOrderAbout.fill(lang, &[&named])
}

/// The little words a row may carry, in one line.
fn marks(trigger: &OrderedTrigger, view: &PlayerView, sheet: &Sheet, lang: Lang) -> String {
    let mut said = Vec::new();
    if sheet.is_remembered(trigger.handle) {
        said.push(Phrase::TriggerOrderMark.text(lang));
    }
    if sheet::is_optional(trigger, view) {
        said.push(Phrase::TriggerOrderOptional.text(lang));
    }
    if trigger.asks_targets {
        said.push(Phrase::TriggerOrderChooseTarget.text(lang));
    }
    if trigger.modal {
        said.push(Phrase::TriggerOrderChooseMode.text(lang));
    }
    said.join(" \u{b7} ")
}

/// One trigger.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one row, read left to right
fn spawn_row(
    commands: &mut Commands,
    lang: Lang,
    sheet: &Sheet,
    row: &BrowseRow,
    trigger: &OrderedTrigger,
    (place, of, copies): (usize, usize, usize),
    tells: bool,
    grid: GridCtx<'_>,
    faces: &FaceCtx<'_>,
    textures: &mut CardTextures,
    assets: &AssetServer,
    cards: &mut Option<&mut UiCards<'_>>,
) -> Entity {
    let fonts = grid.fonts;
    let slot = row_slot(commands, row, TRAY_ROW_H, TRAY_ROW_PAD);
    // Grown by what the card says, and never cut: the words are why the
    // player is sorting at all.
    commands
        .entity(slot)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.height = Val::Auto;
            node.min_height = px(TRAY_ROW_H);
            node.overflow = Overflow::visible();
        });
    commands.entity(slot).observe(dropped_on_a_row);

    let number = dialog_label(
        commands,
        fonts,
        &(place + 1).to_string(),
        15.0,
        palette::CANDLE,
    );
    commands.entity(number).insert(Node {
        width: px(22.0),
        flex_shrink: 0.0,
        ..default()
    });
    let thumb = spawn_thumb(
        commands,
        lang,
        row,
        grid.statics,
        textures,
        assets,
        fonts,
        cards,
        TRAY_THUMB_W,
    );
    // The picture is the link, and not the row: a preview opened from the
    // row would stand over the words the player is reading.
    if let Some(card) = sheet::card_of(trigger, grid.view) {
        commands.entity(thumb).insert((
            Pickable::default(),
            LogLink {
                card: Some(card),
                token: None,
            },
            CardLens,
        ));
    }
    commands.entity(slot).add_children(&[number, thumb]);

    let body = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                row_gap: px(2.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let head = commands
        .spawn((
            Node {
                column_gap: px(TRAY_GAP),
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Baseline,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let name = if copies > 1 {
        format!("{} \u{d7}{copies}", trigger.name)
    } else {
        trigger.name.clone()
    };
    let name = dialog_text(commands, fonts, &name, 13.0, palette::DIALOG_INK);
    commands.entity(head).add_child(name);
    let about = about(trigger, copies, grid.view, grid, lang);
    if tells && !about.is_empty() {
        let about = dialog_text(commands, fonts, &about, 11.0, palette::DIALOG_SOFT);
        commands.entity(head).add_child(about);
    }
    commands.entity(body).add_child(head);
    if let Some(words) = sentence(trigger, faces) {
        let words = dialog_text(commands, fonts, &words, 11.5, palette::DIALOG_SOFT);
        // This row grows with what the card says, so its words wrap, where a
        // fixed band's line would be clipped.
        commands.entity(words).insert(TextLayout::default());
        commands.entity(body).add_child(words);
    }
    let edge = match place {
        0 => Some(Phrase::TriggerOrderResolvesFirst),
        at if at + 1 == of => Some(Phrase::TriggerOrderResolvesLast),
        _ => None,
    };
    let marks = marks(trigger, grid.view, sheet, lang);
    let said: Vec<&str> = edge
        .iter()
        .map(|phrase| phrase.text(lang))
        .chain((!marks.is_empty()).then_some(marks.as_str()))
        .collect();
    if !said.is_empty() {
        let line = said.join(" \u{b7} ");
        let line = dialog_text(commands, fonts, &line, 10.5, palette::CANDLE);
        commands.entity(body).add_child(line);
    }
    commands.entity(slot).add_child(body);

    let twin = trigger.twin;
    let split = (copies > 1 || sheet.is_split(twin)).then(|| {
        let words = if copies > 1 {
            Phrase::TriggerOrderSplit
        } else {
            Phrase::TriggerOrderJoin
        };
        let words = dialog_text(
            commands,
            fonts,
            words.text(lang),
            10.5,
            palette::DIALOG_SOFT,
        );
        let button = commands
            .spawn((
                TriggerSplit(twin),
                UiButton,
                Node {
                    padding: UiRect::axes(px(8.0), px(4.0)),
                    border_radius: btn_radius(),
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Feel::tinting_to(Color::NONE, palette::DIALOG_LIT),
            ))
            .id();
        commands.entity(button).add_child(words);
        button
    });
    if let Some(split) = split {
        commands.entity(slot).add_child(split);
    }

    let arrows = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(2.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let up = spawn_arrow(commands, lang, fonts, row.id, false, place > 0);
    let down = spawn_arrow(commands, lang, fonts, row.id, true, place + 1 < of);
    commands.entity(arrows).add_children(&[up, down]);
    commands.entity(slot).add_child(arrows);
    slot
}

/// One arrow. At the end of the list it keeps its place and answers nothing,
/// so the rows' arrows stand in one column.
fn spawn_arrow(
    commands: &mut Commands,
    lang: Lang,
    fonts: &UiFonts,
    object: ObjectId,
    later: bool,
    live: bool,
) -> Entity {
    let arrow = commands
        .spawn((
            Node {
                width: px(ARROW),
                height: px(ARROW),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .id();
    if live {
        let hint = if later {
            Phrase::TriggerOrderMoveDown
        } else {
            Phrase::TriggerOrderMoveUp
        };
        commands.entity(arrow).insert((
            TriggerMove { object, later },
            UiButton,
            Feel::tinting_to(Color::NONE, palette::DIALOG_LIT),
            Hint(hint.text(lang).to_string()),
        ));
        let glyph = if later { '\u{f078}' } else { '\u{f077}' };
        let glyph = commands
            .spawn((
                Text::new(glyph.to_string()),
                icon_tf(fonts, 11.0),
                TextColor(palette::DIALOG_INK),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(arrow).add_child(glyph);
    } else {
        commands.entity(arrow).insert(Pickable::IGNORE);
    }
    arrow
}

/// One row dragged onto another takes its place.
fn dropped_on_a_row(
    drop: On<PointerDragDrop>,
    rows: Query<&TrayCard>,
    parents: Query<&ChildOf>,
    mut duel: ResMut<crate::Duel>,
) {
    let row_of = |mut entity: Entity| loop {
        if let Ok(row) = rows.get(entity) {
            return Some(row.object);
        }
        entity = parents.get(entity).ok()?.parent();
    };
    let (Some(onto), Some(dragged)) = (row_of(drop.entity), row_of(drop.dropped)) else {
        return;
    };
    crate::triggersheet::drop_on(&mut duel, dragged, onto);
}
