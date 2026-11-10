//! The Create-table sheet (§5).

#[allow(clippy::wildcard_imports)] // the Play screen's own vocabulary
use super::*;

/// Returns `(icon, accent_color, subtitle)` for each table `Template`.
fn template_visuals(template: Template, lang: Lang) -> (char, Color, &'static str) {
    match template {
        Template::Commander => (
            '\u{f521}',
            tokens::GOLD,
            Phrase::CreateTemplateCommander.text(lang),
        ),
        Template::Duel => (
            '\u{f6de}',
            tokens::ACCENT,
            Phrase::CreateTemplateDuel.text(lang),
        ),
        Template::FiveLand => (
            '\u{f0e7}',
            Color::srgb(0.38, 0.84, 0.56),
            Phrase::CreateTemplateFiveLand.text(lang),
        ),
        Template::Mixed => (
            '\u{f074}',
            Color::srgb(0.74, 0.56, 0.96),
            Phrase::CreateTemplateMixed.text(lang),
        ),
    }
}

/// Spawns a numbered, hierarchically structured section card inside the Create-Table modal.
fn spawn_create_section(
    commands: &mut Commands,
    kit: Kit,
    step: &'static str,
    icon: char,
    title: &str,
    subtitle: &str,
    accent: Color,
) -> Entity {
    let card = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(10.0),
                padding: UiRect::all(kit.m.px(14.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.32)),
            BorderColor::all(tokens::BORDER.with_alpha(0.78)),
            Pickable::IGNORE,
        ))
        .id();
    let head = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(10.0),
                padding: UiRect::bottom(kit.m.px(6.0)),
                border: UiRect::bottom(px_fixed(1.0)),
                ..default()
            },
            BorderColor::all(tokens::BORDER.with_alpha(0.45)),
            Pickable::IGNORE,
        ))
        .id();
    let disc = commands
        .spawn((
            Node {
                width: kit.m.px(32.0),
                height: kit.m.px(32.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                border: UiRect::all(px_fixed(1.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(accent.with_alpha(0.16)),
            BorderColor::all(accent.with_alpha(0.45)),
            Pickable::IGNORE,
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new(icon.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.text * 0.94),
            TextColor(accent),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(disc).add_child(glyph);

    let titles = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px(0),
                row_gap: kit.m.px(1.5),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let t_node = commands
        .spawn((
            Text::new(format!("{step}. {title}")),
            crate::hud::tf_bold(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    let s_node = commands
        .spawn((
            Text::new(subtitle),
            crate::hud::tf(kit.fonts, kit.m.small * 0.88),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(titles).add_children(&[t_node, s_node]);
    commands.entity(head).add_children(&[disc, titles]);
    commands.entity(card).add_child(head);
    card
}

/// The Create-table sheet (§5): hierarchically organized into
/// 1. Tisch & Sitzplätze, 2. Spielmodus & Startregeln, and 3. Bedenkzeit & Zugang.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)] // one sheet, top to bottom
pub(super) fn create_sheet(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    metrics: Metrics,
    kit: Kit,
    draft: &TableDraft,
    editing: bool,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let phone = kit.m.frame == ShellFrame::Phone;
    let mut body = Vec::new();
    let label = |commands: &mut Commands, text: &str| parts::caption(commands, kit, text);

    // ---- Live Overview Banner at top of sheet
    let overview = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                flex_wrap: FlexWrap::Wrap,
                column_gap: kit.m.px(8.0),
                row_gap: kit.m.px(6.0),
                padding: UiRect::axes(kit.m.px(12.0), kit.m.px(8.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::ACCENT.with_alpha(0.10)),
            BorderColor::all(tokens::ACCENT.with_alpha(0.35)),
            Pickable::IGNORE,
        ))
        .id();
    let ov_badges = parts::row(commands, kit, true);
    let (tmpl_icon, tmpl_color, _) = template_visuals(draft.template, lang);
    let b_mode = parts::badge_with(
        commands,
        kit,
        tmpl_icon,
        draft.template.phrase().text(lang),
        tmpl_color,
    );
    let b_seats = parts::badge_with(
        commands,
        kit,
        if draft.players <= 2 {
            '\u{f6de}'
        } else {
            '\u{f0c0}'
        },
        &Phrase::CreateSeats.fill(lang, &[&draft.players.to_string()]),
        tokens::ACCENT,
    );
    let b_life = parts::badge_with(commands, kit, '\u{f004}', &draft.summary(lang), tokens::INK);
    let has_pw = !lobby.field(Field::RoomPassword).is_empty();
    let b_access = parts::badge_with(
        commands,
        kit,
        if has_pw { '\u{f023}' } else { '\u{f09c}' },
        if has_pw {
            Phrase::CreatePasswordProtected.text(lang)
        } else {
            Phrase::CreatePublic.text(lang)
        },
        if has_pw {
            tokens::GOLD
        } else {
            Color::srgb(0.36, 0.84, 0.56)
        },
    );
    commands
        .entity(ov_badges)
        .add_children(&[b_mode, b_seats, b_life, b_access]);
    commands.entity(overview).add_child(ov_badges);
    body.push(overview);

    // =========================================================================
    // SECTION 1: Tisch & Sitzplätze (Table Identity & Seats)
    // =========================================================================
    let sec1 = spawn_create_section(
        commands,
        kit,
        "1",
        '\u{f0c0}',
        Phrase::CreateSectionTable.text(lang),
        Phrase::CreateSectionTableAbout.text(lang),
        tokens::ACCENT,
    );

    let name_lbl = label(commands, Phrase::SheetName.text(lang));
    let name_field = text_field(
        commands,
        kit.fonts,
        metrics,
        "",
        &FieldLook {
            buffer: lobby.buffer(Field::RoomName),
            focused: lobby.focus() == Field::RoomName,
            mask: None,
            press: Press::Shared(SharedPress::Focus(Field::RoomName)),
            lead: Some('\u{f015}'),
            hint: None,
            tail: None,
        },
    );
    orders::field(commands, name_field, &orders::CREATE, "name");
    commands.entity(sec1).add_children(&[name_lbl, name_field]);

    let players_head = parts::row(commands, kit, true);
    commands
        .entity(players_head)
        .entry::<Node>()
        .and_modify(|mut n| {
            n.width = percent(100);
            n.justify_content = JustifyContent::SpaceBetween;
        });
    let players_lbl = label(commands, Phrase::SheetPlayers.text(lang));
    let seat_hint_text = match draft.players {
        2 => Phrase::CreateSeatHintTwo,
        3 => Phrase::CreateSeatHintThree,
        4 => Phrase::CreateSeatHintFour,
        _ => Phrase::CreateSeatHintMany,
    }
    .text(lang);
    let seat_hint = parts::badge_with(
        commands,
        kit,
        if draft.players == 4 {
            '\u{f521}'
        } else if draft.players == 2 {
            '\u{f6de}'
        } else {
            '\u{f500}'
        },
        seat_hint_text,
        if draft.players == 4 {
            tokens::GOLD
        } else {
            tokens::ACCENT
        },
    );
    commands
        .entity(players_head)
        .add_children(&[players_lbl, seat_hint]);
    commands.entity(sec1).add_child(players_head);

    let players_ctrl = if phone {
        controls::stepper(
            commands,
            kit,
            &draft.players.to_string(),
            Press::Play(PlayPress::StepPlayers(false)),
            Press::Play(PlayPress::StepPlayers(true)),
        )
    } else {
        let counts: Vec<String> = (2..=8).map(|n| n.to_string()).collect();
        let items: Vec<&str> = counts.iter().map(String::as_str).collect();
        controls::segmented(
            commands,
            kit,
            &items,
            draft.players.saturating_sub(2),
            |i| Press::Play(PlayPress::Players(i + 2)),
        )
    };
    orders::items_of(commands, players_ctrl, &orders::CREATE, "players");
    commands.entity(sec1).add_child(players_ctrl);
    body.push(sec1);

    // =========================================================================
    // SECTION 2: Spielmodus & Startregeln (Format Template & Starting Rules)
    // =========================================================================
    let sec2 = spawn_create_section(
        commands,
        kit,
        "2",
        '\u{f521}',
        Phrase::CreateSectionMode.text(lang),
        Phrase::CreateSectionModeAbout.text(lang),
        tokens::GOLD,
    );
    let tmpl_lbl = label(commands, Phrase::SheetTemplate.text(lang));
    commands.entity(sec2).add_child(tmpl_lbl);

    let cards = commands
        .spawn((
            Node {
                display: if phone { Display::Flex } else { Display::Grid },
                width: percent(100),
                flex_direction: FlexDirection::Column,
                grid_template_columns: if phone {
                    Vec::new()
                } else {
                    vec![RepeatedGridTrack::flex(2, 1.0)]
                },
                column_gap: kit.m.px(8.0),
                row_gap: kit.m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for template in Template::ALL {
        let on = draft.template == template;
        let (t_icon, t_color, t_sub) = template_visuals(template, lang);
        let extra = if on { 1.0 } else { 0.0 };
        let face = commands
            .spawn((
                Role::Segment,
                Node {
                    width: percent(100),
                    min_height: px_fixed(kit.m.control * 1.45),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Row,
                    column_gap: kit.m.px(10.0),
                    padding: UiRect::axes(
                        px_fixed((kit.m.scaled(12.0) - extra).max(0.0)),
                        px_fixed((kit.m.scaled(9.0) - extra).max(0.0)),
                    ),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    border: UiRect::all(px_fixed(if on { 2.0 } else { 1.0 })),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                    ..default()
                },
                BackgroundColor(if on {
                    tokens::SELECTED
                } else {
                    tokens::CONTROL
                }),
                BorderColor::all(if on { tokens::ACCENT } else { tokens::BORDER }),
            ))
            .id();
        let icon_badge = commands
            .spawn((
                Node {
                    width: kit.m.px(32.0),
                    height: kit.m.px(32.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(t_color.with_alpha(if on { 0.24 } else { 0.12 })),
                Pickable::IGNORE,
            ))
            .id();
        let icon_glyph = commands
            .spawn((
                Text::new(t_icon.to_string()),
                crate::hud::icon_tf(kit.fonts, kit.m.text * 0.92),
                TextColor(t_color),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(icon_badge).add_child(icon_glyph);

        let text_col = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    min_width: px(0),
                    row_gap: kit.m.px(2.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let words = controls::label(
            commands,
            kit,
            template.phrase().text(lang),
            kit.m.text,
            tokens::INK,
        );
        let sub_words = commands
            .spawn((
                Text::new(t_sub),
                crate::hud::tf(kit.fonts, kit.m.small * 0.86),
                TextColor(tokens::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(text_col).add_children(&[words, sub_words]);

        let tick = commands
            .spawn((
                Text::new(crate::hud::glyph::CHECK.to_string()),
                crate::hud::icon_tf(kit.fonts, kit.m.small),
                TextColor(if on { tokens::ACCENT } else { Color::NONE }),
                Pickable::IGNORE,
            ))
            .id();
        commands
            .entity(face)
            .add_children(&[icon_badge, text_col, tick]);
        let hit = controls::hit(
            commands,
            kit,
            face,
            Press::Play(PlayPress::Template(template)),
        );
        commands.entity(hit).entry::<Node>().and_modify(|mut n| {
            n.width = percent(100);
            n.flex_grow = 1.0;
        });
        orders::item(
            commands,
            hit,
            &orders::CREATE,
            "templates",
            template as usize,
        );
        commands.entity(cards).add_child(hit);
    }
    commands.entity(sec2).add_child(cards);

    let derived = parts::row(commands, kit, false);
    commands
        .entity(derived)
        .entry::<Node>()
        .and_modify(|mut n| {
            n.width = percent(100);
        });
    let sum_icon = commands
        .spawn((
            Text::new('\u{f004}'.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.90),
            TextColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    let summary = parts::line(
        commands,
        kit,
        &draft.summary(lang),
        kit.m.small,
        tokens::MUTED,
    );
    let gap = parts::grow(commands);
    let adjust = parts::menu_button(
        commands,
        kit,
        Phrase::SheetAdjust.text(lang),
        Weight::Ghost,
        Press::Play(PlayPress::Adjust),
    );
    orders::stop(commands, adjust, &orders::CREATE, "adjust");
    commands
        .entity(derived)
        .add_children(&[sum_icon, summary, gap, adjust]);
    commands.entity(sec2).add_child(derived);

    if draft.adjust {
        let adjust_box = commands
            .spawn((
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    row_gap: kit.m.px(8.0),
                    padding: UiRect::all(kit.m.px(10.0)),
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                    ..default()
                },
                BackgroundColor(tokens::PANEL.with_alpha(0.65)),
                BorderColor::all(tokens::BORDER.with_alpha(0.75)),
                Pickable::IGNORE,
            ))
            .id();
        let life = parts::row(commands, kit, true);
        commands.entity(life).entry::<Node>().and_modify(|mut n| {
            n.width = percent(100);
            n.justify_content = JustifyContent::SpaceBetween;
        });
        let life_lead = parts::row(commands, kit, false);
        let life_icon = commands
            .spawn((
                Text::new('\u{f004}'.to_string()),
                crate::hud::icon_tf(kit.fonts, kit.m.small),
                TextColor(tokens::DANGER),
                Pickable::IGNORE,
            ))
            .id();
        let caption = parts::line(
            commands,
            kit,
            Phrase::RoomLife.text(lang),
            kit.m.text,
            tokens::INK,
        );
        commands
            .entity(life_lead)
            .add_children(&[life_icon, caption]);
        let step = controls::stepper(
            commands,
            kit,
            &draft.life.to_string(),
            Press::Play(PlayPress::Life(-1)),
            Press::Play(PlayPress::Life(1)),
        );
        orders::items_of(commands, step, &orders::CREATE, "life");
        commands.entity(life).add_children(&[life_lead, step]);

        let mull = parts::row(commands, kit, true);
        commands.entity(mull).entry::<Node>().and_modify(|mut n| {
            n.width = percent(100);
            n.justify_content = JustifyContent::SpaceBetween;
        });
        let mull_lead = parts::row(commands, kit, false);
        let mull_icon = commands
            .spawn((
                Text::new('\u{f021}'.to_string()),
                crate::hud::icon_tf(kit.fonts, kit.m.small),
                TextColor(tokens::ACCENT),
                Pickable::IGNORE,
            ))
            .id();
        let caption = parts::line(
            commands,
            kit,
            Phrase::RoomMulligans.text(lang),
            kit.m.text,
            tokens::INK,
        );
        commands
            .entity(mull_lead)
            .add_children(&[mull_icon, caption]);
        let step = controls::stepper(
            commands,
            kit,
            &draft.mulligans.to_string(),
            Press::Play(PlayPress::Mulligans(false)),
            Press::Play(PlayPress::Mulligans(true)),
        );
        orders::items_of(commands, step, &orders::CREATE, "mulligans");
        commands.entity(mull).add_children(&[mull_lead, step]);

        commands.entity(adjust_box).add_children(&[life, mull]);
        commands.entity(sec2).add_child(adjust_box);
    }
    body.push(sec2);

    // =========================================================================
    // SECTION 3: Bedenkzeit, Zugang & Zuschauer (Clock, Privacy & Spectators)
    // =========================================================================
    let sec3 = spawn_create_section(
        commands,
        kit,
        "3",
        '\u{f017}',
        if lobby.offline() {
            Phrase::CreateSectionAccessOffline.text(lang)
        } else {
            Phrase::CreateSectionAccess.text(lang)
        },
        if lobby.offline() {
            Phrase::CreateSectionAccessOfflineAbout.text(lang)
        } else {
            Phrase::CreateSectionAccessAbout.text(lang)
        },
        Color::srgb(0.38, 0.84, 0.56),
    );

    let spec_row = parts::row(commands, kit, true);
    commands
        .entity(spec_row)
        .entry::<Node>()
        .and_modify(|mut n| {
            n.width = percent(100);
            n.justify_content = JustifyContent::SpaceBetween;
        });
    let spectators = controls::button_with_icon(
        commands,
        kit,
        if draft.spectators {
            '\u{f14a}'
        } else {
            '\u{f0c8}'
        },
        Phrase::AllowSpectators.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        Press::Play(PlayPress::Spectators),
    );
    let spec_badge = parts::badge_with(
        commands,
        kit,
        if draft.spectators {
            '\u{f06e}'
        } else {
            '\u{f070}'
        },
        if draft.spectators {
            Phrase::CreateSpectatorsAllowed.text(lang)
        } else {
            Phrase::CreateNoSpectators.text(lang)
        },
        if draft.spectators {
            Color::srgb(0.36, 0.84, 0.56)
        } else {
            tokens::MUTED
        },
    );
    commands
        .entity(spec_row)
        .add_children(&[spectators, spec_badge]);
    commands.entity(sec3).add_child(spec_row);

    if !lobby.offline() {
        let pw_lbl = label(commands, Phrase::SheetPassword.text(lang));
        let pw_field = text_field(
            commands,
            kit.fonts,
            metrics,
            "",
            &FieldLook {
                buffer: lobby.buffer(Field::RoomPassword),
                focused: lobby.focus() == Field::RoomPassword,
                mask: Some(Masked {
                    field: Some(Field::RoomPassword),
                    shown: lobby.showing(Field::RoomPassword),
                }),
                press: Press::Shared(SharedPress::Focus(Field::RoomPassword)),
                lead: Some('\u{f023}'),
                hint: Some(Phrase::SheetNoPassword.text(lang)),
                tail: None,
            },
        );
        orders::field(commands, pw_field, &orders::CREATE, "password");
        commands.entity(sec3).add_children(&[pw_lbl, pw_field]);
    }

    // The clock: the gateway's list, the first the default (M-4). A room's
    // clock is set when it opens, so Apply shows it and offers no change.
    if !lobby.offline() && !editing {
        let clock_lbl = label(commands, Phrase::SheetClock.text(lang));
        let clocks = lobby.clocks();
        let labels: Vec<String> = clocks.iter().map(|c| model::clock_label(lang, c)).collect();
        let at_clock = draft.clock.min(clocks.len().saturating_sub(1));
        let widest = labels
            .iter()
            .max_by_key(|l| l.chars().count())
            .cloned()
            .unwrap_or_default();
        let clock_step = controls::stepper_room(
            commands,
            kit,
            labels.get(at_clock).map_or("", String::as_str),
            &widest,
            Press::Play(PlayPress::StepClock(false)),
            Press::Play(PlayPress::StepClock(true)),
        );
        orders::items_of(commands, clock_step, &orders::CREATE, "clock");
        commands.entity(sec3).add_children(&[clock_lbl, clock_step]);
        if let Some(clock) = clocks.get(draft.clock) {
            let help_line = parts::line(
                commands,
                kit,
                &model::clock_help(lang, clock),
                kit.m.small,
                tokens::MUTED,
            );
            commands.entity(sec3).add_child(help_line);
        }
    }
    body.push(sec3);

    let cancel = controls::button(
        commands,
        kit,
        Phrase::SheetCancel.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        Press::Play(PlayPress::CloseSheet),
    );
    let open = controls::button_with_icon(
        commands,
        kit,
        if editing { '\u{f00c}' } else { '\u{f067}' },
        if editing {
            Phrase::SheetApply
        } else {
            Phrase::SheetOpenTable
        }
        .text(lang),
        Weight::Primary,
        if lobby.busy() {
            Live::No(Phrase::VeilTalking.text(lang))
        } else {
            Live::Yes
        },
        Some("Enter"),
        Press::Play(PlayPress::Open),
    );
    orders::stop(commands, cancel, &orders::CREATE, "cancel");
    orders::stop(commands, open, &orders::CREATE, "open");
    let surface = surfaces::sheet_box_listed(
        commands,
        kit,
        SheetWidth::Medium,
        if editing {
            Phrase::SheetEditRules
        } else {
            Phrase::CreateTable
        }
        .text(lang),
        &body,
        &[cancel, open],
        super::super::Scrollable(super::super::List::Check),
    );
    let close_x = parts::icon_button(
        commands,
        kit,
        crate::hud::glyph::CLOSE,
        Press::Play(PlayPress::CloseSheet),
    );
    let close_inset = px_fixed(kit.m.pad * 0.55);
    commands
        .entity(close_x)
        .insert(super::super::hint::HoverHint(
            Phrase::ShellClose.text(lang).to_string(),
        ))
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.position_type = PositionType::Absolute;
            n.top = close_inset;
            n.right = close_inset;
        });
    commands.entity(surface).add_child(close_x);
    let scrim = surfaces::sheet(commands, surface);
    commands
        .entity(surface)
        .insert(TourAnchor(Anchor::CreateSheet));
    commands
        .entity(scrim)
        .insert(Press::Play(PlayPress::CloseSheet));
    commands
        .entity(surface)
        .insert(Press::Shared(SharedPress::PickerNothing));
    commands.entity(root).add_child(scrim);
}
