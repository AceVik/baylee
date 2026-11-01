//! The Play screen (the shell design, `DESIGN-v5.md` §4; WP2): "Your next
//! game" — the deck, Play the house at a difficulty, Create table — beside
//! the tables list, which is the screen's anchor; this session's recent
//! games under the hero; the Create-table sheet (§5) and the deck picker.
//!
//! - Wide and Vast: the hero is a third, the tables two thirds.
//! - Narrow and Compact: the hero is a strip over the list.
//! - Phone (M4-6): the hero is a 280-px rail left of the list (220 at 640);
//!   the list shows three rows at 844 × 390.
//! - While the player holds a chair, the starts are off and say where they
//!   sit; at their own running table Return to your game is the hero's gold
//!   primary (M-7, S-8). On a phone the header's Return pill is the one way
//!   back and the rail shows no second (S4-3).

mod create;
pub(super) mod discovery;
mod hero;
mod tables;

use super::decks::DecksPress;
use super::menus::{self, ShellMenu};
use super::orders;
use super::parts;
use super::press::Cx;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::surfaces::{self, SheetWidth};
use crate::shellkit::{Frame as ShellFrame, Role, px_fixed, tokens};
use crate::tour::TourAnchor;
use baylee_client_core::tour::Anchor;
use client_core::lobby::play::{
    self as model, Chip, Outcome, TableDraft, TableFilter, TableSort, Template,
};
use client_core::lobby::shelf;
use client_core::lobby::strips::{self, Strip};
use create::create_sheet;
use hero::hero;
use tables::tables;

/// What the Play screen remembers between rebuilds that the lobby's model
/// does not. Presses write it; nothing else does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlayUi {
    /// The table list's chips.
    pub(crate) filter: TableFilter,
    /// The table list's order.
    pub(crate) sort: TableSort,
    /// Play the house's difficulty, by its place in `AIProfile::NAMED`.
    pub(crate) difficulty: usize,
    /// The Create-table sheet, when it is up; `true` beside it when it
    /// edits the room this client hosts (**Apply**) rather than opening one.
    pub(crate) sheet: Option<(TableDraft, bool)>,
    /// The deck picker sheet (Change deck) is up.
    pub(crate) picker: bool,
    /// Stable room identity while its password prompt is open.
    pub(crate) password_for: Option<String>,
}

impl Default for PlayUi {
    fn default() -> Self {
        Self {
            filter: TableFilter::default(),
            sort: TableSort::default(),
            // `steady`: the one-tap game's own, as before the caret.
            difficulty: 2,
            sheet: None,
            picker: false,
            password_for: None,
        }
    }
}

/// The difficulty's wire name.
fn difficulty(at: usize) -> &'static str {
    baylee_core::preset::AIProfile::NAMED
        .get(at)
        .map_or("steady", |(name, _)| name)
}

/// A control of the Play screen and its sheets.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PlayPress {
    /// Play the house at the chosen difficulty.
    PlayHouse,
    /// Choose the difficulty (its place in `AIProfile::NAMED`).
    Difficulty(usize),
    /// Open the Create-table sheet.
    CreateTable,
    /// Open the deck picker.
    ChangeDeck,
    /// Choose a deck in the picker (its index in the deck list).
    PickDeck(usize),
    /// Edit the next game's deck.
    EditNext,
    /// A table filter chip.
    Chip(Chip),
    /// The tables' order.
    Sort(TableSort),
    /// Join a listed table (its index in the listing).
    Join(usize),
    /// Submit the password for the room held by stable id.
    JoinProtected,
    /// Watch a listed table being played (its index in the listing).
    Watch(usize),
    /// The sheet: whether spectators may watch.
    Spectators,
    /// Back to the player's own table, waiting or running.
    Return,
    /// A recent game again, one press (its place in the recent list).
    Rematch(usize),
    /// A recent game's settings in the Create-table sheet.
    EditAndRematch(usize),
    /// The sheet: chairs, by number.
    Players(usize),
    /// The sheet's stepper (a phone's Players): one more or fewer.
    StepPlayers(bool),
    /// The sheet: a template.
    Template(Template),
    /// The sheet: Adjust unfolds the rules.
    Adjust,
    /// The sheet: starting life up or down.
    Life(i32),
    /// The sheet: free mulligans up or down.
    Mulligans(bool),
    /// The sheet's clock stepper: the next slower clock in the gateway's
    /// list (`true`) or the next faster one.
    StepClock(bool),
    /// The sheet's primary: Open table, or Apply over a room.
    Open,
    /// Close a sheet without doing anything.
    CloseSheet,
    /// Re-read the tables after an error.
    Retry,
    /// Clear the table search and read the first page again.
    ClearSearch,
    /// Take the chair kept for this player at a listed rematch room.
    PlayAgain(usize),
}

