//! The deck side (§7): the commander slot, the tabs Main · Sideboard ·
//! Stats, the grouping and the grouped list with `− n +` rows, the foot
//! with what the engine does not fully play; on a phone the 260-px rail
//! (commander, the counts, the last three added, Stats); on Narrow the
//! bottom tab bar.

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;
use crate::lobby::{List, hover_of_card, hover_of_entry};
use crate::shellkit::focus::Current;

/// The zone the deck side shows: the tab's, or (on Stats) where `+` adds.
pub(crate) fn shown_zone(env: &Env) -> Zone {
    env.ui().zone().unwrap_or_else(|| env.deck().zone())
}

/// Whether every section of the shown list is folded.
pub(crate) fn all_folded(env: &Env) -> bool {
    let sections = env.deck().sections(shown_zone(env), env.ui().grouping);
    !sections.is_empty() && sections.iter().all(|s| env.ui().collapsed.contains(&s.key))
}

/// The drawn deck rows in order, as indices into `entries(zone)`: what the
/// deck cursor walks.
pub(crate) fn order(state: &LobbyState) -> Vec<usize> {
    let deck = state.lobby.builder();
    let zone = state.build.zone().unwrap_or_else(|| deck.zone());
    deck.sections(zone, state.build.grouping)
        .into_iter()
        .filter(|s| !state.build.collapsed.contains(&s.key))
        .flat_map(|s| s.rows)
        .collect()
}

/// Whether the body shows the numbers rather than a list.
fn stats_shown(env: &Env) -> bool {
    if env.layout == Layout::Single {
        env.ui().pane == Pane::Stats
    } else {
        env.ui().tab == DeckTab::Stats
    }
}

/// The deck panel and its holders.
pub(super) fn panel(commands: &mut Commands, env: &Env, holders: &mut retained::Holders) -> Entity {
    let kit = env.kit;
    let grow = if env.layout == Layout::Columns {
        2.0
    } else {
        1.0
    };
    let panel = super::panel(commands, kit, grow, None);
    commands.entity(panel).insert(crate::lobby::dock::Dock(5));
    holders.deck_head = Some(holder(
        commands,
        panel,
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: kit.m.px(10.0),
            flex_shrink: 0.0,
            ..default()
        },
    ));
    holders.deck_body = Some(holder(
        commands,
        panel,
        Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            flex_basis: px_fixed(0.0),
            min_height: px_fixed(0.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
    ));
    holders.deck_foot = Some(holder(
        commands,
        panel,
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: kit.m.px(4.0),
            flex_shrink: 0.0,
            ..default()
        },
    ));
    panel
}

/// The phone's deck rail, 260 px.
pub(super) fn rail_panel(
    commands: &mut Commands,
    env: &Env,
    holders: &mut retained::Holders,
) -> Entity {
    let kit = env.kit;
    let panel = super::panel(
        commands,
        kit,
        0.0,
        Some(kit.m.scaled(260.0).clamp(220.0, 300.0)),
    );
    holders.rail = Some(holder(
        commands,
        panel,
        Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: px_fixed(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: kit.m.px(8.0),
            overflow: Overflow::clip_y(),
            ..default()
        },
    ));
    panel
}

