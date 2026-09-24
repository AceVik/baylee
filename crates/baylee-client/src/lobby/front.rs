//! The front door: one card with two faces, turned over between them (#252).
//!
//! The gateway form is the face up at launch. Choosing a gateway turns the
//! card to the account form, and Back turns it again. One card rather than
//! two panels side by side, because the second form means nothing until the
//! first is answered, and a form that is on screen but not yet usable was
//! the least clear thing on this screen.
//!
//! # The turn
//!
//! Played as `scale.x = |cos(πt)|` on the card's [`UiTransform`], the same
//! projection `crate::flip` uses for a card preview, with the faces swapped
//! at `t = 0.5` where the card is edge-on and there is nothing to see. The
//! absolute value keeps the scale positive, so no face is ever drawn
//! mirrored and no text field ever sits under a negative scale. Scale alone
//! has no direction, so the card also drifts a little sideways, one way
//! going forward and the other way coming back, and lifts a little.
//!
//! The tree is rebuilt from state, so the turn cannot live in it: [`FrontTurn`]
//! holds the progress and [`FrontFace`] the face drawn. The face changes only
//! at the edge-on instant, which is the one rebuild a turn costs, and
//! [`pose_front`] writes the transform every frame onto whatever card is
//! standing. A card at rest carries no transform at all.
//!
//! Under `reduce_motion` the faces swap at once and nothing moves.

use super::ui::{Masked, status_ink};
#[allow(clippy::wildcard_imports)] // the lobby widget vocabulary
use super::*;
use bevy::ui::{UiTransform, Val2};

/// How long one turn takes.
///
/// A touch slower than a card preview's turn (`flip::TURN_RATE`, about
/// 0.3 s): this one changes what the screen is for.
const TURN_SECONDS: f32 = 0.36;

/// How far the card drifts sideways at the edge-on instant, in logical
/// pixels: left going forward, right coming back.
const DRIFT_PX: f32 = 16.0;

/// How much taller the card is at the edge-on instant: lifted off the table
/// to be turned.
const LIFT: f32 = 0.02;

/// The card's corner radius, all four corners. The leather inside its one
/// pixel border is cut one pixel tighter (`dock::GROUND_RADIUS`).
pub(super) const CARD_RADIUS: f32 = 14.0;

/// The width of the card on a tablet and a desktop. A phone gives it the
/// whole page.
fn card_width(frame: Frame) -> Val {
    match frame {
        Frame::Phone => percent(100),
        Frame::Tablet => px(480),
        Frame::Desktop => px(520),
    }
}

/// The card's least height, so both faces stand in one frame and neither a
/// turn nor a tab changes its size. A phone scrolls instead.
///
/// Sized to the tallest face there is, measured on a desktop window: the
/// account face on "Create account", two hint lines taller than "Sign in",
/// at 615 px; the gateway face with its list at the four rows it shows
/// before scrolling is 602 px. A tablet's sizes differ from a desktop's by a
/// few pixels either way. The guest entry (#269) will not fit in this: it
/// raises the account face by about 80 px.
fn card_height(frame: Frame) -> Val {
    match frame {
        Frame::Phone => Val::Auto,
        Frame::Tablet | Frame::Desktop => px(620),
    }
}

/// The tab of the account form that is open.
#[derive(Component)]
pub(super) struct ActiveTab;

/// The two faces.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum Face {
    /// Choose a gateway.
    #[default]
    Gateway,
    /// Sign in to it, or create an account there.
    Account,
}

/// Which face is drawn. Changes only at the edge-on instant.
#[derive(Resource, Default, PartialEq, Eq)]
pub(super) struct FrontFace(pub(super) Face);

/// How far round the card is: 0 with the gateway face up, 1 with the
/// account face up.
#[derive(Resource, Default, PartialEq)]
pub(crate) struct FrontTurn {
    t: f32,
    /// Where it is going, which is also which way it drifts.
    toward: f32,
}

impl FrontTurn {
    /// Whether the card is part way round. Nothing on it answers a press
    /// until it has landed.
    pub(crate) fn turning(&self) -> bool {
        self.t > 0.0 && self.t < 1.0
    }

    /// Lands the card on the face the state asks for, without turning.
    #[cfg(test)]
    pub(crate) fn settle(&mut self, account: bool) {
        self.t = if account { 1.0 } else { 0.0 };
        self.toward = self.t;
    }
}

/// The card on screen, for [`pose_front`].
#[derive(Component)]
pub(super) struct FrontCard;

