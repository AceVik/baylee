//! The lobby's shell: the tree's root, its size classes, and the system that
//! rebuilds it and hands each screen its part.
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
pub(crate) struct LobbyRoot;

/// How often [`ui`] has rebuilt the tree, and why (§10 of the shell design).
///
/// The measurement every perf claim about the lobby is held to: a rebuild
/// despawns and respawns every node, so "nothing changed" must read as zero
/// here, and a keystroke as one. Counted by cause because the gate in [`ui`]
/// has four doors and a bare total cannot say which one was open (more than
/// one can be, so the causes may sum past the total). Read by `devctl`'s
/// `/state.ui_rebuilds`; never reset by the client itself.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub(crate) struct UiRebuilds {
    /// Full rebuilds: the tree despawned and drawn again.
    pub(crate) total: u64,
    /// Builder patches: the retained builder updated in place.
    pub(crate) patches: u64,
    /// Rebuilds a changed `LobbyState` asked for.
    pub(crate) state: u64,
    /// Rebuilds changed account preferences asked for.
    pub(crate) prefs: u64,
    /// Rebuilds the front door's cast asked for.
    pub(crate) cast: u64,
    /// Rebuilds a frame change (or an empty tree) asked for.
    pub(crate) frame: u64,
}

/// How much room there is, in three sizes.
///
/// Breakpoints rather than a continuous scale: what changes between a phone
/// and a desktop is the *shape* of the screen — one column or two, a card that
/// fills the width or one that floats — and shape does not interpolate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Frame {
    /// A window narrower than 760 logical pixels (the shell design, §2.7).
    Compact,
    /// 760 to 1180: a tablet, or a half-screen window.
    Narrow,
    /// 1180 and wider: a desktop window, a large tablet.
    Wide,
}

impl Frame {
    /// The frame a window of this width is in.
    pub(crate) fn of(width: f32) -> Self {
        if width < 760.0 {
            Self::Compact
        } else if width < 1180.0 {
            Self::Narrow
        } else {
            Self::Wide
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
            Frame::Compact => Self {
                frame: Frame::Compact,
                text: 15.0,
                head: 17.0,
                small: 12.0,
                tap: 44.0,
                pad: 14.0,
                gap: 12.0,
            },
            Frame::Narrow => Self {
                frame: Frame::Narrow,
                text: 15.0,
                head: 18.0,
                small: 11.5,
                tap: 38.0,
                pad: 16.0,
                gap: 10.0,
            },
            Frame::Wide => Self {
                frame: Frame::Wide,
                text: 16.0,
                head: 22.0,
                small: 12.5,
                tap: 44.0,
                pad: 20.0,
                gap: 12.0,
            },
        }
    }

    /// Whether the table screen stacks its two panels instead of pairing them.
    pub(crate) fn stacked(self) -> bool {
        self.frame == Frame::Compact
    }

    /// The width of the deck panel beside the table list.
    pub(super) fn decks_width(self) -> Val {
        match self.frame {
            Frame::Compact => percent(100),
            Frame::Narrow => px(280),
            Frame::Wide => px(360),
        }
    }
}

/// The lobby's own camera. The duel brings its own and the two never coexist:
/// this one is despawned on the way out of [`DuelPhase::Closed`], before the
/// stage is built.
pub(super) fn spawn_camera(
    mut commands: Commands,
    vista: Option<ResMut<Assets<crate::vista::VistaMaterial>>>,
    assets: Option<Res<AssetServer>>,
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
    // The inner garden is ready under the arrival flight, and remains behind
    // both the lobby and the builder. No unrelated abstract field takes over.
    if let Some(mut vista) = vista {
        let interior = crate::vista::surface(
            &mut commands,
            &mut vista,
            crate::vista::Vista::Interior,
            assets.as_deref(),
        );
        commands
            .entity(interior)
            .insert((LobbyScreen, GlobalZIndex(-2)));
        let scene = crate::vista::surface(
            &mut commands,
            &mut vista,
            crate::vista::Vista::Front,
            assets.as_deref(),
        );
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
    mut rebuilds: ResMut<UiRebuilds>,
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
        && metrics.frame != Frame::Compact
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
        rebuilds.patches += 1;
        return;
    }
    rebuilds.total += 1;
    rebuilds.state += u64::from(state.is_changed());
    rebuilds.prefs += u64::from(prefs.is_changed());
    rebuilds.cast += u64::from(cast.is_changed());
    rebuilds.frame += u64::from(root.is_empty() || *drawn != Some(metrics.frame));
    *builder_drawn = None;
    for entity in &root {
        commands.entity(entity).despawn();
    }
    *drawn = Some(metrics.frame);

    let full_bleed = true;
    // A phone puts the sign-in form near the top instead of centring it: the
    // soft keyboard takes the bottom half of the screen, and a centred form
    // ends up underneath it.
    let top = full_bleed || metrics.frame == Frame::Compact;
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
            scrolled_to.get(List::Settings),
            &state.seat,
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
            super::front::front_door(
                &mut commands,
                root,
                &state,
                &cast,
                &fonts,
                metrics,
                &scrolled_to,
                assets.as_deref(),
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
        // The persistent preparation cover takes over in the same frame.
        Screen::Seated(_) => {}
    }
    if state.lobby.screen() == &Screen::Table
        && let Some(picker) = state.lobby.builder().picker()
    {
        let dialog = crate::buildui::print_picker::printing_picker(
            &mut commands,
            &fonts,
            metrics,
            state.lobby.lang(),
            state.lobby.builder(),
            picker,
            assets.as_deref(),
            cards.as_mut(),
            &scrolled_to,
        );
        commands.entity(root).add_child(dialog);
    }
    super::confirm::draw(&mut commands, root, &state, &fonts, metrics);
    if state.confirmation.is_some() {
        *builder_drawn = None;
    }
}
