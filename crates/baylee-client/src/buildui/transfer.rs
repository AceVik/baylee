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
use crate::lobby::List;
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::surfaces::{self, SheetHeader, SheetWidth};
use crate::shellkit::{InputClass, Platform, ShellMetrics, TextSize, Viewport, px_fixed, tokens};
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

fn kit_of(fonts: &UiFonts, metrics: crate::lobby::Metrics, lang: Lang) -> Kit<'_> {
    let width = match metrics.frame {
        crate::lobby::Frame::Phone => 390.0,
        crate::lobby::Frame::Compact => 640.0,
        crate::lobby::Frame::Narrow => 900.0,
        crate::lobby::Frame::Wide => 1400.0,
        crate::lobby::Frame::Vast => 1920.0,
    };
    let input = if metrics.tap >= 44.0 {
        InputClass::Touch
    } else {
        InputClass::Pointer
    };
    Kit {
        fonts,
        m: ShellMetrics::of(
            Viewport {
                width,
                height: 800.0,
                platform: Platform::current(),
                input,
            },
            TextSize::default(),
        ),
        german: lang == Lang::De,
    }
}

/// The import or export dialog, over the whole builder.
#[allow(clippy::too_many_lines)] // one dialog, two faces, unified onto shellkit::surfaces
pub(crate) fn transfer_dialog(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: crate::lobby::Metrics,
    lang: Lang,
    deck: &DeckBuilder,
    transfer: &Transfer,
    scrolled: &Scrolled,
) -> Entity {
    let kit = kit_of(fonts, metrics, lang);
    let (title, icon) = match transfer {
        Transfer::Import(_) => (Phrase::ImportTitle.text(lang), '\u{f56f}'),
        Transfer::Export(_) => (Phrase::ExportTitle.text(lang), '\u{f56e}'),
    };
    let mut body = Vec::new();
    let mut footer = Vec::new();

    match transfer {
        Transfer::Import(importing) => {
            import_face(commands, kit, lang, deck, importing, &mut body, &mut footer);
        }
        Transfer::Export(exporting) => {
            let written = deck.export(exporting.format);
            let chips = commands
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: kit.m.px(6.0),
                        row_gap: kit.m.px(6.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            for format in FormatId::ALL {
                let chip = controls::chip(
                    commands,
                    kit,
                    format_label(format, lang),
                    format == exporting.format,
                    None,
                    false,
                    Press::Build(BuildPress::ExportFormat(format)),
                );
                commands.entity(chips).add_child(chip);
            }
            body.push(chips);
            body.push(text_box(commands, kit, &written.text, None));
            said(commands, kit, &mut body, &deck.export_lines(&written, lang));
            body.push(surfaces::prose(
                commands,
                kit,
                Phrase::ExportKeys.text(lang),
                true,
            ));

            footer.push(controls::button(
                commands,
                kit,
                Phrase::TransferClose.text(lang),
                Weight::Secondary,
                Live::Yes,
                Some("Esc"),
                Press::Build(BuildPress::TransferClose),
            ));
            if can_save_files() {
                footer.push(controls::button_with_icon(
                    commands,
                    kit,
                    '\u{f019}',
                    Phrase::SaveToFile.text(lang),
                    Weight::Secondary,
                    Live::Yes,
                    None,
                    Press::Build(BuildPress::ExportSave),
                ));
            }
            footer.push(controls::button_with_icon(
                commands,
                kit,
                '\u{f0c5}',
                Phrase::CopyToClipboard.text(lang),
                Weight::Primary,
                Live::Yes,
                None,
                Press::Build(BuildPress::ExportCopy),
            ));
        }
    }

    let surface = surfaces::sheet_box_headed_listed(
        commands,
        kit,
        SheetWidth::Medium,
        SheetHeader::new(title)
            .icon(icon)
            .close(Press::Build(BuildPress::TransferClose)),
        &body,
        &footer,
        (
            crate::lobby::Scrollable(List::Transfer),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::Transfer))),
        ),
    );
    commands
        .entity(surface)
        .insert(Press::Build(BuildPress::TransferNothing));
    let shade = surfaces::sheet(commands, surface);
    commands
        .entity(shade)
        .insert(Press::Build(BuildPress::TransferClose));
    shade
}

