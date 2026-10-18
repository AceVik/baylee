//! Gateway context and quiet, list-local feedback for table discovery.
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

/// Fixed virtual-list pitch, including the gap between entries.
pub(in crate::lobby) const fn row_stride(frame: ShellFrame) -> f32 {
    if matches!(frame, ShellFrame::Phone) {
        92.0
    } else {
        112.0
    }
}

fn panel(commands: &mut Commands, parent: Entity, kit: Kit) -> Entity {
    let panel = commands
        .spawn((
            Role::Panel,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px_fixed(kit.m.pad)),
                row_gap: kit.m.px(16.0),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(14)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(parent).add_child(panel);
    panel
}

/// Discovery is about the gateway and available rooms; decks are chosen there.
pub(super) fn sidebar(commands: &mut Commands, parent: Entity, state: &LobbyState, kit: Kit) {
    let lang = state.lobby.lang();
    let card = panel(commands, parent, kit);
    let eyebrow = parts::line(
        commands,
        kit,
        Phrase::DiscoveryGateway.text(lang),
        kit.m.small,
        tokens::GOLD,
    );
    let name = state
        .gateway_name()
        .unwrap_or_else(|| Phrase::DiscoveryGateway.text(lang).to_owned());
    let title = commands
        .spawn((
            Text::new(name),
            crate::hud::tf_bold(kit.fonts, kit.m.head),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(card).add_children(&[eyebrow, title]);
    if matches!(kit.m.frame, ShellFrame::Wide | ShellFrame::Vast) {
        let address = parts::line(commands, kit, &state.gateway, kit.m.small, tokens::MUTED);
        commands.entity(card).add_child(address);
    }
    let (status, color) = if state.lobby.unreachable() {
        (Phrase::ShellReconnecting, tokens::DANGER)
    } else if state.feed_down {
        (Phrase::DiscoveryConnecting, tokens::GOLD)
    } else {
        (Phrase::DiscoveryLive, tokens::ACCENT)
    };
    let live = parts::glyph_line(commands, kit, '\u{f1eb}', status.text(lang), color);
    commands.entity(card).add_child(live);
    gateway_stats(commands, card, state, kit);
    let details = controls::button(
        commands,
        kit,
        Phrase::DiscoveryDetails.text(lang),
        Weight::Ghost,
        Live::Yes,
        None,
        Press::Header(super::super::header::HeaderPress::Gateway),
    );
    orders::stop(commands, details, &orders::PLAY, "gateway-details");
    commands.entity(card).add_child(details);

    next_table(commands, parent, state, kit);
}

fn next_table(commands: &mut Commands, parent: Entity, state: &LobbyState, kit: Kit) {
    let lang = state.lobby.lang();
    let card = panel(commands, parent, kit);
    commands.entity(card).insert(TourAnchor(Anchor::PlayHero));
    let title = parts::line(
        commands,
        kit,
        Phrase::DiscoveryChoose.text(lang),
        kit.m.head,
        tokens::INK,
    );
    let help =
        crate::shellkit::surfaces::prose(commands, kit, Phrase::DiscoveryHelp.text(lang), false);
    commands.entity(card).add_children(&[title, help]);
    let off = starts_off(state, lang);
    let create = controls::button(
        commands,
        kit,
        Phrase::CreateTable.text(lang),
        Weight::Primary,
        off.as_deref().map_or(Live::Yes, Live::No),
        None,
        Press::Play(PlayPress::CreateTable),
    );
    orders::stop(commands, create, &orders::PLAY, "create");
    commands.entity(card).add_child(create);
    if let Some(reason) = off {
        let why = crate::shellkit::surfaces::prose(commands, kit, &reason, false);
        commands.entity(card).add_child(why);
    }
    let refresh = controls::button(
        commands,
        kit,
        Phrase::DiscoveryRefresh.text(lang),
        Weight::Secondary,
        if state.lobby.busy() {
            Live::No(Phrase::DiscoveryLoading.text(lang))
        } else {
            Live::Yes
        },
        None,
        if state.feed_down {
            Press::Header(super::super::header::HeaderPress::RetryNow)
        } else {
            Press::Hub(HubPress::Refresh)
        },
    );
    orders::stop(commands, refresh, &orders::PLAY, "refresh");
    commands.entity(card).add_child(refresh);
}

fn gateway_stats(commands: &mut Commands, card: Entity, state: &LobbyState, kit: Kit) {
    let lang = state.lobby.lang();
    let stats = strips::gateway_breakdown(state.lobby.stats(), &state.lobby);
    let grid = parts::row(commands, kit, false);
    commands
        .entity(grid)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.width = percent(100);
            n.column_gap = kit.m.px(8.0);
            n.align_items = AlignItems::Stretch;
        });
    for (value, label) in [
        (stats.online, Phrase::DiscoveryOnline),
        (stats.tables_waiting, Phrase::DiscoveryWaiting),
        (stats.games_running, Phrase::DiscoveryPlaying),
    ] {
        let cell = commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    min_height: kit.m.px(82.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: kit.m.px(6.0),
                    padding: UiRect::all(kit.m.px(10.0)),
                    border_radius: BorderRadius::all(px(8)),
                    ..default()
                },
                BackgroundColor(tokens::CONTROL),
                Pickable::IGNORE,
            ))
            .id();
        let number = parts::line(commands, kit, &value.to_string(), kit.m.head, tokens::INK);
        let label = parts::line(commands, kit, label.text(lang), kit.m.small, tokens::MUTED);
        commands.entity(cell).add_children(&[number, label]);
        commands.entity(grid).add_child(cell);
    }
    commands.entity(card).add_child(grid);
}

