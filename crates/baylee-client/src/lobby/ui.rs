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

/// The bar the ways out fall back to when the duel drew no end screen.
///
/// Only ever on a node this module *made*, which is what keeps the teardown
/// honest: the buttons normally live inside the duel's sheet and are taken
/// down with it, and a marker that also sat on the duel's own row would have
/// this module despawning an entity the duel is about to despawn again.
#[derive(Component)]
pub(super) struct LeaveButton;

/// One way out of a finished game, wherever it ended up standing.
///
/// The "are these already placed" question, asked of the buttons rather than
/// of their holder for the same reason: the holder may be the duel's.
///
/// `pub(crate)` for the probe alone: `devctl`'s `exits` row reports whether
/// each on-screen `Press` also carries this, because
/// `systems::leave_keys` filters by it while `leave_clicks`
/// walks the clicked entity's ancestry, and a caller cannot otherwise tell a
/// way out the keyboard is blind to from one that is not there (#135).
#[derive(Component)]
pub(crate) struct DuelExit;

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
                tap: 44.0,
                pad: 14.0,
                gap: 12.0,
            },
            Frame::Tablet => Self {
                frame: Frame::Tablet,
                text: 15.0,
                head: 18.0,
                small: 11.5,
                tap: 38.0,
                pad: 16.0,
                gap: 10.0,
            },
            Frame::Desktop => Self {
                frame: Frame::Desktop,
                text: 15.0,
                head: 20.0,
                small: 12.0,
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
    vista: Option<ResMut<Assets<crate::vista::VistaMaterial>>>,
) {
    commands.spawn((
        LobbyScreen,
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(BACKDROP),
            ..default()
        },
        // UI glyphs and rounded borders already carry coverage antialiasing.
        // Avoid a multisample resolve across two full-screen shader surfaces.
        Msaa::Off,
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
            Color::srgb(0.28, 0.49, 0.9),
            AMBIENT_ENERGY,
            0.0,
        );
        // Under the screen, which is what the negative index says; the
        // loading veil spawns the same surface *inside* itself and must not.
        commands
            .entity(backdrop)
            .insert((LobbyScreen, GlobalZIndex(-2)));
    }
    // The front door's scene (#295), over the field and under the screen. It
    // hides itself once the front door has gone, and the field is the lobby's
    // ground again.
    if let Some(mut vista) = vista {
        let scene = crate::vista::surface(&mut commands, &mut vista, crate::vista::Vista::Front);
        commands
            .entity(scene)
            .insert((LobbyScreen, GlobalZIndex(-1)));
    }
}

