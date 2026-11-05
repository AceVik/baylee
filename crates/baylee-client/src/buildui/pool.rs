//! The pool side (§7): the search with its `?` syntax help and the filter
//! builder's gear, **one** chip row with the Filters disclosure, the count
//! line, and the list of compact rows; on a phone the search, *Filters · n*
//! and the count line.

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;
use crate::lobby::{FieldLook, FieldTail, List};
use baylee_client_core::deckbuilder::BuildField;

/// What the syntax popover offers (`docs/client.md` §"What the search box
/// understands"): an example a press inserts, and what it means.
pub(crate) const SYNTAX: [(&str, Phrase); 17] = [
    ("t:creature", Phrase::BuildSyntaxType),
    ("o:draw", Phrase::BuildSyntaxText),
    ("c:rg", Phrase::BuildSyntaxColour),
    ("id<=wu", Phrase::BuildSyntaxIdentity),
    ("mv<=3", Phrase::BuildSyntaxMana),
    ("pow>=4", Phrase::BuildSyntaxPower),
    ("m:{G}{G}", Phrase::BuildSyntaxCost),
    ("is:commander", Phrase::BuildSyntaxCommander),
    ("is:partial", Phrase::BuildSyntaxPartial),
    ("-t:land", Phrase::BuildSyntaxNot),
    ("!\"Llanowar Elves\"", Phrase::BuildSyntaxExact),
    ("t:elf or t:goblin", Phrase::BuildSyntaxOr),
    ("r:rare", Phrase::BuildSyntaxRarity),
    ("s:m21", Phrase::BuildSyntaxSet),
    ("f:modern", Phrase::BuildSyntaxFormat),
    ("banned:commander", Phrase::BuildSyntaxBanned),
    ("list:\"My Commander\"", Phrase::BuildSyntaxList),
];

/// Whether the search box has the caret (and so the keyboard's letters).
pub(crate) fn search_has_caret(env: &Env) -> bool {
    let deck = env.deck();
    env.ui().nav == Nav::Field
        && deck.focus() == BuildField::Search
        && deck.pool_panel().is_none()
        && deck.picker().is_none()
}

/// Whether the pool's filters are read on a phone (search, *Filters · n*,
/// the count line), not on the chip row.
fn compact(env: &Env) -> bool {
    matches!(env.layout, Layout::Rail | Layout::PhoneSingle)
}