/// Moves the turn towards the face the lobby asks for.
pub(super) fn turn_front(
    time: Res<Time>,
    state: Res<LobbyState>,
    prefs: Res<crate::prefs::Prefs>,
    mut turn: ResMut<FrontTurn>,
    mut face: ResMut<FrontFace>,
) {
    let target = if state.lobby.gateway_chosen() {
        1.0
    } else {
        0.0
    };
    // Anywhere but the front door there is no card to turn, and coming back
    // to it (signing out) should find it already landed.
    let still = prefs.all().reduce_motion || !matches!(state.lobby.screen(), Screen::SignIn { .. });
    let t = if still {
        target
    } else {
        let step = time.delta_secs() / TURN_SECONDS;
        if target > turn.t {
            (turn.t + step).min(target)
        } else {
            (turn.t - step).max(target)
        }
    };
    turn.set_if_neq(FrontTurn { t, toward: target });
    let shown = if t >= 0.5 {
        Face::Account
    } else {
        Face::Gateway
    };
    face.set_if_neq(FrontFace(shown));
}

/// Writes the turn onto the standing card: its scale, its drift and its
/// shadow.
pub(super) fn pose_front(
    turn: Res<FrontTurn>,
    mut cards: Query<(&mut UiTransform, &mut BoxShadow), With<FrontCard>>,
) {
    let angle = std::f32::consts::PI * turn.t;
    let lift = if turn.turning() { angle.sin() } else { 0.0 };
    let posed = if turn.turning() {
        let drift = if turn.toward > 0.5 {
            -DRIFT_PX
        } else {
            DRIFT_PX
        };
        UiTransform {
            translation: Val2::px(drift * lift, 0.0),
            // Never quite zero: a node of no width is one nothing can invert.
            scale: Vec2::new(angle.cos().abs().max(0.001), 1.0 + LIFT * lift),
            ..default()
        }
    } else {
        UiTransform::default()
    };
    for (mut transform, mut shadow) in &mut cards {
        transform.set_if_neq(posed);
        shadow.set_if_neq(card_shadow(lift));
    }
}

/// The card's elevation: a close ambient shadow and a long key shadow, the
/// key reaching further while the card is lifted to turn (`lift` 0 to 1).
///
/// Its own and not [`soft_shadow`]: that one is a control's, and a control
/// sits on a surface where this card stands off the page.
pub(super) fn card_shadow(lift: f32) -> BoxShadow {
    BoxShadow(vec![
        ShadowStyle {
            color: Color::srgba(0.0, 0.0, 0.0, 0.35),
            x_offset: px(0),
            y_offset: px(2),
            spread_radius: px(0),
            blur_radius: px(6),
        },
        ShadowStyle {
            color: Color::srgba(0.0, 0.0, 0.0, 0.50),
            x_offset: px(0),
            y_offset: px(12.0 + 6.0 * lift),
            spread_radius: px(-2),
            blur_radius: px(30.0 + 10.0 * lift),
        },
    ])
}

/// The card, with the face that is up.
pub(super) fn card(
    commands: &mut Commands,
    state: &LobbyState,
    face: Face,
    fonts: &UiFonts,
    metrics: Metrics,
    registering: bool,
    scrolled_to: &Scrolled,
) -> Entity {
    let card = commands
        .spawn((
            FrontCard,
            Node {
                width: card_width(metrics.frame),
                max_width: percent(100),
                min_height: card_height(metrics.frame),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap),
                padding: UiRect::all(px(metrics.pad * 1.6)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(CARD_RADIUS)),
                ..default()
            },
            // No fill of its own: the leather under it is the fill, cut to
            // the same corners, and a square fill here is what showed past
            // the rounding before.
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.45)),
            card_shadow(0.0),
            UiTransform::default(),
            super::dock::Dock(1),
            Pickable::IGNORE,
        ))
        .id();
    match face {
        Face::Gateway => gateway_face(commands, card, state, fonts, metrics, scrolled_to),
        Face::Account => account_face(commands, card, state, fonts, metrics, registering),
    }
    let footer = version_footer(commands, fonts, metrics);
    commands.entity(card).add_child(footer);
    card
}

/// A face's first row: 44 pixels, which is where the leather's tooled rule
/// runs, so the eyebrow sits on it.
fn header(commands: &mut Commands, metrics: Metrics) -> Entity {
    let header = row(commands, metrics, false);
    commands
        .entity(header)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.min_height = px(44);
        });
    header
}

