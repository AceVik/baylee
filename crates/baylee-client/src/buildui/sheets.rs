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
/// phone's Filters and Stats. The card window has its own slot
/// (`cardwindow`).
pub(super) fn sheet(commands: &mut Commands, holder: Entity, env: &Env) {
    let ui = env.ui();
    let surface = if env.state.confirm_leave {
        Some(leave(commands, env))
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
