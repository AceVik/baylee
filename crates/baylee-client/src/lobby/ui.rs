//! The lobby's node tree, and the makers it is built from.
//!
//! Rebuilt whenever [`LobbyState`] changes and never otherwise, which is
//! why the scroll offsets and the hover preview live outside it.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

// -------------------------------------------------------------------- UI

/// Everything the lobby owns on screen, camera included.
#[derive(Component)]
pub(super) struct LobbyScreen;

/// The root of the rebuilt node tree.
#[derive(Component)]
pub(super) struct LobbyRoot;

/// The "leave table" button shown over a finished game.
#[derive(Component)]
pub(super) struct LeaveButton;

/// How much room there is, in three sizes.
///
/// Breakpoints rather than a continuous scale: what changes between a phone
/// and a desktop is the *shape* of the screen — one column or two, a card that
/// fills the width or one that floats — and shape does not interpolate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Frame {
    /// A phone held upright, or a very narrow window.
    Phone,
    /// A tablet, or a half-screen window.
    Tablet,
    /// A desktop window.
    Desktop,
}

impl Frame {
    /// The frame a window of this width is in.
    pub(crate) fn of(width: f32) -> Self {
        if width < 760.0 {
            Self::Phone
        } else if width < 1180.0 {
            Self::Tablet
        } else {
            Self::Desktop
        }
    }
}

/// Every size the layout takes from the frame, in one place.
#[derive(Clone, Copy)]
pub(crate) struct Metrics {
    pub(crate) frame: Frame,
    /// Body text.
    pub(crate) text: f32,
    /// Headings.
    pub(crate) head: f32,
    /// Captions and secondary lines.
    pub(crate) small: f32,
    /// The minimum height of anything meant to be tapped. 44 logical pixels
    /// is the smallest target a finger hits reliably.
    pub(crate) tap: f32,
    /// Padding around and inside panels.
    pub(crate) pad: f32,
    /// Gap between stacked controls.
    pub(crate) gap: f32,
}

impl Metrics {
    pub(crate) fn of(width: f32) -> Self {
        match Frame::of(width) {
            Frame::Phone => Self {
                frame: Frame::Phone,
                text: 15.0,
                head: 17.0,
                small: 12.0,
                tap: 48.0,
                pad: 14.0,
                gap: 12.0,
            },
            Frame::Tablet => Self {
                frame: Frame::Tablet,
                text: 14.0,
                head: 16.0,
                small: 11.5,
                tap: 44.0,
                pad: 16.0,
                gap: 10.0,
            },
            Frame::Desktop => Self {
                frame: Frame::Desktop,
                text: 13.0,
                head: 15.0,
                small: 11.0,
                tap: 38.0,
                pad: 18.0,
                gap: 9.0,
            },
        }
    }

    /// Whether the table screen stacks its two panels instead of pairing them.
    pub(crate) fn stacked(self) -> bool {
        self.frame == Frame::Phone
    }

    /// The width of the deck panel beside the table list.
    fn decks_width(self) -> Val {
        match self.frame {
            Frame::Phone => percent(100),
            Frame::Tablet => px(280),
            Frame::Desktop => px(330),
        }
    }
}

/// How much the lobby's ground moves.
///
/// Well under the loading veil's: this surface is behind text a player is
/// reading and typing into, and a backdrop that competes with the form is a
/// backdrop that has to be turned off. The veil, which is shown *instead* of
/// a screen, can afford to be the thing being looked at.
const AMBIENT_ENERGY: f32 = 0.35;

/// The lobby's own camera. The duel brings its own and the two never coexist:
/// this one is despawned on the way out of [`DuelPhase::Closed`], before the
/// stage is built.
pub(super) fn spawn_camera(
    mut commands: Commands,
    ambience: Option<ResMut<Assets<crate::ambience::AmbienceMaterial>>>,
) {
    commands.spawn((
        LobbyScreen,
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(BACKDROP),
            ..default()
        },
    ));
    // Spawned here rather than in `ui`, and this is the whole reason it is a
    // separate entity: the node tree is despawned and rebuilt on every state
    // change, and a material minted per rebuild would add one asset per
    // keystroke on the sign-in form. This one is made once per visit to the
    // lobby and torn down with the rest of `LobbyScreen`.
    //
    // `Option`, because a headless test has no render plugin and therefore no
    // `Assets` — the lobby's decisions are all tested that way.
    if let Some(mut ambience) = ambience {
        let backdrop = crate::ambience::backdrop(
            &mut commands,
            &mut ambience,
            BACKDROP,
            palette::ACCENT,
            AMBIENT_ENERGY,
            0.0,
        );
        // Under the screen, which is what the negative index says; the
        // loading veil spawns the same surface *inside* itself and must not.
        commands
            .entity(backdrop)
            .insert((LobbyScreen, GlobalZIndex(-1)));
    }
}

