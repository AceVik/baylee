//! What stands over the builder: the `⋯` menus (the header's, a pool
//! row's, a deck row's), and the sheets — the card, "Discard changes?", the
//! phone's Stats and Filters.
//!
//! A menu is spawned hidden and placed under its opener on the next frame
//! (`focus::place_menus`), flipped up at the window's foot; a press outside
//! it closes it. A sheet stands over a scrim and is modal: Tab cycles inside
//! it, Esc closes it (`KEYBOARD.md` §2.5).

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;
use crate::lobby::LibraryPress;
use crate::shellkit::surfaces::{self, MenuItem, SheetWidth};

/// A menu waiting to be placed under the opener that names it.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct PlacedMenu(pub(crate) BuildMenu);

/// The open menu, if any.
#[allow(clippy::too_many_lines)] // three menus, item by item
pub(super) fn menu(commands: &mut Commands, holder: Entity, env: &Env) {
    let Some(open) = env.ui().menu else {
        return;
    };
    let kit = env.kit;
    let lang = env.lang();
    let deck = env.deck();
    let mut items: Vec<MenuItem<(Press, Stop)>> = Vec::new();
    let item = |text: &'static str, press: BuildPress| MenuItem {
        text,
        keys: None,
        destructive: false,
        action: (Press::Build(press), sheet_stop("item")),
    };
    match open {
        BuildMenu::Header => {
            if env.layout != Layout::Columns {
                items.push(item(Phrase::ImportDeck.text(lang), BuildPress::OpenImport));
                items.push(item(Phrase::ExportDeck.text(lang), BuildPress::OpenExport));
                if env.state.lobby.token().is_some() && deck.editing().is_some() {
                    items.push(MenuItem {
                        text: Phrase::DeckHistory.text(lang),
                        keys: None,
                        destructive: false,
                        action: (
                            Press::Library(LibraryPress::BrowseHistory),
                            sheet_stop("item"),
                        ),
                    });
                }
            }
            items.push(MenuItem {
                keys: Some("F2"),
                ..item(Phrase::BuildRename.text(lang), BuildPress::Rename)
            });
            items.push(item(
                Phrase::BuildSettings.text(lang),
                BuildPress::BuilderSettings,
            ));
            if !deck.entries(Zone::Main).is_empty() || !deck.entries(Zone::Side).is_empty() {
                items.push(MenuItem {
                    destructive: true,
                    ..item(Phrase::BuildEmptyDeck.text(lang), BuildPress::ClearDeck)
                });
            }
        }
        BuildMenu::Pool(slot) => {
            let other = match super::deck::shown_zone(env) {
                Zone::Main => (Phrase::BuildAddToSide, true),
                Zone::Side => (Phrase::BuildAddToMain, true),
            };
            items.push(item(
                other.0.text(lang),
                BuildPress::AddFromPool(slot, other.1),
            ));
            items.push(item(
                Phrase::BuildChoosePrinting.text(lang),
                BuildPress::PickPrint(slot),
            ));
            items.push(item(
                Phrase::BuildOpenCard.text(lang),
                BuildPress::Inspect(slot),
            ));
        }
        BuildMenu::Deck(at) => {
            let zone = super::deck::shown_zone(env);
            let to = match zone {
                Zone::Main => Phrase::BuildMoveToSide,
                Zone::Side => Phrase::BuildMoveToMain,
            };
            items.push(item(to.text(lang), BuildPress::MoveRow(at)));
            items.push(item(
                Phrase::BuildChoosePrinting.text(lang),
                BuildPress::PickRowPrint(at),
            ));
            if let Some(entry) = deck.entries(zone).get(at)
                && deck.card(entry.slot).is_some_and(|c| c.commander)
                && !deck.is_commander(entry.slot)
            {
                items.push(item(
                    Phrase::SetCommander.text(lang),
                    BuildPress::SetCommander(entry.slot),
                ));
            }
            items.push(MenuItem {
                destructive: true,
                ..item(Phrase::BuildRemoveAll.text(lang), BuildPress::RemoveAll(at))
            });
        }
    }
    // A press anywhere else closes the menu: a catcher under it, over the
    // whole window.
    let catcher = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px_fixed(0.0),
                top: px_fixed(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            GlobalZIndex(tokens::z::POPOVER - 1),
            Press::Build(BuildPress::CloseMenu),
        ))
        .id();
    let surface = surfaces::menu(commands, kit, items);
    commands.entity(surface).insert((
        Node {
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            min_width: kit.m.px(220.0),
            padding: UiRect::axes(px_fixed(0.0), kit.m.px(4.0)),
            border: UiRect::all(px_fixed(1.0)),
            border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
            ..default()
        },
        Visibility::Hidden,
        PlacedMenu(open),
    ));
    commands.entity(holder).add_children(&[catcher, surface]);
}

