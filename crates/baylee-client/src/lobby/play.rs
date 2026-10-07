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
use client_core::lobby::play::{
    self as model, Chip, Outcome, TableDraft, TableFilter, TableSort, Template,
};
use client_core::lobby::shelf;
use client_core::lobby::strips::{self, Strip};

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
    /// The sheet: a clock, by its place in the gateway's list.
    Clock(usize),
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
    let lane = commands
        .spawn((
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
                overflow: if phone {
                    Overflow::scroll_y()
                } else {
                    Overflow::visible()
                },
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(body).add_child(lane);
    let hero_more = hero(commands, lane, state, metrics, kit);
    if side && !phone {
        recent(commands, lane, state, kit);
    }
    let tables_more = tables(commands, body, state, metrics, kit, scrolled);
    if !side || phone {
        // Narrow and Phone: Recent games is a row under the list.
        recent(commands, body, state, kit);
    }
    let lang = state.lobby.lang();
    if let Some(hint) = parts::key_hint(
        commands,
        kit,
        &[
            ("Enter", Phrase::PlayHintJoin.text(lang)),
            ("/", Phrase::ShellKeySearch.text(lang)),
            ("c", Phrase::CreateTable.text(lang)),
        ],
    ) {
        commands
            .entity(hint)
            .entry::<Node>()
            .and_modify(|mut n| n.margin = UiRect::left(px(20)));
        commands.entity(root).add_child(hint);
    }
    menu(commands, root, state, kit, hero_more, tables_more);
    if state.play.picker {
        picker(commands, root, state, kit);
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

/// The hero: "Your next game".
#[allow(clippy::too_many_lines)] // one panel, read top to bottom
fn hero(
    commands: &mut Commands,
    parent: Entity,
    state: &LobbyState,
    metrics: Metrics,
    kit: Kit,
) -> HeroAnchors {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let phone = kit.m.frame == ShellFrame::Phone;
    let strip_hero = !phone && !matches!(kit.m.frame, ShellFrame::Wide | ShellFrame::Vast);
    let mut anchors = HeroAnchors::default();
    let panel = commands
        .spawn((
            Role::Panel,
            Node {
                width: percent(100),
                flex_direction: if strip_hero {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                },
                flex_wrap: if strip_hero {
                    FlexWrap::Wrap
                } else {
                    FlexWrap::NoWrap
                },
                align_items: if strip_hero {
                    AlignItems::Center
                } else {
                    AlignItems::Stretch
                },
                padding: UiRect::all(px_fixed(kit.m.pad)),
                row_gap: px_fixed(kit.m.gap),
                column_gap: px_fixed(kit.m.gap),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(tokens::BORDER),
            super::dock::Dock(3),
        ))
        .id();
    commands.entity(parent).add_child(panel);
    let off = starts_off(state, lang);
    let deck = lobby.next_deck();
    if deck.is_none() && !lobby.offline() {
        // First run (S-3): three house decks, one press to copy and use.
        let caption = parts::caption(commands, kit, Phrase::PlayPickToStart.text(lang));
        let minis = mini_tiles(commands, state, kit, &orders::PLAY, "minis");
        commands.entity(panel).add_children(&[caption, minis]);
    } else if !strip_hero && !phone {
        let caption = parts::caption(commands, kit, Phrase::PlayYourNextGame.text(lang));
        commands.entity(panel).add_child(caption);
    }
    if let Some(deck) = deck {
        let line = commands
            .spawn((
                Node {
                    column_gap: kit.m.px(12.0),
                    align_items: AlignItems::Start,
                    flex_shrink: 1.0,
                    min_width: px(0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let art_width = if phone || strip_hero { 78.0 } else { 164.0 };
        if let Some(art) = deck.art() {
            let picture = parts::art_with_credit(commands, kit, lang, &art, art_width);
            commands.entity(line).add_child(picture);
        } else {
            // No artist known: the identity gradient alone (principle 4).
            let band = commands
                .spawn((
                    Node {
                        width: kit.m.px(art_width),
                        height: kit.m.px(art_width * 457.0 / 626.0),
                        flex_shrink: 0.0,
                        border_radius: BorderRadius::all(px_fixed(6.0)),
                        ..default()
                    },
                    parts::identity_gradient(&deck.identity),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(line).add_child(band);
        }
        let words = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: kit.m.px(4.0),
                    flex_shrink: 1.0,
                    min_width: px(0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let name = commands
            .spawn((
                Text::new(deck.name.clone()),
                crate::hud::tf_bold(kit.fonts, if phone { kit.m.text } else { kit.m.head }),
                TextColor(tokens::INK),
                Pickable::IGNORE,
            ))
            .id();
        // On a phone and a narrow strip, `⋯` (Change deck · Edit) stands
        // beside the name (M4-6).
        let name_row = parts::row(commands, kit, false);
        commands.entity(name).insert(Node {
            flex_shrink: 1.0,
            min_width: px(0),
            ..default()
        });
        commands.entity(name_row).add_child(name);
        commands.entity(words).add_child(name_row);
        let pips = parts::row(commands, kit, true);
        let format = parts::badge(
            commands,
            kit,
            &shelf::format_label(lang, &deck.format),
            tokens::ACCENT,
        );
        commands.entity(pips).add_child(format);
        if phone {
            if !deck.identity.is_empty() {
                let letters = parts::badge(commands, kit, &deck.identity, tokens::INK);
                commands.entity(pips).add_child(letters);
            }
        } else if !deck.identity.is_empty() {
            let discs = parts::identity_discs(commands, kit, &deck.identity);
            commands.entity(pips).add_child(discs);
        }
        commands.entity(words).add_child(pips);
        if !phone {
            let meta = parts::line(
                commands,
                kit,
                &shelf::meta_line(lang, deck, parts::now_secs()),
                kit.m.small,
                tokens::MUTED,
            );
            commands.entity(words).add_child(meta);
        }
        if phone || strip_hero {
            let more = parts::icon_button(
                commands,
                kit,
                parts::ELLIPSIS,
                Press::Shared(SharedPress::OpenMenu(ShellMenu::Hero)),
            );
            orders::stop(commands, more, &orders::PLAY, "more");
            anchors.more = Some(more);
            commands.entity(name_row).add_child(more);
        } else {
            let links = parts::row(commands, kit, true);
            let change = controls::button(
                commands,
                kit,
                Phrase::PlayChangeDeck.text(lang),
                Weight::Ghost,
                Live::Yes,
                None,
                Press::Play(PlayPress::ChangeDeck),
            );
            let edit = controls::button(
                commands,
                kit,
                Phrase::ShellEdit.text(lang),
                Weight::Ghost,
                Live::Yes,
                None,
                Press::Play(PlayPress::EditNext),
            );
            orders::stop(commands, change, &orders::PLAY, "change-deck");
            orders::stop(commands, edit, &orders::PLAY, "edit");
            commands.entity(links).add_children(&[change, edit]);
            commands.entity(words).add_child(links);
        }
        commands.entity(line).add_child(words);
        commands.entity(panel).add_child(line);
    }

    // Return to your game: gold, the hero's primary while the player holds
    // a chair — not on a phone, where the header's pill is the way (S4-3).
    let playing = matches!(strips::strip(lobby), Some(Strip::Playing { .. }));
    if !phone && playing {
        let back = controls::button(
            commands,
            kit,
            Phrase::ShellReturnToGame.text(lang),
            Weight::Gold,
            Live::Yes,
            Some("r"),
            Press::Play(PlayPress::Return),
        );
        orders::stop(commands, back, &orders::PLAY, "return");
        commands.entity(panel).add_child(back);
    }
    let buttons = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(kit.m.gap),
                flex_grow: if strip_hero { 1.0 } else { 0.0 },
                min_width: if strip_hero { kit.m.px(260.0) } else { px(0) },
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(buttons);
    let house_live = match (&off, deck) {
        (Some(_), _) => Live::No(""),
        (None, None) => Live::No(Phrase::PlayPickToStart.text(lang)),
        (None, Some(_)) if lobby.busy() => Live::No(Phrase::VeilTalking.text(lang)),
        (None, Some(_)) => Live::Yes,
    };
    let house = controls::button(
        commands,
        kit,
        Phrase::PlayTheHouse.text(lang),
        Weight::Primary,
        house_live,
        None,
        Press::Play(PlayPress::PlayHouse),
    );
    let caret = parts::menu_button(
        commands,
        kit,
        super::ai_name(lang, difficulty(state.play.difficulty)),
        Weight::Secondary,
        Press::Shared(SharedPress::OpenMenu(ShellMenu::Difficulty)),
    );
    anchors.caret = Some(caret);
    orders::stop(commands, house, &orders::PLAY, "play-house");
    orders::stop(commands, caret, &orders::PLAY, "difficulty");
    let pair = commands
        .spawn((
            Node {
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Start,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(house)
        .entry::<Node>()
        .and_modify(|mut n| n.flex_grow = 1.0);
    commands.entity(pair).add_children(&[house, caret]);
    let create_live = if off.is_some() {
        Live::No("")
    } else if lobby.busy() {
        Live::No(Phrase::VeilTalking.text(lang))
    } else {
        Live::Yes
    };
    let create = controls::button(
        commands,
        kit,
        Phrase::CreateTable.text(lang),
        Weight::Secondary,
        create_live,
        Some("c"),
        Press::Play(PlayPress::CreateTable),
    );
    orders::stop(commands, create, &orders::PLAY, "create");
    commands.entity(buttons).add_children(&[pair, create]);
    // The reason stands once, under the pair (S4-3).
    if let Some(reason) = &off {
        let why = parts::line(commands, kit, reason, kit.m.small, tokens::GOLD);
        commands.entity(buttons).add_child(why);
    }
    let _ = metrics;
    anchors
}

/// Three house decks as mini tiles: art 78 × 57 with its credit, name,
/// format; one press copies and uses (S-3, through Decks' **Add and use**).
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
    for (index, deck) in house.iter().enumerate().take(3) {
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

/// The tables panel: head, chips, rows, pager. Answers the Sort button.
#[allow(clippy::too_many_lines)] // one panel, read top to bottom
fn tables(
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
            super::dock::Dock(4),
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
                    width: if phone { Val::Auto } else { kit.m.px(320.0) },
                    flex_grow: if phone { 1.0 } else { 0.0 },
                    flex_shrink: 1.0,
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

    // Chips and Sort: one row (on a phone, a row that scrolls sideways).
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
                    flex_shrink: 0.0,
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
        let gap = parts::grow(commands);
        let sort = parts::menu_button(
            commands,
            kit,
            &Phrase::DecksSortBy.fill(lang, &[state.play.sort.phrase().text(lang)]),
            Weight::Ghost,
            Press::Shared(SharedPress::OpenMenu(ShellMenu::TableSort)),
        );
        orders::stop(commands, sort, &orders::PLAY, "sort");
        sort_anchor = Some(sort);
        commands.entity(chips).add_children(&[gap, sort]);
        commands.entity(panel).add_child(chips);
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

    let list = commands
        .spawn((Node {
            width: percent(100),
            flex_grow: 1.0,
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: kit.m.px(4.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },))
        .id();
    commands.entity(panel).add_child(list);
    let order = model::table_order(lobby.games(), state.play.filter, state.play.sort);
    // A list scrolls only when it has rows to scroll.
    if !order.is_empty() {
        commands.entity(list).insert((
            Scrollable(List::Games),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::Games))),
        ));
    }
    if order.is_empty() {
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
    let mine_format = lobby
        .next_deck()
        .map(|d| d.format.clone())
        .unwrap_or_default();
    let mut walked = 0;
    for index in order {
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
    let row = commands
        .spawn((
            Role::Row,
            Node {
                width: percent(100),
                min_height: px_fixed(if kit.m.frame == ShellFrame::Phone {
                    60.0
                } else {
                    kit.m.row
                }),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                row_gap: kit.m.px(4.0),
                padding: UiRect::axes(kit.m.px(12.0), kit.m.px(6.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.03)),
            Pickable::IGNORE,
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
                row_gap: kit.m.px(2.0),
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
    if let Some(format) = model::host_format(game) {
        meta.push(shelf::format_label(lang, format));
    }
    meta.push(Phrase::RulesLife.fill(lang, &[&game.setup.starting_life.to_string()]));
    if let Some(clock) = game.clock {
        meta.push(model::table_clock_label(lang, &lobby.clocks(), clock));
    }
    let meta = parts::line(commands, kit, &meta.join(" · "), kit.m.small, tokens::MUTED);
    commands.entity(words).add_children(&[title, meta]);
    commands.entity(top).add_child(words);
    // The counts: seats, the lock, the AI.
    let counts = parts::row(commands, kit, false);
    let taken = game
        .seats
        .iter()
        .filter(|s| s.taken || s.kind == SeatKind::Ai)
        .count();
    let state_word = if game.state == "playing" {
        Phrase::PlayPlaying.text(lang).to_string()
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
    if wide {
        commands.entity(top).add_child(counts);
    }
    // The one action that applies.
    let mine = game.seated() || game.yours;
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
        let back = controls::button(
            commands,
            kit,
            Phrase::ShellReturnToGame.text(lang),
            Weight::Gold,
            Live::Yes,
            None,
            Press::Play(PlayPress::Return),
        );
        orders::item(commands, back, &orders::PLAY, "tables", *walked);
        *walked += 1;
        commands.entity(top).add_child(back);
    } else if game.joinable() {
        if game.locked && !held {
            let password = commands
                .spawn((
                    Node {
                        width: kit.m.px(160.0),
                        flex_shrink: 1.0,
                        min_width: px(80),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            let field = text_field(
                commands,
                kit.fonts,
                super::Metrics::of(1280.0),
                "",
                &FieldLook {
                    buffer: lobby.buffer(Field::RoomPassword),
                    focused: lobby.focus() == Field::RoomPassword && lobby.typing_here(),
                    mask: Some(Masked {
                        field: Some(Field::RoomPassword),
                        shown: lobby.showing(Field::RoomPassword),
                    }),
                    press: Press::Shared(SharedPress::Focus(Field::RoomPassword)),
                    lead: None,
                    hint: Some(Phrase::PlayPassword.text(lang)),
                    tail: None,
                },
            );
            commands.entity(password).add_child(field);
            commands.entity(top).add_child(password);
        }
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
    commands.entity(row).add_child(top);
    if !wide {
        // The second meta line (Narrow, Phone).
        commands.entity(row).add_child(counts);
    }
    if let Some(warning) = model::format_warning(lang, mine_format, game).filter(|_| !mine) {
        let line = parts::glyph_line(commands, kit, parts::WARNING, &warning, tokens::GOLD);
        commands.entity(row).add_child(line);
    }
    row
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
        }
        _ => {}
    }
}

/// The deck picker (Change deck): the shelf's tiles, one press chooses.
#[allow(clippy::too_many_lines)] // one sheet, drawn top to bottom
fn picker(commands: &mut Commands, root: Entity, state: &LobbyState, kit: Kit) {
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
        let tile = commands
            .spawn((
                Role::Tile,
                Node {
                    column_gap: kit.m.px(10.0),
                    align_items: AlignItems::Center,
                    padding: UiRect::all(kit.m.px(8.0)),
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
    let surface = surfaces::sheet_box(
        commands,
        kit,
        SheetWidth::Medium,
        Phrase::PlayChangeDeck.text(lang),
        &[grid],
        &[cancel],
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

/// The Create-table sheet (§5): name, players, template with its derived
/// line and Adjust, password, clock; Cancel · Open table (Apply over a
/// room).
#[allow(clippy::too_many_lines, clippy::too_many_arguments)] // one sheet, top to bottom
fn create_sheet(
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

    body.push(label(commands, Phrase::SheetTemplate.text(lang)));
    let cards = parts::row(commands, kit, true);
    for template in Template::ALL {
        let on = draft.template == template;
        let face = commands
            .spawn((
                Role::Segment,
                Node {
                    min_height: px_fixed(kit.m.control),
                    min_width: kit.m.px(140.0),
                    flex_grow: 1.0,
                    padding: UiRect::axes(kit.m.px(12.0), kit.m.px(8.0)),
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
        commands.entity(face).add_child(words);
        if on {
            let tick = commands
                .spawn((
                    Text::new(crate::hud::glyph::CHECK.to_string()),
                    crate::hud::icon_tf(kit.fonts, kit.m.small),
                    TextColor(tokens::ACCENT),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(face).add_child(tick);
        }
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
    if !lobby.offline() && !editing {
        body.push(label(commands, Phrase::SheetClock.text(lang)));
        let clocks = lobby.clocks();
        let labels: Vec<String> = clocks.iter().map(|c| model::clock_label(lang, c)).collect();
        let items: Vec<&str> = labels.iter().map(String::as_str).collect();
        let at = body.len();
        body.push(controls::segmented(
            commands,
            kit,
            &items,
            draft.clock.min(clocks.len().saturating_sub(1)),
            |i| Press::Play(PlayPress::Clock(i)),
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
                    state.pane = Pane::Deck;
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
                let game = state.lobby.games().get(index).map(|g| g.id.clone());
                if let Some(game) = game {
                    let request = state.lobby.join(&game);
                    dispatch(state, mailbox, request);
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
            PlayPress::Clock(at) => {
                if let Some((draft, _)) = state.play.sheet.as_mut() {
                    draft.clock = at;
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
