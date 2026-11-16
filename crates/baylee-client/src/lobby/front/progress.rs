//! Polished in-card Login, Connection, and Queue view (`Anmeldevorgang`).
//!
//! While an account sign-in, registration, guest admission, or server login
//! queue is active on the front door, the form's input fields and inner
//! buttons give way to this unified chamber view inside the smoked cut-glass
//! [`super::FrontCard`]. Height changes animate smoothly via
//! [`super::glide_the_stage`], carrying the Baylee emblem above and the four
//! community pills below into position without jumps.

#[allow(clippy::wildcard_imports)]
use super::super::*;
use crate::shellkit::controls::Kit;
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::{surfaces, tokens};
use bevy::ui::{UiTransform, Val2};

/// Active step inside the in-card authentication & connection view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthStep {
    /// Opening TLS / loopback connection to Baylee Central or the gateway.
    Connecting,
    /// Running cryptographic verification (OPAQUE / credentials).
    Authenticating,
    /// Holding a reserved slot in a server login / admission queue.
    Queued {
        position: Option<u32>,
        estimated_seconds: Option<u32>,
    },
    /// Synchronising profile, gateways, and opening the session.
    OpeningSession,
}

/// Optional dev-control / test override to inspect and photograph each stage
/// (including server login queues) deterministically.
#[cfg(any(test, feature = "dev-control"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthDemo {
    Connecting,
    Authenticating,
    Queued {
        position: Option<u32>,
        estimated_seconds: Option<u32>,
    },
    OpeningSession,
}

/// Resource tracking elapsed time and entrance fade of the front-door
/// authentication progress view without triggering per-frame UI tree rebuilds.
#[derive(Resource, Default, Clone, Debug)]
pub(crate) struct AuthProgressMemory {
    pub(crate) active: bool,
    pub(crate) elapsed: f32,
    pub(crate) fade: f32,
    pub(crate) shown_whole_secs: u32,
    #[cfg(any(test, feature = "dev-control"))]
    pub(crate) demo: Option<AuthDemo>,
}

/// Returns whether the front-door account card should display the in-card
/// authentication/connection progress view instead of its form fields/buttons.
#[must_use]
pub(crate) fn is_active(state: &LobbyState) -> bool {
    state.auth_step.is_some()
        || (matches!(state.lobby.screen(), Screen::SignIn { .. })
            && !state.central_ready()
            && !state.lobby.offline()
            && (state.lobby.busy() || state.central_admitting()))
}

fn live_busy(state: &LobbyState, memory: &AuthProgressMemory) -> bool {
    #[cfg(any(test, feature = "dev-control"))]
    if memory.demo.is_some() {
        return true;
    }
    let _ = memory;
    matches!(state.lobby.screen(), Screen::SignIn { .. })
        && !state.central_ready()
        && !state.lobby.offline()
        && (state.lobby.busy() || state.central_admitting())
}

fn resolve_step(state: &LobbyState, memory: &AuthProgressMemory) -> AuthStep {
    #[cfg(any(test, feature = "dev-control"))]
    if let Some(demo) = memory.demo {
        return match demo {
            AuthDemo::Connecting => AuthStep::Connecting,
            AuthDemo::Authenticating => AuthStep::Authenticating,
            AuthDemo::Queued {
                position,
                estimated_seconds,
            } => AuthStep::Queued {
                position,
                estimated_seconds,
            },
            AuthDemo::OpeningSession => AuthStep::OpeningSession,
        };
    }
    if state.central_admitting() {
        return AuthStep::OpeningSession;
    }
    if memory.elapsed < 0.45 {
        AuthStep::Connecting
    } else if memory.elapsed < 3.20 {
        AuthStep::Authenticating
    } else {
        AuthStep::Queued {
            position: None,
            estimated_seconds: None,
        }
    }
}

#[derive(Component)]
pub(crate) struct AuthProgressRoot;

#[derive(Component)]
pub(crate) struct AuthOrb {
    pub(crate) accent: Color,
}

#[derive(Component)]
pub(crate) struct AuthShimmerFill;

#[derive(Component)]
pub(crate) struct AuthElapsedLabel;

fn format_mm_ss(secs: u32) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

