//! One row per chat line (the chat window design §5), on the log's anatomy:
//! `[time][swatch][sentence]`, so a chat line and a log line read as one
//! column. Every chat line carries its name; the colour says the channel
//! and the prefix (All, Private) says it in words too.

use super::zoom::{z, zpx};
use baylee_client_core::i18n::Lang;
use baylee_client_core::social::{
    self, ChatLine, ChatPhrase, ConvId, LineKey, LineKind, LineState, Social,
};
use baylee_protocol::social::Conv;
use bevy::prelude::*;

use crate::hud::logpanel::{
    CLOCK_AIR, CLOCK_W, SWATCH_GAP, SWATCH_W, TEXT_OWN_HEIGHT, clock_bundle, clock_drop_at,
};
use crate::hud::{UiFonts, palette, tf, tf_bold, tf_italic};

/// Sky cyan: the public Lobby chat for everyone online.
pub(crate) const GLOBAL_INK: Color = Color::srgb(0.52, 0.82, 0.96);
/// Sage: team chat (§7).
pub(crate) const TEAM_INK: Color = Color::srgb(0.612, 0.796, 0.561);
/// Rose: whispers, and the pill of a whisper or a mention.
pub(crate) const WHISPER_INK: Color = Color::srgb(0.890, 0.631, 0.812);
/// Periwinkle: groups.
pub(crate) const GROUP_INK: Color = Color::srgb(0.663, 0.722, 0.949);

/// The inks one frame draws in: the table's warm dialog, or the lobby's
/// kit panel.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Inks {
    /// The panel's ground.
    pub ground: Color,
    /// A head's or a lit row's ground.
    pub lit: Color,
    /// Rules and borders.
    pub line: Color,
    /// The base ink (table chat, names).
    pub ink: Color,
    /// The quiet ink (times, notices).
    pub soft: Color,
    /// A card link.
    pub link: Color,
    /// The active tab's underline, the unread pill.
    pub accent: Color,
    /// A line's size.
    pub pt: f32,
}

/// The table's inks: the log's warm panel (§7).
pub(crate) const TABLE_INKS: Inks = Inks {
    ground: palette::DIALOG,
    lit: palette::DIALOG_LIT,
    line: palette::DIALOG_LINE,
    ink: palette::DIALOG_INK,
    soft: palette::DIALOG_SOFT,
    link: palette::CANDLE,
    accent: palette::CANDLE,
    pt: crate::hud::logpanel::LOG_LINE_PT,
};

/// The lobby's chat surfaces: the kit's panel navy, opaque (the kit's own
/// token is a hair translucent, and over the vista a list must not show
/// the picture through it).
pub(crate) const LOBBY_GROUND: Color = Color::srgb(0.035, 0.075, 0.105);

/// The lobby's inks: the kit's panel.
pub(crate) const LOBBY_INKS: Inks = Inks {
    ground: LOBBY_GROUND,
    lit: Color::srgba(0.07, 0.13, 0.17, 0.98),
    line: crate::shellkit::tokens::BORDER,
    ink: crate::shellkit::tokens::INK,
    soft: crate::shellkit::tokens::MUTED,
    link: palette::CANDLE,
    accent: crate::shellkit::tokens::ACCENT,
    pt: crate::hud::logpanel::LOG_LINE_PT,
};

/// The ink a conversation's lines are drawn in (the channel's colour).
pub(crate) fn channel_ink(conv: &ConvId, inks: &Inks) -> Color {
    match conv {
        Conv::Global => GLOBAL_INK,
        Conv::Table { .. } => inks.ink,
        Conv::Team { .. } => TEAM_INK,
        Conv::Dm { .. } => WHISPER_INK,
        Conv::Group { .. } => GROUP_INK,
    }
}

/// A card link in a chat line: hovering it previews the card where the
/// table can (§12.3).
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChatCardLink {
    /// The registry index.
    pub card: u32,
    /// The face.
    pub face: u8,
    /// The printing named by the sender, when there was one.
    pub print: Option<social::BodyPrint>,
}

/// A name in a chat line: a press whispers them (§5).
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChatName {
    /// Their handle.
    pub handle: String,
    /// Their tag.
    pub tag: String,
}