/// Seat occupancy is a small structural diagram as well as a text count.
pub(super) fn seat_marks(commands: &mut Commands, parent: Entity, kit: Kit, game: &GameSummary) {
    let marks = parts::row(commands, kit, false);
    commands
        .entity(marks)
        .entry::<Node>()
        .and_modify(move |mut n| n.column_gap = kit.m.px(4.0));
    for seat in &game.seats {
        let mark = commands
            .spawn((
                Node {
                    width: kit.m.px(13.0),
                    height: kit.m.px(5.0),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(if seat.taken || seat.kind == SeatKind::Ai {
                    tokens::ACCENT
                } else {
                    tokens::SKELETON
                }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(marks).add_child(mark);
    }
    commands.entity(parent).add_child(marks);
}

/// Reserved list-local status row; becoming busy never moves the page header.
#[derive(Component)]
pub(crate) struct ListLoading;

#[derive(Component)]
pub(crate) struct LoadingDot(f32);

pub(super) fn loading_notice(commands: &mut Commands, parent: Entity, kit: Kit, count: usize) {
    let lang = if kit.german { Lang::De } else { Lang::En };
    let row = parts::row(commands, kit, false);
    commands
        .entity(row)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.width = percent(100);
            node.height = kit.m.px(26.0);
            node.flex_shrink = 0.0;
        });
    let summary = parts::line(
        commands,
        kit,
        &Phrase::DiscoveryResults.fill(lang, &[&count.to_string()]),
        kit.m.small,
        tokens::MUTED,
    );
    let space = parts::grow(commands);
    commands.entity(row).add_children(&[summary, space]);
    let line = commands
        .spawn((
            ListLoading,
            Node {
                height: kit.m.px(26.0),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(8.0),
                ..default()
            },
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .id();
    for phase in [0.0, 1.0, 2.0] {
        let dot = commands
            .spawn((
                LoadingDot(phase),
                Node {
                    width: kit.m.px(5.0),
                    height: kit.m.px(5.0),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(tokens::ACCENT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(line).add_child(dot);
    }
    let lang = if kit.german { Lang::De } else { Lang::En };
    let text = parts::line(
        commands,
        kit,
        Phrase::DiscoveryLoading.text(lang),
        kit.m.small,
        tokens::MUTED,
    );
    commands.entity(line).add_child(text);
    commands.entity(row).add_child(line);
    commands.entity(parent).add_child(row);
}

/// Delayed appearance and a short minimum dwell avoid spinner flashes on fast requests.
pub(in crate::lobby) fn animate_loading(
    time: Res<Time>,
    state: Res<LobbyState>,
    feed: Option<Res<super::super::feed::Feed>>,
    prefs: Res<crate::prefs::Prefs>,
    (mut elapsed, mut dwell): (Local<f32>, Local<f32>),
    mut notices: Query<&mut Visibility, With<ListLoading>>,
    mut dots: Query<(&LoadingDot, &mut BackgroundColor)>,
) {
    let pending = state.lobby.refreshing()
        || state.feed_down
        || feed.as_ref().is_some_and(|feed| feed.loading());
    if pending {
        *elapsed += time.delta_secs();
    } else {
        *elapsed = 0.0;
    }
    if *elapsed >= 0.15 {
        *dwell = 0.3;
    } else {
        *dwell = (*dwell - time.delta_secs()).max(0.0);
    }
    let visible = *dwell > 0.0;
    for mut notice in &mut notices {
        let want = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *notice != want {
            *notice = want;
        }
    }
    if !visible {
        return;
    }
    let still = prefs.all().reduce_motion;
    for (dot, mut color) in &mut dots {
        let alpha = if still {
            0.8
        } else {
            0.5 + 0.35 * (time.elapsed_secs() * 5.0 - dot.0).sin()
        };
        let next = tokens::ACCENT.with_alpha(alpha);
        if color.0 != next {
            color.0 = next;
        }
    }
}

/// Ask only after choosing a protected table, leaving every listing column aligned.
pub(super) fn password_sheet(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    metrics: Metrics,
    kit: Kit,
    id: &str,
) {
    let Some(game) = state.lobby.games().iter().find(|game| game.id == id) else {
        return;
    };
    let lang = state.lobby.lang();
    let name = parts::line(
        commands,
        kit,
        &strips::table_name(game),
        kit.m.head,
        tokens::INK,
    );
    let input = text_field(
        commands,
        kit.fonts,
        metrics,
        Phrase::PlayPassword.text(lang),
        &FieldLook {
            buffer: state.lobby.buffer(Field::RoomPassword),
            focused: true,
            mask: Some(Masked {
                field: Some(Field::RoomPassword),
                shown: state.lobby.showing(Field::RoomPassword),
            }),
            press: Press::Shared(SharedPress::Focus(Field::RoomPassword)),
            lead: Some(parts::LOCK),
            hint: None,
            tail: None,
        },
    );
    let cancel = controls::button(
        commands,
        kit,
        Phrase::SheetCancel.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        Press::Play(PlayPress::CloseSheet),
    );
    let join = controls::button(
        commands,
        kit,
        Phrase::Join.text(lang),
        Weight::Primary,
        if state.lobby.busy() {
            Live::No(Phrase::VeilTalking.text(lang))
        } else {
            Live::Yes
        },
        Some("Enter"),
        Press::Play(PlayPress::JoinProtected),
    );
    orders::field(commands, input, &orders::CREATE, "password");
    orders::stop(commands, cancel, &orders::CREATE, "cancel");
    orders::stop(commands, join, &orders::CREATE, "open");
    let surface = surfaces::sheet_box(
        commands,
        kit,
        SheetWidth::Small,
        Phrase::DiscoveryProtected.text(lang),
        &[name, input],
        &[cancel, join],
    );
    let scrim = surfaces::sheet(commands, surface);
    commands
        .entity(scrim)
        .insert(Press::Play(PlayPress::CloseSheet));
    commands
        .entity(surface)
        .insert(Press::Shared(SharedPress::PickerNothing));
    commands.entity(root).add_child(scrim);
}

/// A stable hover surface for one table entry.
#[derive(Component)]
pub(crate) struct TableCard;

type HoverCards<'w, 's> = Query<
    'w,
    's,
    (&'static crate::ambience::Feel, &'static mut BorderColor),
    (With<TableCard>, Changed<crate::ambience::Feel>),
>;

/// The rim follows the existing hover easing without moving the hit rectangle.
pub(in crate::lobby) fn hover_cards(mut cards: HoverCards<'_, '_>) {
    for (feel, mut border) in &mut cards {
        let next = BorderColor::all(
            tokens::BORDER
                .with_alpha(0.45)
                .mix(&tokens::ACCENT.with_alpha(0.75), feel.warmth.max(0.0)),
        );
        if *border != next {
            *border = next;
        }
    }
}