/// Spawns the in-card authentication, connection, and queue progress view
/// directly inside `parent` (`FrontCardInner`).
#[allow(clippy::too_many_lines)]
pub(crate) fn draw(commands: &mut Commands, parent: Entity, state: &LobbyState, kit: Kit) {
    use baylee_client_core::lobby::Face;
    use bevy::ui::{ColorStop, InterpolationColorSpace, LinearGradient};

    let lang = state.lobby.lang();
    let step = state.auth_step.unwrap_or(if state.central_admitting() {
        AuthStep::OpeningSession
    } else {
        AuthStep::Connecting
    });
    let is_queued = matches!(step, AuthStep::Queued { .. });
    let accent = if is_queued {
        tokens::GOLD
    } else {
        tokens::ACCENT
    };

    let (glyph, title_str, desc_str, active_idx) = match step {
        AuthStep::Connecting => (
            '\u{f1eb}',
            match state.lobby.face() {
                Face::Create => Phrase::AuthProgressRegisterTitle.text(lang),
                Face::Guest => Phrase::AuthProgressGuestTitle.text(lang),
                Face::SignIn | Face::NewPassword => Phrase::AuthProgressTitle.text(lang),
            },
            if state.central_pending() {
                Phrase::AuthDescConnecting.text(lang)
            } else {
                Phrase::AuthDescConnectingGateway.text(lang)
            },
            0_usize,
        ),
        AuthStep::Authenticating => (
            '\u{f3ed}',
            match state.lobby.face() {
                Face::Create => Phrase::AuthProgressRegisterTitle.text(lang),
                Face::Guest => Phrase::AuthProgressGuestTitle.text(lang),
                Face::SignIn | Face::NewPassword => Phrase::AuthProgressTitle.text(lang),
            },
            Phrase::AuthDescAuthenticating.text(lang),
            1_usize,
        ),
        AuthStep::Queued { .. } => (
            '\u{f252}',
            Phrase::AuthProgressQueueTitle.text(lang),
            Phrase::AuthQueueStandby.text(lang),
            1_usize,
        ),
        AuthStep::OpeningSession => (
            '\u{f084}',
            Phrase::AuthProgressTitle.text(lang),
            Phrase::AuthDescOpeningSession.text(lang),
            2_usize,
        ),
    };

    let root = commands
        .spawn((
            AuthProgressRoot,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::axes(kit.m.px(8.0), kit.m.px(10.0)),
                row_gap: kit.m.px(14.0),
                ..default()
            },
            UiTransform::default(),
            Pickable::IGNORE,
        ))
        .id();

    // 1. Glowing orbital emblem medallion (`56 × 56`).
    let orb_top = crate::ambience::blend(Color::srgba(0.12, 0.24, 0.34, 0.94), accent, 0.26);
    let orb_bot = crate::ambience::blend(Color::srgba(0.03, 0.07, 0.12, 0.94), accent, 0.10);
    let orb = commands
        .spawn((
            AuthOrb { accent },
            Node {
                width: kit.m.px(56.0),
                height: kit.m.px(56.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.5)),
                border_radius: BorderRadius::all(px_fixed(28.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::NONE),
            BackgroundGradient::from(
                LinearGradient::to_bottom(vec![
                    ColorStop::percent(orb_top, 0.0),
                    ColorStop::percent(orb_bot, 100.0),
                ])
                .in_color_space(InterpolationColorSpace::Srgba),
            ),
            BorderColor::all(accent.with_alpha(0.68)),
            BoxShadow(vec![ShadowStyle {
                color: accent.with_alpha(0.32),
                x_offset: px_fixed(0.0),
                y_offset: px_fixed(0.0),
                spread_radius: px_fixed(1.5),
                blur_radius: px_fixed(18.0),
            }]),
            UiTransform::default(),
            Pickable::IGNORE,
        ))
        .with_child((
            Text::new(glyph.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.head * 1.05),
            TextColor(crate::ambience::lighter(accent, 0.22)),
            Pickable::IGNORE,
        ))
        .id();

    // 2. Centered title + optional account pill + live phase subtitle.
    let head_col = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: kit.m.px(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(title_str),
            crate::hud::tf_bold(kit.fonts, surfaces::whole_pixels(kit.m.head * 1.06)),
            TextColor(tokens::INK),
            TextLayout::justify(Justify::Center),
            bevy::ui::LayoutConfig {
                use_rounding: false,
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(head_col).add_child(title);

    let typed_user = state.lobby.field(Field::Username).trim();
    if !typed_user.is_empty() {
        let user_pill = commands
            .spawn((
                Node {
                    padding: UiRect::axes(kit.m.px(10.0), kit.m.px(2.5)),
                    column_gap: kit.m.px(6.0),
                    align_items: AlignItems::Center,
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.09, 0.15, 0.68)),
                BorderColor::all(accent.with_alpha(0.36)),
                Pickable::IGNORE,
            ))
            .id();
        let u_icon = commands
            .spawn((
                Text::new("\u{f007}"),
                crate::hud::icon_tf(kit.fonts, kit.m.small * 0.80),
                TextColor(accent),
                Pickable::IGNORE,
            ))
            .id();
        let u_name = commands
            .spawn((
                Text::new(typed_user),
                crate::hud::tf_bold(kit.fonts, surfaces::whole_pixels(kit.m.small)),
                TextColor(tokens::INK),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(user_pill).add_children(&[u_icon, u_name]);
        commands.entity(head_col).add_child(user_pill);
    }

    let desc = commands
        .spawn((
            Text::new(desc_str),
            crate::hud::tf(kit.fonts, surfaces::whole_pixels(kit.m.small)),
            TextColor(tokens::MUTED),
            TextLayout::justify(Justify::Center),
            Node {
                max_width: kit.m.px(500.0),
                ..default()
            },
            bevy::ui::LayoutConfig {
                use_rounding: false,
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(head_col).add_child(desc);

    // 3. Sleek animated shimmer progress track.
    let track = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                max_width: kit.m.px(440.0),
                height: px_fixed(6.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(3.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.04, 0.08, 0.78)),
            BorderColor::all(tokens::BORDER.with_alpha(0.65)),
            Pickable::IGNORE,
        ))
        .id();
    let shimmer = commands
        .spawn((
            AuthShimmerFill,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),
                top: px_fixed(0.0),
                bottom: px_fixed(0.0),
                width: Val::Percent(42.0),
                border_radius: BorderRadius::all(px_fixed(3.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BackgroundGradient::from(
                LinearGradient::to_right(vec![
                    ColorStop::percent(accent.with_alpha(0.15), 0.0),
                    ColorStop::percent(crate::ambience::lighter(accent, 0.28), 50.0),
                    ColorStop::percent(accent.with_alpha(0.15), 100.0),
                ])
                .in_color_space(InterpolationColorSpace::Srgba),
            ),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(track).add_child(shimmer);

    // 4. 3-step connection stage strip (`Connection · Verification · Session`).
    let steps_row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: kit.m.px(8.0),
                row_gap: kit.m.px(6.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (idx, label_phrase) in [
        Phrase::AuthStepConnect,
        Phrase::AuthStepVerify,
        Phrase::AuthStepSession,
    ]
    .into_iter()
    .enumerate()
    {
        if idx > 0 {
            let arrow = commands
                .spawn((
                    Text::new("\u{f054}"),
                    crate::hud::icon_tf(kit.fonts, kit.m.small * 0.68),
                    TextColor(if idx <= active_idx {
                        tokens::ACCENT.with_alpha(0.62)
                    } else {
                        tokens::MUTED.with_alpha(0.38)
                    }),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(steps_row).add_child(arrow);
        }
        let pill = step_pill(
            commands,
            kit,
            idx,
            active_idx,
            label_phrase.text(lang),
            accent,
        );
        commands.entity(steps_row).add_child(pill);
    }

    commands
        .entity(root)
        .add_children(&[orb, head_col, track, steps_row]);

    // 5. Dedicated Queue card when waiting in a login / server capacity queue.
    if let AuthStep::Queued {
        position,
        estimated_seconds,
    } = step
    {
        let q_card = spawn_queue_banner(commands, kit, lang, position, estimated_seconds);
        commands.entity(root).add_child(q_card);
    }

    commands.entity(parent).add_child(root);
}

fn step_pill(
    commands: &mut Commands,
    kit: Kit,
    idx: usize,
    active_idx: usize,
    text: &str,
    accent: Color,
) -> Entity {
    let done = idx < active_idx;
    let current = idx == active_idx;
    let (bg, border, ink, icon_opt) = if done {
        (
            Color::srgba(0.06, 0.19, 0.18, 0.65),
            tokens::ACCENT.with_alpha(0.50),
            tokens::ACCENT,
            Some('\u{f00c}'),
        )
    } else if current {
        (
            crate::ambience::blend(Color::srgba(0.06, 0.13, 0.21, 0.80), accent, 0.18),
            accent.with_alpha(0.68),
            tokens::INK,
            None,
        )
    } else {
        (
            Color::srgba(0.03, 0.06, 0.10, 0.45),
            tokens::BORDER.with_alpha(0.42),
            tokens::MUTED.with_alpha(0.75),
            None,
        )
    };

    let pill = commands
        .spawn((
            Node {
                padding: UiRect::axes(kit.m.px(10.0), kit.m.px(4.0)),
                column_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(border),
            Pickable::IGNORE,
        ))
        .id();

    let badge = if let Some(check) = icon_opt {
        commands
            .spawn((
                Text::new(check.to_string()),
                crate::hud::icon_tf(kit.fonts, kit.m.small * 0.78),
                TextColor(ink),
                Pickable::IGNORE,
            ))
            .id()
    } else {
        commands
            .spawn((
                Text::new(format!("{}", idx + 1)),
                crate::hud::tf_bold(kit.fonts, surfaces::whole_pixels(kit.m.small * 0.86)),
                TextColor(if current { accent } else { ink }),
                Pickable::IGNORE,
            ))
            .id()
    };
    let word = commands
        .spawn((
            Text::new(text),
            if current {
                crate::hud::tf_bold(kit.fonts, surfaces::whole_pixels(kit.m.small * 0.92))
            } else {
                crate::hud::tf(kit.fonts, surfaces::whole_pixels(kit.m.small * 0.92))
            },
            TextColor(ink),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(pill).add_children(&[badge, word]);
    pill
}

fn spawn_queue_banner(
    commands: &mut Commands,
    kit: Kit,
    lang: Lang,
    position: Option<u32>,
    estimated_seconds: Option<u32>,
) -> Entity {
    let box_node = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                max_width: kit.m.px(460.0),
                padding: UiRect::axes(kit.m.px(14.0), kit.m.px(10.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: kit.m.px(12.0),
                row_gap: kit.m.px(6.0),
                flex_wrap: FlexWrap::Wrap,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.14, 0.11, 0.04, 0.58)),
            BorderColor::all(tokens::GOLD.with_alpha(0.52)),
            Pickable::IGNORE,
        ))
        .id();

    let left = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(2.0),
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    let headline_text = position.map_or_else(
        || Phrase::AuthProgressQueueTitle.text(lang).to_string(),
        |pos| Phrase::AuthQueueBanner.fill(lang, &[&pos.to_string()]),
    );
    let headline = commands
        .spawn((
            Text::new(headline_text),
            crate::hud::tf_bold(kit.fonts, surfaces::whole_pixels(kit.m.small)),
            TextColor(tokens::GOLD),
            bevy::ui::LayoutConfig {
                use_rounding: false,
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(left).add_child(headline);

    if let Some(est) = estimated_seconds {
        let sub = commands
            .spawn((
                Text::new(Phrase::AuthQueueEstimate.fill(lang, &[&est.to_string()])),
                crate::hud::tf(kit.fonts, surfaces::whole_pixels(kit.m.small * 0.90)),
                TextColor(tokens::INK.with_alpha(0.85)),
                bevy::ui::LayoutConfig {
                    use_rounding: false,
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(left).add_child(sub);
    }

    let timer_pill = commands
        .spawn((
            Node {
                padding: UiRect::axes(kit.m.px(9.0), kit.m.px(3.0)),
                column_gap: kit.m.px(5.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.05, 0.08, 0.72)),
            BorderColor::all(tokens::GOLD.with_alpha(0.38)),
            Pickable::IGNORE,
        ))
        .id();
    let clock_icon = commands
        .spawn((
            Text::new("\u{f252}"),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.78),
            TextColor(tokens::GOLD),
            Pickable::IGNORE,
        ))
        .id();
    let elapsed_txt = commands
        .spawn((
            AuthElapsedLabel,
            Text::new("00:00"),
            crate::hud::tf_bold(kit.fonts, surfaces::whole_pixels(kit.m.small * 0.90)),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(timer_pill)
        .add_children(&[clock_icon, elapsed_txt]);
    commands.entity(box_node).add_children(&[left, timer_pill]);
    box_node
}

/// Runs before `shell::ui`: advances [`AuthProgressMemory`] and updates
/// [`LobbyState::auth_step`] only when the discrete step changes.
pub(crate) fn tick_auth_progress(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut state: ResMut<LobbyState>,
    mut memory: ResMut<AuthProgressMemory>,
) {
    let busy_now = live_busy(&state, &memory);
    if !busy_now {
        if memory.active {
            memory.active = false;
            memory.elapsed = 0.0;
            memory.fade = 0.0;
            memory.shown_whole_secs = 0;
        }
        if state.auth_step.is_some() {
            state.auth_step = None;
        }
        return;
    }

    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let raw_dt = time.delta_secs();
    let dt = if raw_dt.is_finite() {
        raw_dt.clamp(0.0, 0.1)
    } else {
        0.0
    };

    if memory.active {
        memory.elapsed += dt;
        if still {
            memory.fade = 1.0;
        } else if memory.fade < 1.0 {
            memory.fade = (memory.fade + dt / 0.22).min(1.0);
        }
    } else {
        memory.active = true;
        memory.elapsed = 0.0;
        memory.fade = if still { 1.0 } else { 0.0 };
        memory.shown_whole_secs = u32::MAX;
    }

    let wanted = Some(resolve_step(&state, &memory));
    if state.auth_step != wanted {
        state.auth_step = wanted;
    }
}

/// Runs after `shell::ui`: paints the orbital glow, entrance glide, shimmer
/// bar, and elapsed timer without touching `LobbyState` (zero writes at rest).
#[allow(clippy::type_complexity)]
pub(crate) fn paint_auth_progress(
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut memory: ResMut<AuthProgressMemory>,
    mut roots: Query<&mut UiTransform, (With<AuthProgressRoot>, Without<AuthOrb>)>,
    mut orbs: Query<(&AuthOrb, &mut BoxShadow, &mut BorderColor)>,
    mut shimmers: Query<&mut Node, With<AuthShimmerFill>>,
    mut timers: Query<&mut Text, With<AuthElapsedLabel>>,
) {
    if !memory.active {
        return;
    }
    let ease = 1.0 - (1.0 - memory.fade.clamp(0.0, 1.0)).powi(3);
    for mut tf in &mut roots {
        tf.set_if_neq(UiTransform {
            translation: Val2::new(px_fixed(0.0), px_fixed((1.0 - ease) * 5.0)),
            ..default()
        });
    }

    let whole_secs = memory.elapsed.max(0.0) as u32;
    if !timers.is_empty() && whole_secs != memory.shown_whole_secs {
        memory.shown_whole_secs = whole_secs;
        let label = format_mm_ss(whole_secs);
        for mut txt in &mut timers {
            if txt.0 != label {
                txt.0.clone_from(&label);
            }
        }
    }

    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    if still {
        return;
    }

    let pulse = 0.5 + 0.5 * (memory.elapsed * 3.2).sin();
    for (orb, mut shadow, mut border) in &mut orbs {
        let glow_alpha = 0.24 + 0.24 * pulse;
        let spread = 1.0 + 2.0 * pulse;
        shadow.set_if_neq(BoxShadow(vec![ShadowStyle {
            color: orb.accent.with_alpha(glow_alpha),
            x_offset: px_fixed(0.0),
            y_offset: px_fixed(0.0),
            spread_radius: px_fixed(spread),
            blur_radius: px_fixed(16.0 + 6.0 * pulse),
        }]));
        border.set_if_neq(BorderColor::all(orb.accent.with_alpha(0.52 + 0.36 * pulse)));
    }

    let cycle = (memory.elapsed / 1.65).fract();
    let left_pct = -42.0 + 142.0 * cycle;
    for mut node in &mut shimmers {
        node.left = Val::Percent(left_pct);
    }
}
