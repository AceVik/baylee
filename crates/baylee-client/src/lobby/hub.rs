//! The signed-in hub: Play and Decks, side by side or stacked.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

/// The signed-in screen: decks and tables, side by side or stacked.
#[allow(clippy::too_many_lines)] // two panels and a bar, built in order
pub(super) fn table(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let phone = metrics.frame == Frame::Compact;

    if phone {
        commands
            .entity(root)
            .entry::<Node>()
            .and_modify(|mut n| n.overflow = Overflow::scroll_y());
        commands.entity(root).insert((
            Scrollable(List::Table),
            ScrollPosition(Vec2::new(0.0, scrolled_to.get(List::Table))),
        ));
    }

    // Use the full viewport for discovery and multiplayer configuration.
    let frame = commands
        .spawn((
            Node {
                width: percent(100),
                height: if phone { Val::Auto } else { percent(100) },
                min_height: if phone { percent(100) } else { px(0) },
                flex_shrink: 0.0,
                align_self: AlignSelf::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(frame);
    let root = frame;

    // ---- top bar
    let bar = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(metrics.tap + metrics.pad),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(metrics.gap),
                row_gap: px(6),
                flex_wrap: FlexWrap::Wrap,
                padding: UiRect::axes(px(metrics.pad), px(metrics.pad * 0.5)),
                ..default()
            },
            BackgroundColor(palette::SANCTUARY_PANEL),
        ))
        .id();
    let brand = commands
        .spawn((
            Text::new(Phrase::AppName.text(lang)),
            tf(fonts, metrics.head * 1.2),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(bar).add_child(brand);
    // The build, beside the name, on every screen including a phone.
    //
    // It survives the narrowing that drops the gateway address below,
    // because the two answer different questions: the address is reassurance
    // a player already has, and this is the only thing on screen that says
    // *which* baylee this is. It is what a bug report is worthless without,
    // and it is half of the AGPL offer the gateway answers in full at
    // `GET /source` — a version, so that "the source is over there" names a
    // particular source. Twenty characters at the smallest size the lobby
    // has, which is what lets it afford to be unconditional.
    let build = commands
        .spawn((
            Text::new(baylee_build::short()),
            tf(fonts, metrics.small * 0.9),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(bar).add_child(build);
    // The gateway address is reassurance, not information, and the first thing
    // a narrow screen can do without — and offline it is not even that: the
    // address is printed from settings and nothing has been dialled.
    if !phone && !lobby.offline() {
        let host = commands
            .spawn((
                Text::new(state.gateway.clone()),
                tf(fonts, metrics.small * 0.9),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(bar).add_child(host);
    }
    let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
    let status = commands
        .spawn((
            Text::new(lobby.status()),
            tf(fonts, metrics.small),
            TextColor(status_ink(lobby.tone())),
            Pickable::IGNORE,
        ))
        .id();
    let settings = button(
        commands,
        fonts,
        metrics,
        Phrase::Settings.text(lang),
        Press::FrontMenu,
        palette::PANEL_LIT,
        true,
    );
    let out = button(
        commands,
        fonts,
        metrics,
        Phrase::SignOut.text(lang),
        Press::SignOut,
        palette::PANEL_LIT,
        true,
    );
    commands.entity(bar).add_child(gap);
    commands.entity(bar).add_child(status);
    commands.entity(bar).add_child(settings);
    commands.entity(bar).add_child(out);
    commands.entity(root).add_child(bar);
    if state.front_menu {
        let menu = super::front::gear_menu(commands, state, fonts, metrics, metrics.pad);
        commands.entity(bar).add_child(menu);
    }

    if let Some(handover) = lobby.awaiting()
        && let Some(index) = lobby
            .games()
            .iter()
            .position(|g| g.id == handover.game_id && g.state == "waiting")
    {
        super::room::draw(commands, root, state, fonts, metrics, scrolled_to, index);
        return;
    }

    if let Some(handover) = lobby.awaiting() {
        let banner = commands
            .spawn((
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    flex_shrink: 0.0,
                    padding: UiRect::axes(px(metrics.pad), px(metrics.pad * 0.5)),
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                Pickable::IGNORE,
            ))
            .id();
        // Written as a phrase and not a `format!`, which is what it was:
        // a hand-typed English sentence renders a German screen half in
        // English, and it does it silently — the compile error that a
        // missing translation is arrives only for text that goes through
        // `Phrase`.
        let words = if lobby.offline() {
            Phrase::TableOpenHouseWaiting.text(lang).to_string()
        } else {
            Phrase::TableOpenWaiting.fill(lang, &[&short_id(&handover.game_id)])
        };
        let line = commands
            .spawn((
                Text::new(words),
                tf(fonts, metrics.small),
                TextColor(palette::ACTIVE),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(banner).add_child(line);
        commands.entity(root).add_child(banner);
    }

    // A guest is told what a guest is, for as long as it plays as one (#269):
    // the account and its decks go when its session lapses, about thirty
    // days after its last visit (the expiry slides on every request, not
    // only on a game), or at once when it signs out.
    if lobby.guest() {
        let banner = commands
            .spawn((
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    flex_shrink: 0.0,
                    padding: UiRect::axes(px(metrics.pad), px(metrics.pad * 0.5)),
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                Pickable::IGNORE,
            ))
            .id();
        let line = commands
            .spawn((
                Text::new(Phrase::GuestNotice.text(lang)),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(banner).add_child(line);
        commands.entity(root).add_child(banner);
    }

    let navigation = row(commands, metrics, true);
    commands.entity(navigation).insert(Node {
        width: percent(100),
        align_items: AlignItems::Center,
        flex_shrink: 0.0,
        padding: UiRect::axes(px(metrics.pad), px(6)),
        column_gap: px(metrics.gap),
        row_gap: px(metrics.gap * 0.5),
        flex_wrap: FlexWrap::Wrap,
        ..default()
    });
    for (hub, phrase) in [(Hub::Play, Phrase::HubPlay), (Hub::Decks, Phrase::HubDecks)] {
        let tab = chip(
            commands,
            fonts,
            metrics,
            phrase.text(lang),
            Press::Hub(hub),
            state.hub == hub,
        );
        commands.entity(navigation).add_child(tab);
    }
    let house = chip(
        commands,
        fonts,
        metrics,
        Phrase::HouseDecks.text(lang),
        Press::BrowseHouse,
        false,
    );
    let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
    let stats = note(
        commands,
        fonts,
        metrics,
        &Phrase::LobbyCounts.fill(
            lang,
            &[&lobby.decks().len().to_string(), &lobby.total().to_string()],
        ),
    );
    commands
        .entity(navigation)
        .add_children(&[house, gap, stats]);
    // What stops a game from starting stays in sight for as long as it
    // holds, not only until the status line says something else.
    let blocked = if lobby.unreachable() {
        Some(Phrase::GatewayUnreachable)
    } else if !lobby.games_can_start() {
        Some(Phrase::NoNewGames)
    } else {
        None
    };
    if let Some(blocked) = blocked {
        let warning = commands
            .spawn((
                Text::new(blocked.text(lang)),
                tf(fonts, metrics.small),
                TextColor(status_ink(Tone::Refusal)),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(navigation).add_child(warning);
    }
    if state.hub == Hub::Play {
        let create = button(
            commands,
            fonts,
            metrics,
            Phrase::CreateTable.text(lang),
            Press::OpenRoom(2),
            palette::ACCENT,
            !lobby.busy() && lobby.games_can_start(),
        );
        commands.entity(navigation).add_child(create);
    }
    commands.entity(root).add_child(navigation);
    // ---- body
    let body = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(0),
                flex_grow: 1.0,
                flex_direction: if metrics.stacked() {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
                column_gap: px(metrics.pad),
                row_gap: px(metrics.pad),
                padding: UiRect::all(px(metrics.pad)),
                // A phone runs out of height long before it runs out of
                // games; without this the list is simply cut off.
                overflow: if phone {
                    Overflow::visible()
                } else {
                    Overflow::scroll_y()
                },
                ..default()
            },
            Scrollable(List::Table),
            ScrollPosition(Vec2::new(0.0, scrolled_to.get(List::Table))),
        ))
        .id();
    commands.entity(root).add_child(body);
    if phone {
        commands
            .entity(body)
            .remove::<(Scrollable, ScrollPosition)>();
    }

    // ---- decks
    let decks = panel(
        commands,
        metrics,
        if state.hub == Hub::Decks {
            percent(100)
        } else {
            metrics.decks_width()
        },
        if state.hub == Hub::Decks { 1.0 } else { 0.0 },
    );
    commands.entity(decks).insert(super::dock::Dock(3));
    let decks_head = heading(
        commands,
        fonts,
        metrics,
        if state.hub == Hub::Play {
            Phrase::SelectedDeck
        } else {
            Phrase::YourDecks
        }
        .text(lang),
    );
    commands.entity(decks).add_child(decks_head);
    let deck_tools = row(commands, metrics, true);
    let new_deck = button(
        commands,
        fonts,
        metrics,
        Phrase::NewDeck.text(lang),
        Press::NewDeck,
        palette::ACCENT,
        true,
    );
    // A deck from elsewhere (a Moxfield export, a file of ours) opens the
    // builder with the import dialog in front of it.
    let import_deck = button(
        commands,
        fonts,
        metrics,
        Phrase::ImportDeck.text(lang),
        Press::ImportDeck,
        palette::PANEL_LIT,
        true,
    );
    commands
        .entity(deck_tools)
        .add_children(&[new_deck, import_deck]);
    if lobby.decks().is_empty() {
        commands.entity(deck_tools).despawn();
    } else {
        commands.entity(decks).add_child(deck_tools);
    }
    let deck_grid = row(commands, metrics, true);
    commands.entity(decks).add_child(deck_grid);
    if lobby.decks().is_empty() {
        let empty = super::empty::state(
            commands,
            fonts,
            metrics,
            Phrase::FirstDeck.text(lang),
            Phrase::NoOwnDecks.text(lang),
        );
        let action = button(
            commands,
            fonts,
            metrics,
            Phrase::HouseDecks.text(lang),
            Press::BrowseHouse,
            palette::ACCENT,
            !lobby.busy(),
        );
        let build = button(
            commands,
            fonts,
            metrics,
            Phrase::NewDeck.text(lang),
            Press::NewDeck,
            palette::PANEL_LIT,
            true,
        );
        let import = button(
            commands,
            fonts,
            metrics,
            Phrase::ImportDeck.text(lang),
            Press::ImportDeck,
            palette::PANEL_LIT,
            true,
        );
        commands
            .entity(empty)
            .add_children(&[action, build, import]);
        commands.entity(decks).add_child(empty);
    }
    for (index, deck) in lobby.decks().iter().enumerate() {
        if state.hub == Hub::Play && lobby.selected() != Some(index) {
            continue;
        }
        let row = commands
            .spawn((
                Node {
                    width: if state.hub == Hub::Decks && !phone {
                        percent(47)
                    } else {
                        percent(100)
                    },
                    flex_grow: 1.0,
                    min_width: px(0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(metrics.gap),
                    min_height: px(metrics.tap),
                    align_items: AlignItems::Stretch,
                    column_gap: px(metrics.gap),
                    padding: UiRect::all(px(metrics.pad)),
                    border: UiRect::all(px(1)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                BorderColor::all(if lobby.selected() == Some(index) {
                    palette::ACCENT
                } else {
                    Color::NONE
                }),
                Press::SelectDeck(index),
                crate::ambience::Feel::new(palette::PANEL_LIT),
            ))
            .id();
        let name = commands
            .spawn((
                Text::new(format!(
                    "{}{}",
                    deck.name,
                    if deck.commanders.is_empty() {
                        String::new()
                    } else {
                        format!("\n{}", deck.commanders.join(" / "))
                    }
                )),
                tf(fonts, metrics.text),
                TextColor(palette::INK),
                Pickable::IGNORE,
            ))
            .id();
        let size = commands
            .spawn((
                Text::new(
                    if (deck.copies > 0 || deck.cards == 0)
                        && (deck.side_copies > 0 || deck.sideboard == 0)
                    {
                        Phrase::LibraryCounts.fill(
                            lang,
                            &[&deck.copies.to_string(), &deck.side_copies.to_string()],
                        )
                    } else {
                        // Older gateways omit copy totals. Keep their row counts
                        // honest instead of relabelling them as cards.
                        Phrase::DeckRows.fill(
                            lang,
                            &[&deck.cards.to_string(), &deck.sideboard.to_string()],
                        )
                    },
                ),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        for child in [name, size] {
            commands.entity(row).add_child(child);
        }
        let actions = self::row(commands, metrics, true);
        commands.entity(row).add_child(actions);
        // Nested inside a row that is itself a `Press`: `in_lineage` takes the
        // nearest one, so these win over selecting the deck.
        for (label, press) in [
            (Phrase::Edit.text(lang), Press::EditDeck(index)),
            (Phrase::Delete.text(lang), Press::DeleteDeck(index)),
        ] {
            let tool = chip(commands, fonts, metrics, label, press, false);
            commands.entity(actions).add_child(tool);
        }
        if state.hub == Hub::Decks {
            let history = button(
                commands,
                fonts,
                metrics,
                Phrase::DeckHistory.text(lang),
                Press::DeckHistory(index),
                palette::PANEL_LIT,
                lobby.token().is_some() && !lobby.busy(),
            );
            commands.entity(actions).add_child(history);
        }
        commands.entity(deck_grid).add_child(row);
    }
    if state.hub == Hub::Play && !lobby.decks().is_empty() {
        let choose = button(
            commands,
            fonts,
            metrics,
            Phrase::ChooseDeck.text(lang),
            Press::Hub(Hub::Decks),
            palette::PANEL_LIT,
            true,
        );
        commands.entity(decks).add_child(choose);
    }
    commands.entity(body).add_child(decks);
    if state.hub == Hub::Decks {
        return;
    }

    // ---- tables
    let games = panel(commands, metrics, percent(100), 1.0);
    commands
        .entity(games)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.height = if metrics.stacked() {
                px(600)
            } else {
                percent(100)
            };
            n.min_height = px(0);
        });
    commands.entity(games).insert(super::dock::Dock(4));
    let head_row = commands
        .spawn((
            Node {
                width: percent(100),
                align_items: AlignItems::Center,
                column_gap: px(metrics.gap),
                row_gap: px(metrics.gap),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let head = heading(commands, fonts, metrics, Phrase::Tables.text(lang));
    commands.entity(head_row).add_child(head);
    commands.entity(games).add_child(head_row);
    let search_tools = row(commands, metrics, true);
    commands
        .entity(search_tools)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.align_items = AlignItems::FlexEnd;
            node.column_gap = px(metrics.gap);
            node.row_gap = px(metrics.gap);
        });
    // The search box sits with the buttons rather than over the list,
    // because on a phone the list is the screen and a bar above it is the
    // only place a control can be without pushing a table off the bottom.
    //
    // Offline there is nothing to search: the only table that can exist is
    // the one this process is holding, and it is already on the screen. The
    // same goes for the refresh beside it and the password below — see
    // [`Lobby::offline`].
    let alone = lobby.offline();
    if !alone {
        let hunt = commands
            .spawn((
                Node {
                    width: px(280),
                    max_width: percent(100),
                    flex_grow: 1.0,
                    min_width: px(0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let box_ = text_field(
            commands,
            fonts,
            metrics,
            Phrase::Search.text(lang),
            &FieldLook {
                buffer: lobby.buffer(Field::Search),
                focused: lobby.focus() == Field::Search,
                mask: None,
                press: Press::Focus(Field::Search),
                lead: Some(crate::hud::glyph::MAGNIFIER),
                hint: Some(Phrase::SearchTables.text(lang)),
                tail: None,
            },
        );
        commands.entity(hunt).add_child(box_);
        commands.entity(search_tools).add_child(hunt);
    }
    let mut controls = Vec::new();
    if !alone {
        controls.splice(
            ..0,
            [
                (
                    Phrase::DoSearch.text(lang),
                    Press::Search,
                    palette::PANEL_LIT,
                ),
                (
                    Phrase::Refresh.text(lang),
                    Press::Refresh,
                    palette::PANEL_LIT,
                ),
            ],
        );
    }
    for (label, press, tone) in controls {
        let b = button(commands, fonts, metrics, label, press, tone, !lobby.busy());
        commands.entity(search_tools).add_child(b);
    }
    if alone {
        commands.entity(search_tools).despawn();
    } else {
        commands.entity(games).add_child(search_tools);
    }
    if lobby.selected().is_some() && (!alone || !lobby.games().is_empty()) {
        let play = button(
            commands,
            fonts,
            metrics,
            Phrase::PlayTheHouse.text(lang),
            Press::Host(GameMode::Ai),
            palette::PANEL_LIT,
            !lobby.busy() && lobby.games_can_start(),
        );
        commands.entity(decks).add_child(play);
    }

    if lobby.games().is_empty() {
        // An empty lobby and an empty search are different news: one says
        // open a table, the other says the tables are elsewhere.
        let hunt = lobby.field(Field::Search).trim();
        let (title, said, action, press) = if !hunt.is_empty() {
            (
                Phrase::NoMatches,
                Phrase::NoTableMatches.fill(lang, &[hunt]),
                Phrase::ClearTableSearch,
                Press::ClearSearch,
            )
        } else if lobby.selected().is_none() {
            (
                Phrase::EmptyTablesTitle,
                Phrase::ChooseDeckToBegin.text(lang).to_string(),
                Phrase::HouseDecks,
                Press::BrowseHouse,
            )
        } else if alone {
            (
                Phrase::EmptyTablesTitle,
                Phrase::OfflineReadyToPlay.text(lang).to_string(),
                Phrase::PlayTheHouse,
                Press::Host(GameMode::Ai),
            )
        } else {
            (
                Phrase::EmptyTablesTitle,
                Phrase::NoTablesOpen.text(lang).to_string(),
                Phrase::CreateTable,
                Press::OpenRoom(2),
            )
        };
        let empty = super::empty::state(commands, fonts, metrics, title.text(lang), &said);
        // Online, the adjacent room form owns its submit. Offline, the empty
        // area owns the single direct house-game action.
        if alone || !hunt.is_empty() || lobby.selected().is_none() {
            let action = button(
                commands,
                fonts,
                metrics,
                action.text(lang),
                press,
                palette::ACCENT,
                !lobby.busy(),
            );
            commands.entity(empty).add_child(action);
        }
        commands.entity(games).add_child(empty);
    }
    let game_list = scroller(commands, metrics, List::Games, scrolled_to.get(List::Games));
    if lobby.games().is_empty() {
        commands.entity(game_list).despawn();
    } else {
        super::scrollbars::attach(commands, games, game_list, metrics);
    }
    for (index, game) in lobby.games().iter().enumerate() {
        let row = commands
            .spawn((
                Node {
                    width: percent(100),
                    min_height: px(metrics.tap),
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    column_gap: px(metrics.gap),
                    row_gap: px(6),
                    flex_wrap: FlexWrap::Wrap,
                    padding: UiRect::axes(px(metrics.pad * 0.7), px(metrics.pad * 0.4)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                Pickable::IGNORE,
            ))
            .id();
        // The headline: what the table is called, who opened it, how it is
        // going, and the one button that applies to the whole thing.
        let label = commands
            .spawn((
                Text::new(if game.name.trim().is_empty() {
                    short_id(&game.id)
                } else {
                    game.name.clone()
                }),
                tf(fonts, metrics.text),
                TextColor(palette::INK),
                Pickable::IGNORE,
            ))
            .id();
        let state = match game.state.as_str() {
            "waiting" => Phrase::StateWaiting,
            "playing" => Phrase::StatePlaying,
            _ => Phrase::StateOver,
        }
        .text(lang);
        let by = match &game.host {
            Some(host) => format!("{state}  ·  {host}  ·  {}", host_note(lang, game)),
            None => format!("{state}  ·  {}", host_note(lang, game)),
        };
        let seats = commands
            .spawn((
                Text::new(by),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
        commands.entity(row).add_child(label);
        commands.entity(row).add_child(seats);
        commands.entity(row).add_child(gap);
        if game.joinable() && !game.seated() {
            if game.locked {
                let password = text_field(
                    commands,
                    fonts,
                    metrics,
                    Phrase::RoomPassword.text(lang),
                    &FieldLook {
                        buffer: lobby.buffer(Field::RoomPassword),
                        focused: lobby.focus() == Field::RoomPassword,
                        mask: Some(Masked {
                            field: Some(Field::RoomPassword),
                            shown: lobby.showing(Field::RoomPassword),
                        }),
                        press: Press::Focus(Field::RoomPassword),
                        lead: None,
                        hint: None,
                        tail: None,
                    },
                );
                commands.entity(row).add_child(password);
            }
            let join = button(
                commands,
                fonts,
                metrics,
                Phrase::Join.text(lang),
                Press::Join(index),
                palette::ACCENT,
                !lobby.busy(),
            );
            commands.entity(row).add_child(join);
        }
        if game.seated() && game.rematch && !game.i_am_ready() {
            let b = button(
                commands,
                fonts,
                metrics,
                Phrase::PlayAgain.text(lang),
                Press::Rematch(index),
                palette::ACCENT,
                !lobby.busy(),
            );
            commands.entity(row).add_child(b);
        }
        commands.entity(game_list).add_child(row);
    }
    // The pager, and only when there is more than one page. A lobby with
    // four tables in it should not be asked to explain what page it is on.
    if lobby.total() > lobby.games().len() {
        let bar = commands
            .spawn((
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    column_gap: px(metrics.gap),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let first = lobby.offset() + 1;
        let last = lobby.offset() + lobby.games().len();
        let count = note(
            commands,
            fonts,
            metrics,
            &Phrase::PageOf.fill(
                lang,
                &[
                    &first.to_string(),
                    &last.to_string(),
                    &lobby.total().to_string(),
                ],
            ),
        );
        commands.entity(bar).add_child(count);
        for (label, forwards, live) in [
            (Phrase::PageBack.text(lang), false, lobby.offset() > 0),
            (Phrase::PageMore.text(lang), true, lobby.more()),
        ] {
            let b = button(
                commands,
                fonts,
                metrics,
                label,
                Press::Page(forwards),
                palette::PANEL_LIT,
                live && !lobby.busy(),
            );
            commands.entity(bar).add_child(b);
        }
        commands.entity(games).add_child(bar);
    }
    commands.entity(body).add_child(games);
}

/// How a table reads under its name: how full it is, and what it waits for.
///
/// Seated and ready are counted separately, because since a player has to say
/// they are ready the two answer different questions — a full table can still
/// be waiting for everyone in it.
pub(super) fn host_note(lang: Lang, game: &GameSummary) -> String {
    let total = game.seats.len();
    if game.state != "waiting" {
        return Phrase::SeatCount.fill(lang, &[&total.to_string()]);
    }
    let seated = game
        .seats
        .iter()
        .filter(|s| s.taken || s.kind == SeatKind::Ai)
        .count();
    let waiting = game.seats.iter().filter(|s| !s.ready).count();
    let mut note = Phrase::Seated.fill(lang, &[&seated.to_string(), &total.to_string()]);
    note.push_str(" · ");
    if waiting == 0 {
        note.push_str(Phrase::AllReady.text(lang));
    } else {
        note.push_str(&Phrase::WaitingFor.fill(lang, &[&waiting.to_string()]));
    }
    if game.locked {
        note.push_str(" · ");
        note.push_str(Phrase::Locked.text(lang));
    }
    note
}

/// A house AI's difficulty, in the player's own language.
///
/// The name itself stays the gateway's word — it is what `Press::SeatAi`
/// sends and what `SeatSpec` stores; only the label is translated. The
/// lookup is [`baylee_client_core::i18n::ai_name`] rather than a `match`
/// here, because the *table* needs the same answer this list gives and did
/// not have it: a chair arranged here as "Solide" sat down called
/// `steady 1`.
///
/// An unknown spelling keeps this list's own long-standing answer — the
/// middle difficulty — because a row in a lobby always draws something and
/// the caller above already defaults a missing value to `"steady"`.
pub(super) fn ai_name(lang: Lang, name: &str) -> &'static str {
    baylee_client_core::i18n::ai_name(lang, name).unwrap_or_else(|| Phrase::AiSteady.text(lang))
}

/// The head of an opaque game id — enough to tell two tables apart, and short
/// enough to fit on a phone.
fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}