/// A chat line's row: a right-press opens its menu.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChatRow {
    /// The line.
    pub key: LineKey,
}

/// How the row is drawn on the tab it is drawn on.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RowStyle {
    /// Prefix the channel (`[Table]`, `[Friday crew]`): the All tab.
    pub prefix: bool,
    /// The time column.
    pub timestamps: bool,
}

/// A line's time on the device's clock, `HH:MM`.
pub(crate) fn clock(at_ms: u64) -> String {
    let offset = i64::try_from(at_ms / 1000)
        .ok()
        .and_then(|secs| time::OffsetDateTime::from_unix_timestamp(secs).ok())
        .and_then(|utc| time::UtcOffset::local_offset_at(utc).ok())
        .map_or(0, time::UtcOffset::whole_seconds);
    let local = (i64::try_from(at_ms / 1000).unwrap_or(0) + i64::from(offset)).rem_euclid(86_400);
    format!("{:02}:{:02}", local / 3600, (local % 3600) / 60)
}

/// The prefix a line carries on the All tab, or Private's arrow form.
pub(crate) fn prefix(
    machine: &Social,
    conv: &ConvId,
    line: &ChatLine,
    lang: Lang,
) -> Option<String> {
    match conv {
        Conv::Global => Some(format!("[{}]", ChatPhrase::TabGlobal.text(lang))),
        Conv::Table { .. } => Some(format!("[{}]", ChatPhrase::TabTable.text(lang))),
        Conv::Team { .. } => Some(format!("[{}]", ChatPhrase::TabTeam.text(lang))),
        Conv::Group { id } => machine
            .contacts()
            .group(id)
            .map(|g| format!("[{}]", g.name)),
        Conv::Dm { .. } => {
            let partner = conv.partner(&machine.me().tag)?;
            let name = short_name(&machine.shown_name(&machine.person(partner)));
            let mine = matches!(&line.kind, LineKind::Message { mine: true, .. });
            let phrase = if mine {
                ChatPhrase::WhisperTo
            } else {
                ChatPhrase::WhisperFrom
            };
            // WoW's compact whisper labels (owner, 13.10.): "To Bob:",
            // "From Bob:"; the handle's tag is on the name's hover.
            Some(phrase.fill(lang, &[&name]))
        }
    }
}

/// A handle without its tag (`Bob#0012` → `Bob`); a nickname as it is.
pub(crate) fn short_name(name: &str) -> String {
    match name.rsplit_once('#') {
        Some((base, tag)) if !base.is_empty() && baylee_protocol::social::valid_tag(tag) => {
            base.to_string()
        }
        _ => name.to_string(),
    }
}

/// What a notice line says.
fn notice_text(phrase: ChatPhrase, args: &[String], lang: Lang) -> String {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    phrase.fill(lang, &args)
}

/// A line as plain text: what Copy puts on the clipboard and `/state.chat`
/// reads (§6): `HH:MM [Table] Bob: text`.
pub(crate) fn plain_text(machine: &Social, conv: &ConvId, line: &ChatLine) -> String {
    let lang = Lang::En;
    let time = clock(line.at);
    match &line.kind {
        LineKind::Message { from, body, .. } => {
            let name = machine.shown_name(from);
            let prefix = prefix(machine, conv, line, lang).unwrap_or_default();
            if matches!(conv, Conv::Dm { .. }) {
                format!("{time} {prefix} {}", body.text)
            } else if body.emote {
                format!("{time} {prefix} * {name} {}", body.text)
            } else {
                format!("{time} {prefix} {name}: {}", body.text)
            }
        }
        LineKind::Notice { phrase, args } | LineKind::Server { phrase, args } => {
            format!("{time} {}", notice_text(*phrase, args, lang))
        }
        LineKind::Divider => ChatPhrase::NewTable.text(lang).to_string(),
    }
}