/// The pool panel and its holders.
pub(super) fn panel(commands: &mut Commands, env: &Env, holders: &mut retained::Holders) -> Entity {
    let kit = env.kit;
    let grow = if env.layout == Layout::Columns {
        3.0
    } else {
        1.0
    };
    let panel = super::panel(commands, kit, grow, None);
    commands.entity(panel).insert(crate::lobby::dock::Dock(6));
    holders.toolbar = Some(holder(
        commands,
        panel,
        Node {
            width: Val::Percent(100.0),
            min_height: if env.layout == Layout::Columns {
                kit.m.px(95.0)
            } else {
                px_fixed(0.0)
            },
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            row_gap: kit.m.px(8.0),
            flex_shrink: 0.0,
            ..default()
        },
    ));
    holders.pool_list = Some(holder(
        commands,
        panel,
        Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            flex_basis: px_fixed(0.0),
            min_height: px_fixed(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: px_fixed(kit.m.gap),
            ..default()
        },
    ));
    holders.pool_over = Some(holder(
        commands,
        panel,
        Node {
            position_type: PositionType::Absolute,
            left: px_fixed(0.0),
            top: px_fixed(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
    ));
    panel
}

/// The footer of key hints: `↑↓ move · ↵ add · ⇧↵ other list · / search ·
/// Space card`.
fn hints(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let lang = env.lang();
    let row = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(6.0),
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                overflow: Overflow::clip_x(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (keys, what) in [
        ("\u{2191}\u{2193}", Phrase::BuildHintMove),
        ("Enter", Phrase::BuildHintAdd),
        ("Shift+Enter", Phrase::BuildHintOther),
        ("/", Phrase::BuildHintSearch),
        ("Space", Phrase::BuildHintPreview),
    ] {
        if let Some(cap) = controls::key_cap(commands, kit, keys) {
            commands.entity(cap).insert(Node {
                padding: UiRect::axes(kit.m.px(5.0), px_fixed(1.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(4.0)),
                ..default()
            });
            commands.entity(row).add_child(cap);
        }
        let said = words(commands, kit, what.text(lang), kit.m.small, tokens::MUTED);
        commands.entity(row).add_child(said);
    }
    row
}

/// The toolbar: the search and the chip row.
#[allow(clippy::too_many_lines)] // the pool's controls, read left to right, top to bottom
pub(super) fn toolbar(commands: &mut Commands, holder: Entity, env: &Env) {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let ui = env.ui();

    // ---- the search box, its syntax help, the filter builder's gear
    let search = crate::lobby::text_field(
        commands,
        kit.fonts,
        env.lobby_metrics(),
        "",
        &FieldLook {
            buffer: deck.buffer(BuildField::Search),
            focused: search_has_caret(env),
            mask: None,
            press: Press::Build(BuildPress::FocusBuild(BuildField::Search)),
            lead: Some(crate::hud::glyph::MAGNIFIER),
            hint: Some(Phrase::SearchCards.text(lang)),
            tail: Some(FieldTail {
                glyph: '\u{f1de}',
                press: Press::Build(BuildPress::ToggleSyntax),
                lit: ui.syntax,
            }),
        },
    );
    commands.entity(search).insert((
        Node {
            flex_grow: 1.0,
            flex_basis: px_fixed(0.0),
            min_width: px_fixed(0.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        stop("search"),
        crate::tour::TourAnchor(baylee_client_core::tour::Anchor::BuildSearch),
    ));
    let mut first = vec![search];
    if compact(env) {
        first.push(filters_button(commands, env));
    }
    first.push(icon_button(
        commands,
        kit,
        crate::hud::glyph::GEAR,
        deck.pool_panel().is_some(),
        (Press::Build(BuildPress::ToggleFilterPanel), stop("gear")),
    ));
    let top = line(commands, kit, &first);
    commands.entity(holder).add_child(top);

    // The filter builder the gear opens: what the box holds, taken apart.
    if let Some(built) = deck.pool_panel() {
        let also = chips_in_words(deck, lang);
        let rows = crate::filterui::build(
            commands,
            kit.fonts,
            built,
            deck.surface(),
            also.as_deref(),
            lang,
            crate::filterui::Register::LOBBY,
        );
        commands.entity(holder).add_child(rows);
    }

    // ---- the one chip row (not on a phone, where Filters · n holds it)
    if !compact(env) {
        let row = chip_row(commands, env);
        commands.entity(holder).add_child(row);
    }

    // ---- choosing a commander narrows the list to who may lead
    if let Some(partner) = env.state.commander_pick {
        let hint = words(
            commands,
            kit,
            if partner {
                Phrase::PartnerHint
            } else {
                Phrase::CommanderHint
            }
            .text(lang),
            m.small,
            tokens::MUTED,
        );
        commands.entity(hint).insert(Node {
            flex_shrink: 1.0,
            min_width: px_fixed(0.0),
            ..default()
        });
        commands.entity(hint).remove::<TextLayout>();
        let done = link(
            commands,
            kit,
            Phrase::DoneChoosing.text(lang),
            Press::Build(BuildPress::CancelCommanderPick),
        );
        let row = line(commands, kit, &[hint, done]);
        commands.entity(holder).add_child(row);
    }
}

/// The results the list shows: all of them, or while a commander is being
/// chosen the ones who may lead, filtered by active card-favourite filters.
pub(crate) fn shown(env: &Env) -> Vec<usize> {
    let deck = env.deck();
    let ui = env.ui();
    let deck_key = deck.editing().unwrap_or("new");
    let fav_filter = ui.fav_deck || ui.fav_global;
    deck.results()
        .iter()
        .copied()
        .filter(|slot| match env.state.commander_pick {
            Some(true) => deck.can_partner(*slot),
            Some(false) => deck.card(*slot).is_some_and(|card| card.commander),
            None => true,
        })
        .filter(|slot| {
            if !fav_filter {
                return true;
            }
            let Some(card) = deck.card(*slot) else {
                return false;
            };
            (ui.fav_deck && env.prefs.deck_card_favourite(deck_key, &card.english_name))
                || (ui.fav_global && env.prefs.card_favourite(&card.english_name))
        })
        .collect()
}

/// The chip row: colours · the active chips · Filters · Favourites · Playable only ·
/// Sort · Clear.
#[allow(clippy::too_many_lines)] // filter chips, grouping toggle, sort, and view switcher in one row
fn chip_row(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                column_gap: m.px(6.0),
                row_gap: m.px(6.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let colour_discs = colour_toggles(commands, env);
    let colour_group = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: m.px(2.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(colour_group).add_children(&colour_discs);
    let mut kids = vec![colour_group];
    if let Some(kind) = deck.kind() {
        let name = KINDS
            .iter()
            .find(|(k, _)| *k == kind)
            .map_or(kind, |(_, p)| p.text(lang));
        kids.push(controls::chip(
            commands,
            kit,
            name,
            true,
            None,
            true,
            (
                Press::Build(BuildPress::SetKind(None)),
                Stop::item(BUILDER, "chips", 0),
            ),
        ));
    }
    if let Some(cmc) = deck.cmc() {
        kids.push(controls::chip(
            commands,
            kit,
            &mana_value_words(cmc, lang),
            true,
            None,
            true,
            (
                Press::Build(BuildPress::ClearCmc),
                Stop::item(BUILDER, "chips", 1),
            ),
        ));
    }
    if let Some(rarity) = deck.rarity() {
        kids.push(controls::chip(
            commands,
            kit,
            rarity_label(rarity, lang),
            true,
            None,
            true,
            (
                Press::Build(BuildPress::SetRarity(None)),
                Stop::item(BUILDER, "chips", 2),
            ),
        ));
    }
    kids.push(filters_button(commands, env));
    let deck_key = deck.editing().unwrap_or("new");
    let deck_fav_count =
        u32::try_from(env.prefs.deck_card_favourites_for(deck_key).len()).unwrap_or(0);
    let global_fav_count = u32::try_from(env.prefs.card_favourites.len()).unwrap_or(0);
    let fav_deck_tip = Phrase::BuildFavDeckTip.text(lang);
    kids.push(controls::chip(
        commands,
        kit,
        "\u{2605} Deck",
        env.ui().fav_deck,
        (deck_fav_count > 0).then_some(deck_fav_count),
        false,
        (
            Press::Build(BuildPress::ToggleFilterFavDeck),
            stop("fav-deck"),
            crate::lobby::HoverHint(fav_deck_tip.to_string()),
        ),
    ));
    let fav_global_label = Phrase::BuildFavGlobalChip.text(lang);
    let fav_global_tip = Phrase::BuildFavGlobalTip.text(lang);
    kids.push(controls::chip(
        commands,
        kit,
        fav_global_label,
        env.ui().fav_global,
        (global_fav_count > 0).then_some(global_fav_count),
        false,
        (
            Press::Build(BuildPress::ToggleFilterFavGlobal),
            stop("fav-global"),
            crate::lobby::HoverHint(fav_global_tip.to_string()),
        ),
    ));
    kids.push(spring(commands));
    kids.push(controls::chip(
        commands,
        kit,
        Phrase::PlayableOnly.text(lang),
        deck.playable_only(),
        None,
        false,
        (Press::Build(BuildPress::TogglePlayable), stop("playable")),
    ));
    let group_label = Phrase::BuildGroupByType.text(lang);
    kids.push(controls::chip(
        commands,
        kit,
        group_label,
        env.ui().pool_grouped,
        None,
        false,
        (
            Press::Build(BuildPress::TogglePoolGrouped),
            stop("pool-group"),
        ),
    ));
    kids.push(sort_controls(commands, env));
    let views = [PoolView::List, PoolView::Grid, PoolView::Compact];
    let selected_view = views
        .iter()
        .position(|&v| v == env.ui().pool_view)
        .unwrap_or(0);
    kids.push(controls::segmented_icons(
        commands,
        kit,
        &[
            (
                crate::hud::glyph::VIEW_ROWS,
                Phrase::BuildViewList.text(lang),
            ),
            (
                crate::hud::glyph::VIEW_GRID,
                Phrase::BuildViewGrid.text(lang),
            ),
            (
                crate::hud::glyph::VIEW_BIG,
                Phrase::BuildViewCompact.text(lang),
            ),
        ],
        selected_view,
        |i| Press::Build(BuildPress::SetPoolView(views[i])),
    ));
    if deck.filtered() || env.ui().fav_deck || env.ui().fav_global {
        kids.push(link(
            commands,
            kit,
            Phrase::ClearFilters.text(lang),
            (Press::Build(BuildPress::ClearFilters), stop("clear")),
        ));
    }
    commands.entity(row).add_children(&kids);
    row
}

/// "Mana value 3", "Mana value 7+".
fn mana_value_words(cmc: u32, lang: Lang) -> String {
    if cmc as usize + 1 >= CURVE_BUCKETS {
        Phrase::BuildManaValueUp.fill(lang, &[&cmc.to_string()])
    } else {
        Phrase::BuildManaValue.fill(lang, &[&cmc.to_string()])
    }
}

/// The six colour discs of the identity filter (W U B R G C), letters in
/// the discs so colour is never the only carrier (§2.4).
fn colour_toggles(commands: &mut Commands, env: &Env) -> Vec<Entity> {
    let kit = env.kit;
    let m = kit.m;
    let deck = env.deck();
    let side = m.scaled(26.0).max(20.0);
    COLORS
        .iter()
        .enumerate()
        .map(|(i, (letter, _))| {
            let on = deck.colors().contains(letter);
            let disc = commands
                .spawn((
                    Role::Toggle,
                    Node {
                        width: px_fixed(side),
                        height: px_fixed(side),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(px_fixed(if on { 2.0 } else { 1.0 })),
                        border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                        ..default()
                    },
                    BackgroundColor(if on {
                        mana_tone(*letter)
                    } else {
                        tokens::CONTROL
                    }),
                    BorderColor::all(if on { tokens::ACCENT } else { tokens::BORDER }),
                ))
                .id();
            let letter_ink = if on {
                crate::shellkit::tokens::INK_ON_LIGHT
            } else {
                tokens::MUTED
            };
            let said = crate::manaui::spawn_mark_on(commands, kit.fonts, *letter, side, letter_ink);
            commands.entity(disc).add_child(said);
            controls::hit(
                commands,
                kit,
                disc,
                (
                    Press::Build(BuildPress::ToggleColor(*letter)),
                    Stop::item(BUILDER, "colours", u8::try_from(i).unwrap_or(0)),
                ),
            )
        })
        .collect()
}

/// *Filters* with a caret and the count of rail filters in force; on a
/// phone it opens the filters as a sheet.
fn filters_button(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let deck = env.deck();
    let active = u32::from(deck.kind().is_some())
        + u32::from(deck.cmc().is_some())
        + u32::from(deck.rarity().is_some())
        + u32::from(env.ui().fav_deck)
        + u32::from(env.ui().fav_global);
    controls::chip(
        commands,
        kit,
        Phrase::BuildFilters.text(env.lang()),
        env.ui().rail,
        (active > 0).then_some(active),
        false,
        (Press::Build(BuildPress::ToggleRail), stop("filters")),
    )
}

/// The sort dropdown (criterion) and its adjacent sort-direction toggle button (`Asc` / `Desc` / `Unset`).
#[allow(clippy::too_many_lines)] // dropdown pill + direction icon button + test carriers
fn sort_controls(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let menu_open = env.ui().menu == Some(BuildMenu::Sort);
    let height = if m.touch() { 40.0 } else { m.scaled(32.0) };

    let sort_glyph = deck.sort().icon();
    let face = commands
        .spawn((
            Role::Chip,
            Node {
                min_height: px_fixed(height),
                padding: UiRect::axes(m.px(10.0), px_fixed(0.0)),
                column_gap: m.px(6.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(if menu_open {
                tokens::SELECTED
            } else {
                tokens::CONTROL
            }),
            BorderColor::all(if menu_open {
                tokens::ACCENT
            } else {
                tokens::BORDER
            }),
        ))
        .id();
    let sign = glyph(commands, kit, sort_glyph, m.small * 0.9, tokens::ACCENT);
    let said = words(
        commands,
        kit,
        deck.sort().label().text(lang),
        m.small,
        tokens::INK,
    );
    let caret = glyph(commands, kit, mark::OPEN, m.small * 0.78, tokens::MUTED);
    commands.entity(face).add_children(&[sign, said, caret]);
    let dropdown = controls::hit(
        commands,
        kit,
        face,
        (
            Press::Build(BuildPress::ToggleSortMenu),
            stop("sort-menu"),
            focus::MenuOpener(BuildMenu::Sort),
            crate::lobby::HoverHint(Phrase::BuildSortCriterionTip.text(lang).to_string()),
        ),
    );

    let (dir_glyph, dir_tip, dir_lit) = match deck.sort_dir() {
        baylee_client_core::deckbuilder::SortDir::Asc => {
            ('\u{f160}', Phrase::BuildSortDirAscTip.text(lang), true)
        }
        baylee_client_core::deckbuilder::SortDir::Desc => {
            ('\u{f161}', Phrase::BuildSortDirDescTip.text(lang), true)
        }
        baylee_client_core::deckbuilder::SortDir::Unset => {
            ('\u{f0dc}', Phrase::BuildSortDirUnsetTip.text(lang), false)
        }
    };
    let dir_btn = icon_button(
        commands,
        kit,
        dir_glyph,
        dir_lit,
        (
            Press::Build(BuildPress::CycleSort),
            stop("sort"),
            crate::lobby::HoverHint(dir_tip.to_string()),
        ),
    );

    let group = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: m.px(3.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(group).add_children(&[dropdown, dir_btn]);
    group
}

/// The list: skeleton rows while the pool is on its way, then the rows,
/// and the footer bar with key hints on the left and `"X von Y Karten"` on the right.
pub(super) fn list(commands: &mut Commands, holder: Entity, env: &Env) {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let list_box = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_height: px_fixed(0.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(holder).add_child(list_box);

    let results = shown(env);
    let shown_count = results.len();
    if !deck.loaded() {
        let rows = crate::shellkit::states::skeleton(commands, kit, 6);
        commands.entity(list_box).add_child(rows);
    } else if results.is_empty() {
        let said = crate::shellkit::states::empty(
            commands,
            kit,
            crate::hud::glyph::MAGNIFIER,
            // Nothing matched because nothing could be asked, which is a
            // different thing to be told.
            deck.search_lacking()
                .unwrap_or(Phrase::NothingMatches)
                .text(lang),
            None,
        );
        commands.entity(list_box).add_child(said);
    } else {
        let content = virtual_rows::pool(commands, env, results);
        super::mount_virtual_list(commands, list_box, env, List::Pool, "pool", content);
    }

    let foot = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: m.px(38.0),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: m.px(10.0),
                padding: UiRect::axes(m.px(12.0), px_fixed(0.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.45)),
            BorderColor::all(tokens::BORDER.with_alpha(0.65)),
            Pickable::IGNORE,
        ))
        .id();
    if env.layout == Layout::Columns && !kit.m.touch() {
        let hint_row = hints(commands, env);
        commands.entity(foot).add_child(hint_row);
    }
    let gap = spring(commands);
    commands.entity(foot).add_child(gap);
    let tally = if deck.loaded() {
        Phrase::PoolTally.fill(
            lang,
            &[&shown_count.to_string(), &deck.pool().len().to_string(), ""],
        )
    } else {
        Phrase::LoadingPool.text(lang).to_string()
    };
    let count = cell(commands, kit, &tally, m.small, tokens::INK, true);
    commands.entity(count).insert((
        Node {
            flex_shrink: 0.0,
            ..default()
        },
        crate::tour::TourAnchor(baylee_client_core::tour::Anchor::BuildCount),
    ));
    commands.entity(foot).add_child(count);
    commands.entity(holder).add_child(foot);
}

/// The pool's overlays: the Filters rail over its left edge, the syntax
/// popover under the search.
pub(super) fn over(commands: &mut Commands, holder: Entity, env: &Env) {
    let kit = env.kit;
    let m = kit.m;
    let ui = env.ui();
    let rail_open = ui.rail && !compact(env);
    if rail_open || ui.syntax {
        let dismiss = if rail_open {
            Press::Build(BuildPress::ToggleRail)
        } else {
            Press::Build(BuildPress::ToggleSyntax)
        };
        let veil = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px_fixed(0.0),
                    right: px_fixed(0.0),
                    top: px_fixed(0.0),
                    bottom: px_fixed(0.0),
                    ..default()
                },
                GlobalZIndex(tokens::z::POPOVER - 1),
                dismiss,
            ))
            .id();
        commands.entity(holder).add_child(veil);
    }
    if rail_open {
        let rail = commands
            .spawn((
                Role::Opaque,
                Node {
                    position_type: PositionType::Absolute,
                    left: px_fixed(0.0),
                    top: px_fixed(0.0),
                    bottom: px_fixed(0.0),
                    width: m.px(320.0),
                    max_width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px_fixed(m.gap),
                    padding: UiRect::all(px_fixed(m.pad)),
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                BackgroundColor(tokens::OPAQUE),
                BorderColor::all(tokens::BORDER),
                GlobalZIndex(tokens::z::POPOVER),
                Press::Build(BuildPress::TransferNothing),
            ))
            .id();
        let head = rail_head(commands, env);
        commands.entity(rail).add_child(head);
        let body = filters_body(commands, env);
        commands.entity(rail).add_children(&body);
        commands.entity(holder).add_child(rail);
    }
    if ui.syntax {
        let pop = syntax(commands, env);
        commands.entity(pop).insert((
            Node {
                position_type: PositionType::Absolute,
                top: px_fixed(m.pad + m.control + m.scaled(6.0)),
                right: px_fixed(m.pad),
                max_width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: m.px(2.0),
                padding: UiRect::all(px_fixed(m.pad)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            Press::Build(BuildPress::TransferNothing),
        ));
        commands.entity(holder).add_child(pop);
    }
}

/// The rail's head: its name and its close.
pub(crate) fn rail_head(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let title = words(
        commands,
        kit,
        Phrase::BuildFilters.text(env.lang()),
        kit.m.head,
        tokens::INK,
    );
    let gap = spring(commands);
    let close = icon_button(
        commands,
        kit,
        mark::CLOSE,
        false,
        (Press::Build(BuildPress::ToggleRail), stop("rail")),
    );
    line(commands, kit, &[title, gap, close])
}

/// The rail's sections: colour, card type, mana value, what to show, and
/// Clear. The pool carries no rarity, set or keyword facts, so the rail
/// offers none (principle 5; they are round 2, §15).
#[allow(clippy::too_many_lines)] // filter sections: colour, type, mana value, playable/favourites, clear
pub(crate) fn filters_body(commands: &mut Commands, env: &Env) -> Vec<Entity> {
    let kit = env.kit;
    let lang = env.lang();
    let deck = env.deck();
    let mut out = Vec::new();
    let wrap = |commands: &mut Commands, kids: &[Entity]| {
        let row = commands
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
        commands.entity(row).add_children(kids);
        row
    };
    if compact(env) {
        out.push(caption(commands, kit, Phrase::FilterKeyIdentity.text(lang)));
        let discs = colour_toggles(commands, env);
        out.push(wrap(commands, &discs));
    }
    out.push(caption(commands, kit, Phrase::BuildRailType.text(lang)));
    let kinds: Vec<Entity> = KINDS
        .iter()
        .map(|(kind, name)| {
            let on = deck.kind() == Some(*kind);
            controls::chip(
                commands,
                kit,
                name.text(lang),
                on,
                None,
                false,
                Press::Build(BuildPress::SetKind(Some(*kind))),
            )
        })
        .collect();
    out.push(wrap(commands, &kinds));
    out.push(caption(commands, kit, Phrase::BuildRailMana.text(lang)));
    let values: Vec<Entity> = (0..u32::try_from(CURVE_BUCKETS).unwrap_or(8))
        .map(|cmc| {
            let last = cmc as usize + 1 == CURVE_BUCKETS;
            let label = if last {
                format!("{cmc}+")
            } else {
                cmc.to_string()
            };
            controls::chip(
                commands,
                kit,
                &label,
                deck.cmc() == Some(cmc),
                None,
                false,
                Press::Build(BuildPress::SetCmc(cmc)),
            )
        })
        .collect();
    out.push(wrap(commands, &values));
    out.push(caption(commands, kit, Phrase::FilterKeyRarity.text(lang)));
    let rarities: Vec<Entity> = [
        baylee_client_core::deckbuilder::Rarity::Common,
        baylee_client_core::deckbuilder::Rarity::Uncommon,
        baylee_client_core::deckbuilder::Rarity::Rare,
        baylee_client_core::deckbuilder::Rarity::Mythic,
    ]
    .into_iter()
    .map(|rarity| {
        controls::chip(
            commands,
            kit,
            rarity_label(rarity, lang),
            deck.rarity() == Some(rarity),
            None,
            false,
            Press::Build(BuildPress::SetRarity(Some(rarity))),
        )
    })
    .collect();
    out.push(wrap(commands, &rarities));
    out.push(caption(commands, kit, Phrase::BuildRailShow.text(lang)));
    let playable = controls::chip(
        commands,
        kit,
        Phrase::PlayableOnly.text(lang),
        deck.playable_only(),
        None,
        false,
        Press::Build(BuildPress::TogglePlayable),
    );
    let fav_deck = controls::chip(
        commands,
        kit,
        "\u{2605} Deck",
        env.ui().fav_deck,
        None,
        false,
        Press::Build(BuildPress::ToggleFilterFavDeck),
    );
    let fav_global = controls::chip(
        commands,
        kit,
        Phrase::BuildFavGlobalChip.text(lang),
        env.ui().fav_global,
        None,
        false,
        Press::Build(BuildPress::ToggleFilterFavGlobal),
    );
    out.push(wrap(commands, &[playable, fav_deck, fav_global]));
    if deck.filtered() || env.ui().fav_deck || env.ui().fav_global {
        let clear = link(
            commands,
            kit,
            Phrase::ClearFilters.text(lang),
            Press::Build(BuildPress::ClearFilters),
        );
        out.push(clear);
    }
    out
}

/// The syntax popover: each example, what it means; a press inserts it.
fn syntax(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let pop = commands
        .spawn((
            Role::Opaque,
            Node::default(),
            BackgroundColor(tokens::OPAQUE),
            BorderColor::all(tokens::BORDER),
            GlobalZIndex(tokens::z::POPOVER),
        ))
        .id();
    let title = words(
        commands,
        kit,
        Phrase::BuildSyntaxTitle.text(lang),
        m.text,
        tokens::INK,
    );
    commands.entity(pop).add_child(title);
    for (i, (example, meaning)) in SYNTAX.iter().enumerate() {
        let row = commands
            .spawn((
                Node {
                    min_height: px_fixed(if m.touch() { m.hit } else { m.scaled(28.0) }),
                    align_items: AlignItems::Center,
                    column_gap: m.px(12.0),
                    padding: UiRect::axes(m.px(6.0), px_fixed(0.0)),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                crate::ambience::Feel::tinting_to(Color::NONE, tokens::SELECTED),
                Press::Build(BuildPress::InsertSyntax(i)),
            ))
            .id();
        let code = commands
            .spawn((
                Text::new(*example),
                tf_bold(kit.fonts, m.small),
                TextColor(tokens::ACCENT),
                TextLayout::no_wrap(),
                Node {
                    width: m.px(150.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let what = words(commands, kit, meaning.text(lang), m.small, tokens::MUTED);
        commands.entity(row).add_children(&[code, what]);
        commands.entity(pop).add_child(row);
    }
    pop
}