/// The open sheet, if any: leaving with unsaved changes first, then the
/// card, the phone's Filters and Stats.
pub(super) fn sheet(commands: &mut Commands, holder: Entity, env: &Env) {
    let ui = env.ui();
    let surface = if env.state.confirm_leave {
        Some(leave(commands, env))
    } else if let Some(slot) = env.deck().inspecting() {
        card(commands, env, slot)
    } else if ui.rail && matches!(env.layout, Layout::Rail | Layout::PhoneSingle) {
        Some(filters(commands, env))
    } else if ui.stats_sheet {
        Some(stats(commands, env))
    } else {
        None
    };
    if let Some(surface) = surface {
        let scrim = surfaces::sheet(commands, surface);
        commands.entity(holder).add_child(scrim);
    }
}

/// "Discard changes?": Keep editing (focused) and Discard, the builder's
/// one confirm (KEYBOARD §2.5).
fn leave(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let lang = env.lang();
    let body = surfaces::prose(commands, kit, Phrase::BuildDiscardBody.text(lang), false);
    let discard = controls::button(
        commands,
        kit,
        Phrase::BuildDiscard.text(lang),
        Weight::Danger,
        Live::Yes,
        None,
        (
            Press::Build(BuildPress::DiscardAndLeave),
            sheet_stop("discard"),
        ),
    );
    let keep = controls::button(
        commands,
        kit,
        Phrase::BuildKeepEditing.text(lang),
        Weight::Primary,
        Live::Yes,
        None,
        (Press::Build(BuildPress::KeepEditing), sheet_stop("keep")),
    );
    surfaces::sheet_box(
        commands,
        kit,
        SheetWidth::Small,
        Phrase::BuildDiscardQuestion.text(lang),
        &[body],
        &[discard, keep],
    )
}