/// Drops the whole lobby when a duel takes the screen.
pub(super) fn teardown(mut commands: Commands, screen: Query<Entity, With<LobbyScreen>>) {
    for entity in &screen {
        commands.entity(entity).despawn();
    }
}

/// Rebuilds the node tree when the lobby changed, or when the window crossed
/// into a different frame.
///
/// The same retained-UI trick the HUD uses, with change detection standing in
/// for a revision struct. Resizing *within* a frame is left to flexbox — the
/// layout is written in percentages and gaps for exactly that reason.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
pub(super) fn ui(
    mut commands: Commands,
    state: Res<LobbyState>,
    scrolled_to: Res<Scrolled>,
    fonts: Option<Res<UiFonts>>,
    windows: Query<&Window>,
    root: Query<Entity, With<LobbyRoot>>,
    // Only the printing picker draws a remote image, and a headless test has
    // no asset server {2014} nor should it reach the CDN to build a tree.
    assets: Option<Res<AssetServer>>,
    ui_materials: Option<ResMut<UiCardMaterials>>,
    material_assets: Option<ResMut<Assets<CardUiMaterial>>>,
    prefs: Res<crate::prefs::Prefs>,
    mut drawn: Local<Option<Frame>>,
) {
    let mut cards = match (ui_materials, material_assets) {
        (Some(cache), Some(assets)) => Some((cache, assets)),
        _ => None,
    };
    let mut cards = cards.as_mut().map(|(cache, assets)| UiCards {
        cache: cache.as_mut(),
        assets: assets.as_mut(),
    });
    let width = windows
        .iter()
        .next()
        .map_or(1280.0, |w| w.resolution.width());
    let metrics = Metrics::of(width);
    if !state.is_changed()
        && !prefs.is_changed()
        && !root.is_empty()
        && *drawn == Some(metrics.frame)
    {
        return;
    }
    // The fonts are inserted by the duel plugin's startup system, so the first
    // frame or two has none. Leaving the tree empty until then is correct; the
    // `root.is_empty()` arm above brings us back.
    let Some(fonts) = fonts else {
        return;
    };
    for entity in &root {
        commands.entity(entity).despawn();
    }
    *drawn = Some(metrics.frame);

    let full_bleed =
        state.settings.is_open() || matches!(state.lobby.screen(), Screen::Table | Screen::Build);
    // A phone puts the sign-in form near the top instead of centring it: the
    // soft keyboard takes the bottom half of the screen, and a centred form
    // ends up underneath it.
    let top = full_bleed || metrics.frame == Frame::Phone;
    let root = commands
        .spawn((
            LobbyScreen,
            LobbyRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: if full_bleed {
                    AlignItems::Stretch
                } else {
                    AlignItems::Center
                },
                justify_content: if top {
                    JustifyContent::FlexStart
                } else {
                    JustifyContent::Center
                },
                padding: if full_bleed {
                    UiRect::ZERO
                } else {
                    UiRect::all(px(metrics.pad))
                },
                ..default()
            },
            // Transparent, so the drifting ground behind it shows. The
            // camera's clear colour is the same `BACKDROP`, which is what the
            // surface is drawn over — a solid fill here would hide it.
            BackgroundColor(Color::NONE),
        ))
        .id();

    // Settings sit over the lobby rather than beside it: they are the
    // account's, not the gateway's, and coming back has to land exactly where
    // the player left — including halfway through a deck.
    if state.settings.is_open() {
        crate::settingsui::screen(
            &mut commands,
            root,
            prefs.all(),
            state.settings.capturing(),
            state.lobby.token().is_some(),
            &fonts,
            metrics,
        );
        return;
    }

    match state.lobby.screen() {
        Screen::SignIn { registering } => {
            let panel = sign_in(&mut commands, &state, &fonts, metrics, *registering);
            commands.entity(root).add_child(panel);
        }
        Screen::Table => table(&mut commands, root, &state, &fonts, metrics, &scrolled_to),
        Screen::Build => crate::buildui::builder(
            &mut commands,
            root,
            &state,
            &fonts,
            metrics,
            &scrolled_to,
            assets.as_deref(),
            cards.as_mut(),
        ),
        Screen::Seated(_) => {
            let note = commands
                .spawn((
                    Text::new("taking your seat…"),
                    tf(&fonts, metrics.head),
                    TextColor(palette::MUTED),
                ))
                .id();
            commands.entity(root).add_child(note);
        }
    }
}