/// A reserved slot for the lobby's status line, two lines deep, so a refusal
/// arriving does not push the form about.
fn status_slot(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let slot = commands
        .spawn((
            Node {
                width: percent(100),
                min_height: px(metrics.small * 2.8),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let status = commands
        .spawn((
            Text::new(state.lobby.status()),
            tf(fonts, metrics.small),
            TextColor(status_ink(state.lobby.tone())),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(slot).add_child(status);
    slot
}

/// The build, centred at the foot of either face.
///
/// The same string the signed-in header draws (`baylee_build::short()`), for
/// the same reason: it is what a bug report is worthless without.
fn version_footer(commands: &mut Commands, fonts: &UiFonts, metrics: Metrics) -> Entity {
    let footer = commands
        .spawn((
            Node {
                width: percent(100),
                justify_content: JustifyContent::Center,
                // Down to the foot, whatever the face above it takes.
                margin: UiRect::top(Val::Auto),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let build = commands
        .spawn((
            Text::new(baylee_build::short()),
            tf(fonts, metrics.small * 0.9),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(footer).add_child(build);
    footer
}

/// Two controls sharing a row in equal halves.
fn halves(commands: &mut Commands, metrics: Metrics, controls: &[Entity]) -> Entity {
    let halves = row(commands, metrics, false);
    for &control in controls {
        commands
            .entity(control)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.flex_basis = px(0);
                node.flex_grow = 1.0;
                node.min_width = px(0);
                node.justify_content = JustifyContent::Center;
            });
        commands.entity(halves).add_child(control);
    }
    halves
}

/// Face one: which gateway.
fn gateway_face(
    commands: &mut Commands,
    card: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
) {
    let lang = state.lobby.lang();
    let header = header(commands, metrics);
    let eyebrow = note(commands, fonts, metrics, Phrase::GatewayStep.text(lang));
    commands.entity(header).add_child(eyebrow);
    let title = heading(commands, fonts, metrics, Phrase::ChooseGateway.text(lang));
    let hint = note(commands, fonts, metrics, Phrase::GatewayHint.text(lang));
    commands.entity(card).add_children(&[header, title, hint]);

    if !state.gateways.is_empty() {
        let list = super::gateway::list(commands, state, fonts, metrics, scrolled_to);
        commands.entity(card).add_child(list);
    }

    let field = text_field(
        commands,
        fonts,
        metrics,
        Phrase::GatewayAddress.text(lang),
        &FieldLook {
            buffer: state.lobby.buffer(Field::Gateway),
            focused: state.lobby.focus() == Field::Gateway,
            mask: None,
            press: Press::Focus(Field::Gateway),
            lead: None,
            hint: Some("https://"),
            tail: None,
        },
    );
    let save = button(
        commands,
        fonts,
        metrics,
        Phrase::SaveGateway.text(lang),
        Press::AddGateway,
        palette::PANEL_LIT,
        !state.lobby.busy() && state.adding.is_none(),
    );
    if metrics.frame == Frame::Phone {
        commands.entity(card).add_children(&[field, save]);
    } else {
        // The address and its button, one group on one line: the box grows,
        // the button keeps its width, and the two share a baseline because
        // the row sits at its foot, under the box's caption.
        let add = row(commands, metrics, false);
        commands.entity(add).entry::<Node>().and_modify(|mut node| {
            node.align_items = AlignItems::FlexEnd;
        });
        commands
            .entity(field)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.flex_grow = 1.0;
                node.min_width = px(0);
            });
        commands
            .entity(save)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.width = px(150);
                node.justify_content = JustifyContent::Center;
            });
        commands.entity(add).add_children(&[field, save]);
        commands.entity(card).add_child(add);
    }
    let status = status_slot(commands, state, fonts, metrics);
    commands.entity(card).add_child(status);

    let rule = commands
        .spawn((
            Node {
                width: percent(100),
                height: px(1),
                ..default()
            },
            BackgroundColor(palette::DOCK_EDGE.with_alpha(0.30)),
            Pickable::IGNORE,
        ))
        .id();
    // Two doors of one weight, apart from the list: playing without a
    // gateway, and the settings that are this device's.
    let offline = button(
        commands,
        fonts,
        metrics,
        Phrase::PlayOffline.text(lang),
        Press::PlayOffline,
        palette::PANEL,
        true,
    );
    commands.entity(offline).insert(super::hint::HoverHint(
        Phrase::OfflineBenefit.text(lang).to_string(),
    ));
    let settings = button(
        commands,
        fonts,
        metrics,
        Phrase::Settings.text(lang),
        Press::OpenSettings,
        palette::PANEL,
        true,
    );
    let doors = halves(commands, metrics, &[offline, settings]);
    commands.entity(card).add_children(&[rule, doors]);
}

/// Face two: the account, at the gateway chosen on face one.
#[allow(clippy::too_many_lines)] // one flat form, read top to bottom
fn account_face(
    commands: &mut Commands,
    card: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    registering: bool,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();

    // Back on the left, the eyebrow on the right: the turned card carries
    // its header the other way round from face one.
    let header = header(commands, metrics);
    let back = crate::hud::answer_sized(
        commands,
        fonts,
        Phrase::Back.text(lang),
        if lobby.busy() {
            crate::hud::ButtonWeight::Dead
        } else {
            crate::hud::ButtonWeight::Ghost
        },
        None,
        metrics.tap,
        metrics.text,
    );
    super::button_style::icon(commands, fonts, back, Press::LeaveGateway, metrics.small);
    if !lobby.busy() {
        commands.entity(back).insert(Press::LeaveGateway);
    }
    let gap = commands.spawn((spacer(), Pickable::IGNORE)).id();
    let eyebrow = note(commands, fonts, metrics, Phrase::AccountStep.text(lang));
    commands.entity(header).add_children(&[back, gap, eyebrow]);
    let title = heading(
        commands,
        fonts,
        metrics,
        if registering {
            Phrase::CreateAccount
        } else {
            Phrase::SignIn
        }
        .text(lang),
    );
    let plaque = super::gateway::plaque(commands, state, fonts, metrics);
    commands.entity(card).add_children(&[header, title, plaque]);

    // Playing as a guest (#269) goes here, between the plaque and the tabs,
    // when it comes: a full-width button and a rule reading "or". Nothing is
    // drawn for it until then (see `card_height`).

    let mut tabs = Vec::new();
    for (label, active) in [
        (Phrase::SignIn, !registering),
        (Phrase::CreateAccount, registering),
    ] {
        let enabled =
            !lobby.busy() && (label != Phrase::CreateAccount || lobby.registration_enabled());
        let tab = button(
            commands,
            fonts,
            metrics,
            label.text(lang),
            if active {
                Press::PickerNothing
            } else {
                Press::ToggleRegistering
            },
            palette::PANEL,
            enabled,
        );
        // `button` draws every enabled tone but the loud ones as the same
        // key, so the tab that is open is lit here.
        if active && enabled {
            commands.entity(tab).insert((
                ActiveTab,
                BackgroundColor(palette::PANEL_HOT),
                crate::ambience::Feel::new(palette::PANEL_HOT),
            ));
        }
        tabs.push(tab);
    }
    let tabs = halves(commands, metrics, &tabs);
    commands.entity(card).add_child(tabs);

    let email = text_field(
        commands,
        fonts,
        metrics,
        Phrase::Email.text(lang),
        &FieldLook {
            buffer: lobby.buffer(Field::Email),
            focused: lobby.focus() == Field::Email,
            mask: None,
            press: Press::Focus(Field::Email),
            lead: None,
            hint: None,
            tail: None,
        },
    );
    commands.entity(card).add_child(email);
    if registering {
        let name = text_field(
            commands,
            fonts,
            metrics,
            Phrase::DisplayName.text(lang),
            &FieldLook {
                buffer: lobby.buffer(Field::DisplayName),
                focused: lobby.focus() == Field::DisplayName,
                mask: None,
                press: Press::Focus(Field::DisplayName),
                lead: None,
                hint: None,
                tail: None,
            },
        );
        let hint = note(commands, fonts, metrics, Phrase::AccountNameHint.text(lang));
        commands.entity(card).add_children(&[name, hint]);
    }
    let password = text_field(
        commands,
        fonts,
        metrics,
        Phrase::Password.text(lang),
        &FieldLook {
            buffer: lobby.buffer(Field::Password),
            focused: lobby.focus() == Field::Password,
            mask: Some(Masked {
                field: Field::Password,
                shown: lobby.showing(Field::Password),
            }),
            press: Press::Focus(Field::Password),
            lead: None,
            hint: None,
            tail: None,
        },
    );
    commands.entity(card).add_child(password);
    if registering {
        let hint = note(
            commands,
            fonts,
            metrics,
            Phrase::AccountPasswordHint.text(lang),
        );
        commands.entity(card).add_child(hint);
    }

    let submit = button(
        commands,
        fonts,
        metrics,
        if registering {
            Phrase::CreateAccount.text(lang)
        } else {
            Phrase::SignIn.text(lang)
        },
        Press::Submit,
        palette::ACCENT,
        state.gateway_selected && !lobby.busy(),
    );
    commands
        .entity(submit)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.justify_content = JustifyContent::Center;
        });
    let status = status_slot(commands, state, fonts, metrics);
    commands.entity(card).add_children(&[submit, status]);
}