/// The deck head: the commander slot, the tabs, the grouping.
pub(super) fn head(commands: &mut Commands, holder: Entity, env: &Env) {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let ui = env.ui();
    let slot = commander_slot(commands, env, false);
    commands.entity(holder).add_child(slot);
    // Narrow's Stats pane is the numbers alone: its tabs are the bar's.
    if env.layout == Layout::Single && stats_shown(env) {
        return;
    }

    let counts = deck.counts();
    let mut items: Vec<(&str, Option<u32>)> = vec![
        (Phrase::BuildTabMain.text(lang), Some(counts.main)),
        (Phrase::BuildTabSide.text(lang), Some(counts.side)),
    ];
    // Narrow keeps Stats in its bottom tab bar.
    if env.layout != Layout::Single {
        items.push((Phrase::BuildTabStats.text(lang), None));
    }
    let selected = match ui.tab {
        DeckTab::Main => 0,
        DeckTab::Side => 1,
        DeckTab::Stats => 2,
    }
    .min(items.len() - 1);
    let tabs = controls::tabs(commands, kit, &items, selected, |i| {
        let tab = match i {
            0 => DeckTab::Main,
            1 => DeckTab::Side,
            _ => DeckTab::Stats,
        };
        (
            Press::Build(BuildPress::SetTab(tab)),
            Stop::item(BUILDER, "tabs", u8::try_from(i).unwrap_or(0)),
            Current(i == selected),
        )
    });
    let mut row = vec![tabs, spring(commands)];
    if env.layout == Layout::Columns {
        let said = Phrase::DeckMakeup.fill(
            lang,
            &[
                &counts.lands.to_string(),
                &counts.creatures.to_string(),
                &counts.spells.to_string(),
            ],
        );
        row.push(cell(commands, kit, &said, m.small, tokens::MUTED, false));
    }
    let tabs_line = line(commands, kit, &row);
    commands.entity(holder).add_child(tabs_line);

    if !stats_shown(env) {
        let chosen = Grouping::ALL
            .iter()
            .position(|g| *g == ui.grouping)
            .unwrap_or(0);
        let names: Vec<&str> = Grouping::ALL.iter().map(|g| g.label().text(lang)).collect();
        let switch = controls::segmented(commands, kit, &names, chosen, |i| {
            (
                Press::Build(BuildPress::SetGrouping(Grouping::ALL[i.min(2)])),
                Stop::item(BUILDER, "group", u8::try_from(i).unwrap_or(0)),
                Current(i == chosen),
            )
        });
        let folded = all_folded(env);
        let collapse = link(
            commands,
            kit,
            if folded {
                Phrase::BuildExpandAll
            } else {
                Phrase::BuildCollapseAll
            }
            .text(lang),
            (Press::Build(BuildPress::CollapseAll), stop("collapse")),
        );
        let gap = spring(commands);
        let group_line = line(commands, kit, &[switch, gap, collapse]);
        commands.entity(holder).add_child(group_line);
    }
}

/// The commander slot: the leader's print (or an empty frame), who it is
/// or "none · choose", the badge and the identity's discs.
#[allow(clippy::too_many_lines)] // one slot, read left to right
fn commander_slot(commands: &mut Commands, env: &Env, compact: bool) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let leaders = deck.commanders();
    let height = if compact {
        m.scaled(48.0)
    } else {
        m.scaled(67.0)
    };
    let mut kids = Vec::new();
    if let Some(&first) = leaders.first()
        && let Some(card) = deck.card(first)
    {
        let held = deck
            .commander_row(first)
            .and_then(|at| deck.entries(Zone::Main).get(at));
        let hover = held.map_or_else(
            || hover_of_card(card),
            |entry| hover_of_entry(card, &entry.print),
        );
        let thumb = crate::lobby::thumbnails::spawn(commands, &hover);
        commands.entity(thumb).insert((
            Node {
                width: px_fixed((height * 5.0 / 7.0).round()),
                height: px_fixed(height.round()),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px_fixed(4.0)),
                ..default()
            },
            hover,
        ));
        if held.is_some() {
            commands.entity(thumb).insert((
                Press::Build(BuildPress::PickCommanderPrint(first)),
                Pickable::default(),
            ));
        }
        kids.push(thumb);
    } else {
        let frame = commands
            .spawn((
                Node {
                    width: px_fixed((height * 5.0 / 7.0).round()),
                    height: px_fixed(height.round()),
                    flex_shrink: 0.0,
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(4.0)),
                    ..default()
                },
                BorderColor::all(tokens::BORDER),
                Pickable::IGNORE,
            ))
            .id();
        kids.push(frame);
    }
    let column = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                row_gap: m.px(2.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let label = caption(commands, kit, Phrase::BuildCommander.text(lang));
    commands.entity(column).add_child(label);
    let mut second = Vec::new();
    if leaders.is_empty() {
        second.push(words(
            commands,
            kit,
            &format!("\u{2014} {} \u{b7}", Phrase::BuildNoCommander.text(lang)),
            m.text,
            tokens::INK,
        ));
    } else {
        let names: Vec<String> = leaders
            .iter()
            .filter_map(|slot| deck.card(*slot).map(|c| c.name.clone()))
            .collect();
        second.push(cell(
            commands,
            kit,
            &names.join(" & "),
            m.text,
            tokens::INK,
            true,
        ));
    }
    second.push(link(
        commands,
        kit,
        Phrase::BuildChoose.text(lang),
        (
            Press::Build(BuildPress::ChooseCommander(false)),
            stop("commander"),
        ),
    ));
    if leaders.len() == 1 && !compact {
        second.push(link(
            commands,
            kit,
            Phrase::BuildPartner.text(lang),
            Press::Build(BuildPress::ChooseCommander(true)),
        ));
    }
    if !compact {
        for &slot in leaders {
            second.push(icon_button(
                commands,
                kit,
                mark::CLOSE,
                false,
                Press::Build(BuildPress::RemoveCommander(slot)),
            ));
        }
    }
    let second = line(commands, kit, &second);
    commands.entity(column).add_child(second);
    kids.push(column);
    if !leaders.is_empty() && !compact {
        let mut identity = String::new();
        for slot in leaders {
            if let Some(card) = deck.card(*slot) {
                for letter in card.identity.chars() {
                    if !identity.contains(letter) {
                        identity.push(letter);
                    }
                }
            }
        }
        let discs = identity_discs(commands, kit, &identity);
        kids.push(discs);
    }
    let row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                column_gap: m.px(12.0),
                padding: UiRect::bottom(m.px(8.0)),
                border: UiRect::bottom(px_fixed(1.0)),
                ..default()
            },
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_children(&kids);
    row
}

