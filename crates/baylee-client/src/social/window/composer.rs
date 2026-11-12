//! The composer: who a line goes to, the field and its placeholder.

#[allow(clippy::wildcard_imports)] // the window's own vocabulary
use super::*;

/// What the chip says for a target.
pub(crate) fn target_label(machine: &Social, target: &Target, lang: Lang) -> (String, Color, Inks) {
    let inks = TABLE_INKS;
    match target {
        Target::Global => (
            ChatPhrase::TabGlobal.text(lang).to_string(),
            rows::GLOBAL_INK,
            inks,
        ),
        Target::Table => (ChatPhrase::TabTable.text(lang).to_string(), inks.ink, inks),
        Target::Team => (
            ChatPhrase::TabTeam.text(lang).to_string(),
            rows::TEAM_INK,
            inks,
        ),
        Target::Whisper(handle) => (
            ChatPhrase::ToTarget.fill(lang, &[handle]),
            rows::WHISPER_INK,
            inks,
        ),
        Target::Group(GroupRef::Number(n)) => {
            let name = machine
                .contacts()
                .groups
                .get(usize::from(*n).saturating_sub(1))
                .map_or_else(|| format!("/{n}"), |g| g.name.clone());
            (name, rows::GROUP_INK, inks)
        }
        Target::Group(GroupRef::Current) => (
            ChatPhrase::TabGroup.text(lang).to_string(),
            rows::GROUP_INK,
            inks,
        ),
        Target::Sticky => ("…".to_string(), inks.soft, inks),
    }
}