/// The card: what is printed on it, what this build does with it, and
/// where it can go.
#[allow(clippy::too_many_lines)] // one sheet, read top to bottom
fn card(commands: &mut Commands, env: &Env, slot: usize) -> Option<Entity> {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let card = deck.card(slot)?;
    let mut body = Vec::new();
    let mut top = vec![cell(
        commands,
        kit,
        &match &card.stats {
            Some(stats) if !stats.is_empty() => format!("{} \u{b7} {stats}", card.type_line),
            _ => card.type_line.clone(),
        },
        m.small,
        tokens::MUTED,
        false,
    )];
    top.push(spring(commands));
    if let Some(cost) =
        crate::manaui::spawn_cost_or_text(commands, kit.fonts, &card.mana_cost, m.small * 1.2)
    {
        top.push(cost);
    }
    body.push(line(commands, kit, &top));
    // The gateway serves rules text only with a catalog; without one the
    // compiled English Oracle stands in, never an empty box.
    let text = if card.oracle_text.is_empty() {
        baylee_cards::generated_oracle::ORACLE
            .get(card.index as usize)
            .map(|faces| faces.join("\n\n"))
            .unwrap_or_default()
    } else {
        card.oracle_text.clone()
    };
    if text.is_empty() {
        body.push(surfaces::prose(
            commands,
            kit,
            Phrase::NoRulesText.text(lang),
            true,
        ));
    } else {
        body.push(surfaces::prose(commands, kit, &text, false));
    }
    if let Some((mark, ink)) = coverage_mark(card.coverage) {
        let said = mark.text(lang);
        let why = match &card.note {
            Some(note) => format!("{said}: {note}"),
            None => Phrase::NotAsPrinted.fill(lang, &[said]),
        };
        let line = commands
            .spawn((
                Text::new(why),
                tf(kit.fonts, m.small),
                TextColor(ink),
                Pickable::IGNORE,
            ))
            .id();
        body.push(line);
    }
    let (main, side) = (
        deck.count_of(slot, Zone::Main),
        deck.count_of(slot, Zone::Side),
    );
    let held = match (main, side) {
        (0, 0) => String::new(),
        (m, 0) => Phrase::HeldInDeck.fill(lang, &[&m.to_string()]),
        (0, s) => Phrase::HeldInSideboard.fill(lang, &[&s.to_string()]),
        (m, s) => Phrase::HeldInBoth.fill(lang, &[&m.to_string(), &s.to_string()]),
    };
    if !held.is_empty() {
        body.push(surfaces::prose(commands, kit, &held, true));
    }
    let mut actions = vec![
        controls::button(
            commands,
            kit,
            Phrase::BuildAddToMain.text(lang),
            Weight::Primary,
            Live::Yes,
            Some("Enter"),
            (
                Press::Build(BuildPress::AddCardTo(slot, Zone::Main)),
                sheet_stop("add"),
            ),
        ),
        controls::button(
            commands,
            kit,
            Phrase::BuildAddToSide.text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            (
                Press::Build(BuildPress::AddCardTo(slot, Zone::Side)),
                sheet_stop("other"),
            ),
        ),
        controls::button(
            commands,
            kit,
            Phrase::BuildChoosePrinting.text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            (
                Press::Build(BuildPress::PickPrint(slot)),
                sheet_stop("printing"),
            ),
        ),
    ];
    if card.commander {
        let leading = deck.is_commander(slot);
        actions.push(controls::button(
            commands,
            kit,
            if leading {
                Phrase::IsCommander
            } else {
                Phrase::SetCommander
            }
            .text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            (
                Press::Build(if leading {
                    BuildPress::ClearCommander
                } else {
                    BuildPress::SetCommander(slot)
                }),
                sheet_stop("commander"),
            ),
        ));
    }
    let wrap = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_wrap: FlexWrap::Wrap,
                column_gap: m.px(8.0),
                row_gap: m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(wrap).add_children(&actions);
    body.push(wrap);
    let close = controls::button(
        commands,
        kit,
        Phrase::ShellClose.text(lang),
        Weight::Secondary,
        Live::Yes,
        Some("Esc"),
        (Press::Build(BuildPress::CloseCard), sheet_stop("close")),
    );
    Some(surfaces::sheet_box(
        commands,
        kit,
        SheetWidth::Medium,
        &card.name,
        &body,
        &[close],
    ))
}

/// The phone's Filters, as a sheet.
fn filters(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let body = super::pool::filters_body(commands, env);
    let close = controls::button(
        commands,
        kit,
        Phrase::ShellClose.text(env.lang()),
        Weight::Primary,
        Live::Yes,
        None,
        (Press::Build(BuildPress::ToggleRail), sheet_stop("close")),
    );
    surfaces::sheet_box(
        commands,
        kit,
        SheetWidth::Small,
        Phrase::BuildFilters.text(env.lang()),
        &body,
        &[close],
    )
}

/// The phone's Stats, as a sheet.
fn stats(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let numbers = super::stats::draw(commands, env);
    let close = controls::button(
        commands,
        kit,
        Phrase::ShellClose.text(env.lang()),
        Weight::Primary,
        Live::Yes,
        None,
        (
            Press::Build(BuildPress::CloseStatsSheet),
            sheet_stop("close"),
        ),
    );
    surfaces::sheet_box(
        commands,
        kit,
        SheetWidth::Large,
        Phrase::BuildTabStats.text(env.lang()),
        &[numbers],
        &[close],
    )
}
