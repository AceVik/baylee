//! The open tables: the list, its filters and sort, and a row per table.

#[allow(clippy::wildcard_imports)] // the Play screen's own vocabulary
use super::*;

/// The tables panel: head, chips, rows, pager. Answers the Sort button.
#[allow(clippy::too_many_lines)] // one panel, read top to bottom
pub(super) fn tables(
    commands: &mut Commands,
    parent: Entity,
    state: &LobbyState,
    metrics: Metrics,
    kit: Kit,
    scrolled: &Scrolled,
) -> Option<Entity> {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let phone = kit.m.frame == ShellFrame::Phone;
    let wide = matches!(kit.m.frame, ShellFrame::Wide | ShellFrame::Vast);
    let panel = commands
        .spawn((
            TourAnchor(Anchor::PlayTables),
            Role::Panel,
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                width: if wide || phone {
                    Val::Auto
                } else {
                    percent(100)
                },
                height: if wide || phone {
                    percent(100)
                } else {
                    Val::Auto
                },
                align_self: AlignSelf::Stretch,
                max_width: if wide { kit.m.px(1000.0) } else { Val::Auto },
                max_height: percent(100),
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px_fixed(kit.m.pad)),
                row_gap: px_fixed(kit.m.gap),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(tokens::BORDER),
            super::super::dock::Dock(4),
        ))
        .id();
    commands.entity(parent).add_child(panel);
    let offline = lobby.offline();

    // Head: "Tables · 3 waiting · updated just now" and the search.
    let head = parts::row(commands, kit, !phone);
    commands
        .entity(head)
        .entry::<Node>()
        .and_modify(|mut n| n.flex_shrink = 0.0);
    if !phone {
        let title = commands
            .spawn((
                Text::new(Phrase::Tables.text(lang)),
                crate::hud::tf(kit.fonts, kit.m.head),
                TextColor(tokens::INK),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(head).add_child(title);
        let waiting = lobby
            .games()
            .iter()
            .filter(|g| g.state == "waiting")
            .count();
        let fresh = if lobby.unreachable() || state.feed_down {
            String::new()
        } else {
            format!(" · {}", Phrase::ShellUpdatedJustNow.text(lang))
        };
        let note = parts::line(
            commands,
            kit,
            &format!(
                "{}{fresh}",
                Phrase::ShellTablesWaiting.fill(lang, &[&waiting.to_string()])
            ),
            kit.m.small,
            tokens::MUTED,
        );
        commands.entity(head).add_child(note);
        let gap = parts::grow(commands);
        commands.entity(head).add_child(gap);
    }
    if !offline {
        let hunt = commands
            .spawn((
                Node {
                    width: if phone { percent(42) } else { kit.m.px(320.0) },
                    flex_shrink: if phone { 0.0 } else { 1.0 },
                    min_width: px(100),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let field = text_field(
            commands,
            kit.fonts,
            metrics,
            "",
            &FieldLook {
                buffer: lobby.buffer(Field::Search),
                focused: lobby.focus() == Field::Search && lobby.typing_here(),
                mask: None,
                press: Press::Shared(SharedPress::Focus(Field::Search)),
                lead: Some(crate::hud::glyph::MAGNIFIER),
                hint: Some(Phrase::SearchTables.text(lang)),
                tail: None,
            },
        );
        orders::field(commands, field, &orders::PLAY, "search");
        commands.entity(hunt).add_child(field);
        commands.entity(head).add_child(hunt);
    }
    commands.entity(panel).add_child(head);

    // Chips and Sort: one row; on a phone they share the search's row and
    // scroll sideways there, Sort first (M4-6: search and chips in one 44
    // row).
    let mut sort_anchor = None;
    if !offline {
        let chips = commands
            .spawn((
                Node {
                    align_items: AlignItems::Center,
                    column_gap: kit.m.px(8.0),
                    row_gap: kit.m.px(6.0),
                    flex_wrap: if phone {
                        FlexWrap::NoWrap
                    } else {
                        FlexWrap::Wrap
                    },
                    overflow: if phone {
                        Overflow::scroll_x()
                    } else {
                        Overflow::visible()
                    },
                    flex_shrink: if phone { 1.0 } else { 0.0 },
                    flex_grow: if phone { 1.0 } else { 0.0 },
                    min_width: px(0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        for chip in Chip::ALL {
            let c = controls::chip(
                commands,
                kit,
                chip.phrase().text(lang),
                state.play.filter.on(chip),
                None,
                false,
                Press::Play(PlayPress::Chip(chip)),
            );
            orders::item(commands, c, &orders::PLAY, "chips", chip as usize);
            commands.entity(chips).add_child(c);
        }
        let sort = parts::menu_button(
            commands,
            kit,
            &Phrase::PlaySortBy.fill(lang, &[state.play.sort.phrase().text(lang)]),
            Weight::Ghost,
            Press::Shared(SharedPress::OpenMenu(ShellMenu::TableSort)),
        );
        orders::stop(commands, sort, &orders::PLAY, "sort");
        sort_anchor = Some(sort);
        if phone {
            commands.entity(chips).insert_children(0, &[sort]);
            commands.entity(head).add_child(chips);
        } else {
            let gap = parts::grow(commands);
            commands.entity(chips).add_children(&[gap, sort]);
            commands.entity(panel).add_child(chips);
        }
    }

    // While seated, Join is off for every other row, said once (§2.1).
    let held = strips::strip(lobby);
    if let Some(Strip::Seated { table, .. } | Strip::Playing { table, .. }) = &held {
        let line = parts::line(
            commands,
            kit,
            &Phrase::PlayJoinOff.fill(lang, &[table]),
            kit.m.small,
            tokens::GOLD,
        );
        commands.entity(panel).add_child(line);
    }
    if lobby.unreachable() && !offline {
        let retry = controls::button(
            commands,
            kit,
            Phrase::ShellRetry.text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            Press::Play(PlayPress::Retry),
        );
        let name = state
            .gateway_name()
            .unwrap_or_else(|| state.gateway.clone());
        let line = crate::shellkit::states::error_line(
            commands,
            kit,
            &Phrase::ShellCouldNotReach.fill(lang, &[&name]),
            retry,
        );
        commands.entity(panel).add_child(line);
    }

    let query = lobby.field(Field::Search).trim();
    let order: Vec<usize> = model::table_order(lobby.games(), state.play.filter, state.play.sort)
        .into_iter()
        .filter(|&i| model::matches_search(&lobby.games()[i], query))
        .collect();
    if !offline {
        discovery::loading_notice(commands, panel, kit, order.len());
    }
    let list = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(8.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            Role::Scroll,
            Pickable::default(),
        ))
        .id();
    let bar = super::super::scrollbars::attach(commands, panel, list, metrics);
    commands
        .entity(bar)
        .insert(super::super::scrollbars::OnlyWhenNeeded)
        .entry::<Node>()
        .and_modify(|mut node| node.display = Display::None);
    // A list scrolls only when it has rows to scroll.
    if !order.is_empty() {
        commands.entity(list).insert((
            Scrollable(List::Games),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::Games))),
        ));
    }
    if order.is_empty() && lobby.refreshing() {
        let skeleton = crate::shellkit::states::skeleton(commands, kit, 3);
        commands.entity(list).add_child(skeleton);
    } else if order.is_empty() {
        let hunt = lobby.field(Field::Search).trim();
        let (said, action) = if offline {
            (Phrase::OfflineReadyToPlay.text(lang).to_string(), None)
        } else if !hunt.is_empty() {
            (
                Phrase::NoTableMatches.fill(lang, &[hunt]),
                Some((
                    Phrase::ClearTableSearch,
                    Press::Play(PlayPress::ClearSearch),
                )),
            )
        } else {
            (
                Phrase::ShellNoOneWaiting.text(lang).to_string(),
                starts_off(state, lang)
                    .is_none()
                    .then_some((Phrase::CreateTable, Press::Play(PlayPress::CreateTable))),
            )
        };
        let action = action.map(|(phrase, press)| {
            controls::button(
                commands,
                kit,
                phrase.text(lang),
                Weight::Secondary,
                Live::Yes,
                None,
                press,
            )
        });
        let empty =
            crate::shellkit::states::empty(commands, kit, crate::hud::glyph::HOUSE, &said, action);
        commands.entity(list).add_child(empty);
    }
    // Deck selection belongs to the room; browsing has no selected-deck warning.
    let mine_format = String::new();
    // Lazy windowed virtualization for large table listings (> 24 rows),
    // keeping DOM/ECS entity count bounded while preserving exact scroll height.
    let lazy_window = 24usize;
    let total_rows = order.len();
    let row_stride = discovery::row_stride(kit.m.frame) * kit.m.scale;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (start, end) = if total_rows > lazy_window {
        let scroll_y = scrolled.get(List::Games).max(0.0);
        let first = (scroll_y / row_stride).floor() as usize;
        let s = first
            .saturating_sub(4)
            .min(total_rows.saturating_sub(lazy_window));
        (s, (s + lazy_window).min(total_rows))
    } else {
        (0, total_rows)
    };
    #[allow(clippy::cast_precision_loss)]
    if start > 0 {
        let top_spacer = commands
            .spawn((
                Node {
                    width: percent(100),
                    height: px_fixed((start as f32 * row_stride - 8.0 * kit.m.scale).max(0.0)),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(list).add_child(top_spacer);
    }
    let mut walked = start;
    for &index in &order[start..end] {
        let game = &lobby.games()[index];
        let row = table_row(
            commands,
            state,
            kit,
            index,
            game,
            &mine_format,
            held.is_some(),
            &mut walked,
        );
        commands.entity(list).add_child(row);
    }
    #[allow(clippy::cast_precision_loss)]
    if end < total_rows {
        let bottom_spacer = commands
            .spawn((
                Node {
                    width: percent(100),
                    height: px_fixed(
                        ((total_rows - end) as f32 * row_stride - 8.0 * kit.m.scale).max(0.0),
                    ),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(list).add_child(bottom_spacer);
    }
    // The pager, only with more than one page.
    if lobby.total() > lobby.games().len() {
        let bar = parts::row(commands, kit, false);
        let first = lobby.offset() + 1;
        let last = lobby.offset() + lobby.games().len();
        let count = parts::line(
            commands,
            kit,
            &Phrase::PageOf.fill(
                lang,
                &[
                    &first.to_string(),
                    &last.to_string(),
                    &lobby.total().to_string(),
                ],
            ),
            kit.m.small,
            tokens::MUTED,
        );
        commands.entity(bar).add_child(count);
        for (label, forwards, live) in [
            (Phrase::PageBack.text(lang), false, lobby.offset() > 0),
            (Phrase::PageMore.text(lang), true, lobby.more()),
        ] {
            let b = controls::button(
                commands,
                kit,
                label,
                Weight::Ghost,
                if live && !lobby.busy() {
                    Live::Yes
                } else {
                    Live::No("")
                },
                None,
                Press::Hub(HubPress::Page(forwards)),
            );
            orders::item(commands, b, &orders::PLAY, "pager", usize::from(forwards));
            commands.entity(bar).add_child(b);
        }
        commands.entity(panel).add_child(bar);
    }
    sort_anchor
}

/// One table: name, host · format · life; seats, lock, AI count; Join (the
/// password inline when locked), Return at the player's own, nothing at
/// somebody else's running table (C-3). On narrower frames the counts go
/// to a second meta line.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // the row's inputs; one row, left to right
fn table_row(
    commands: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    index: usize,
    game: &GameSummary,
    mine_format: &str,
    held: bool,
    walked: &mut usize,
) -> Entity {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let wide = matches!(kit.m.frame, ShellFrame::Wide | ShellFrame::Vast);
    // A phone's row is two lines, 60 high (M4-6): the name, then the counts
    // with the format warning as a badge; host, life and clock wait for the
    // room. Three rows stand over the fold at 844 x 390.
    let phone = kit.m.frame == ShellFrame::Phone;
    let row = commands
        .spawn((
            Role::Row,
            discovery::TableCard,
            Node {
                width: percent(100),
                min_height: kit.m.px(discovery::row_stride(kit.m.frame) - 8.0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                row_gap: kit.m.px(4.0),
                padding: UiRect::axes(kit.m.px(16.0), kit.m.px(12.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(12.0)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(tokens::BORDER.with_alpha(0.45)),
            UiTransform::default(),
            crate::ambience::Feel::tinting_to(tokens::CONTROL, tokens::SELECTED),
            Pickable::default(),
        ))
        .id();
    let top = parts::row(commands, kit, false);
    commands
        .entity(top)
        .entry::<Node>()
        .and_modify(|mut n| n.width = percent(100));
    let words = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px(0),
                overflow: Overflow::clip_x(),
                row_gap: kit.m.px(4.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(strips::table_name(game)),
            crate::hud::tf_bold(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    let mut meta = Vec::new();
    if let Some(host) = &game.host {
        meta.push(host.clone());
    }
    if let Some(format) = model::table_format(game) {
        meta.push(shelf::format_label(lang, format));
    }
    meta.push(Phrase::RulesLife.fill(lang, &[&game.setup.starting_life.to_string()]));
    if let Some(clock) = game.clock {
        meta.push(model::table_clock_label(lang, &lobby.clocks(), clock));
    }
    commands.entity(words).add_child(title);
    if !phone {
        let meta = parts::line(commands, kit, &meta.join(" · "), kit.m.small, tokens::MUTED);
        commands.entity(words).add_child(meta);
    }
    commands.entity(top).add_child(words);
    // The counts: seats, the lock, the AI (wrapping on a phone, where they
    // share the name's column with Join beside it).
    let counts = parts::row(commands, kit, phone);
    if wide {
        commands
            .entity(counts)
            .entry::<Node>()
            .and_modify(move |mut n| {
                n.width = kit.m.px(160.0);
                n.flex_shrink = 0.0;
                n.flex_direction = FlexDirection::Column;
                n.align_items = AlignItems::Start;
                n.row_gap = kit.m.px(6.0);
            });
    }
    let taken = game
        .seats
        .iter()
        .filter(|s| s.taken || s.kind == SeatKind::Ai)
        .count();
    let state_word = if game.state == "playing" {
        // How long it has run, on every frame (a phone has no meta line).
        match game.running_secs {
            Some(secs) => format!(
                "{} \u{b7} {}",
                Phrase::PlayPlaying.text(lang),
                model::running_label(lang, secs)
            ),
            None => Phrase::PlayPlaying.text(lang).to_string(),
        }
    } else {
        Phrase::PlaySeats.fill(lang, &[&taken.to_string(), &game.seats.len().to_string()])
    };
    let seats = parts::badge(commands, kit, &state_word, tokens::INK);
    commands.entity(counts).add_child(seats);
    if game.locked {
        let lock = commands
            .spawn((
                Text::new(parts::LOCK.to_string()),
                crate::hud::icon_tf(kit.fonts, kit.m.small),
                TextColor(tokens::GOLD),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(counts).add_child(lock);
    }
    let ais = game.seats.iter().filter(|s| s.kind == SeatKind::Ai).count();
    if ais > 0 {
        let ai = parts::badge(
            commands,
            kit,
            &Phrase::PlayAiCount.fill(lang, &[&ais.to_string()]),
            tokens::MUTED,
        );
        commands.entity(counts).add_child(ai);
    }
    discovery::seat_marks(commands, counts, kit, game);
    // The one action that applies. "Return to game" is the compact exception
    // to the wide rows' ordinary 132-pixel action width.
    let mine = game.seated() || game.yours;
    let mut compact_action = None;
    if phone
        && !mine
        && model::format_warning(lang, mine_format, game).is_some()
        && let Some(theirs) = model::table_format(game)
    {
        let warn = parts::badge_with(
            commands,
            kit,
            parts::WARNING,
            &Phrase::RoomOtherFormat.fill(lang, &[&shelf::format_label(lang, theirs)]),
            tokens::GOLD,
        );
        commands.entity(counts).add_child(warn);
    }
    if wide {
        commands.entity(top).add_child(counts);
    } else if phone {
        commands.entity(words).add_child(counts);
    }
    if game.seated() && game.rematch && !game.i_am_ready() {
        // A rematch room keeps this player's chair reserved: it is
        // claimed by playing again, never by Ready.
        let again = controls::button(
            commands,
            kit,
            Phrase::PlayAgain.text(lang),
            Weight::Primary,
            if lobby.busy() {
                Live::No(Phrase::VeilTalking.text(lang))
            } else {
                Live::Yes
            },
            None,
            Press::Play(PlayPress::PlayAgain(index)),
        );
        orders::item(commands, again, &orders::PLAY, "tables", *walked);
        *walked += 1;
        commands.entity(top).add_child(again);
    } else if mine {
        let back = controls::button_compact(
            commands,
            kit,
            None,
            Phrase::ShellReturnToGame.text(lang),
            Weight::Gold,
            Live::Yes,
            Press::Play(PlayPress::Return),
        );
        compact_action = Some(back);
        orders::item(commands, back, &orders::PLAY, "tables", *walked);
        *walked += 1;
        commands.entity(top).add_child(back);
    } else if game.watchable() {
        let words = if game.spectators > 0 {
            format!(
                "{} \u{b7} {}",
                Phrase::Watch.text(lang),
                Phrase::SpectatorCount.fill(lang, &[&game.spectators.to_string()])
            )
        } else {
            Phrase::Watch.text(lang).to_string()
        };
        let watch = controls::button(
            commands,
            kit,
            &words,
            Weight::Secondary,
            if lobby.busy() {
                Live::No(Phrase::VeilTalking.text(lang))
            } else {
                Live::Yes
            },
            None,
            Press::Play(PlayPress::Watch(index)),
        );
        orders::item(commands, watch, &orders::PLAY, "tables", *walked);
        *walked += 1;
        commands.entity(top).add_child(watch);
    } else if game.joinable() {
        let join = controls::button(
            commands,
            kit,
            Phrase::Join.text(lang),
            Weight::Primary,
            if held {
                Live::No("")
            } else if lobby.busy() {
                Live::No(Phrase::VeilTalking.text(lang))
            } else {
                Live::Yes
            },
            None,
            Press::Play(PlayPress::Join(index)),
        );
        orders::item(commands, join, &orders::PLAY, "tables", *walked);
        *walked += 1;
        commands.entity(top).add_child(join);
    }
    if wide {
        let action_width = kit.m.px(132.0);
        let compact_padding = kit.m.px(6.0);
        commands.queue(move |world: &mut World| {
            if let Some(back) = compact_action {
                let face = world
                    .get::<Children>(back)
                    .and_then(|children| children.iter().next());
                if let Some(face) = face
                    && let Some(mut node) = world.get_mut::<Node>(face)
                {
                    node.padding = UiRect::axes(compact_padding, px_fixed(0.0));
                }
            }
            let actions: Vec<_> = world
                .get::<Children>(top)
                .map(|children| {
                    children
                        .iter()
                        .filter(|id| world.get::<Press>(*id).is_some())
                        .collect()
                })
                .unwrap_or_default();
            for action in actions {
                if Some(action) == compact_action {
                    continue;
                }
                if let Some(mut node) = world.get_mut::<Node>(action) {
                    node.width = action_width;
                    node.flex_shrink = 0.0;
                }
            }
        });
    }
    commands.entity(row).add_child(top);
    if !wide && !phone {
        // The second meta line (Narrow).
        commands.entity(row).add_child(counts);
    }
    if let Some(warning) =
        model::format_warning(lang, mine_format, game).filter(|_| !mine && !phone)
    {
        let line = parts::glyph_line(commands, kit, parts::WARNING, &warning, tokens::GOLD);
        commands.entity(row).add_child(line);
    }
    row
}