/// The composer: the chip, the box, the hint and send; the suggestions
/// stand above it.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one part, top to bottom
pub(super) fn composer_parts(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    machine: &Social,
    ui: &ChatUi,
    suggest: &[String],
    highlight: usize,
    frame: Frame,
) -> Vec<Entity> {
    let inks = frame.inks();
    let c = machine.composer();
    let focused = ui.typing && ui.field == Field::Composer;
    let (chip_text, chip_ink, _) = target_label(machine, c.target(), lang);
    let chip = commands
        .spawn((
            UiButton,
            ChatPress::Chip,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                align_self: AlignSelf::FlexEnd,
                margin: UiRect::bottom(zpx(1.0)),
                column_gap: zpx(4.0),
                padding: UiRect::axes(zpx(7.0), zpx(2.0)),
                height: zpx(22.0),
                border_radius: BorderRadius::all(zpx(4.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(chip_ink.with_alpha(0.14)),
            Feel::new(chip_ink.with_alpha(0.14)),
            children![
                (
                    Text::new(chip_text),
                    tf_bold(fonts, z(11.0)),
                    TextColor(chip_ink),
                    Pickable::IGNORE,
                ),
                (
                    Text::new(glyph::CHEVRON_DOWN.to_string()),
                    icon_tf(fonts, z(7.0)),
                    TextColor(chip_ink.with_alpha(0.85)),
                    Pickable::IGNORE,
                ),
            ],
        ))
        .id();
    let sep = commands
        .spawn((
            Node {
                width: zpx(1.0),
                height: zpx(14.0),
                align_self: AlignSelf::Center,
                margin: UiRect::ZERO,
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(inks.line.with_alpha(0.45)),
            Pickable::IGNORE,
        ))
        .id();
    let row = commands
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: zpx(6.0),
            padding: UiRect::axes(zpx(8.0), zpx(5.0)),
            min_height: zpx(38.0),
            ..default()
        },))
        .id();
    let frame = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                min_width: zpx(0.0),
                min_height: zpx(28.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: zpx(4.0),
                padding: UiRect::axes(zpx(4.0), zpx(2.0)),
                border: UiRect::all(zpx(1.0)),
                border_radius: BorderRadius::all(zpx(6.0)),
                ..default()
            },
            // Opaque: what is typed is read on the field alone. At half
            // strength the lobby's version chip, which the docked window
            // stands over, was read through it under the caret.
            BackgroundColor(Color::srgb(0.015, 0.036, 0.054)),
            BorderColor::all(if focused {
                inks.accent.with_alpha(0.85)
            } else {
                inks.line.with_alpha(0.55)
            }),
            Pickable::IGNORE,
        ))
        .id();
    // A guest reads the lobby's chat and does not write in it: the box gives
    // way to the reason, and the chip still reaches a table's chat.
    if machine.me().guest && machine.target_conv(c.target()) == Some(Conv::Global) {
        let why = commands
            .spawn((
                Text::new(ChatPhrase::GuestNoLobbyChat.text(lang)),
                tf_italic(fonts, z(inks.pt)),
                TextColor(inks.soft),
                Node {
                    flex_grow: 1.0,
                    min_width: zpx(0.0),
                    padding: UiRect::new(zpx(0.0), zpx(4.0), zpx(3.0), zpx(3.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(frame).add_children(&[chip, sep, why]);
        commands.entity(row).add_child(frame);
        return vec![row];
    }
    let text = c.text();
    let field = commands
        .spawn((
            UiButton,
            ChatPress::Field,
            ComposerTextBox,
            bevy::ui::RelativeCursorPosition::default(),
            Text::default(),
            tf(fonts, z(inks.pt)),
            TextColor(inks.ink),
            Node {
                flex_grow: 1.0,
                min_width: zpx(0.0),
                padding: UiRect::new(zpx(0.0), zpx(4.0), zpx(3.0), zpx(3.0)),
                ..default()
            },
        ))
        .id();
    if text.is_empty() && !focused {
        let placeholder = placeholder(machine, c.target(), lang);
        let span = commands
            .spawn((
                TextSpan::new(placeholder),
                tf_italic(fonts, z(inks.pt)),
                TextColor(inks.soft),
            ))
            .id();
        commands.entity(field).add_child(span);
    } else {
        let buffer = c.buffer();
        let caret = buffer.cursor().min(text.len());
        let selection = buffer
            .selection()
            .filter(|r| focused && r.start < r.end && r.end <= text.len());
        // The `[Name]` links the line would carry, in bytes: drawn as links
        // and previewed as the history's are (§12.3).
        let byte = |chars: usize| {
            text.char_indices()
                .nth(chars)
                .map_or(text.len(), |(b, _)| b)
        };
        let links: Vec<(std::ops::Range<usize>, rows::ChatCardLink)> = c
            .body_of(text, false)
            .links
            .iter()
            .map(|l| {
                (
                    byte(l.at[0])..byte(l.at[1]),
                    rows::ChatCardLink {
                        card: l.card,
                        face: l.face,
                        print: l.print.clone(),
                    },
                )
            })
            .collect();
        let mut cuts: Vec<usize> = vec![0, text.len()];
        if focused && selection.is_none() {
            cuts.push(caret);
        }
        if let Some(range) = &selection {
            cuts.extend([range.start, range.end]);
        }
        for (range, ..) in &links {
            cuts.extend([range.start, range.end]);
        }
        cuts.sort_unstable();
        cuts.dedup();
        let link_font = TextFont {
            font: bevy::text::FontSource::Handle(fonts.medium.clone()),
            ..tf(fonts, z(inks.pt))
        };
        for pair in cuts.windows(2) {
            let (from, to) = (pair[0], pair[1]);
            if focused && selection.is_none() && from == caret {
                let bar = commands
                    .spawn((
                        TextSpan::new("|"),
                        tf(fonts, z(inks.pt)),
                        TextColor(inks.accent),
                    ))
                    .id();
                commands.entity(field).add_child(bar);
            }
            if from >= to {
                continue;
            }
            let link = links.iter().find(|(r, ..)| r.start <= from && to <= r.end);
            let chosen = selection
                .as_ref()
                .is_some_and(|r| r.start <= from && to <= r.end);
            let (font, colour) = match (chosen, link) {
                (true, _) => (tf(fonts, z(inks.pt)), Color::WHITE),
                (false, Some(_)) => (link_font.clone(), inks.link),
                (false, None) => (tf(fonts, z(inks.pt)), inks.ink),
            };
            let mut span_cmd =
                commands.spawn((TextSpan::new(&text[from..to]), font, TextColor(colour)));
            if chosen {
                span_cmd.insert(BackgroundColor(inks.accent.with_alpha(0.32)));
            }
            if let Some((_, link)) = link {
                span_cmd.insert(link.clone());
            }
            let span = span_cmd.id();
            commands.entity(field).add_child(span);
        }
        if focused && selection.is_none() && caret == text.len() {
            let bar = commands
                .spawn((
                    TextSpan::new("|"),
                    tf(fonts, z(inks.pt)),
                    TextColor(inks.accent),
                ))
                .id();
            commands.entity(field).add_child(bar);
        }
    }
    let count = c.chars();
    let hint_text = if count >= 900 {
        Some(format!("{count} / 1000"))
    } else {
        ui.hint.map(|(phrase, _)| phrase.text(lang).to_string())
    };
    let hint = hint_text.map(|text| {
        commands
            .spawn((
                Text::new(text),
                tf(fonts, z(10.5)),
                TextColor(if ui.hint.is_some() {
                    palette::DANGER
                } else {
                    inks.soft
                }),
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id()
    });
    let can_send = !text.trim().is_empty();
    let send_bg = if can_send {
        inks.accent.with_alpha(0.16)
    } else {
        Color::NONE
    };
    let send = commands
        .spawn((
            UiButton,
            ChatPress::Send,
            Node {
                width: zpx(24.0),
                height: zpx(22.0),
                align_self: AlignSelf::FlexEnd,
                margin: UiRect::bottom(zpx(1.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(zpx(4.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(send_bg),
            Feel::new(send_bg),
            children![(
                Text::new(glyph::SEND.to_string()),
                icon_tf(fonts, z(10.0)),
                TextColor(if can_send {
                    inks.accent
                } else {
                    inks.soft.with_alpha(0.55)
                }),
                Pickable::IGNORE,
            )],
        ))
        .id();
    let light = commands
        .spawn(crate::social::glass::layer(
            crate::social::glass::Glass::Field,
            zpx(6.0),
        ))
        .id();
    let mut inner = vec![chip, sep, field];
    inner.extend(hint);
    inner.push(send);
    inner.push(light);
    commands.entity(frame).add_children(&inner);
    commands.entity(row).add_child(frame);
    let mut out = vec![row];
    if !suggest.is_empty() {
        let pop = commands
            .spawn((
                ChatSurface,
                Node {
                    position_type: PositionType::Absolute,
                    bottom: zpx(40.0),
                    left: zpx(60.0),
                    width: zpx(320.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(zpx(4.0)),
                    border: UiRect::all(zpx(1.0)),
                    border_radius: BorderRadius::all(zpx(6.0)),
                    ..default()
                },
                BackgroundColor(inks.ground.with_alpha(0.94)),
                BorderColor::all(inks.accent.with_alpha(0.6)),
                stacked(30),
            ))
            .id();
        for (i, label) in suggest.iter().take(6).enumerate() {
            let lit = i == highlight;
            let row = commands
                .spawn((
                    UiButton,
                    ChatPress::Suggestion(i),
                    Node {
                        padding: UiRect::axes(zpx(8.0), zpx(3.0)),
                        border_radius: BorderRadius::all(zpx(3.0)),
                        ..default()
                    },
                    BackgroundColor(if lit { inks.lit } else { Color::NONE }),
                    children![(
                        Text::new(format!("{}{label}", if lit { "\u{25b8} " } else { "  " })),
                        tf(fonts, z(12.0)),
                        TextColor(if lit { inks.ink } else { inks.soft }),
                        Pickable::IGNORE,
                    )],
                ))
                .id();
            commands.entity(pop).add_child(row);
        }
        out.push(pop);
    }
    out
}

/// The box's placeholder (§8).
fn placeholder(machine: &Social, target: &Target, lang: Lang) -> String {
    let write = match target {
        Target::Whisper(handle) => ChatPhrase::WriteWhisper.fill(lang, &[handle]),
        Target::Sticky => ChatPhrase::TypeWhisper.text(lang).to_string(),
        other => {
            let (name, _, _) = target_label(machine, other, lang);
            ChatPhrase::WriteTo.fill(lang, &[&name])
        }
    };
    format!("{write} · {}", ChatPhrase::ComposeHint.text(lang))
}

/// Marker on the composer's editable text node for mouse cursor placement and drag selection.
#[derive(Component)]
pub(super) struct ComposerTextBox;

/// How far across the box the pointer is, 0 at its left edge and 1 at its
/// right, clamped. Bevy's `normalized` has the node's centre at 0 and its
/// edges at ±0.5 (`ComputedNode::normalize_point`).
pub(super) fn across(rel: &bevy::ui::RelativeCursorPosition) -> f32 {
    rel.normalized.map_or(0.0, |p| (p.x + 0.5).clamp(0.0, 1.0))
}

/// Places the caret or extends the selection when clicking and dragging inside the composer box.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(super) fn composer_cursor_drag(
    buttons: Option<Res<ButtonInput<MouseButton>>>,
    mut host: ResMut<SocialHost>,
    mut ui: ResMut<ChatUi>,
    boxes: Query<(&bevy::ui::RelativeCursorPosition, &ComputedNode), With<ComposerTextBox>>,
    mut anchor: Local<Option<usize>>,
) {
    let Some(buttons) = buttons else {
        return;
    };
    if !buttons.pressed(MouseButton::Left) {
        *anchor = None;
        return;
    }
    let Ok((rel, computed)) = boxes.single() else {
        *anchor = None;
        return;
    };
    let Some(machine) = host.machine.as_mut() else {
        return;
    };
    let width = (computed.size().x * computed.inverse_scale_factor()).max(1.0);
    let x = across(rel) * width;
    let char_w = (z(TABLE_INKS.pt) * 0.52).max(1.0);
    let char_idx = (x / char_w).round() as usize;
    let text = machine.composer().text();
    let byte = text
        .char_indices()
        .nth(char_idx)
        .map_or(text.len(), |(b, _)| b);
    if buttons.just_pressed(MouseButton::Left) && rel.cursor_over() {
        ui.typing = true;
        ui.field = Field::Composer;
        *anchor = Some(byte);
        machine.composer_mut().place(byte, None);
    } else if let Some(start) = *anchor
        && (machine.composer().buffer().cursor() != byte
            || machine.composer().buffer().selection().is_some() != (start != byte))
    {
        machine.composer_mut().place(byte, Some(start));
    }
}
