//! Pool rows and their count badge.

#[allow(clippy::wildcard_imports)] // the builder module's shared vocabulary
use super::*;

/// Returns `(is_deck_fav, is_global_fav)` for `english_name`.
pub(crate) fn fav_flags(
    state: &LobbyState,
    prefs: Option<&baylee_client_core::prefs::Preferences>,
    english_name: &str,
) -> (bool, bool) {
    let deck_key = state.lobby.builder().editing().unwrap_or("new");
    let is_deck = prefs.is_some_and(|p| p.deck_card_favourite(deck_key, english_name));
    let is_global = prefs.is_some_and(|p| p.card_favourite(english_name));
    (is_deck, is_global)
}

/// Spawns a coloured rarity indicator dot (`Common` slate/silver, `Uncommon`
/// silver-blue, `Rare` gold, `Mythic` orange-red) with a hover tooltip.
pub(crate) fn rarity_dot(
    commands: &mut Commands,
    kit: Kit,
    rarity: baylee_client_core::deckbuilder::Rarity,
    lang: Lang,
) -> Entity {
    let m = kit.m;
    let color = crate::lobby::decks::rarity_dot_color(rarity);
    let label = rarity_label(rarity, lang == Lang::De);
    commands
        .spawn((
            Node {
                width: m.px(8.0),
                height: m.px(8.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(color),
            BorderColor::all(color.with_alpha(0.45)),
            crate::lobby::HoverHint(label.to_string()),
            Pickable::default(),
        ))
        .id()
}

/// One pool row placed at explicit vertical offset `top`.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)] // one row, read left to right
pub(crate) fn pool_row_at(
    commands: &mut Commands,
    kit: Kit,
    state: &LobbyState,
    prefs: Option<&baylee_client_core::prefs::Preferences>,
    slot: usize,
    at: usize,
    top: f32,
    pitch: f32,
) -> Option<Entity> {
    let deck = state.lobby.builder();
    let lang = state.lobby.lang();
    let card = deck.card(slot)?;
    let (is_deck_fav, is_global_fav) = fav_flags(state, prefs, &card.english_name);
    let m = kit.m;
    let gap = m.scaled(4.0);
    let height = pitch - gap;
    let hover = hover_of_card(card);
    let row = commands
        .spawn((
            Role::Row,
            frame(kit, top, height),
            BackgroundColor(ROW_GROUND),
            crate::ambience::Feel::tinting_to(ROW_GROUND, ROW_HOT),
            Press::Build(BuildPress::Inspect(slot)),
            hover.clone(),
            PoolRowAt(at),
        ))
        .id();
    if at == 0 {
        commands.entity(row).insert(crate::tour::TourAnchor(
            baylee_client_core::tour::Anchor::BuildFirstRow,
        ));
    }
    let thumb = print(
        commands,
        &hover,
        (height - m.scaled(4.0)).min(m.scaled(67.0)),
    );
    commands.entity(thumb).insert((
        Press::Build(BuildPress::PickPrint(slot)),
        Pickable::default(),
    ));

    let info = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                row_gap: m.px(2.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let r_dot = rarity_dot(commands, kit, deck.card_rarity(slot), lang);
    let name = cell(commands, kit, &card.name, m.text, tokens::INK, true);
    let mut top = vec![r_dot, name];
    if is_deck_fav {
        let star = commands
            .spawn((
                Text::new("\u{f005}"),
                icon_tf(kit.fonts, m.small * 0.82),
                TextColor(tokens::ACCENT),
                TextLayout::no_wrap(),
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let flag = commands
            .spawn((
                Text::new("Deck"),
                tf_bold(kit.fonts, m.small * 0.88),
                TextColor(tokens::ACCENT),
                TextLayout::no_wrap(),
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        top.extend([star, flag]);
    }
    if is_global_fav {
        let label = match lang {
            Lang::De => "Allg.",
            Lang::En => "Gen.",
        };
        let star = commands
            .spawn((
                Text::new("\u{f005}"),
                icon_tf(kit.fonts, m.small * 0.82),
                TextColor(tokens::GOLD),
                TextLayout::no_wrap(),
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let flag = commands
            .spawn((
                Text::new(label),
                tf_bold(kit.fonts, m.small * 0.88),
                TextColor(tokens::GOLD),
                TextLayout::no_wrap(),
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        top.extend([star, flag]);
    }
    if let Some((said, ink)) = coverage_mark(card.coverage) {
        let flag = commands
            .spawn((
                Text::new(said.text(lang)),
                tf_bold(kit.fonts, m.small * 0.9),
                TextColor(ink),
                TextLayout::no_wrap(),
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        top.push(flag);
    }
    // Blocked on this gateway: the deck that holds it will not sit down
    // here (`docs/protocol.md` §"Blocked cards"); the sheet says why.
    if deck.blocked(card.index).is_some() {
        let flag = commands
            .spawn((
                Text::new(Phrase::BuildBlocked.text(lang)),
                tf_bold(kit.fonts, m.small * 0.9),
                TextColor(tokens::DANGER),
                TextLayout::no_wrap(),
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        top.push(flag);
    }
    let first = line(commands, kit, &top);
    let second = cell(commands, kit, &subline(card), m.small, tokens::MUTED, false);
    commands.entity(info).add_children(&[first, second]);

    let mut kids = vec![thumb, info];
    if let Some(cost) =
        crate::manaui::spawn_cost_or_text(commands, kit.fonts, &card.mana_cost, m.small * 1.25)
    {
        // Keep the mana renderer's scaled gap between complete symbols.
        kids.push(cost);
    }
    kids.push(badge(commands, kit, slot, deck));
    kids.push(icon_button(
        commands,
        kit,
        '\u{f067}',
        false,
        Press::Build(BuildPress::AddFromPool(slot, false)),
    ));
    if card.commander && state.commander_pick.is_some() {
        let partner = state.commander_pick == Some(true);
        kids.push(controls::button(
            commands,
            kit,
            if partner {
                Phrase::BuildPartner.text(lang)
            } else {
                Phrase::BuildChoose.text(lang)
            },
            Weight::Primary,
            Live::Yes,
            None,
            if partner {
                Press::Build(BuildPress::AddPartner(slot))
            } else {
                Press::Build(BuildPress::SetCommander(slot))
            },
        ));
    }
    kids.push(icon_button(
        commands,
        kit,
        mark::MORE,
        state.build.menu == Some(BuildMenu::Pool(slot)),
        (
            Press::Build(BuildPress::PoolMenu(slot)),
            focus::MenuOpener(BuildMenu::Pool(slot)),
        ),
    ));
    commands.entity(row).add_children(&kids);
    Some(row)
}

/// The count badge: how many the deck holds, both lists together; hidden
/// at none.
fn badge(commands: &mut Commands, kit: Kit, slot: usize, deck: &DeckBuilder) -> Entity {
    let held = deck.count_of(slot, Zone::Main) + deck.count_of(slot, Zone::Side);
    let m = kit.m;
    let disc = commands
        .spawn((
            Node {
                min_width: px_fixed(m.scaled(22.0).max(m.small * 1.6)),
                min_height: px_fixed(m.scaled(22.0).max(m.small * 1.6)),
                padding: UiRect::axes(m.px(6.0), px_fixed(0.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                display: if held == 0 {
                    Display::None
                } else {
                    Display::Flex
                },
                ..default()
            },
            BackgroundColor(tokens::GOLD.with_alpha(0.22)),
            Pickable::IGNORE,
            InDeck(slot),
        ))
        .id();
    let n = commands
        .spawn((
            Text::new(held.to_string()),
            tf_bold(kit.fonts, m.small),
            TextColor(tokens::GOLD),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(disc).add_child(n);
    disc
}

/// High-density compact pool row: name, type/stats, mana cost, count badge, `★`, `+` and `⋯`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn pool_compact_row(
    commands: &mut Commands,
    kit: Kit,
    state: &LobbyState,
    prefs: Option<&baylee_client_core::prefs::Preferences>,
    slot: usize,
    at: usize,
    top: f32,
    pitch: f32,
) -> Option<Entity> {
    let deck = state.lobby.builder();
    let card = deck.card(slot)?;
    let (is_deck_fav, is_global_fav) = fav_flags(state, prefs, &card.english_name);
    let m = kit.m;
    let gap = m.scaled(3.0);
    let height = pitch - gap;
    let hover = hover_of_card(card);
    let row = commands
        .spawn((
            Role::Row,
            frame(kit, top, height),
            BackgroundColor(ROW_GROUND),
            crate::ambience::Feel::tinting_to(ROW_GROUND, ROW_HOT),
            Press::Build(BuildPress::Inspect(slot)),
            hover,
            PoolRowAt(at),
        ))
        .id();
    if at == 0 {
        commands.entity(row).insert(crate::tour::TourAnchor(
            baylee_client_core::tour::Anchor::BuildFirstRow,
        ));
    }
    let lang = state.lobby.lang();
    let r_dot = rarity_dot(commands, kit, deck.card_rarity(slot), lang);
    let name = cell(commands, kit, &card.name, m.text, tokens::INK, true);
    commands.entity(name).insert(Node {
        flex_grow: 1.0,
        flex_basis: px_fixed(0.0),
        min_width: px_fixed(0.0),
        overflow: Overflow::clip_x(),
        ..default()
    });
    let mut kids = vec![r_dot, name];
    if is_deck_fav || is_global_fav {
        let star = commands
            .spawn((
                Text::new("\u{f005}"),
                icon_tf(kit.fonts, m.small * 0.85),
                TextColor(if is_deck_fav {
                    tokens::ACCENT
                } else {
                    tokens::GOLD
                }),
                TextLayout::no_wrap(),
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        kids.push(star);
    }
    let sub = cell(
        commands,
        kit,
        &card.type_line,
        m.small * 0.92,
        tokens::MUTED,
        false,
    );
    commands.entity(sub).insert(Node {
        max_width: m.px(180.0),
        flex_shrink: 1.0,
        overflow: Overflow::clip_x(),
        ..default()
    });
    kids.push(sub);
    if let Some(cost) =
        crate::manaui::spawn_cost_or_text(commands, kit.fonts, &card.mana_cost, m.small * 1.15)
    {
        kids.push(cost);
    }
    kids.push(badge(commands, kit, slot, deck));
    kids.push(icon_button(
        commands,
        kit,
        '\u{f067}',
        false,
        Press::Build(BuildPress::AddFromPool(slot, false)),
    ));
    kids.push(icon_button(
        commands,
        kit,
        mark::MORE,
        state.build.menu == Some(BuildMenu::Pool(slot)),
        (
            Press::Build(BuildPress::PoolMenu(slot)),
            focus::MenuOpener(BuildMenu::Pool(slot)),
        ),
    ));
    commands.entity(row).add_children(&kids);
    Some(row)
}

/// Visual card-art grid row containing up to `cols` card tiles side by side.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub(crate) fn pool_grid_row(
    commands: &mut Commands,
    kit: Kit,
    state: &LobbyState,
    prefs: Option<&baylee_client_core::prefs::Preferences>,
    cells: &[Option<(usize, usize)>; 4],
    cols: usize,
    top: f32,
    pitch: f32,
) -> Entity {
    let deck = state.lobby.builder();
    let m = kit.m;
    let gap = m.scaled(8.0);
    let height = pitch - gap;
    let row = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px_fixed(top),
                left: px_fixed(0.0),
                width: Val::Percent(100.0),
                height: px_fixed(height),
                align_items: AlignItems::Stretch,
                column_gap: px_fixed(gap),
                padding: UiRect::axes(m.px(4.0), px_fixed(0.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let art_h = (height - m.scaled(52.0)).max(96.0).round();
    let art_w = (art_h * 5.0 / 7.0).round();
    for cell_item in cells.iter().take(cols) {
        let Some((slot, at)) = *cell_item else {
            let filler = commands
                .spawn((
                    Node {
                        flex_grow: 1.0,
                        flex_basis: px_fixed(0.0),
                        min_width: px_fixed(0.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(row).add_child(filler);
            continue;
        };
        let Some(card) = deck.card(slot) else {
            continue;
        };
        let hover = hover_of_card(card);
        let tile = commands
            .spawn((
                Role::Row,
                Node {
                    flex_grow: 1.0,
                    flex_basis: px_fixed(0.0),
                    min_width: px_fixed(0.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::all(m.px(8.0)),
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(8.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(ROW_GROUND),
                BorderColor::all(tokens::BORDER.with_alpha(0.45)),
                crate::ambience::Feel::tinting_to(ROW_GROUND, ROW_HOT),
                Press::Build(BuildPress::Inspect(slot)),
                hover.clone(),
                PoolRowAt(at),
            ))
            .id();
        if at == 0 {
            commands.entity(tile).insert(crate::tour::TourAnchor(
                baylee_client_core::tour::Anchor::BuildFirstRow,
            ));
        }
        let art_wrap = commands
            .spawn((
                Node {
                    width: px_fixed(art_w),
                    height: px_fixed(art_h),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let thumb = crate::lobby::thumbnails::spawn_sized(
            commands,
            &hover,
            px_fixed(art_w),
            px_fixed(art_h),
            px_fixed(6.0),
        );
        commands.entity(thumb).insert((
            Press::Build(BuildPress::PickPrint(slot)),
            Pickable::default(),
        ));
        let count_badge = badge(commands, kit, slot, deck);
        commands
            .entity(count_badge)
            .entry::<Node>()
            .and_modify(|mut n| {
                n.position_type = PositionType::Absolute;
                n.top = px_fixed(4.0);
                n.right = px_fixed(4.0);
            });
        commands
            .entity(art_wrap)
            .add_children(&[thumb, count_badge]);

        let bottom = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: m.px(4.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let meta = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_basis: px_fixed(0.0),
                    min_width: px_fixed(0.0),
                    row_gap: m.px(2.0),
                    overflow: Overflow::clip_x(),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let lang = state.lobby.lang();
        let (is_deck_fav, is_global_fav) = fav_flags(state, prefs, &card.english_name);
        let title_row = commands
            .spawn((
                Node {
                    align_items: AlignItems::Center,
                    column_gap: m.px(4.0),
                    min_width: px_fixed(0.0),
                    overflow: Overflow::clip_x(),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let r_dot = rarity_dot(commands, kit, deck.card_rarity(slot), lang);
        let title = cell(commands, kit, &card.name, m.small, tokens::INK, true);
        commands.entity(title_row).add_children(&[r_dot, title]);
        if is_deck_fav || is_global_fav {
            let star = commands
                .spawn((
                    Text::new("\u{f005}"),
                    icon_tf(kit.fonts, m.small * 0.82),
                    TextColor(if is_deck_fav {
                        tokens::ACCENT
                    } else {
                        tokens::GOLD
                    }),
                    TextLayout::no_wrap(),
                    Node {
                        flex_shrink: 0.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(title_row).add_child(star);
        }
        commands.entity(meta).add_child(title_row);
        if let Some(cost) =
            crate::manaui::spawn_cost_or_text(commands, kit.fonts, &card.mana_cost, m.small * 0.95)
        {
            commands.entity(meta).add_child(cost);
        }
        let add = icon_button(
            commands,
            kit,
            '\u{f067}',
            false,
            Press::Build(BuildPress::AddFromPool(slot, false)),
        );
        let more = icon_button(
            commands,
            kit,
            mark::MORE,
            state.build.menu == Some(BuildMenu::Pool(slot)),
            (
                Press::Build(BuildPress::PoolMenu(slot)),
                focus::MenuOpener(BuildMenu::Pool(slot)),
            ),
        );
        commands.entity(bottom).add_children(&[meta, add, more]);
        commands.entity(tile).add_children(&[art_wrap, bottom]);
        commands.entity(row).add_child(tile);
    }
    row
}