/// A colour identity as discs with their letters (colour is never the only
/// carrier, §2.4), in `WUBRG` order.
fn identity_discs(commands: &mut Commands, kit: Kit, identity: &str) -> Entity {
    let m = kit.m;
    let row = commands
        .spawn((
            Node {
                column_gap: m.px(3.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for letter in "WUBRG".chars().filter(|c| identity.contains(*c)) {
        let side = m.scaled(18.0).max(16.0);
        let disc = commands
            .spawn((
                Node {
                    width: px_fixed(side),
                    height: px_fixed(side),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                    ..default()
                },
                BackgroundColor(mana_tone(letter)),
                Pickable::IGNORE,
            ))
            .id();
        let said = commands
            .spawn((
                Text::new(letter.to_string()),
                tf_bold(kit.fonts, m.small * 0.75),
                TextColor(crate::shellkit::tokens::INK_ON_LIGHT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(disc).add_child(said);
        commands.entity(row).add_child(disc);
    }
    row
}

/// The deck body: the shown list in its sections, or the numbers.
pub(super) fn body(commands: &mut Commands, holder: Entity, env: &Env) {
    let kit = env.kit;
    let lang = env.lang();
    let metrics = env.lobby_metrics();
    if stats_shown(env) {
        let list =
            crate::lobby::scroller(commands, metrics, List::Deck, env.scrolled.get(List::Deck));
        crate::lobby::scrollbars::attach(commands, holder, list, metrics);
        let numbers = super::stats::draw(commands, env);
        commands.entity(list).add_child(numbers);
        return;
    }
    let zone = shown_zone(env);
    let sections = env.deck().sections(zone, env.ui().grouping);
    if sections.is_empty() {
        let said = crate::shellkit::states::empty(
            commands,
            kit,
            crate::hud::glyph::LIBRARY,
            Phrase::DeckEmptyHint.text(lang),
            None,
        );
        commands.entity(holder).add_child(said);
        return;
    }
    let list = crate::lobby::scroller(commands, metrics, List::Deck, env.scrolled.get(List::Deck));
    commands.entity(list).insert(stop("deck"));
    crate::lobby::scrollbars::attach(commands, holder, list, metrics);
    let content = virtual_rows::deck(commands, env, zone, &sections);
    commands.entity(list).add_child(content);
}

/// The deck foot: what will not save, and what does not play as printed.
pub(super) fn foot(commands: &mut Commands, holder: Entity, env: &Env) {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let shaky = deck.counts().shaky;
    if shaky > 0 {
        let sign = glyph(commands, kit, mark::WARN, m.small, tokens::GOLD);
        let said = if shaky == 1 {
            Phrase::BuildShakyOne.text(lang).to_string()
        } else {
            Phrase::BuildShakyMany.fill(lang, &[&shaky.to_string()])
        };
        let text = cell(commands, kit, &said, m.small, tokens::GOLD, false);
        let row = line(commands, kit, &[sign, text]);
        commands.entity(holder).add_child(row);
    }
    let problems = deck.problems(lang);
    let advisory = problems.iter().filter(|p| !p.blocking).take(2);
    for problem in problems.iter().filter(|p| p.blocking).chain(advisory) {
        let ink = if problem.blocking {
            tokens::DANGER
        } else {
            tokens::MUTED
        };
        let line = cell(commands, kit, &problem.message, m.small, ink, false);
        commands.entity(holder).add_child(line);
    }
    for missing in deck.missing() {
        let said = Phrase::DroppedCards.fill(lang, &[missing]);
        let line = cell(commands, kit, &said, m.small, tokens::DANGER, false);
        commands.entity(holder).add_child(line);
    }
}

/// The phone's rail: the commander, Main n · Sideboard n, the last three
/// added, Stats.
pub(super) fn rail(commands: &mut Commands, holder: Entity, env: &Env) {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let slot = commander_slot(commands, env, true);
    commands.entity(holder).add_child(slot);
    let counts = deck.counts();
    let selected = usize::from(env.ui().tab == DeckTab::Side);
    let tabs = controls::tabs(
        commands,
        kit,
        &[
            (Phrase::BuildTabMain.text(lang), Some(counts.main)),
            (Phrase::BuildTabSide.text(lang), Some(counts.side)),
        ],
        selected,
        |i| {
            (
                Press::Build(BuildPress::SetTab(if i == 0 {
                    DeckTab::Main
                } else {
                    DeckTab::Side
                })),
                Stop::item(BUILDER, "tabs", u8::try_from(i).unwrap_or(0)),
                Current(i == selected),
            )
        },
    );
    commands.entity(holder).add_child(tabs);
    let label = caption(commands, kit, Phrase::BuildLastAdded.text(lang));
    commands.entity(holder).add_child(label);
    let zone = env.ui().zone().unwrap_or(Zone::Main);
    for &slot in deck.last_added() {
        let Some(card) = deck.card(slot) else {
            continue;
        };
        let hover = hover_of_card(card);
        let row = commands
            .spawn((
                Role::Row,
                Node {
                    width: Val::Percent(100.0),
                    height: px_fixed(m.hit),
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    column_gap: m.px(8.0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                Press::Build(BuildPress::Inspect(slot)),
                hover.clone(),
            ))
            .id();
        let thumb = crate::lobby::thumbnails::spawn(commands, &hover);
        let height = m.hit - 6.0;
        commands.entity(thumb).insert(Node {
            width: px_fixed((height * 5.0 / 7.0).round()),
            height: px_fixed(height.round()),
            flex_shrink: 0.0,
            border_radius: BorderRadius::all(px_fixed(3.0)),
            ..default()
        });
        let name = cell(commands, kit, &card.name, m.small, tokens::INK, true);
        commands.entity(name).insert(Node {
            flex_grow: 1.0,
            flex_basis: px_fixed(0.0),
            min_width: px_fixed(0.0),
            overflow: Overflow::clip_x(),
            ..default()
        });
        let n = words(
            commands,
            kit,
            &deck.count_of(slot, zone).to_string(),
            m.small,
            tokens::GOLD,
        );
        commands.entity(row).add_children(&[thumb, name, n]);
        commands.entity(holder).add_child(row);
    }
    let gap = spring(commands);
    commands.entity(holder).add_child(gap);
    let stats = controls::button(
        commands,
        kit,
        Phrase::BuildTabStats.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        (Press::Build(BuildPress::OpenStatsSheet), stop("lit")),
    );
    commands.entity(holder).add_child(stats);
}

/// Narrow's bottom tab bar: Pool · Deck n · Stats.
pub(super) fn tabbar(commands: &mut Commands, holder: Entity, env: &Env) {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let main = env.deck().counts().main;
    let bar = commands
        .spawn((
            Role::Panel,
            Node {
                width: Val::Percent(100.0),
                height: m.px(52.0),
                flex_shrink: 0.0,
                border: UiRect::top(px_fixed(1.0)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(tokens::BORDER),
        ))
        .id();
    let panes = [
        (Pane::Pool, Phrase::BuildPanePool.text(lang).to_string()),
        (
            Pane::Deck,
            format!("{} \u{b7} {main}", Phrase::BuildPaneDeck.text(lang)),
        ),
        (Pane::Stats, Phrase::BuildTabStats.text(lang).to_string()),
    ];
    for (i, (pane, said)) in panes.into_iter().enumerate() {
        let on = env.ui().pane == pane;
        let face = commands
            .spawn((
                Role::Tab,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::top(px_fixed(2.0)),
                    ..default()
                },
                BackgroundColor(if on { tokens::SELECTED } else { Color::NONE }),
                BorderColor::all(if on { tokens::ACCENT } else { Color::NONE }),
            ))
            .id();
        let text = words(
            commands,
            kit,
            &said,
            m.text,
            if on { tokens::INK } else { tokens::MUTED },
        );
        commands.entity(face).add_child(text);
        let hit = controls::hit(
            commands,
            kit,
            face,
            (
                Press::Build(BuildPress::SetPane(pane)),
                Stop::item(BUILDER, "panes", u8::try_from(i).unwrap_or(0)),
                Current(on),
            ),
        );
        commands.entity(hit).insert(Node {
            flex_grow: 1.0,
            flex_basis: px_fixed(0.0),
            height: Val::Percent(100.0),
            min_width: px_fixed(0.0),
            ..default()
        });
        commands.entity(bar).add_child(hit);
    }
    commands.entity(holder).add_child(bar);
}
