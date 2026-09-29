//! The import and export dialogs, and the one system that touches the
//! clipboard and the file system for them.
//!
//! Every decision is `baylee_client_core::deckbuilder::transfer`'s; this is
//! the node tree over the builder, as the printing picker is, and the
//! effects a model cannot have: reading and writing the clipboard (through
//! `bevy_clipboard`, which is the system clipboard natively and the browser's
//! in a page) and saving a file (the player's downloads folder natively, a
//! browser download in a page).

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::deckbuilder::transfer::{
    FormatId, Importing, Line, Said, Stage, Tone, Transfer, format_label,
};
use std::fmt::Write as _;

/// The most of a pasted or exported text the dialog shows, in lines. The
/// whole text is what is read or written; this is only the preview.
const PREVIEW_LINES: usize = 40;

/// An effect a button or a key asked for, carried out by [`act`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ask {
    /// Read the clipboard into the import box.
    Paste,
    /// Put the export on the clipboard.
    Copy,
    /// Save the export as a file.
    Save,
}

/// The import or export dialog, over the whole builder.
#[allow(clippy::too_many_lines)] // one dialog, two faces, each a column of rows
pub(crate) fn transfer_dialog(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
    deck: &DeckBuilder,
    transfer: &Transfer,
    scrolled: &Scrolled,
) -> Entity {
    let shade = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(px(metrics.pad)),
                ..default()
            },
            BackgroundColor(palette::SHADOW.with_alpha(0.82)),
            Press::TransferClose,
            ZIndex(20),
        ))
        .id();
    let panel = commands
        .spawn((
            Node {
                width: percent(100),
                max_height: percent(100),
                overflow: Overflow::scroll_y(),
                max_width: px(if metrics.frame == Frame::Phone {
                    520.0
                } else {
                    760.0
                }),
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap),
                padding: UiRect::all(px(metrics.pad)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(14)),
                ..default()
            },
            BackgroundColor(palette::PANEL.with_alpha(0.98)),
            BorderColor::all(palette::DOCK_EDGE),
            crate::hud::soft_shadow(),
            Press::TransferNothing,
            crate::lobby::Scrollable(List::Transfer),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::Transfer))),
        ))
        .id();
    commands.entity(shade).add_child(panel);

    let head = row(commands, metrics, false);
    let title = heading(
        commands,
        fonts,
        metrics,
        match transfer {
            Transfer::Import(_) => Phrase::ImportTitle,
            Transfer::Export(_) => Phrase::ExportTitle,
        }
        .text(lang),
    );
    let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
    let close = button(
        commands,
        fonts,
        metrics,
        Phrase::TransferClose.text(lang),
        Press::TransferClose,
        palette::PANEL_LIT,
        true,
    );
    for child in [title, gap, close] {
        commands.entity(head).add_child(child);
    }
    commands.entity(panel).add_child(head);

    match transfer {
        Transfer::Import(importing) => {
            import_face(commands, fonts, metrics, lang, deck, importing, panel);
        }
        Transfer::Export(exporting) => {
            let written = deck.export(exporting.format);
            let chips = row(commands, metrics, true);
            for format in FormatId::ALL {
                let chip = chip(
                    commands,
                    fonts,
                    metrics,
                    format_label(format, lang),
                    Press::ExportFormat(format),
                    format == exporting.format,
                );
                commands.entity(chips).add_child(chip);
            }
            commands.entity(panel).add_child(chips);
            let preview = text_box(commands, fonts, metrics, &written.text, None);
            commands.entity(panel).add_child(preview);
            said(
                commands,
                fonts,
                metrics,
                panel,
                &deck.export_lines(&written, lang),
            );
            let actions = row(commands, metrics, true);
            let copy = button(
                commands,
                fonts,
                metrics,
                Phrase::CopyToClipboard.text(lang),
                Press::ExportCopy,
                palette::ACCENT,
                true,
            );
            commands.entity(actions).add_child(copy);
            if can_save_files() {
                let save = button(
                    commands,
                    fonts,
                    metrics,
                    Phrase::SaveToFile.text(lang),
                    Press::ExportSave,
                    palette::PANEL_LIT,
                    true,
                );
                commands.entity(actions).add_child(save);
            }
            commands.entity(panel).add_child(actions);
            let keys = note(commands, fonts, metrics, Phrase::ExportKeys.text(lang));
            commands.entity(panel).add_child(keys);
        }
    }
    shade
}

