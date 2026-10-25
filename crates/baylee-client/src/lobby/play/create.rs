//! The Create-table sheet (§5).

#[allow(clippy::wildcard_imports)] // the Play screen's own vocabulary
use super::*;

/// The Create-table sheet (§5): name, players, template with its derived
/// line and Adjust, password, clock; Cancel · Open table (Apply over a
/// room).
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

    body.push(label(commands, Phrase::SheetName.text(lang)));
    let at = body.len();
    body.push(text_field(
        commands,
        kit.fonts,
        metrics,
        "",
        &FieldLook {
            buffer: lobby.buffer(Field::RoomName),
            focused: lobby.focus() == Field::RoomName,
            mask: None,
            press: Press::Shared(SharedPress::Focus(Field::RoomName)),
            lead: None,
            hint: None,
            tail: None,
        },
    ));
    orders::field(commands, body[at], &orders::CREATE, "name");

    body.push(label(commands, Phrase::SheetPlayers.text(lang)));
    let at = body.len();
    if phone {
        body.push(controls::stepper(
            commands,
            kit,
            &draft.players.to_string(),
            Press::Play(PlayPress::StepPlayers(false)),
            Press::Play(PlayPress::StepPlayers(true)),
        ));
    } else {
        let counts: Vec<String> = (2..=8).map(|n| n.to_string()).collect();
        let items: Vec<&str> = counts.iter().map(String::as_str).collect();
        body.push(controls::segmented(
            commands,
            kit,
            &items,
            draft.players.saturating_sub(2),
            |i| Press::Play(PlayPress::Players(i + 2)),
        ));
    }
    orders::items_of(commands, body[at], &orders::CREATE, "players");

    let spectators = controls::button(
        commands,
        kit,
        &format!(
            "{} {}",
            if draft.spectators {
                '\u{2611}'
            } else {
                '\u{2610}'
            },
            Phrase::AllowSpectators.text(lang)
        ),
        Weight::Secondary,
        Live::Yes,
        None,
        Press::Play(PlayPress::Spectators),
    );
    body.push(spectators);

    body.push(label(commands, Phrase::SheetTemplate.text(lang)));
    let cards = parts::row(commands, kit, true);
    for template in Template::ALL {
        let on = draft.template == template;
        let extra = if on { 1.0 } else { 0.0 };
        let face = commands
            .spawn((
                Role::Segment,
                Node {
                    min_height: px_fixed(kit.m.control),
                    min_width: kit.m.px(140.0),
                    flex_grow: 1.0,
                    padding: UiRect::axes(
                        px_fixed((kit.m.scaled(12.0) - extra).max(0.0)),
                        px_fixed((kit.m.scaled(8.0) - extra).max(0.0)),
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
        let words = controls::label(
            commands,
            kit,
            template.phrase().text(lang),
            kit.m.text,
            tokens::INK,
        );
        let tick = commands
            .spawn((
                Text::new(crate::hud::glyph::CHECK.to_string()),
                crate::hud::icon_tf(kit.fonts, kit.m.small),
                TextColor(if on { tokens::ACCENT } else { Color::NONE }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(face).add_children(&[words, tick]);
        let hit = controls::hit(
            commands,
            kit,
            face,
            Press::Play(PlayPress::Template(template)),
        );
        commands
            .entity(hit)
            .entry::<Node>()
            .and_modify(|mut n| n.flex_grow = 1.0);
        orders::item(
            commands,
            hit,
            &orders::CREATE,
            "templates",
            template as usize,
        );
        commands.entity(cards).add_child(hit);
    }
    body.push(cards);
    let derived = parts::row(commands, kit, false);
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
        .add_children(&[summary, gap, adjust]);
    body.push(derived);
    if draft.adjust {
        let life = parts::row(commands, kit, true);
        let caption = parts::line(
            commands,
            kit,
            Phrase::RoomLife.text(lang),
            kit.m.text,
            tokens::INK,
        );
        let step = controls::stepper(
            commands,
            kit,
            &draft.life.to_string(),
            Press::Play(PlayPress::Life(-1)),
            Press::Play(PlayPress::Life(1)),
        );
        orders::items_of(commands, step, &orders::CREATE, "life");
        commands.entity(life).add_children(&[caption, step]);
        let mull = parts::row(commands, kit, true);
        let caption = parts::line(
            commands,
            kit,
            Phrase::RoomMulligans.text(lang),
            kit.m.text,
            tokens::INK,
        );
        let step = controls::stepper(
            commands,
            kit,
            &draft.mulligans.to_string(),
            Press::Play(PlayPress::Mulligans(false)),
            Press::Play(PlayPress::Mulligans(true)),
        );
        orders::items_of(commands, step, &orders::CREATE, "mulligans");
        commands.entity(mull).add_children(&[caption, step]);
        body.push(life);
        body.push(mull);
    }

    if !lobby.offline() {
        body.push(label(commands, Phrase::SheetPassword.text(lang)));
        let at = body.len();
        body.push(text_field(
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
                lead: None,
                hint: Some(Phrase::SheetNoPassword.text(lang)),
                tail: None,
            },
        ));
        orders::field(commands, body[at], &orders::CREATE, "password");
    }

    // The clock: the gateway's list, the first the default (M-4). A room's
    // clock is set when it opens, so Apply shows it and offers no change.
    // Fourteen paces (owner, 10.10.2026) are a stepper, not a row of
    // segments: a row that long fits no phone. It keeps room for the widest
    // label, so the `+` stands still while the player steps through them.
    if !lobby.offline() && !editing {
        body.push(label(commands, Phrase::SheetClock.text(lang)));
        let clocks = lobby.clocks();
        let labels: Vec<String> = clocks.iter().map(|c| model::clock_label(lang, c)).collect();
        let at_clock = draft.clock.min(clocks.len().saturating_sub(1));
        let widest = labels
            .iter()
            .max_by_key(|l| l.chars().count())
            .cloned()
            .unwrap_or_default();
        let at = body.len();
        body.push(controls::stepper_room(
            commands,
            kit,
            labels.get(at_clock).map_or("", String::as_str),
            &widest,
            Press::Play(PlayPress::StepClock(false)),
            Press::Play(PlayPress::StepClock(true)),
        ));
        orders::items_of(commands, body[at], &orders::CREATE, "clock");
        if let Some(clock) = clocks.get(draft.clock) {
            body.push(parts::line(
                commands,
                kit,
                &model::clock_help(lang, clock),
                kit.m.small,
                tokens::MUTED,
            ));
        }
    }

    let cancel = controls::button(
        commands,
        kit,
        Phrase::SheetCancel.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        Press::Play(PlayPress::CloseSheet),
    );
    let open = controls::button(
        commands,
        kit,
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
    let surface = surfaces::sheet_box(
        commands,
        kit,
        SheetWidth::Small,
        if editing {
            Phrase::SheetEditRules
        } else {
            Phrase::CreateTable
        }
        .text(lang),
        &body,
        &[cancel, open],
    );
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
