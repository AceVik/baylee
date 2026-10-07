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
    /// The builder's sections those patches redrew (§10 #2): a key typed
    /// into its search is one patch of two sections, the toolbar and the
    /// list.
    pub(crate) sections: u64,
    /// Rebuilds a changed `LobbyState` asked for.
    pub(crate) state: u64,
    /// Rebuilds changed account preferences asked for.
    pub(crate) prefs: u64,
    /// Rebuilds the front door's cast asked for.
    pub(crate) cast: u64,
    /// Rebuilds a frame change (or an empty tree) asked for.
    pub(crate) frame: u64,
}

/// How much room there is: the shell's five size classes (§2.7), one enum
/// for the lobby and the kit.
///
/// Breakpoints rather than a continuous scale: what changes between a phone
/// and a desktop is the *shape* of the screen — one column or two, a card that
/// fills the width or one that floats — and shape does not interpolate. The
/// lobby's screens still read [`Frame::of`], the width-only reading that
/// answers only `Compact`, `Narrow` and `Wide`; each screen's package moves
/// it onto `Frame::classify` (raw height, text step) with the shell.
pub(crate) use crate::shellkit::Frame;

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
            Frame::Compact | Frame::Phone => Self {
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
            Frame::Wide | Frame::Vast => Self {
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

/// What the kit and the settings screen read beside the lobby (one
/// parameter, under Bevy's sixteen).
type KitInputs<'w, 's> = (
    Res<'w, crate::shellkit::InputClass>,
    Option<Res<'w, crate::settings::ClientSettings>>,
    Option<Res<'w, crate::quality::InUse>>,
    Option<Res<'w, crate::quality::DisplayTrial>>,
    Query<'w, 's, (), With<bevy::window::Monitor>>,
);

/// What the lobby's tree was last built for, on the kit's side: the size
/// class read on the raw height, the text step, the input class, and
/// whether a seated strip stands (the builder's patch path draws none).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct KitDrawn {
    frame: Frame,
    step: crate::shellkit::TextSize,
    input: crate::shellkit::InputClass,
    strip: bool,
    /// Tall enough for the front door's full colophon.
    tall: bool,
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
    // The shell's header and strips are measured by the kit: the text step
    // and the input class (WP0b-3).
    // And the settings screen's graphics rows (WP5): what is in force,
    // the display mode's trial, how many monitors there are.
    kit_inputs: KitInputs,
    mut drawn: Local<Option<Frame>>,
    mut kit_drawn: Local<Option<KitDrawn>>,
    mut builder_drawn: Local<Option<crate::buildui::Retained>>,
    mut rebuilds: ResMut<UiRebuilds>,
) {
    let (width, height) = windows.iter().next().map_or((1280.0, 800.0), |w| {
        (w.resolution.width(), w.resolution.height())
    });
    let metrics = Metrics::of(width);
    let step = kit_inputs
        .1
        .as_deref()
        .map_or_else(Default::default, |s| s.text_size);
    let shell_m = super::header::kit_metrics(width, height, step, *kit_inputs.0);
    // Only what the header draws from the kit's side; compared, never read
    // off a change flag (a settings save is not a rebuild).
    let kit_now = KitDrawn {
        frame: shell_m.frame,
        step,
        input: *kit_inputs.0,
        strip: super::header::seated(&state).is_some(),
        tall: height >= super::front::door::FULL_COLOPHON_HEIGHT,
    };
    let kit_same = kit_drawn.as_ref() == Some(&kit_now);
    if !state.is_changed()
        && !prefs.is_changed()
        && !cast.is_changed()
        && !root.is_empty()
        && *drawn == Some(metrics.frame)
        && kit_same
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
    let kit = crate::shellkit::controls::Kit {
        fonts: &fonts,
        m: shell_m,
        german: state.lobby.lang() == Lang::De,
    };
    // The builder's Save key cap, from the account's keymap.
    let save_keys = prefs.all().shell_keys.hint(
        baylee_client_core::shellkeys::ShellAction::SaveDeck,
        crate::shellkit::keys::mac(),
        &baylee_client_core::shellkeys::Learnt::default(),
    );
    let build_env = crate::buildui::Env {
        kit,
        state: &state,
        scrolled: &scrolled_to,
        layout: crate::buildui::Layout::of(shell_m.frame, width),
        save_keys: save_keys.as_deref(),
    };
    if state.lobby.screen() == &Screen::Build
        && !state.settings.is_open()
        && state.lobby.library().page.is_none()
        && state.confirmation.is_none()
        && !prefs.is_changed()
        && *drawn == Some(metrics.frame)
        && kit_same
        && !root.is_empty()
        && let Some(cached) = builder_drawn.as_mut()
        && cached.fits(&build_env)
    {
        let redrawn = cached.patch(&mut commands, &build_env, assets.as_deref(), cards.as_mut());
        rebuilds.patches += 1;
        rebuilds.sections += u64::from(redrawn);
        return;
    }
    rebuilds.total += 1;
    rebuilds.state += u64::from(state.is_changed());
    rebuilds.prefs += u64::from(prefs.is_changed());
    rebuilds.cast += u64::from(cast.is_changed());
    rebuilds.frame += u64::from(root.is_empty() || *drawn != Some(metrics.frame) || !kit_same);
    *builder_drawn = None;
    for entity in &root {
        commands.entity(entity).despawn();
    }
    *drawn = Some(metrics.frame);
    *kit_drawn = Some(kit_now);

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
        super::header::draw(&mut commands, root, &state, kit, metrics);
        let (_, _, in_use, trial, monitors) = &kit_inputs;
        let settings = kit_inputs.1.as_deref();
        let view = crate::settingsui::View {
            state: &state,
            prefs: prefs.all(),
            settings,
            graphics: crate::settingsui::keys::shown_graphics(settings, in_use.as_deref()),
            trial: trial
                .as_deref()
                .and_then(|t| t.previous.map(|_| t.seconds())),
            monitors: monitors.iter().count(),
            builds: crate::settingsui::builds(),
            metrics,
            scroll: scrolled_to.get(List::Settings),
            sheet_scroll: scrolled_to.get(List::ProfileSheet),
            nav_scroll: scrolled_to.get(List::SettingsNav),
        };
        crate::settingsui::screen(&mut commands, root, &view, kit);
        super::confirm::draw_deletion(&mut commands, root, &state, &fonts, metrics);
        return;
    }

    // The builder's own history page (WP4); on the Decks screen a
    // deck's history is a sheet, and the house list is a tab (WP3).
    if state.lobby.library().page.is_some() && state.lobby.screen() == &Screen::Build {
        super::header::draw(&mut commands, root, &state, kit, metrics);
        super::library_ui::screen(&mut commands, root, &state, &fonts, metrics, &scrolled_to);
        return;
    }
    match state.lobby.screen() {
        Screen::SignIn { .. } => {
            commands.entity(root).insert((
                Scrollable(List::Table),
                ScrollPosition(Vec2::new(0.0, scrolled_to.get(List::Table))),
            ));
            let music_on = kit_inputs.1.as_deref().is_none_or(|s| s.music.gain() > 0.0);
            super::front::front_door(
                &mut commands,
                root,
                &state,
                &cast,
                kit,
                &scrolled_to,
                assets.as_deref(),
                music_on,
                height,
            );
            super::front::door::about(&mut commands, root, &state, kit, &scrolled_to);
        }
        Screen::Table => table(
            &mut commands,
            root,
            &state,
            &fonts,
            metrics,
            &scrolled_to,
            kit,
            prefs.all(),
        ),
        Screen::Build => {
            // The builder keeps its own header; the seated strip stands
            // above it.
            super::header::draw(&mut commands, root, &state, kit, metrics);
            *builder_drawn = Some(crate::buildui::builder(
                &mut commands,
                root,
                &build_env,
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
    // The terms stand over whatever the sign-in led to, until answered.
    super::front::terms::sheet(&mut commands, root, &state, kit, &scrolled_to);
}