/// The import dialog's body: the paste box, what it was read as, and the
/// way to take it.
#[allow(clippy::too_many_arguments)] // one face of the dialog, drawn into its panel
fn import_face(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
    deck: &DeckBuilder,
    importing: &Importing,
    panel: Entity,
) {
    let done = matches!(importing.stage(), Stage::Done(_));
    if !done {
        let tools = row(commands, metrics, true);
        let paste = button(
            commands,
            fonts,
            metrics,
            Phrase::PasteFromClipboard.text(lang),
            Press::ImportPaste,
            palette::ACCENT,
            true,
        );
        let clear = button(
            commands,
            fonts,
            metrics,
            Phrase::ImportClear.text(lang),
            Press::ImportClear,
            palette::PANEL_LIT,
            !importing.pasted().is_empty(),
        );
        commands.entity(tools).add_children(&[paste, clear]);
        commands.entity(panel).add_child(tools);
        let pasted = text_box(
            commands,
            fonts,
            metrics,
            importing.pasted(),
            Some(Phrase::ImportBoxEmpty.text(lang)),
        );
        commands.entity(panel).add_child(pasted);
    }
    said(commands, fonts, metrics, panel, &deck.import_lines(lang));
    let actions = row(commands, metrics, true);
    if done {
        let close = button(
            commands,
            fonts,
            metrics,
            Phrase::TransferClose.text(lang),
            Press::TransferClose,
            palette::ACCENT,
            true,
        );
        commands.entity(actions).add_child(close);
    } else {
        let take = button(
            commands,
            fonts,
            metrics,
            Phrase::ImportTake.text(lang),
            Press::ImportTake,
            palette::ACCENT,
            importing.ready(),
        );
        commands.entity(actions).add_child(take);
    }
    commands.entity(panel).add_child(actions);
    if !done {
        let keys = note(commands, fonts, metrics, Phrase::ImportKeys.text(lang));
        commands.entity(panel).add_child(keys);
    }
}