/// Draws the Play screen under the header.
pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    metrics: Metrics,
    kit: Kit,
    scrolled: &Scrolled,
) {
    let frame = kit.m.frame;
    let phone = frame == ShellFrame::Phone;
    let side = phone || matches!(frame, ShellFrame::Wide | ShellFrame::Vast);
    let body = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: if side {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                },
                justify_content: JustifyContent::Center,
                align_items: if side {
                    AlignItems::Start
                } else {
                    AlignItems::Stretch
                },
                column_gap: px_fixed(kit.m.body),
                row_gap: px_fixed(kit.m.body),
                padding: UiRect::all(px_fixed(kit.m.body)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(body);
    let mut lane = commands.spawn((
        Node {
            width: if phone {
                px(if metrics.frame == Frame::Compact {
                    220
                } else {
                    280
                })
            } else if side {
                percent(30)
            } else {
                percent(100)
            },
            max_width: if side && !phone {
                kit.m.px(460.0)
            } else {
                Val::Auto
            },
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px_fixed(kit.m.body),
            max_height: percent(100),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        Scrollable(List::Discovery),
        ScrollPosition(Vec2::new(0.0, scrolled.get(List::Discovery))),
        Pickable::default(),
    ));
    if side && !phone {
        lane.insert(crate::vista::Framed(crate::vista::Vista::Interior));
    }
    let lane = lane.id();
    commands.entity(body).add_child(lane);
    let hero_more = if state.lobby.offline() {
        hero(commands, lane, state, metrics, kit)
    } else {
        discovery::sidebar(commands, lane, state, kit);
        HeroAnchors::default()
    };
    if side && !phone {
        recent(commands, lane, state, kit);
    }
    let tables_more = tables(commands, body, state, metrics, kit, scrolled);
    if !side || phone {
        // Narrow and Phone: Recent games is a row under the list.
        recent(commands, body, state, kit);
    }
    menu(commands, root, state, kit, hero_more, tables_more);
    if state.play.picker {
        picker(commands, root, state, kit, scrolled);
    }
    if let Some(id) = &state.play.password_for {
        discovery::password_sheet(commands, root, state, metrics, kit, id);
    }
    if let Some((draft, editing)) = &state.play.sheet {
        create_sheet(commands, root, state, metrics, kit, draft, *editing);
    }
}

/// Anchors the hero hands back: the difficulty caret and the `⋯`.
#[derive(Clone, Copy, Default)]
pub(super) struct HeroAnchors {
    caret: Option<Entity>,
    more: Option<Entity>,
}

/// Why the starts are off, if they are: a held chair, a running game, no
/// game host, no deck.
fn starts_off(state: &LobbyState, lang: Lang) -> Option<String> {
    let lobby = &state.lobby;
    match strips::strip(lobby) {
        Some(Strip::Seated { table, .. }) => {
            return Some(Phrase::ShellSeatedAt.fill(lang, &[&table]));
        }
        Some(Strip::Playing { table, .. }) => {
            return Some(Phrase::PlayPlayingAt.fill(lang, &[&table]));
        }
        None => {}
    }
    if !lobby.games_can_start() {
        return Some(Phrase::PlayNoHost.text(lang).to_string());
    }
    None
}

/// Three house decks as mini tiles: art 78 × 57 with its credit, name,
/// format; one press copies and uses (S-3, through Decks' **Add and use**).
/// Decks holding a card blocked on the gateway are passed over.
pub(super) fn mini_tiles(
    commands: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    order: &'static crate::shellkit::focus::TabOrder,
    id: &'static str,
) -> Entity {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let row = commands
        .spawn((
            Node {
                flex_direction: if kit.m.frame == ShellFrame::Phone {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
                flex_wrap: FlexWrap::Wrap,
                column_gap: px_fixed(kit.m.gap),
                row_gap: px_fixed(kit.m.gap),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let house = &lobby.library().house;
    if house.is_empty() {
        let waiting = if lobby.library().loading {
            crate::shellkit::states::skeleton(commands, kit, 2)
        } else {
            parts::line(
                commands,
                kit,
                Phrase::LibraryEmpty.text(lang),
                kit.m.small,
                tokens::MUTED,
            )
        };
        commands.entity(row).add_child(waiting);
        return row;
    }
    // A house deck holding a card blocked on this gateway is not offered
    // here: one press would copy it and the game would be refused (its own
    // tile on the House shelf says why). The index stays the shelf's.
    for (index, deck) in house
        .iter()
        .enumerate()
        .filter(|(_, deck)| deck.blocked.is_empty())
        .take(3)
    {
        let tile = commands
            .spawn((
                Role::Tile,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: kit.m.px(4.0),
                    padding: UiRect::all(kit.m.px(8.0)),
                    width: kit.m.px(150.0),
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                    ..default()
                },
                BackgroundColor(tokens::CONTROL),
                BorderColor::all(tokens::BORDER),
                parts::identity_gradient(&deck.identity),
                Press::Decks(DecksPress::AddAndUse(index)),
                crate::ambience::Feel::new(tokens::CONTROL),
            ))
            .id();
        if let Some(art) = deck.art() {
            let picture = parts::art_with_credit(commands, kit, lang, &art, 78.0);
            commands.entity(tile).add_child(picture);
        }
        let name = commands
            .spawn((
                Text::new(deck.name.clone()),
                crate::hud::tf_bold(kit.fonts, kit.m.small),
                TextColor(tokens::INK),
                Pickable::IGNORE,
            ))
            .id();
        let format = parts::line(
            commands,
            kit,
            &shelf::format_label(lang, &deck.format),
            kit.m.small,
            tokens::MUTED,
        );
        let action = parts::line(
            commands,
            kit,
            Phrase::ShellAddAndUse.text(lang),
            kit.m.small,
            tokens::ACCENT,
        );
        commands.entity(tile).add_children(&[name, format, action]);
        orders::item(commands, tile, order, id, index);
        commands.entity(row).add_child(tile);
    }
    row
}

/// Recent games (this session; absent when empty).
fn recent(commands: &mut Commands, parent: Entity, state: &LobbyState, kit: Kit) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    if lobby.recent().is_empty() {
        return;
    }
    let panel = surfaces::panel(commands, kit, percent(100));
    commands.entity(panel).insert(super::dock::Dock(6));
    let caption = parts::caption(commands, kit, Phrase::PlayRecentGames.text(lang));
    commands.entity(panel).add_child(caption);
    for (at, game) in lobby.recent().iter().enumerate() {
        let against = if game.opponents.is_empty() {
            Phrase::PlayVsHouse.text(lang).to_string()
        } else {
            Phrase::PlayVs.fill(lang, &[&game.opponents.join(", ")])
        };
        let (said, ink) = match game.outcome {
            Outcome::Won => (Phrase::PlayWon, tokens::ACCENT),
            Outcome::Lost => (Phrase::PlayLost, tokens::DANGER),
            Outcome::Drawn => (Phrase::PlayDrawn, tokens::MUTED),
        };
        let line = parts::row(commands, kit, true);
        let what = parts::line(
            commands,
            kit,
            &format!("{} · {against} · ", game.at),
            kit.m.small,
            tokens::MUTED,
        );
        let how = parts::line(commands, kit, said.text(lang), kit.m.small, ink);
        let deck = parts::line(
            commands,
            kit,
            &format!(" · {}", game.deck),
            kit.m.small,
            tokens::MUTED,
        );
        let words = commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(words).add_children(&[what, how, deck]);
        let off = starts_off(state, lang).is_some() || lobby.busy();
        let again = controls::button(
            commands,
            kit,
            Phrase::PlayRematch.text(lang),
            Weight::Secondary,
            if off { Live::No("") } else { Live::Yes },
            None,
            Press::Play(PlayPress::Rematch(at)),
        );
        let edit = controls::button(
            commands,
            kit,
            Phrase::PlayEditAndRematch.text(lang),
            Weight::Ghost,
            if off { Live::No("") } else { Live::Yes },
            None,
            Press::Play(PlayPress::EditAndRematch(at)),
        );
        orders::item(commands, again, &orders::PLAY, "recent", 2 * at);
        orders::item(commands, edit, &orders::PLAY, "recent", 2 * at + 1);
        commands.entity(line).add_children(&[words, again, edit]);
        commands.entity(panel).add_child(line);
    }
    commands.entity(parent).add_child(panel);
}

/// The open menu of this screen, hung from its button.
fn menu(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    hero: HeroAnchors,
    sort: Option<Entity>,
) {
    let lang = state.lobby.lang();
    match state.menu {
        Some(ShellMenu::Difficulty) => {
            let Some(anchor) = hero.caret else { return };
            let items: Vec<_> = baylee_core::preset::AIProfile::NAMED
                .iter()
                .enumerate()
                .map(|(at, (name, _))| {
                    menus::item(
                        super::ai_name(lang, name),
                        Press::Play(PlayPress::Difficulty(at)),
                    )
                })
                .collect();
            menus::draw(commands, root, kit, anchor, items);
        }
        Some(ShellMenu::Hero) => {
            let Some(anchor) = hero.more else { return };
            let items = vec![
                menus::item(
                    Phrase::PlayChangeDeck.text(lang),
                    Press::Play(PlayPress::ChangeDeck),
                ),
                menus::item(
                    Phrase::ShellEdit.text(lang),
                    Press::Play(PlayPress::EditNext),
                ),
            ];
            menus::draw(commands, root, kit, anchor, items);
        }
        Some(ShellMenu::TableSort) => {
            let Some(anchor) = sort else { return };
            let items: Vec<_> = TableSort::ALL
                .into_iter()
                .map(|s| menus::item(s.phrase().text(lang), Press::Play(PlayPress::Sort(s))))
                .collect();
            menus::draw(commands, root, kit, anchor, items);
            menus::selected(commands, Press::Play(PlayPress::Sort(state.play.sort)));
        }
        _ => {}
    }
}

/// The deck picker (Change deck): the shelf's tiles, one press chooses.
#[allow(clippy::too_many_lines)] // one sheet, drawn top to bottom
fn picker(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    scrolled: &Scrolled,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let grid = commands
        .spawn((
            Node {
                display: Display::Grid,
                grid_template_columns: vec![RepeatedGridTrack::minmax(
                    GridTrackRepetition::AutoFill,
                    MinTrackSizingFunction::Px(kit.m.scaled(200.0)),
                    MaxTrackSizingFunction::Fraction(1.0),
                )],
                column_gap: px_fixed(kit.m.gap),
                row_gap: px_fixed(kit.m.gap),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (walked, index) in shelf::order(
        lobby.decks(),
        "",
        shelf::Sort::LastSaved,
        lobby.staged_delete(),
    )
    .into_iter()
    .enumerate()
    {
        let deck = &lobby.decks()[index];
        let chosen = lobby.selected() == Some(index);
        let extra = if chosen { 1.0 } else { 0.0 };
        let tile = commands
            .spawn((
                Role::Tile,
                Node {
                    column_gap: kit.m.px(10.0),
                    align_items: AlignItems::Center,
                    padding: UiRect::all(px_fixed((kit.m.scaled(8.0) - extra).max(0.0))),
                    border: UiRect::all(px_fixed(if chosen { 2.0 } else { 1.0 })),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                    ..default()
                },
                BorderColor::all(if chosen {
                    tokens::ACCENT
                } else {
                    tokens::BORDER
                }),
                parts::identity_gradient(&deck.identity),
                Press::Play(PlayPress::PickDeck(index)),
                crate::ambience::Feel::new(Color::NONE),
            ))
            .id();
        if let Some(art) = deck.art() {
            let picture = parts::art_with_credit(commands, kit, lang, &art, 78.0);
            commands.entity(tile).add_child(picture);
        }
        let words = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    min_width: px(0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let name = parts::line(commands, kit, &deck.name, kit.m.text, tokens::INK);
        let format = parts::line(
            commands,
            kit,
            &shelf::format_label(lang, &deck.format),
            kit.m.small,
            tokens::MUTED,
        );
        commands.entity(words).add_children(&[name, format]);
        if let Some(said) = shelf::blocked_badge(lang, &deck.blocked) {
            let blocked = parts::line(commands, kit, &said, kit.m.small, tokens::DANGER);
            commands.entity(words).add_child(blocked);
        }
        commands.entity(tile).add_child(words);
        orders::item(commands, tile, &orders::PICKER, "tiles", walked);
        commands
            .entity(tile)
            .insert(crate::shellkit::focus::Current(chosen));
        commands.entity(grid).add_child(tile);
    }
    let cancel = controls::button(
        commands,
        kit,
        Phrase::ShellClose.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        Press::Play(PlayPress::CloseSheet),
    );
    // A shelf of decks can run past the sheet: the wheel moves it and a bar
    // says so (report 30.10.2026: neither did).
    let surface = surfaces::sheet_box_listed(
        commands,
        kit,
        SheetWidth::Medium,
        Phrase::PlayChangeDeck.text(lang),
        &[grid],
        &[cancel],
        (
            Scrollable(List::DeckPicker),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::DeckPicker))),
        ),
    );
    orders::stop(commands, cancel, &orders::PICKER, "cancel");
    let scrim = surfaces::sheet(commands, surface);
    commands
        .entity(scrim)
        .insert(Press::Play(PlayPress::CloseSheet));
    commands
        .entity(surface)
        .insert(Press::Shared(SharedPress::PickerNothing));
    commands.entity(root).add_child(scrim);
}

/// The Create-table sheet where it is up: over Play, and over the room for
/// Edit rules.
pub(super) fn sheet_over(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    metrics: Metrics,
    kit: Kit,
) {
    if let Some((draft, editing)) = &state.play.sheet {
        create_sheet(commands, root, state, metrics, kit, draft, *editing);
    }
}

/// Opens the Create-table sheet: the name prefilled "<handle>'s table",
/// the password box empty, the caret in the name.
pub(super) fn open_sheet(state: &mut LobbyState, draft: TableDraft, editing: bool) {
    if !editing {
        let lang = state.lobby.lang();
        let who = state
            .lobby
            .me()
            .map(|me| me.handle.split('#').next().unwrap_or_default().to_string())
            .filter(|h| !h.is_empty());
        let name = who.map_or_else(
            || Phrase::RoomTitle.text(lang).to_string(),
            |who| Phrase::SheetDefaultName.fill(lang, &[&who]),
        );
        state.lobby.set_field(Field::RoomName, &name);
        state.lobby.set_field(Field::RoomPassword, "");
    }
    state.lobby.focus_on(Field::RoomName);
    state.play.sheet = Some((draft, editing));
}

impl PlayPress {
    /// What a press on this control does.
    #[allow(clippy::too_many_lines)] // one flat match, read top to bottom
    pub(super) fn handle(self, cx: Cx<'_, '_, '_, '_, '_>) {
        let Cx {
            state,
            prefs,
            scrolled,
            mailbox,
            settings,
        } = cx;
        match self {
            PlayPress::PlayHouse => {
                let level = difficulty(state.play.difficulty);
                let request = state.lobby.play_house(level);
                dispatch(state, mailbox, request);
            }
            PlayPress::Difficulty(at) => state.play.difficulty = at,
            PlayPress::CreateTable => {
                if starts_off(state, state.lobby.lang()).is_none() {
                    open_sheet(state, TableDraft::default(), false);
                }
            }
            PlayPress::ChangeDeck => state.play.picker = true,
            PlayPress::PickDeck(index) => {
                state.lobby.select_deck(index);
                state.play.picker = false;
            }
            PlayPress::EditNext => {
                if let Some(index) = state.lobby.selected() {
                    state.commander_pick = None;
                    state.build = crate::buildui::BuildUi::opened();
                    let request = state.lobby.edit_deck(index);
                    dispatch(state, mailbox, request);
                }
            }
            PlayPress::Chip(chip) => {
                state.play.filter.toggle(chip);
                scrolled.set(List::Games, 0.0);
            }
            PlayPress::Sort(sort) => state.play.sort = sort,
            PlayPress::Join(index) => {
                let game = state
                    .lobby
                    .games()
                    .get(index)
                    .map(|g| (g.id.clone(), g.locked));
                if let Some((game, locked)) = game {
                    if locked {
                        state.play.password_for = Some(game);
                        state.lobby.set_field(Field::RoomPassword, "");
                        state.lobby.focus_on(Field::RoomPassword);
                    } else {
                        let request = state.lobby.join(&game);
                        dispatch(state, mailbox, request);
                    }
                }
            }
            PlayPress::JoinProtected => {
                if let Some(game) = state.play.password_for.clone() {
                    let request = state.lobby.join(&game);
                    dispatch(state, mailbox, request);
                }
            }
            PlayPress::Watch(index) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    state.lobby.watch_table(&game);
                }
            }
            PlayPress::Spectators => {
                if let Some((draft, _)) = state.play.sheet.as_mut() {
                    draft.spectators = !draft.spectators;
                }
            }
            PlayPress::Return => {
                super::header::HeaderPress::Return.handle(Cx {
                    state,
                    prefs,
                    scrolled,
                    mailbox,
                    settings,
                });
            }
            PlayPress::Rematch(at) => {
                let Some(game) = state.lobby.recent().get(at).cloned() else {
                    return;
                };
                let request = if state.lobby.offline() || game.game_id.is_empty() {
                    // Offline there is no gateway to keep a chair: the
                    // house again, with the same deck.
                    let level = difficulty(state.play.difficulty);
                    state.lobby.play_house(level)
                } else {
                    state.lobby.rematch(&game.game_id)
                };
                dispatch(state, mailbox, request);
            }
            PlayPress::EditAndRematch(at) => {
                let chairs = state.lobby.recent().get(at).map_or(2, |g| g.chairs);
                let template = if chairs == 2 {
                    Template::Duel
                } else {
                    Template::Commander
                };
                open_sheet(state, TableDraft::of(template, chairs), false);
            }
            PlayPress::Players(n) => {
                if let Some((draft, _)) = state.play.sheet.as_mut() {
                    draft.players = n.clamp(2, 8);
                }
            }
            PlayPress::StepPlayers(more) => {
                if let Some((draft, _)) = state.play.sheet.as_mut() {
                    draft.step_players(more);
                }
            }
            PlayPress::Template(template) => {
                if let Some((draft, _)) = state.play.sheet.as_mut() {
                    draft.pick(template);
                }
            }
            PlayPress::Adjust => {
                if let Some((draft, _)) = state.play.sheet.as_mut() {
                    draft.adjust = !draft.adjust;
                }
            }
            PlayPress::Life(delta) => {
                if let Some((draft, _)) = state.play.sheet.as_mut() {
                    draft.life = (draft.life + delta).clamp(1, 999);
                }
            }
            PlayPress::Mulligans(more) => {
                if let Some((draft, _)) = state.play.sheet.as_mut() {
                    draft.mulligans = if more {
                        (draft.mulligans + 1).min(7)
                    } else {
                        draft.mulligans.saturating_sub(1)
                    };
                }
            }
            PlayPress::StepClock(later) => {
                let count = state.lobby.clocks().len();
                if let Some((draft, _)) = state.play.sheet.as_mut() {
                    draft.clock = model::step_clock(draft.clock, count, later);
                }
            }
            PlayPress::Open => {
                let Some((draft, editing)) = state.play.sheet.take() else {
                    return;
                };
                let request = if editing {
                    state.lobby.apply_table(&draft)
                } else {
                    state.room_setup_seat = None;
                    state.room_card_edit = None;
                    state.room_deck_seat = None;
                    state.lobby.open_table(&draft)
                };
                if request.is_none() {
                    // Refused (the status line says why): the sheet stays.
                    state.play.sheet = Some((draft, editing));
                } else {
                    // The name box is gone with the sheet; the caret must
                    // not stay in a field nothing draws.
                    state.lobby.focus_on(Field::Search);
                }
                dispatch(state, mailbox, request);
            }
            PlayPress::CloseSheet => {
                state.play.sheet = None;
                state.play.picker = false;
                state.play.password_for = None;
                if matches!(state.lobby.focus(), Field::RoomName | Field::RoomPassword) {
                    state.lobby.focus_on(Field::Search);
                }
            }
            PlayPress::ClearSearch => {
                state.lobby.set_field(Field::Search, "");
                let request = state.lobby.search_again();
                dispatch(state, mailbox, request);
            }
            PlayPress::PlayAgain(index) => {
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.rematch(&game);
                    dispatch(state, mailbox, request);
                }
            }
            PlayPress::Retry => {
                if !state.lobby.field(Field::Search).is_empty() {
                    state.lobby.set_field(Field::Search, "");
                }
                let request = state.lobby.search_again();
                dispatch(state, mailbox, request);
            }
        }
    }
}

/// Asks for the house decks once a signed-in player turns out to have none
/// of their own (Play's first run, Decks' empty shelf), or opens the house
/// tab without them: the list is the mini-tiles' and the tab's data.
pub(super) fn ask_for_house(
    mut state: ResMut<LobbyState>,
    mailbox: Res<Mailbox>,
    mut asked: Local<bool>,
) {
    if !state.is_changed() {
        return;
    }
    let lobby = &state.lobby;
    let signed_in = lobby.token().is_some() || lobby.offline();
    if !signed_in {
        *asked = false;
        return;
    }
    let want = matches!(lobby.screen(), Screen::Table)
        && lobby.awaiting().is_none()
        && !lobby.busy()
        && lobby.library().page.is_none()
        && lobby.library().house.is_empty()
        && (lobby.decks().is_empty() || state.decks.tab == super::decks::DecksTab::House);
    if want && !*asked {
        *asked = true;
        let request = state.lobby.browse_house();
        dispatch(&mut state, &mailbox, request);
    }
    if state.lobby.library().page.is_none() && state.decks.tab == super::decks::DecksTab::House {
        // Coming back to the house tab later asks again.
        *asked = false;
    }
}

/// Forgets having stepped away from a room once there is no room (M-7): the
/// next chair is shown its room.
pub(super) fn follow_the_room(mut state: ResMut<LobbyState>) {
    if state.play.password_for.as_ref().is_some_and(|id| {
        state.lobby.awaiting().is_some() || !state.lobby.games().iter().any(|game| &game.id == id)
    }) {
        state.play.password_for = None;
        state.lobby.focus_on(Field::Search);
    }
    if state.lobby.awaiting().is_some() {
        return;
    }
    if state.room_away {
        state.room_away = false;
    }
    if state.invite_copied || state.teams_edit || state.chair_sheet.is_some() {
        state.invite_copied = false;
        state.teams_edit = false;
        state.chair_sheet = None;
    }
}

/// Remembers a finished game for Recent games (this session), as the
/// lobby comes back from it.
pub(super) fn remember(
    state: &mut LobbyState,
    duel: Option<&crate::Duel>,
    result: baylee_engine::win::GameResult,
    seat: baylee_core::ids::PlayerId,
    team: Option<u8>,
) {
    let Screen::Seated(handover) = state.lobby.screen().clone() else {
        return;
    };
    let outcome = match client_core::interaction::outcome(&result, seat, team).won() {
        Some(true) => Outcome::Won,
        Some(false) => Outcome::Lost,
        None => Outcome::Drawn,
    };
    let names: Vec<String> = duel
        .and_then(|d| d.statics.as_ref())
        .map(|statics| {
            statics
                .seats
                .iter()
                .filter(|s| s.player != seat)
                .map(|s| s.display_name.clone())
                .collect()
        })
        .unwrap_or_default();
    let listed = state
        .lobby
        .games()
        .iter()
        .find(|g| g.id == handover.game_id)
        .cloned();
    let deck = state
        .lobby
        .next_deck()
        .map(|d| d.name.clone())
        .unwrap_or_default();
    let house = handover.local
        || listed.as_ref().is_some_and(|g| {
            g.seats
                .iter()
                .filter(|s| !s.you)
                .all(|s| s.kind == SeatKind::Ai)
        });
    state.lobby.remember_game(model::RecentGame {
        game_id: if handover.local {
            String::new()
        } else {
            handover.game_id.clone()
        },
        at: parts::local_hh_mm(),
        opponents: names,
        outcome,
        deck,
        chairs: listed.as_ref().map_or(2, |g| g.seats.len()),
        name: listed.map(|g| g.name).unwrap_or_default(),
        house,
    });
}

#[cfg(test)]
mod tests;