fn import_toolbar(commands: &mut Commands, kit: Kit, lang: Lang, has_pasted: bool) -> Entity {
    let tools = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(8.0),
                row_gap: kit.m.px(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let paste = controls::button_with_icon(
        commands,
        kit,
        '\u{f0ea}',
        Phrase::PasteFromClipboard.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        Press::Build(BuildPress::ImportPaste),
    );
    let clear = if has_pasted {
        controls::button(
            commands,
            kit,
            Phrase::ImportClear.text(lang),
            Weight::Ghost,
            Live::Yes,
            None,
            Press::Build(BuildPress::ImportClear),
        )
    } else {
        controls::button(
            commands,
            kit,
            Phrase::ImportClear.text(lang),
            Weight::Ghost,
            Live::No(Phrase::ImportBoxEmpty.text(lang)),
            None,
            (),
        )
    };
    commands.entity(tools).add_children(&[paste, clear]);
    tools
}

/// The import dialog's body and sticky footer actions.
fn import_face(
    commands: &mut Commands,
    kit: Kit,
    lang: Lang,
    deck: &DeckBuilder,
    importing: &Importing,
    body: &mut Vec<Entity>,
    footer: &mut Vec<Entity>,
) {
    let done = matches!(importing.stage(), Stage::Done(_));
    if !done {
        body.push(import_toolbar(
            commands,
            kit,
            lang,
            !importing.pasted().is_empty(),
        ));
        body.push(text_box(
            commands,
            kit,
            importing.pasted(),
            Some(Phrase::ImportBoxEmpty.text(lang)),
        ));
    }
    said(commands, kit, body, &deck.import_lines(lang));
    if !done {
        body.push(surfaces::prose(
            commands,
            kit,
            Phrase::ImportKeys.text(lang),
            true,
        ));
    }

    if done {
        footer.push(controls::button(
            commands,
            kit,
            Phrase::TransferClose.text(lang),
            Weight::Primary,
            Live::Yes,
            Some("Esc"),
            Press::Build(BuildPress::TransferClose),
        ));
    } else {
        footer.push(controls::button(
            commands,
            kit,
            Phrase::TransferClose.text(lang),
            Weight::Secondary,
            Live::Yes,
            Some("Esc"),
            Press::Build(BuildPress::TransferClose),
        ));
        let take = if importing.ready() {
            controls::button_with_icon(
                commands,
                kit,
                mark::CHECK,
                Phrase::ImportTake.text(lang),
                Weight::Primary,
                Live::Yes,
                None,
                Press::Build(BuildPress::ImportTake),
            )
        } else {
            controls::button_with_icon(
                commands,
                kit,
                mark::CHECK,
                Phrase::ImportTake.text(lang),
                Weight::Primary,
                Live::No(Phrase::ImportBoxEmpty.text(lang)),
                None,
                (),
            )
        };
        footer.push(take);
    }
}

/// A framed block of text: the paste, or the export. At most
/// [`PREVIEW_LINES`] of it; the count of the rest after that.
fn text_box(commands: &mut Commands, kit: Kit, text: &str, empty: Option<&str>) -> Entity {
    let frame = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(kit.m.px(12.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                min_height: kit.m.px(72.0),
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.55)),
            BorderColor::all(tokens::BORDER),
            surfaces::elevation_subcard(),
            Pickable::IGNORE,
        ))
        .id();
    let (shown, ink) = if text.trim().is_empty() {
        (empty.unwrap_or_default().to_string(), tokens::MUTED)
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
        (shown, tokens::INK)
    };
    let body = commands
        .spawn((
            Text::new(shown),
            tf(kit.fonts, kit.m.small),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(frame).add_child(body);
    frame
}

/// The dialog's lines, coloured by what they are.
fn said(commands: &mut Commands, kit: Kit, body: &mut Vec<Entity>, lines: &[Line]) {
    for line in lines {
        let ink = match line.tone {
            Tone::Info => tokens::MUTED,
            Tone::Good => tokens::INK,
            Tone::Warn => tokens::GOLD,
        };
        let text = commands
            .spawn((
                Text::new(line.text.clone()),
                tf(kit.fonts, kit.m.small),
                TextColor(ink),
                Pickable::IGNORE,
            ))
            .id();
        body.push(text);
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