/// Spawns a chat line's row. `swatch` is the sender's seat colour at this
/// table, when they hold a chair here.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one row builder for every kind of line (§5)
pub(crate) fn spawn_row(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    machine: &Social,
    conv: &ConvId,
    line: &ChatLine,
    inks: &Inks,
    style: RowStyle,
    swatch: Option<Color>,
) -> Entity {
    let ink = channel_ink(conv, inks);
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: zpx(SWATCH_GAP),
                flex_shrink: 0.0,
                padding: UiRect::horizontal(zpx(2.0)),
                border_radius: BorderRadius::all(zpx(3.0)),
                ..default()
            },
            ChatRow {
                key: line.key.clone(),
            },
            Pickable {
                should_block_lower: false,
                is_hoverable: true,
            },
        ))
        .id();
    if line.mentions_me {
        commands
            .entity(row)
            .insert(BackgroundColor(palette::ACTIVE.with_alpha(0.10)));
    }
    if style.timestamps && !matches!(line.kind, LineKind::Divider) {
        // The log's own time column (`logpanel::clock_bundle`), at the
        // window's zoom: the two lists read alike (feedback of 24.10.2026).
        let time = commands
            .spawn(clock_bundle(fonts, z(inks.pt), inks.soft, clock(line.at)))
            .insert(Node {
                width: zpx(CLOCK_W),
                margin: UiRect {
                    top: zpx(clock_drop_at(inks.pt)),
                    right: zpx(CLOCK_AIR),
                    ..default()
                },
                flex_shrink: 0.0,
                ..default()
            })
            .id();
        commands.entity(row).add_child(time);
    }
    let swatch = commands
        .spawn((
            Node {
                width: zpx(SWATCH_W),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(zpx(SWATCH_W / 2.0)),
                ..default()
            },
            BackgroundColor(swatch.unwrap_or(Color::NONE)),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(swatch);
    let sentence = commands
        .spawn((
            Text::default(),
            tf(fonts, z(inks.pt)),
            TextColor(ink),
            Node {
                flex_grow: 1.0,
                min_width: zpx(0.0),
                overflow: TEXT_OWN_HEIGHT,
                ..default()
            },
            Pickable {
                should_block_lower: false,
                is_hoverable: true,
            },
        ))
        .id();
    commands.entity(row).add_child(sentence);
    let span = |commands: &mut Commands, text: String, font: TextFont, colour: Color| {
        let id = commands
            .spawn((TextSpan::new(text), font, TextColor(colour)))
            .id();
        commands.entity(sentence).add_child(id);
        id
    };
    match &line.kind {
        LineKind::Divider => {
            span(
                commands,
                format!("{} ─────", ChatPhrase::NewTable.text(lang)),
                tf_italic(fonts, z(inks.pt)),
                inks.soft,
            );
        }
        LineKind::Notice { phrase, args } | LineKind::Server { phrase, args } => {
            let colour = if is_error(*phrase) {
                palette::DANGER
            } else {
                inks.soft
            };
            span(
                commands,
                notice_text(*phrase, args, lang),
                tf_italic(fonts, z(inks.pt)),
                colour,
            );
        }
        LineKind::Message { from, body, mine } => {
            if line.hidden {
                span(
                    commands,
                    ChatPhrase::HiddenLine.text(lang).to_string(),
                    tf_italic(fonts, z(inks.pt)),
                    inks.soft,
                );
                return row;
            }
            let faded = if line.state == LineState::Pending {
                0.6
            } else {
                1.0
            };
            let ink = ink.with_alpha(faded);
            let private = matches!(conv, Conv::Dm { .. });
            if (style.prefix || private)
                && let Some(prefix) = prefix(machine, conv, line, lang)
            {
                // A whisper's label is its name (bold, a press answers it);
                // a channel's prefix is quieter.
                let font = if private {
                    tf_bold(fonts, z(inks.pt))
                } else {
                    tf(fonts, z(inks.pt))
                };
                let id = span(commands, format!("{prefix} "), font, ink);
                if private && let Some(partner) = conv.partner(&machine.me().tag) {
                    let person = machine.person(partner);
                    commands.entity(id).insert(ChatName {
                        handle: person.handle,
                        tag: person.tag,
                    });
                }
            }
            let name = machine.shown_name(from);
            // A whisper's label names the partner already: `From Bob: text`.
            if !private {
                let label = if body.emote {
                    format!("* {name} ")
                } else {
                    format!("{name}: ")
                };
                let id = span(commands, label, tf_bold(fonts, z(inks.pt)), ink);
                if !*mine {
                    commands.entity(id).insert(ChatName {
                        handle: from.handle.clone(),
                        tag: from.tag.clone(),
                    });
                }
            }
            for (text, piece) in pieces(body) {
                match piece {
                    Piece::Words => {
                        let font = if body.emote {
                            tf_italic(fonts, z(inks.pt))
                        } else {
                            tf(fonts, z(inks.pt))
                        };
                        span(commands, text, font, ink);
                    }
                    Piece::Mention => {
                        span(commands, text, tf_bold(fonts, z(inks.pt)), ink);
                    }
                    Piece::Link(link) => {
                        let font = TextFont {
                            font: bevy::text::FontSource::Handle(fonts.medium.clone()),
                            ..tf(fonts, z(inks.pt))
                        };
                        let id = span(commands, text, font, inks.link.with_alpha(faded));
                        commands.entity(id).insert(link);
                    }
                }
            }
            match line.state {
                LineState::Pending => {
                    span(
                        commands,
                        " \u{25cc}".into(),
                        tf(fonts, z(inks.pt)),
                        inks.soft,
                    );
                }
                LineState::Failed => {
                    let why = line
                        .failure
                        .unwrap_or(ChatPhrase::SendFailed)
                        .text(lang)
                        .to_string();
                    span(
                        commands,
                        format!(" ! {why}"),
                        tf_bold(fonts, z(inks.pt)),
                        palette::DANGER,
                    );
                }
                LineState::Final => {}
            }
        }
    }
    row
}

/// A notice that is a refusal or an error, drawn in `DANGER` (§5).
fn is_error(phrase: ChatPhrase) -> bool {
    matches!(
        phrase,
        ChatPhrase::UnknownCommand
            | ChatPhrase::MissingName
            | ChatPhrase::MissingText
            | ChatPhrase::NoTable
            | ChatPhrase::NoTeam
            | ChatPhrase::NoGroup
            | ChatPhrase::NoGroupNumber
            | ChatPhrase::NoReply
            | ChatPhrase::NoSuchPlayer
            | ChatPhrase::AmbiguousName
            | ChatPhrase::GuestsCannot
            | ChatPhrase::SendNotAllowed
            | ChatPhrase::SendTooLarge
            | ChatPhrase::SendRefused
            | ChatPhrase::SendStale
            | ChatPhrase::ChatOff
            | ChatPhrase::RosterMismatch
            | ChatPhrase::MessageDropped
            | ChatPhrase::UnknownDevice
            | ChatPhrase::MisaddressedDropped
    )
}

/// A piece of a body's text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Piece {
    /// Its words.
    Words,
    /// A mention, bold.
    Mention,
    /// A card link, with the printing the line named.
    Link(ChatCardLink),
}

/// A body cut at its links and mentions (spans in characters).
pub(crate) fn pieces(body: &social::Body) -> Vec<(String, Piece)> {
    let chars: Vec<char> = body.text.chars().collect();
    let mut marks: Vec<([usize; 2], Piece)> = body
        .links
        .iter()
        .map(|l| {
            (
                l.at,
                Piece::Link(ChatCardLink {
                    card: l.card,
                    face: l.face,
                    print: l.print.clone(),
                }),
            )
        })
        .chain(body.mentions.iter().map(|m| (m.at, Piece::Mention)))
        .collect();
    marks.sort_by_key(|(at, _)| at[0]);
    let mut out = Vec::new();
    let mut at = 0;
    for (span, piece) in marks {
        if span[0] < at || span[1] > chars.len() || span[0] >= span[1] {
            continue;
        }
        if span[0] > at {
            out.push((chars[at..span[0]].iter().collect(), Piece::Words));
        }
        out.push((chars[span[0]..span[1]].iter().collect(), piece));
        at = span[1];
    }
    if at < chars.len() {
        out.push((chars[at..].iter().collect(), Piece::Words));
    }
    out
}