/// A framed block of text: the paste, or the export. At most
/// [`PREVIEW_LINES`] of it; the count of the rest after that.
fn text_box(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    text: &str,
    empty: Option<&str>,
) -> Entity {
    let frame = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(metrics.pad * 0.6)),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                min_height: px(metrics.tap * 1.5),
                ..default()
            },
            BackgroundColor(palette::SHADOW.with_alpha(0.35)),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.5)),
            Pickable::IGNORE,
        ))
        .id();
    let (shown, ink) = if text.trim().is_empty() {
        (empty.unwrap_or_default().to_string(), palette::MUTED)
    } else {
        let lines: Vec<&str> = text.lines().collect();
        let mut shown = lines
            .iter()
            .take(PREVIEW_LINES)
            .copied()
            .collect::<Vec<_>>()
            .join("\n");
        if lines.len() > PREVIEW_LINES {
            let _ = write!(shown, "\n… +{}", lines.len() - PREVIEW_LINES);
        }
        (shown, palette::INK)
    };
    let body = commands
        .spawn((
            Text::new(shown),
            tf(fonts, metrics.small),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(frame).add_child(body);
    frame
}

/// The dialog's lines, coloured by what they are.
fn said(commands: &mut Commands, fonts: &UiFonts, metrics: Metrics, panel: Entity, lines: &[Line]) {
    for line in lines {
        let ink = match line.tone {
            Tone::Info => palette::MUTED,
            Tone::Good => palette::INK,
            Tone::Warn => palette::ACTIVE,
        };
        let text = commands
            .spawn((
                Text::new(line.text.clone()),
                tf(fonts, metrics.small),
                TextColor(ink),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(panel).add_child(text);
    }
}

/// Whether this build can save a file for the player: natively where there
/// is a downloads folder, and in a browser always.
pub(crate) fn can_save_files() -> bool {
    use baylee_client_core::userdirs::{Kind, Os, real_env, user_dir};
    cfg!(target_arch = "wasm32") || user_dir(Kind::Downloads, Os::current(), &real_env).is_some()
}

/// A paste the clipboard has not answered yet.
#[derive(Default)]
pub(crate) struct Pending(Option<bevy::clipboard::ClipboardRead>);

/// Carries out what the dialogs asked for: the clipboard and the file
/// system, which the model cannot touch.
///
/// Reads the state before it writes it, because a `ResMut` taken on a quiet
/// frame marks the lobby changed and rebuilds the screen.
pub(crate) fn act(
    mut state: ResMut<LobbyState>,
    mut clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
    mut pending: Local<Pending>,
) {
    if let Some(answer) = pending
        .0
        .as_mut()
        .and_then(bevy::clipboard::ClipboardRead::poll_result)
    {
        pending.0 = None;
        if let Ok(text) = answer {
            state.lobby.builder_mut().import_paste(&text);
        }
    }
    if state.transfer_asks.is_empty() {
        return;
    }
    let asks = std::mem::take(&mut state.transfer_asks);
    for ask in asks {
        match ask {
            Ask::Paste => {
                let Some(clipboard) = clipboard.as_deref_mut() else {
                    continue;
                };
                let mut read = clipboard.fetch_text();
                match read.poll_result() {
                    Some(Ok(text)) => state.lobby.builder_mut().import_paste(&text),
                    Some(Err(_)) => {}
                    None => pending.0 = Some(read),
                }
            }
            Ask::Copy => {
                let Some(Transfer::Export(exporting)) = state.lobby.builder().transfer() else {
                    continue;
                };
                let text = state.lobby.builder().export(exporting.format).text;
                let copied = clipboard
                    .as_deref_mut()
                    .is_some_and(|clipboard| clipboard.set_text(text).is_ok());
                state.lobby.builder_mut().export_said(if copied {
                    Said::Copied
                } else {
                    Said::CopyFailed
                });
            }
            Ask::Save => {
                let Some(Transfer::Export(exporting)) = state.lobby.builder().transfer() else {
                    continue;
                };
                let format = exporting.format;
                let deck = state.lobby.builder();
                let (name, text) = (deck.export_file_name(format), deck.export(format).text);
                let said = save(&name, format.media_type(), &text);
                state.lobby.builder_mut().export_said(said);
            }
        }
    }
}

/// Saves an export where this platform keeps a player's files.
#[cfg(not(target_arch = "wasm32"))]
fn save(name: &str, _media_type: &str, text: &str) -> Said {
    use baylee_client_core::userdirs::{Kind, Os, real_env, user_dir};
    let Some(dir) = user_dir(Kind::Downloads, Os::current(), &real_env) else {
        return Said::SaveFailed("no downloads folder".into());
    };
    match baylee_client_core::deckbuilder::transfer::save_to(&dir, name, text) {
        Ok(path) => Said::Saved(path.display().to_string()),
        Err(error) => Said::SaveFailed(error.to_string()),
    }
}

/// Hands the export to the browser as a download: an anchor with a
/// `download` name and a `data:` URL, clicked once and dropped.
#[cfg(target_arch = "wasm32")]
fn save(name: &str, media_type: &str, text: &str) -> Said {
    use wasm_bindgen::JsCast;
    let url = baylee_client_core::deckbuilder::transfer::data_url(media_type, text);
    let anchor = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.create_element("a").ok())
        .and_then(|element| element.dyn_into::<web_sys::HtmlAnchorElement>().ok());
    match anchor {
        Some(anchor) => {
            anchor.set_href(&url);
            anchor.set_download(name);
            anchor.click();
            Said::Downloaded(name.to_string())
        }
        None => Said::SaveFailed("no document".into()),
    }
}