/// Drops the whole lobby when a duel takes the screen.
///
/// The veil goes with it. `waiting` raises it while a seat is being fetched
/// and only that system ever lowers it — and that system stops running the
/// moment the duel opens, which is exactly when the seat has arrived. Left
/// alone, "Taking your seat" sat over the table for the rest of the game.
pub(super) fn teardown(
    mut commands: Commands,
    mut loading: ResMut<crate::loading::Loading>,
    screen: Query<Entity, With<LobbyScreen>>,
) {
    loading.clear();
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
#[allow(clippy::too_many_lines)] // one screen per arm, and nothing else
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
    cast: Res<super::front::FrontCast>,
    mut drawn: Local<Option<Frame>>,
    mut builder_drawn: Local<Option<crate::buildui::Retained>>,
) {
    let width = windows
        .iter()
        .next()
        .map_or(1280.0, |w| w.resolution.width());
    let metrics = Metrics::of(width);
    if !state.is_changed()
        && !prefs.is_changed()
        && !cast.is_changed()
        && !root.is_empty()
        && *drawn == Some(metrics.frame)
    {
        return;
    }
    let mut cards = match (ui_materials, material_assets) {
        (Some(cache), Some(assets)) => Some((cache, assets)),
        _ => None,
    };
    let mut cards = cards.as_mut().map(|(cache, assets)| UiCards {
        cache: cache.as_mut(),
        assets: assets.as_mut(),
    });
    // The fonts are inserted by the duel plugin's startup system, so the first
    // frame or two has none. Leaving the tree empty until then is correct; the
    // `root.is_empty()` arm above brings us back.
    let Some(fonts) = fonts else {
        return;
    };
    if state.lobby.screen() == &Screen::Build
        && !state.settings.is_open()
        && state.lobby.library().page.is_none()
        && state.confirmation.is_none()
        && !prefs.is_changed()
        && *drawn == Some(metrics.frame)
        && metrics.frame != Frame::Phone
        && !root.is_empty()
        && let Some(cached) = builder_drawn.as_mut()
    {
        cached.patch(
            &mut commands,
            &state,
            &fonts,
            metrics,
            &scrolled_to,
            assets.as_deref(),
            cards.as_mut(),
        );
        return;
    }
    *builder_drawn = None;
    for entity in &root {
        commands.entity(entity).despawn();
    }
    *drawn = Some(metrics.frame);

    let full_bleed = true;
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
                overflow: Overflow::scroll_y(),
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
            state.lobby.lang(),
            &fonts,
            metrics,
        );
        super::confirm::draw_deletion(&mut commands, root, &state, &fonts, metrics);
        return;
    }

    if state.lobby.library().page.is_some() {
        super::library_ui::screen(&mut commands, root, &state, &fonts, metrics, &scrolled_to);
        return;
    }
    match state.lobby.screen() {
        Screen::SignIn { .. } => {
            commands.entity(root).insert((
                Scrollable(List::Table),
                ScrollPosition(Vec2::new(0.0, scrolled_to.get(List::Table))),
            ));
            front_door(
                &mut commands,
                root,
                &state,
                &cast,
                &fonts,
                metrics,
                &scrolled_to,
            );
        }
        Screen::Table => table(&mut commands, root, &state, &fonts, metrics, &scrolled_to),
        Screen::Build => {
            *builder_drawn = Some(crate::buildui::builder(
                &mut commands,
                root,
                &state,
                &fonts,
                metrics,
                &scrolled_to,
                assets.as_deref(),
                cards.as_mut(),
            ));
        }
        Screen::Seated(_) => {
            let note = commands
                .spawn((
                    Text::new(Phrase::TakingYourSeat.text(state.lobby.lang())),
                    tf(&fonts, metrics.head),
                    TextColor(palette::MUTED),
                ))
                .id();
            commands.entity(root).add_child(note);
        }
    }
    super::confirm::draw(&mut commands, root, &state, &fonts, metrics);
    if state.confirmation.is_some() {
        *builder_drawn = None;
    }
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
    let lang = lobby.lang();
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
        Press::OpenSettings,
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
    let music = music_toggle(commands, fonts, metrics, lang);
    commands.entity(bar).add_child(gap);
    commands.entity(bar).add_child(status);
    commands.entity(bar).add_child(music);
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
        padding: UiRect::axes(px(metrics.pad), px(6)),
        column_gap: px(metrics.gap),
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
    commands.entity(root).add_child(navigation);
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
    commands.entity(deck_tools).add_child(new_deck);
    commands.entity(decks).add_child(deck_tools);
    let deck_grid = row(commands, metrics, true);
    commands.entity(decks).add_child(deck_grid);
    if lobby.decks().is_empty() {
        let empty = note(commands, fonts, metrics, Phrase::NoOwnDecks.text(lang));
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
                Text::new(Phrase::DeckRows.fill(
                    lang,
                    &[&deck.cards.to_string(), &deck.sideboard.to_string()],
                )),
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
    if !phone {
        let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
        commands.entity(head_row).add_child(gap);
    }
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
        commands.entity(head_row).add_child(hunt);
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
        commands.entity(head_row).add_child(b);
    }
    commands.entity(games).add_child(head_row);
    let creation = row(commands, metrics, true);
    let play = button(
        commands,
        fonts,
        metrics,
        Phrase::PlayTheHouse.text(lang),
        Press::Host(GameMode::Ai),
        palette::ACCENT,
        !lobby.busy() && lobby.selected().is_some(),
    );
    commands.entity(creation).add_child(play);
    // One box, two uses: it locks a room the moment it is opened, and it is
    // what a locked room is joined with. They are never both wanted at once,
    // and two boxes a player has to tell apart would be worse than one that
    // says what it is for.
    if !alone {
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
            Phrase::RoomPassword.text(lang),
            &FieldLook {
                buffer: lobby.buffer(Field::RoomPassword),
                focused: lobby.focus() == Field::RoomPassword,
                mask: Some(Masked {
                    field: Field::RoomPassword,
                    shown: lobby.showing(Field::RoomPassword),
                }),
                press: Press::Focus(Field::RoomPassword),
                lead: None,
                hint: None,
                tail: None,
            },
        );
        commands.entity(lock).add_child(box_);
        commands.entity(creation).add_child(lock);
    }
    let fewer = button(
        commands,
        fonts,
        metrics,
        "−",
        Press::RoomSize(false),
        palette::PANEL_LIT,
        state.room_chairs > MIN_CHAIRS,
    );
    let size = note(
        commands,
        fonts,
        metrics,
        &Phrase::PlayerCount.fill(lang, &[&state.room_chairs.to_string()]),
    );
    let more = button(
        commands,
        fonts,
        metrics,
        "+",
        Press::RoomSize(true),
        palette::PANEL_LIT,
        state.room_chairs < MAX_CHAIRS,
    );
    let create = button(
        commands,
        fonts,
        metrics,
        Phrase::CreateTable.text(lang),
        Press::OpenRoom(state.room_chairs),
        palette::PANEL_LIT,
        !lobby.busy() && lobby.selected().is_some(),
    );
    commands
        .entity(creation)
        .add_children(&[fewer, size, more, create]);
    commands.entity(games).add_child(creation);

    if lobby.games().is_empty() {
        // An empty lobby and an empty search are different news: one says
        // open a table, the other says the tables are elsewhere.
        let hunt = lobby.field(Field::Search).trim();
        let said = if hunt.is_empty() {
            Phrase::NoTablesOpen.text(lang).to_string()
        } else {
            Phrase::NoTableMatches.fill(lang, &[hunt])
        };
        let empty = note(commands, fonts, metrics, &said);
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
        if game.seated() && game.state == "waiting" {
            // Ready is the player's own statement and start is the host's:
            // two different buttons because they are two different claims,
            // and a host has to make both.
            //
            // A chair at a rematch room is a third thing again: it is being
            // *kept* for this player, and claiming it is what says they are
            // ready. Pressing ready there would be answered `200` and leave
            // the chair exactly as unready as it was, so the button has to be
            // the other one until the chair is claimed.
            let ready = game.i_am_ready();
            let say = if game.rematch && !ready {
                button(
                    commands,
                    fonts,
                    metrics,
                    Phrase::PlayAgain.text(lang),
                    Press::Rematch(index),
                    palette::ACCENT,
                    !lobby.busy(),
                )
            } else {
                button(
                    commands,
                    fonts,
                    metrics,
                    if ready {
                        Phrase::NotReady.text(lang)
                    } else {
                        Phrase::Ready.text(lang)
                    },
                    Press::Ready(index, !ready),
                    if ready {
                        palette::PANEL
                    } else {
                        palette::ACCENT
                    },
                    !lobby.busy(),
                )
            };
            commands.entity(row).add_child(say);
            if game.yours {
                let start = button(
                    commands,
                    fonts,
                    metrics,
                    Phrase::Start.text(lang),
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
                Phrase::Leave.text(lang),
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
            let chairs = seat_rows(commands, fonts, metrics, lang, game, index, lobby.busy());
            commands.entity(games).add_child(chairs);
        }
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
fn host_note(lang: Lang, game: &GameSummary) -> String {
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

/// One row per chair: who is in it, what they brought, and — for the host —
/// the controls that arrange it.
#[allow(clippy::too_many_lines)] // one chair, and everything offered on it
fn seat_rows(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
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
        let chair = seat.seat.to_string();
        let who = match (seat.kind, seat.player.as_deref()) {
            (SeatKind::Ai, _) => Phrase::SeatAi.fill(
                lang,
                &[
                    &chair,
                    ai_name(lang, seat.ai.as_deref().unwrap_or("steady")),
                ],
            ),
            (SeatKind::Human, Some(name)) if seat.you => {
                Phrase::SeatYours.fill(lang, &[&chair, name])
            }
            (SeatKind::Human, Some(name)) => Phrase::SeatTaken.fill(lang, &[&chair, name]),
            (SeatKind::Human, None) => Phrase::SeatOpen.fill(lang, &[&chair]),
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
                Phrase::UseMyDeck.text(lang),
                Press::SeatDeck(index, seat.seat),
                false,
            );
            commands.entity(line).add_child(set);
        }
        // Only the host arranges chairs, and never one somebody is sitting in.
        if game.yours && (mine || !seat.taken) {
            let (label, press) = if ai_chair {
                (
                    Phrase::SeatToOpen.text(lang),
                    Press::SeatKind(index, seat.seat, SeatKind::Human),
                )
            } else {
                (
                    Phrase::SeatToAi.text(lang),
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
                for (name, _) in baylee_core::preset::AIProfile::NAMED {
                    let lit = seat.ai.as_deref() == Some(name);
                    let pick = chip(
                        commands,
                        fonts,
                        metrics,
                        ai_name(lang, name),
                        Press::SeatAi(index, seat.seat, name),
                        lit,
                    );
                    commands.entity(line).add_child(pick);
                }
            }
        }
        // Which side the chair plays for, cycled by the host: nothing, then
        // team 1, 2, … and back. One chip rather than one per team, because
        // a table of eight would otherwise carry nine buttons per row for a
        // setting most tables never touch.
        if game.yours {
            let seats = game.seats.len() as u8;
            let next = seat.team.map_or(1, |t| if t >= seats { 0 } else { t + 1 });
            let label = match seat.team {
                None => Phrase::SeatSideNone.text(lang).to_string(),
                Some(team) => Phrase::SeatSide.fill(lang, &[&team.to_string()]),
            };
            let side = chip(
                commands,
                fonts,
                metrics,
                &label,
                Press::SeatTeam(index, seat.seat, next),
                seat.team.is_some(),
            );
            commands.entity(line).add_child(side);
        }
        // The room can be handed to anyone else who is sitting at it, which
        // is also how a host leaves without taking the table with them.
        if game.yours && !mine && seat.taken && seat.kind == SeatKind::Human {
            let pass = chip(
                commands,
                fonts,
                metrics,
                Phrase::MakeHost.text(lang),
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
                Phrase::SitHere.text(lang),
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
    let height = if metrics.frame == Frame::Phone {
        metrics.tap
    } else {
        metrics.tap * 0.8
    };
    let id = crate::hud::answer_sized(
        commands,
        fonts,
        label,
        crate::hud::ButtonWeight::Secondary,
        None,
        height,
        metrics.small,
    );
    if on {
        super::button_style::primary(commands, id);
    }
    super::button_style::icon(commands, fonts, id, press, metrics.small);
    commands.entity(id).insert(press);
    id
}

/// The head of an opaque game id — enough to tell two tables apart, and short
/// enough to fit on a phone.
fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

/// The ways out of a finished game, put in the row the duel's end screen left
/// for them.
///
/// An `Update` system and not an `OnEnter` one, and that is the whole of the
/// seam. `hud::spawn_finish` runs on the same edge, its `Commands` are applied
/// at the end of that schedule, and a system merely ordered *after* it would
/// query a row that does not exist yet — while an explicit sync point between
/// two plugins that do not know each other is exactly the coupling the marker
/// exists to avoid. So this runs every frame the game is over and stops the
/// moment its buttons are standing.
///
/// If there is no such row — the screen draws nothing without a roster, and an
/// embedder may have its own — the buttons fall back to a bar of their own
/// over the board, which is where they lived before the screen existed. A
/// player with no way out of a finished table is the one outcome worth a
/// fallback.
pub(super) fn spawn_leave_button(
    mut commands: Commands,
    state: Res<LobbyState>,
    fonts: Option<Res<UiFonts>>,
    placed: Query<Entity, With<DuelExit>>,
    exits: Query<Entity, With<crate::hud::FinishExits>>,
) {
    let Some(fonts) = fonts else {
        return;
    };
    if !placed.is_empty() {
        return;
    }
    let lang = state.lobby.lang();
    // Play again first, because it is what most players want and the one that
    // needs the other three still at the table. Only for a game reached
    // through the gateway: an offline duel against the house has no table to
    // ask for another of, and the request would have no account to make it.
    //
    // `local` and not merely `Seated`, which is what this asked before and
    // which was wrong the whole time: an offline duel is seated too
    // (`systems::poll` reads `Screen::Seated(handover)` and branches on
    // `handover.local`), so playing the house put a *play again* over the
    // finished game that would have asked a gateway for another of a table
    // it has never heard of. It was floating over the board where nobody
    // looked; the end screen put it in the middle of the sheet.
    let networked = matches!(state.lobby.screen(), Screen::Seated(handover) if !handover.local);
    let mut ways: Vec<(&str, Press)> = Vec::new();
    if networked {
        ways.push((Phrase::PlayAgain.text(lang), Press::PlayAgain));
    }
    ways.push((Phrase::BackToLobby.text(lang), Press::Leave));

    // In the sheet these are the slip's own answers, so they obey the slip's
    // own rule: the first one is what the sheet is *for* and is the only one
    // in brass. That makes the lone "back to the lobby" of an offline duel a
    // lead answer, which is right — there is nothing left for it to be
    // quieter than.
    let holder = exits.iter().next().unwrap_or_else(|| {
        commands
            .spawn((
                LeaveButton,
                Node {
                    position_type: PositionType::Absolute,
                    top: px(64),
                    width: percent(100),
                    justify_content: JustifyContent::Center,
                    column_gap: px(12),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id()
    });
    for (i, (label, press)) in ways.into_iter().enumerate() {
        let way = crate::hud::answer_button(&mut commands, &fonts, label, i == 0);
        commands.entity(way).insert((press, DuelExit));
        commands.entity(holder).add_child(way);
    }
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

/// Everything a [`text_field`] draws that is not its label.
pub(crate) struct FieldLook<'a> {
    /// The text, the caret and the selection to draw.
    pub(crate) buffer: &'a TextBuffer,
    /// Whether this is the field with the caret.
    pub(crate) focused: bool,
    /// Set on a password, and `None` on every other box.
    ///
    /// Masked *here* and not by the caller: the caret and the selection are
    /// byte offsets into the real text, and a caller that handed over a
    /// string of bullets would be handing over offsets into a different
    /// string — a bullet is three bytes and the letter it stands for is one
    /// to four.
    pub(crate) mask: Option<Masked>,
    /// What a tap on it means.
    pub(crate) press: Press,
    /// A glyph button at the far end of the box.
    ///
    /// Its own field and not a second shape of [`FieldLook::mask`]: the eye
    /// belongs to a password and is *about* the text, and this is about what
    /// the box is for. A box may have both — a search box has a gear and no
    /// eye, and nothing has two of either.
    pub(crate) tail: Option<FieldTail>,
    /// A glyph from the icon face, drawn before the text.
    ///
    /// A search box is the one shape a player recognises without reading it,
    /// and the magnifier is what makes it that shape. It is `Pickable::IGNORE`
    /// like every other label inside a control, so the tap finds the box.
    pub(crate) lead: Option<char>,
    /// What the box says while nothing has been typed into it.
    ///
    /// Beside the caption above the box rather than instead of it: the caption
    /// says what the box *is* and survives being typed into, and this says
    /// what may go in it and is gone the moment anything does. A box with
    /// neither was a rectangle a player had to guess at.
    pub(crate) hint: Option<&'a str>,
}

/// A glyph button at the end of a field: what it draws and what it means.
#[derive(Clone, Copy)]
pub(crate) struct FieldTail {
    /// The mark, from the icon face.
    pub(crate) glyph: char,
    /// What a tap on it means.
    pub(crate) press: Press,
    /// Whether what it opens is open, which is what lights it.
    pub(crate) lit: bool,
}

/// A password box: what the eye beside it addresses, and whether it is open.
#[derive(Clone, Copy)]
pub(crate) struct Masked {
    /// The field the eye toggles.
    pub(crate) field: Field,
    /// Whether the player has asked to read what they are typing.
    pub(crate) shown: bool,
}

/// The caret drawn in the field that has it.
///
/// A component with a system of its own rather than a glyph in the string,
/// because the tree is retained: making a bar appear and disappear twice a
/// second by rebuilding it would rebuild every row of the table list with it.
/// `at` is where the caret stands and what stands around it, so a rebuild
/// that puts it back exactly where it was leaves the blink where it was too.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(super) struct Caret {
    /// The caret offset, the length of the selection, and the length of the
    /// text — enough for "the same caret in the same field" and no more.
    at: (usize, usize, usize),
}

/// How long the caret spends lit, and then dark.
///
/// 530 ms is the rate every desktop text field has blinked at for thirty
/// years. It is not a number to improve on: a caret is recognised rather
/// than read, and one blinking at a rate nothing else does reads as a fault.
const BLINK_SECS: f32 = 0.530;

/// Blinks the caret, and only the caret.
///
/// Touches one `BackgroundColor` and no state, so a quiet frame still costs
/// nothing and the tree is never rebuilt for it.
pub(super) fn blink(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut carets: Query<(&Caret, &mut BackgroundColor)>,
    mut since: Local<f32>,
    mut drawn: Local<Option<Caret>>,
) {
    let Ok((caret, mut colour)) = carets.single_mut() else {
        *drawn = None;
        return;
    };
    // A caret that has just moved, or has just had a letter typed at it, is a
    // caret being looked at: it goes solid and the cycle starts again.
    *since = if *drawn == Some(*caret) {
        (*since + time.delta_secs()) % (BLINK_SECS * 2.0)
    } else {
        0.0
    };
    *drawn = Some(*caret);
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let want = if caret_lit(*since, still) {
        palette::INK
    } else {
        Color::NONE
    };
    if colour.0 != want {
        colour.0 = want;
    }
}

/// Whether the bar is drawn, `since` seconds after the caret last moved.
///
/// A phase rather than a toggle, so nothing has to be kept in step with
/// anything: the answer is a function of how long the caret has stood still,
/// and a system that missed a frame is right again on the next one.
pub(super) fn caret_lit(since: f32, still: bool) -> bool {
    // `reduce_motion` is a promise that nothing moves, and a bar that comes
    // and goes twice a second is movement.
    still || since % (BLINK_SECS * 2.0) < BLINK_SECS
}

/// A labelled text box that takes the caret when tapped.
pub(crate) fn text_field(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    label: &str,
    look: &FieldLook,
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
    let boxed = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(metrics.tap),
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(metrics.pad * 0.7), px(6)),
                // Two pixels whether or not it has the caret, so that taking
                // the caret rings the box instead of moving everything in it
                // a pixel to the left.
                border: UiRect::all(px(2)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            BorderColor::all(if look.focused {
                palette::ACCENT
            } else {
                Color::srgba(1.0, 1.0, 1.0, 0.06)
            }),
            look.press,
        ))
        .id();
    if let Some(glyph) = look.lead {
        let mark = commands
            .spawn((
                Text::new(glyph.to_string()),
                crate::hud::icon_tf(fonts, metrics.small * 0.85),
                TextColor(palette::MUTED),
                Node {
                    margin: UiRect::right(px(metrics.gap * 0.5)),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(boxed).add_child(mark);
    }
    for run in field_runs(commands, fonts, metrics, look) {
        commands.entity(boxed).add_child(run);
    }
    // After the runs, so the caret stands in front of it the way a caret
    // stands in front of an empty `<input>`'s placeholder.
    if let Some(words) = look.hint.filter(|_| look.buffer.text().is_empty()) {
        let ghost = commands
            .spawn((
                Text::new(words.to_string()),
                tf(fonts, metrics.text),
                TextColor(palette::MUTED),
                Node {
                    flex_shrink: 1.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(boxed).add_child(ghost);
    }
    if look.mask.is_some() || look.tail.is_some() {
        // Pushed to the far end of the row, so a button is in the same place
        // whatever is typed and the letters never run into it.
        let gap = commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    min_width: px(metrics.gap),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(boxed).add_child(gap);
        if let Some(masked) = look.mask {
            let eye = eye_button(commands, fonts, metrics, masked);
            commands.entity(boxed).add_child(eye);
        }
        if let Some(tail) = look.tail {
            let button = icon_button(commands, fonts, metrics, tail);
            commands.entity(boxed).add_child(button);
        }
    }
    commands.entity(column).add_child(caption);
    commands.entity(column).add_child(boxed);
    column
}

/// What colour the line under a form is written in.
///
/// The only thing a [`Tone`] changes: a refusal is the one line a player has
/// to do something about, and in the same grey as "signing in…" it was read
/// straight past. `DANGER` is this palette's word for *this is what went
/// wrong* — the same one the table writes lethal damage in.
pub(super) fn status_ink(tone: Tone) -> Color {
    match tone {
        Tone::Note => palette::MUTED,
        Tone::Refusal => palette::DANGER,
    }
}

/// The eye at the end of a password box.
///
/// A glyph and no word, which is the one place in this interface where that
/// is right: there is no room for a label beside the text inside a box this
/// tall, and the eye is read the same way in every language — every sign-in
/// form on the web has one. It is `Pickable` by default and the box around
/// it is not, so the click finds the eye rather than the field under it.
fn eye_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    masked: Masked,
) -> Entity {
    icon_button(
        commands,
        fonts,
        metrics,
        FieldTail {
            glyph: if masked.shown {
                crate::hud::glyph::EYE_SLASH
            } else {
                crate::hud::glyph::EYE
            },
            press: Press::Reveal(masked.field),
            lit: masked.shown,
        },
    )
}

/// One glyph button at the end of a field.
fn icon_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    tail: FieldTail,
) -> Entity {
    let mark = commands
        .spawn((
            Text::new(tail.glyph.to_string()),
            crate::hud::icon_tf(fonts, metrics.small),
            TextColor(if tail.lit {
                palette::ACCENT
            } else {
                palette::MUTED
            }),
            Pickable::IGNORE,
        ))
        .id();
    let button = commands
        .spawn((
            Node {
                // A finger's worth of height, and enough width to be hit
                // without pushing the text out of a narrow box.
                min_width: px(metrics.tap * 0.7),
                align_self: AlignSelf::Stretch,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            tail.press,
        ))
        .id();
    commands.entity(button).add_child(mark);
    button
}

/// The text inside a field: up to three runs with a bar between two of them.
///
/// No glyph metrics anywhere. The row is already measuring the letters, so a
/// caret put into it as the next thing in the row lands exactly where the
/// letters end — which is the one placement that cannot drift from the font.
fn field_runs(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    look: &FieldLook,
) -> Vec<Entity> {
    let seg = look.buffer.segments();
    let caret = look.focused.then(|| {
        spawn_caret(
            commands,
            metrics,
            Caret {
                at: (
                    look.buffer.cursor(),
                    seg.selected.len(),
                    look.buffer.text().len(),
                ),
            },
        )
    });
    // Head, caret, selection, tail — with the caret on the other side of the
    // selection when that is the end the player is holding. An empty run is
    // no node: a field with the caret at its start is one bar and nothing.
    let mut out = Vec::new();
    // Before the selection, or before the tail: the two sides of a selected
    // run, and the same seam when nothing is selected and the run is empty.
    let caret_at = usize::from(seg.caret_after_selection) + 1;
    // Only the field with the caret shows a selection. Both marks say the
    // same thing — *this is where the typing goes* — so a field that has
    // neither the caret nor the typing must show neither, and the accent
    // that rings the focused box was standing in two boxes at once.
    let runs = [
        (seg.head, false),
        (seg.selected, look.focused),
        (seg.tail, false),
    ];
    for (i, (text, selected)) in runs.into_iter().enumerate() {
        if i == caret_at
            && let Some(caret) = caret
        {
            out.push(caret);
        }
        if !text.is_empty() {
            let mask = look.mask.is_some_and(|masked| !masked.shown);
            out.push(spawn_run(commands, fonts, metrics, text, mask, selected));
        }
    }
    out
}

/// One run of a field's text, masked where the field is a password.
fn spawn_run(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    text: &str,
    mask: bool,
    selected: bool,
) -> Entity {
    let shown = if mask {
        "\u{2022}".repeat(text.chars().count())
    } else {
        text.to_string()
    };
    let mut run = commands.spawn((
        Text::new(shown),
        tf(fonts, metrics.text),
        TextColor(palette::INK),
        Pickable::IGNORE,
    ));
    if selected {
        run.insert(BackgroundColor(palette::SELECTION));
    }
    run.id()
}

/// The bar itself: one logical pixel, half of it borrowed from each side so
/// that showing it moves no letter.
fn spawn_caret(commands: &mut Commands, metrics: Metrics, caret: Caret) -> Entity {
    commands
        .spawn((
            caret,
            Node {
                width: px(1),
                // Bevy lays a text node out at 1.2 times the font size, so a
                // shorter bar would stand lower than the selection beside it
                // and read as a fault rather than as a caret.
                height: px(metrics.text * 1.2),
                margin: UiRect::horizontal(px(-0.5)),
                ..default()
            },
            BackgroundColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id()
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
    let weight = if !enabled {
        crate::hud::ButtonWeight::Dead
    } else if tone == palette::DANGER {
        crate::hud::ButtonWeight::Danger
    } else {
        crate::hud::ButtonWeight::Secondary
    };
    let id = crate::hud::answer_sized(
        commands,
        fonts,
        label,
        weight,
        None,
        metrics.tap,
        metrics.text,
    );
    if enabled && (tone == palette::ACCENT || tone == palette::ACTIVE) {
        super::button_style::primary(commands, id);
    }
    super::button_style::icon(commands, fonts, id, press, metrics.small);
    if enabled {
        commands.entity(id).insert(press);
    }
    id
}

/// The music's switch, wherever a screen puts it (#296).
#[derive(Component)]
pub(crate) struct MusicToggle;

/// The speaker on it, which [`show_the_music_level`] turns to a crossed one
/// while the music is silent.
#[derive(Component)]
pub(super) struct MusicSpeaker;

/// Font Awesome's speaker, and its speaker with a cross.
const SPEAKER: [char; 2] = ['\u{f028}', '\u{f6a9}'];

/// The music's switch: a speaker and a word, as the lobby's other buttons
/// are drawn, which a press turns off or on again (`Press::ToggleMusic`).
///
/// It assumes nothing about where it stands: it is one flex item for its
/// parent to place, like [`button`]. The music plays before anybody has
/// signed in and on until a table opens, so every screen it plays on offers
/// the switch (WCAG 1.4.2). Its speaker and word follow the level every
/// frame ([`show_the_music_level`]), so a screen that does not redraw after
/// a press still shows it pressed.
pub(crate) fn music_toggle(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    lang: Lang,
) -> Entity {
    let id = button(
        commands,
        fonts,
        metrics,
        Phrase::MusicPlaying.text(lang),
        Press::ToggleMusic,
        palette::PANEL_LIT,
        true,
    );
    let speaker = commands
        .spawn((
            Text::new(SPEAKER[0].to_string()),
            crate::hud::icon_tf(fonts, metrics.small),
            TextColor(palette::DOCK_INK),
            Pickable::IGNORE,
            MusicSpeaker,
        ))
        .id();
    commands
        .entity(id)
        .insert(MusicToggle)
        .insert_children(0, &[speaker]);
    id
}

/// Keeps every music switch saying what the music is doing: the speaker and
/// "Music" while it can be heard, the crossed speaker and "Music off" while
/// it cannot, muted or turned all the way down. Written only when it
/// differs, so a switch left alone is not re-laid out every frame.
pub(super) fn show_the_music_level(
    settings: Option<Res<crate::settings::ClientSettings>>,
    state: Res<LobbyState>,
    toggles: Query<&Children, With<MusicToggle>>,
    mut texts: Query<(&mut Text, Has<MusicSpeaker>)>,
) {
    let level = settings.map(|settings| settings.music).unwrap_or_default();
    let heard = level.gain() > 0.0;
    let lang = state.lobby.lang();
    let speaker = SPEAKER[usize::from(!heard)].to_string();
    let words = if heard {
        Phrase::MusicPlaying
    } else {
        Phrase::MusicSilent
    }
    .text(lang);
    for children in &toggles {
        let mut part = texts.iter_many_mut(children);
        while let Some((mut text, is_speaker)) = part.fetch_next() {
            let want = if is_speaker { speaker.as_str() } else { words };
            if text.0 != want {
                want.clone_into(&mut text.0);
            }
        }
    }
}

/// A column panel: a fixed width beside its neighbour, or the full width
/// above it.
pub(crate) fn panel(commands: &mut Commands, metrics: Metrics, width: Val, grow: f32) -> Entity {
    commands
        .spawn((
            Node {
                width,
                flex_grow: grow,
                // A panel that grows to fill the row is also the one that has
                // to give way, and `min_width` has to be told: a flex item's
                // default minimum is its *content*, so a panel holding a row
                // wider than the window silently pushed the window's edge
                // instead of letting that row wrap. Seven table sizes made
                // that visible; two never had.
                flex_shrink: if grow > 0.0 { 1.0 } else { 0.0 },
                min_width: px(0),
                // Height comes from the content, with the screen as a floor.
                // Stretched to the row instead — which is what a flex item
                // does unasked — a panel is exactly one screen tall while its
                // rows carry on past the bottom of it, so a scrolled list
                // leaves the panel behind and is drawn on the backdrop.
                align_self: AlignSelf::Start,
                min_height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap * 0.8),
                padding: UiRect::all(px(metrics.pad * 1.5)),
                border_radius: BorderRadius::all(px(14)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.4)),
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
        Some(Finish::Holographic) => parts.push("holographic".to_string()),
        Some(Finish::Glitter) => parts.push("glitter".to_string()),
        Some(Finish::Galaxy) => parts.push("galaxy".to_string()),
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

/// A shared HUD surface used by the front door and account library.
pub(super) fn surface(commands: &mut Commands, metrics: Metrics) -> Entity {
    commands
        .spawn((
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap),
                padding: UiRect::all(px(metrics.pad)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(14)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.45)),
            Pickable::IGNORE,
        ))
        .id()
}

fn front_door(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    cast: &super::front::FrontCast,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
) {
    let page = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: percent(100),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(px(metrics.pad * 1.5)),
                row_gap: px(metrics.pad * 1.5),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(page);
    let brand = commands
        .spawn((
            Text::new("baylee"),
            TextFont {
                font: fonts.serif.clone().into(),
                font_size: (metrics.head * 2.8).into(),
                ..default()
            },
            TextColor(palette::DOCK_INK),
            Pickable::IGNORE,
        ))
        .id();
    let tagline = note(
        commands,
        fonts,
        metrics,
        Phrase::WelcomeNote.text(state.lobby.lang()),
    );
    let stage = super::front::stage(commands, state, cast, fonts, metrics, scrolled_to);
    let colophon = super::front::colophon(commands, state, fonts, metrics);
    commands
        .entity(page)
        .add_children(&[brand, tagline, stage, colophon]);
}