/// The sign-in card.
#[allow(clippy::too_many_lines)] // one flat form, read top to bottom
fn sign_in(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    registering: bool,
) -> Entity {
    let lobby = &state.lobby;
    let panel = commands
        .spawn((
            Node {
                // Fills a phone, floats on anything wider.
                width: percent(100),
                max_width: px(420),
                margin: if metrics.frame == Frame::Phone {
                    UiRect::top(px(metrics.pad * 2.0))
                } else {
                    UiRect::ZERO
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap),
                padding: UiRect::all(px(metrics.pad * 1.4)),
                border_radius: BorderRadius::all(px(12)),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            soft_shadow(),
        ))
        .id();

    let title = commands
        .spawn((
            Text::new("baylee"),
            tf(fonts, metrics.head * 1.8),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    let where_ = commands
        .spawn((
            Text::new(state.gateway.clone()),
            tf(fonts, metrics.small * 0.9),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(title);
    commands.entity(panel).add_child(where_);

    let email = text_field(
        commands,
        fonts,
        metrics,
        "E-MAIL",
        lobby.field(Field::Email),
        lobby.focus() == Field::Email,
        Field::Email,
    );
    commands.entity(panel).add_child(email);
    if registering {
        let name = text_field(
            commands,
            fonts,
            metrics,
            "DISPLAY NAME",
            lobby.field(Field::DisplayName),
            lobby.focus() == Field::DisplayName,
            Field::DisplayName,
        );
        commands.entity(panel).add_child(name);
    }
    let secret = "•".repeat(lobby.field(Field::Password).chars().count());
    let password = text_field(
        commands,
        fonts,
        metrics,
        "PASSWORD",
        &secret,
        lobby.focus() == Field::Password,
        Field::Password,
    );
    commands.entity(panel).add_child(password);

    let submit = button(
        commands,
        fonts,
        metrics,
        if registering {
            "Create account"
        } else {
            "Sign in"
        },
        Press::Submit,
        palette::ACCENT,
        !lobby.busy(),
    );
    commands.entity(panel).add_child(submit);

    if lobby.registration_enabled() || registering {
        let swap = button(
            commands,
            fonts,
            metrics,
            if registering {
                "I already have an account"
            } else {
                "Create an account"
            },
            Press::ToggleRegistering,
            palette::PANEL,
            true,
        );
        commands.entity(panel).add_child(swap);
    }

    let status = commands
        .spawn((
            Text::new(lobby.status()),
            tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(panel).add_child(status);

    let rule = commands
        .spawn((
            Node {
                width: percent(100),
                height: px(1),
                margin: UiRect::vertical(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.08)),
            Pickable::IGNORE,
        ))
        .id();
    let offline = button(
        commands,
        fonts,
        metrics,
        "Play the house AI offline",
        Press::PlayOffline,
        palette::PANEL,
        true,
    );
    // Reachable before signing in as well: an offline duel against the house
    // AI is played with the same keys, and a player with a keyboard they
    // cannot use is not going to make an account first.
    let settings = button(
        commands,
        fonts,
        metrics,
        "Settings",
        Press::OpenSettings,
        palette::PANEL,
        true,
    );
    commands.entity(panel).add_child(rule);
    commands.entity(panel).add_child(offline);
    commands.entity(panel).add_child(settings);
    panel
}

/// The signed-in screen: decks and tables, side by side or stacked.
#[allow(clippy::too_many_lines)] // two panels and a bar, built in order
fn table(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
) {
    let lobby = &state.lobby;
    let phone = metrics.frame == Frame::Phone;

    // ---- top bar
    let bar = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(metrics.tap + metrics.pad),
                align_items: AlignItems::Center,
                column_gap: px(metrics.gap),
                row_gap: px(6),
                flex_wrap: FlexWrap::Wrap,
                padding: UiRect::axes(px(metrics.pad), px(metrics.pad * 0.5)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
        ))
        .id();
    let brand = commands
        .spawn((
            Text::new("baylee"),
            tf(fonts, metrics.head * 1.2),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(bar).add_child(brand);
    // The gateway address is reassurance, not information, and the first thing
    // a narrow screen can do without.
    if !phone {
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
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let settings = button(
        commands,
        fonts,
        metrics,
        "Settings",
        Press::OpenSettings,
        palette::PANEL_LIT,
        true,
    );
    let out = button(
        commands,
        fonts,
        metrics,
        "Sign out",
        Press::SignOut,
        palette::PANEL_LIT,
        true,
    );
    commands.entity(bar).add_child(gap);
    commands.entity(bar).add_child(status);
    commands.entity(bar).add_child(settings);
    commands.entity(bar).add_child(out);
    commands.entity(root).add_child(bar);

    if let Some(handover) = lobby.awaiting() {
        let banner = commands
            .spawn((
                Node {
                    width: percent(100),
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(px(metrics.pad), px(metrics.pad * 0.5)),
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                Pickable::IGNORE,
            ))
            .id();
        let line = commands
            .spawn((
                Text::new(format!(
                    "your table {} is open — waiting for an opponent",
                    short_id(&handover.game_id)
                )),
                tf(fonts, metrics.small),
                TextColor(palette::ACTIVE),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(banner).add_child(line);
        commands.entity(root).add_child(banner);
    }

    // ---- body
    let body = commands
        .spawn((
            Node {
                width: percent(100),
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
                overflow: Overflow::scroll_y(),
                ..default()
            },
            Scrollable(List::Table),
            ScrollPosition(Vec2::new(0.0, scrolled_to.get(List::Table))),
        ))
        .id();
    commands.entity(root).add_child(body);

    // ---- decks
    let decks = panel(commands, metrics, metrics.decks_width(), 0.0);
    let decks_head = heading(commands, fonts, metrics, "Your decks");
    commands.entity(decks).add_child(decks_head);
    let deck_tools = row(commands, metrics, true);
    let new_deck = button(
        commands,
        fonts,
        metrics,
        "New deck",
        Press::NewDeck,
        palette::ACCENT,
        true,
    );
    let starter = button(
        commands,
        fonts,
        metrics,
        "Add the starter deck",
        Press::StarterDeck,
        palette::PANEL_LIT,
        !lobby.busy(),
    );
    commands.entity(deck_tools).add_child(new_deck);
    commands.entity(deck_tools).add_child(starter);
    commands.entity(decks).add_child(deck_tools);
    if lobby.decks().is_empty() {
        let empty = note(
            commands,
            fonts,
            metrics,
            "no decks yet — add the starter deck",
        );
        commands.entity(decks).add_child(empty);
    }
    for (index, deck) in lobby.decks().iter().enumerate() {
        let row = commands
            .spawn((
                Node {
                    width: percent(100),
                    min_height: px(metrics.tap),
                    align_items: AlignItems::Center,
                    column_gap: px(metrics.gap),
                    padding: UiRect::axes(px(metrics.pad * 0.7), px(metrics.pad * 0.4)),
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
            ))
            .id();
        let name = commands
            .spawn((
                Text::new(deck.name.clone()),
                tf(fonts, metrics.text),
                TextColor(palette::INK),
                Pickable::IGNORE,
            ))
            .id();
        let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
        let size = commands
            .spawn((
                Text::new(if deck.sideboard == 0 {
                    format!("{} rows", deck.cards)
                } else {
                    format!("{} + {}", deck.cards, deck.sideboard)
                }),
                tf(fonts, metrics.small),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        for child in [name, gap, size] {
            commands.entity(row).add_child(child);
        }
        // Nested inside a row that is itself a `Press`: `in_lineage` takes the
        // nearest one, so these win over selecting the deck.
        for (label, press) in [
            ("Edit", Press::EditDeck(index)),
            ("Delete", Press::DeleteDeck(index)),
        ] {
            let tool = chip(commands, fonts, metrics, label, press, false);
            commands.entity(row).add_child(tool);
        }
        commands.entity(decks).add_child(row);
    }
    commands.entity(body).add_child(decks);

    // ---- tables
    let games = panel(commands, metrics, percent(100), 1.0);
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
    let head = heading(commands, fonts, metrics, "Tables");
    commands.entity(head_row).add_child(head);
    if !phone {
        let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
        commands.entity(head_row).add_child(gap);
    }
    for (label, press, tone) in [
        ("Refresh", Press::Refresh, palette::PANEL_LIT),
        ("Play the house", Press::Host(GameMode::Ai), palette::ACCENT),
    ] {
        let b = button(commands, fonts, metrics, label, press, tone, !lobby.busy());
        commands.entity(head_row).add_child(b);
    }
    // One box, two uses: it locks a room the moment it is opened, and it is
    // what a locked room is joined with. They are never both wanted at once,
    // and two boxes a player has to tell apart would be worse than one that
    // says what it is for.
    let secret = "\u{2022}".repeat(lobby.room_password().chars().count());
    let lock = commands
        .spawn((
            Node {
                width: px(metrics.tap * 4.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let box_ = text_field(
        commands,
        fonts,
        metrics,
        "ROOM PASSWORD",
        &secret,
        lobby.focus() == Field::RoomPassword,
        Field::RoomPassword,
    );
    commands.entity(lock).add_child(box_);
    commands.entity(head_row).add_child(lock);
    // How many chairs is the one thing that cannot be changed after the
    // table exists, so it is asked before it does.
    for chairs in MIN_CHAIRS..=MAX_CHAIRS {
        let b = button(
            commands,
            fonts,
            metrics,
            &format!("Open a table for {chairs}"),
            Press::OpenRoom(chairs),
            palette::PANEL_LIT,
            !lobby.busy(),
        );
        commands.entity(head_row).add_child(b);
    }
    commands.entity(games).add_child(head_row);

    if lobby.games().is_empty() {
        let empty = note(commands, fonts, metrics, "no tables are open — start one");
        commands.entity(games).add_child(empty);
    }
    for (index, game) in lobby.games().iter().enumerate() {
        let row = commands
            .spawn((
                Node {
                    width: percent(100),
                    min_height: px(metrics.tap),
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
        let by = match &game.host {
            Some(host) => format!("{}  ·  {host}  ·  {}", game.state, host_note(game)),
            None => format!("{}  ·  {}", game.state, host_note(game)),
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
            let join = button(
                commands,
                fonts,
                metrics,
                "Join",
                Press::Join(index),
                palette::ACCENT,
                !lobby.busy(),
            );
            commands.entity(row).add_child(join);
        }
        if game.seated() && game.state == "waiting" {
            // Ready is the player's own statement and start is the host's:
            // two different buttons because they are two different claims,
            // and a host has to make both.
            let ready = game.i_am_ready();
            let say = button(
                commands,
                fonts,
                metrics,
                if ready { "Not ready" } else { "Ready" },
                Press::Ready(index, !ready),
                if ready {
                    palette::PANEL
                } else {
                    palette::ACCENT
                },
                !lobby.busy(),
            );
            commands.entity(row).add_child(say);
            if game.yours {
                let start = button(
                    commands,
                    fonts,
                    metrics,
                    "Start",
                    Press::StartRoom(index),
                    palette::ACCENT,
                    !lobby.busy() && game.startable,
                );
                commands.entity(row).add_child(start);
            }
            let leave = button(
                commands,
                fonts,
                metrics,
                // A host who leaves no longer closes the room — it passes to
                // whoever has been there longest — so the button says the
                // same thing for everyone.
                "Leave",
                Press::LeaveTable(index),
                palette::PANEL,
                !lobby.busy(),
            );
            commands.entity(row).add_child(leave);
        }
        commands.entity(games).add_child(row);

        // Its chairs, one row each. A room is arranged in the open, so this
        // is drawn for every table and not only for the one you are at.
        if game.state == "waiting" {
            let chairs = seat_rows(commands, fonts, metrics, game, index, lobby.busy());
            commands.entity(games).add_child(chairs);
        }
    }
    commands.entity(body).add_child(games);
}

/// How a table reads under its name: how full it is, and what it waits for.
///
/// Seated and ready are counted separately, because since a player has to say
/// they are ready the two answer different questions — a full table can still
/// be waiting for everyone in it.
fn host_note(game: &GameSummary) -> String {
    let total = game.seats.len();
    if game.state != "waiting" {
        return format!("{total} seats");
    }
    let seated = game
        .seats
        .iter()
        .filter(|s| s.taken || s.kind == SeatKind::Ai)
        .count();
    let waiting = game.seats.iter().filter(|s| !s.ready).count();
    let lock = if game.locked { " · locked" } else { "" };
    if waiting == 0 {
        format!("{seated}/{total} seated · ready{lock}")
    } else {
        format!("{seated}/{total} seated · waiting for {waiting}{lock}")
    }
}

/// One row per chair: who is in it, what they brought, and — for the host —
/// the controls that arrange it.
#[allow(clippy::too_many_lines)] // one chair, and everything offered on it
fn seat_rows(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    game: &GameSummary,
    index: usize,
    busy: bool,
) -> Entity {
    let holder = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                padding: UiRect::new(
                    px(metrics.pad * 1.4),
                    px(metrics.pad * 0.7),
                    px(2),
                    px(metrics.pad * 0.4),
                ),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for seat in &game.seats {
        let line = row(commands, metrics, true);
        let who = match (seat.kind, seat.player.as_deref()) {
            (SeatKind::Ai, _) => format!(
                "seat {} · AI ({})",
                seat.seat,
                seat.ai.as_deref().unwrap_or("steady")
            ),
            (SeatKind::Human, Some(name)) if seat.you => {
                format!("seat {} · {name} (you)", seat.seat)
            }
            (SeatKind::Human, Some(name)) => format!("seat {} · {name}", seat.seat),
            (SeatKind::Human, None) => format!("seat {} · open", seat.seat),
        };
        let label = commands
            .spawn((
                Text::new(who),
                tf(fonts, metrics.small),
                TextColor(if seat.ready {
                    palette::INK
                } else {
                    palette::MUTED
                }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(line).add_child(label);
        if !seat.deck.is_empty() {
            let deck = note(commands, fonts, metrics, &seat.deck);
            commands.entity(line).add_child(deck);
        }
        let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
        commands.entity(line).add_child(gap);

        // A player brings their own deck; the host brings an AI's. The
        // gateway checks both again — this only decides what to offer.
        let mine = seat.you;
        let ai_chair = seat.kind == SeatKind::Ai;
        if mine || (game.yours && ai_chair) {
            let set = chip(
                commands,
                fonts,
                metrics,
                "use my deck",
                Press::SeatDeck(index, seat.seat),
                false,
            );
            commands.entity(line).add_child(set);
        }
        // Only the host arranges chairs, and never one somebody is sitting in.
        if game.yours && (mine || !seat.taken) {
            let (label, press) = if ai_chair {
                (
                    "\u{2192} open",
                    Press::SeatKind(index, seat.seat, SeatKind::Human),
                )
            } else {
                (
                    "\u{2192} AI",
                    Press::SeatKind(index, seat.seat, SeatKind::Ai),
                )
            };
            // The host's own chair is theirs as a player, not as the host:
            // handing it to the AI would seat them out of their own table.
            if !mine {
                let swap = chip(commands, fonts, metrics, label, press, false);
                commands.entity(line).add_child(swap);
            }
            if ai_chair {
                for name in ["novice", "steady", "sharp"] {
                    let lit = seat.ai.as_deref() == Some(name);
                    let pick = chip(
                        commands,
                        fonts,
                        metrics,
                        name,
                        Press::SeatAi(index, seat.seat, name),
                        lit,
                    );
                    commands.entity(line).add_child(pick);
                }
            }
        }
        // The room can be handed to anyone else who is sitting at it, which
        // is also how a host leaves without taking the table with them.
        if game.yours && !mine && seat.taken && seat.kind == SeatKind::Human {
            let pass = chip(
                commands,
                fonts,
                metrics,
                "make host",
                Press::HandOver(index, seat.seat),
                false,
            );
            commands.entity(line).add_child(pass);
        }
        // A free chair is one anyone else can take, by name rather than by
        // whichever one the gateway would have picked.
        if seat.open() && !game.seated() && !busy {
            let sit = chip(
                commands,
                fonts,
                metrics,
                "sit here",
                Press::JoinSeat(index, seat.seat),
                false,
            );
            commands.entity(line).add_child(sit);
        }
        commands.entity(holder).add_child(line);
    }
    holder
}

/// A wrapping row of controls.
pub(crate) fn row(commands: &mut Commands, metrics: Metrics, wrap: bool) -> Entity {
    commands
        .spawn((
            Node {
                width: percent(100),
                align_items: AlignItems::Center,
                column_gap: px(metrics.gap * 0.5),
                row_gap: px(metrics.gap * 0.5),
                flex_wrap: if wrap {
                    FlexWrap::Wrap
                } else {
                    FlexWrap::NoWrap
                },
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// A list that scrolls inside its panel rather than pushing it off screen.
///
/// Deliberately *not* `Pickable::IGNORE`: a wheel over the gap between two
/// rows has to land on something, and [`scrolls`] walks up from whatever the
/// pointer hit to find this.
pub(crate) fn scroller(commands: &mut Commands, metrics: Metrics, which: List, at: f32) -> Entity {
    commands
        .spawn((
            Scrollable(which),
            // Not implied by the overflow: Bevy reads this component when it
            // has one and never adds it, so a list without it clips its rows
            // away and nothing can bring them back. It is seeded from where
            // the player left this list, because adding a card rebuilds the
            // tree and a list that jumped to the top on every tap would be
            // unusable.
            ScrollPosition(Vec2::new(0.0, at)),
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap * 0.35),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id()
}

/// A small toggle. Same shape as [`button`], sized for a row of them.
pub(crate) fn chip(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    press: Press,
    on: bool,
) -> Entity {
    let text = commands
        .spawn((
            Text::new(label),
            tf(fonts, metrics.small),
            TextColor(if on { palette::INK } else { palette::MUTED }),
            Pickable::IGNORE,
        ))
        .id();
    let id = commands
        .spawn((
            Node {
                // Still a finger target on a phone: the chips are the busiest
                // controls on the screen, and a 30px one is a mis-tap.
                min_height: px(metrics.tap * 0.8),
                min_width: px(metrics.tap * 0.8),
                padding: UiRect::axes(px(metrics.pad * 0.6), px(2)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(if on {
                palette::ACCENT
            } else {
                palette::PANEL_LIT
            }),
            press,
            crate::ambience::Feel::new(if on {
                palette::ACCENT
            } else {
                palette::PANEL_LIT
            }),
        ))
        .id();
    commands.entity(id).add_child(text);
    id
}

/// A labelled text box that takes the caret when tapped, addressed by a
/// [`Press`] of the caller's choosing.
///
/// [`text_field`] is the same control bound to the sign-in form's [`Field`];
/// this one serves the builder's two boxes.
pub(crate) fn text_box(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    value: &str,
    focused: bool,
    press: Press,
) -> Entity {
    let column = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let caption = commands
        .spawn((
            Text::new(label),
            tf(fonts, metrics.small * 0.8),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let text = commands
        .spawn((
            Text::new(if focused {
                format!("{value}▏")
            } else {
                value.to_string()
            }),
            tf(fonts, metrics.text),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    let boxed = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(metrics.tap),
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(metrics.pad * 0.7), px(6)),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            BorderColor::all(if focused {
                palette::ACCENT
            } else {
                Color::srgba(1.0, 1.0, 1.0, 0.08)
            }),
            press,
        ))
        .id();
    commands.entity(boxed).add_child(text);
    commands.entity(column).add_child(caption);
    commands.entity(column).add_child(boxed);
    column
}

/// The head of an opaque game id — enough to tell two tables apart, and short
/// enough to fit on a phone.
fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

/// The "leave table" button, over a game that has ended.
pub(super) fn spawn_leave_button(
    mut commands: Commands,
    fonts: Option<Res<UiFonts>>,
    windows: Query<&Window>,
) {
    let Some(fonts) = fonts else {
        return;
    };
    let width = windows
        .iter()
        .next()
        .map_or(1280.0, |w| w.resolution.width());
    let metrics = Metrics::of(width);
    let holder = commands
        .spawn((
            LeaveButton,
            Node {
                position_type: PositionType::Absolute,
                top: px(64),
                width: percent(100),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let leave = button(
        &mut commands,
        &fonts,
        metrics,
        "Back to the lobby",
        Press::Leave,
        palette::ACCENT,
        true,
    );
    commands.entity(holder).add_child(leave);
}

/// Removes it again on the way out.
pub(super) fn despawn_leave_button(
    mut commands: Commands,
    buttons: Query<Entity, With<LeaveButton>>,
) {
    for entity in &buttons {
        commands.entity(entity).despawn();
    }
}

// ----------------------------------------------------------- node makers

/// A labelled text box that takes the caret when tapped.
pub(crate) fn text_field(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    value: &str,
    focused: bool,
    field: Field,
) -> Entity {
    let column = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let caption = commands
        .spawn((
            Text::new(label),
            tf(fonts, metrics.small * 0.8),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    // The caret is drawn into the string: one glyph is cheaper than a second
    // node, and the lobby has no text selection to speak of.
    let shown = if focused {
        format!("{value}▏")
    } else {
        value.to_string()
    };
    let text = commands
        .spawn((
            Text::new(shown),
            tf(fonts, metrics.text),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    let boxed = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(metrics.tap),
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(metrics.pad * 0.7), px(6)),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            BorderColor::all(if focused {
                palette::ACCENT
            } else {
                Color::srgba(1.0, 1.0, 1.0, 0.08)
            }),
            Press::Focus(field),
        ))
        .id();
    commands.entity(boxed).add_child(text);
    commands.entity(column).add_child(caption);
    commands.entity(column).add_child(boxed);
    column
}

/// A button. A disabled one carries no [`Press`], so a click cannot find it.
pub(crate) fn button(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    press: Press,
    tone: Color,
    enabled: bool,
) -> Entity {
    let text = commands
        .spawn((
            Text::new(label),
            tf(fonts, metrics.text),
            TextColor(if enabled { palette::INK } else { palette::DEAD }),
            Pickable::IGNORE,
        ))
        .id();
    let id = {
        let mut entity = commands.spawn((
            Node {
                min_height: px(metrics.tap),
                padding: UiRect::axes(px(metrics.pad), px(metrics.pad * 0.45)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(if enabled { tone } else { palette::PANEL }),
            soft_shadow(),
        ));
        if enabled {
            // The tone travels with the button because the animation writes
            // `BackgroundColor` every frame: after one hover the node no
            // longer knows what colour it started at.
            entity.insert((press, crate::ambience::Feel::new(tone)));
        }
        entity.id()
    };
    commands.entity(id).add_child(text);
    id
}

/// A column panel: a fixed width beside its neighbour, or the full width
/// above it.
pub(crate) fn panel(commands: &mut Commands, metrics: Metrics, width: Val, grow: f32) -> Entity {
    commands
        .spawn((
            Node {
                width,
                flex_grow: grow,
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap * 0.8),
                padding: UiRect::all(px(metrics.pad * 0.8)),
                border_radius: BorderRadius::all(px(12)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            Pickable::IGNORE,
        ))
        .id()
}

/// A panel heading.
pub(crate) fn heading(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
) -> Entity {
    commands
        .spawn((
            Text::new(label),
            tf(fonts, metrics.head),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id()
}

/// A muted line where a list would be.
pub(crate) fn note(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
) -> Entity {
    commands
        .spawn((
            Text::new(label),
            tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id()
}

/// A deck row's printing, short enough to sit at the end of a list line.
///
/// Not the row's own text form: that repeats the count and the name, both of
/// which are already on the line, and it would be the widest thing on it.
pub(crate) fn print_mark(print: &baylee_core::deckrow::PrintChoice) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(set) = &print.set {
        parts.push(match &print.collector_number {
            Some(number) => format!("{set} {number}"),
            None => set.clone(),
        });
    } else if print.scryfall_id.is_some() {
        // A row pinned to one printing by id has nothing readable to show; it
        // still must not look like the plain row next to it.
        parts.push("pinned".to_string());
    }
    if let Some(lang) = &print.lang {
        parts.push(lang.to_uppercase());
    }
    match print.finish {
        Some(Finish::Foil) => parts.push("foil".to_string()),
        Some(Finish::Etched) => parts.push("etched".to_string()),
        Some(Finish::Normal) | None => {}
    }
    parts.join(" \u{b7} ")
}

/// The stretch between the left and right halves of a row.
pub(crate) fn spacer() -> Node {
    Node {
        flex_grow: 1.0,
        ..default()
    }
}
