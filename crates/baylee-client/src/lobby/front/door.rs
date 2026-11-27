//! The front door's frame around the card (`DESIGN-v5` §3, §2.7): the
//! logo, the tagline, the text row, the colophon and the About sheet — and
//! where each stands in each size class.
//!
//! Principle 8: the notices travel with the door. Every face at every size
//! class carries a colophon on a mist plate: on Wide and Vast the full text
//! (the build, the Fan Content Policy's notice word for word, Scryfall's
//! credit, the source link) and the QR code; on the smaller classes the
//! one-line colophon, whose notice opens the About sheet with the full
//! text one press away (`docs/legal.md` §"The front door's notices").
//! No text stands on the painting without a mist plate (§2.2).

#[allow(clippy::wildcard_imports)] // the lobby widget vocabulary
use super::super::*;
use super::keys;
use super::{FAN_CONTENT_NOTICE, FrontPress, source_address};
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::focus::{Current, Stop};
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::role::Role;
use crate::shellkit::{surfaces, tokens};

/// Whether the full colophon (and the QR) stands under the card: on Wide
/// and Vast, where the window is tall enough to hold it beside the card
/// and the text row (an 820-px tablet takes the one-line colophon and
/// About, as the smaller classes do).
pub(super) fn full_colophon(kit: Kit, height: f32) -> bool {
    matches!(kit.m.frame, Frame::Wide | Frame::Vast) && height >= FULL_COLOPHON_HEIGHT
}

/// The least window height (logical px) the full colophon stands in.
pub(in crate::lobby) const FULL_COLOPHON_HEIGHT: f32 = 900.0;

/// Bottom inset (logical px) shared by the bottom-left version pill, the
/// bottom-center colophon box, and the bottom-right info pill so all three
/// bottom edges sit flush on one horizontal line.
pub(crate) const FOOTER_BOTTOM: f32 = 10.0;

/// Fixed height of the bottom-left version pill and bottom-right info pill.
const FOOTER_PILL_H: f32 = 28.0;

/// Leading release-tag glyph (`fa-tag`, `U+F02B`) inside the bottom-left version badge.
pub(crate) const VERSION_TAG_GLYPH: char = '\u{f02b}';

/// Airy 3-stop smoked cut-glass gradient for bottom colophon and tagline mist plates.
///
/// Paired with `BackgroundColor(Color::NONE)` so Bevy 0.20 draws a single airy
/// glass quad (`0.38..0.48` alpha) over the animated world vista.
pub(crate) fn glass_mist_gradient() -> BackgroundGradient {
    use bevy::ui::{ColorStop, InterpolationColorSpace, LinearGradient};
    BackgroundGradient::from(
        LinearGradient::to_bottom(vec![
            ColorStop::percent(Color::srgba(0.060, 0.110, 0.170, 0.46), 0.0),
            ColorStop::percent(Color::srgba(0.030, 0.060, 0.100, 0.38), 50.0),
            ColorStop::percent(Color::srgba(0.020, 0.045, 0.080, 0.48), 100.0),
        ])
        .in_color_space(InterpolationColorSpace::Srgba),
    )
}

/// Subtle directional specular glass border for mist and colophon plates.
pub(crate) const fn glass_mist_border() -> BorderColor {
    BorderColor {
        top: Color::srgba(0.86, 0.80, 0.66, 0.30),
        right: Color::srgba(0.58, 0.66, 0.76, 0.20),
        bottom: Color::srgba(0.46, 0.42, 0.34, 0.22),
        left: Color::srgba(0.58, 0.66, 0.76, 0.20),
    }
}

/// A mist plate holding a row of things that would stand on the painting.
fn plate(commands: &mut Commands, kit: Kit) -> Entity {
    commands
        .spawn((
            Role::Mist,
            Node {
                padding: UiRect::axes(kit.m.px(10.0), kit.m.px(2.0)),
                column_gap: kit.m.px(2.0),
                row_gap: kit.m.px(2.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_wrap: FlexWrap::Wrap,
                max_width: Val::Percent(100.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            glass_mist_gradient(),
            glass_mist_border(),
            surfaces::elevation_control(),
            Pickable::IGNORE,
        ))
        .id()
}

/// Retains the front-door emblem's smooth warmth (`-1.0` pressed .. `0.0`
/// resting .. `+1.0` hovered/focused) across UI rebuilds without layout shift.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub(crate) struct FrontLogoMemory {
    warmth: f32,
}

#[derive(Component)]
pub(crate) struct FrontLogoButton(Option<f32>);

#[derive(Component)]
pub(crate) struct FrontLogoFace;

#[derive(Component)]
pub(crate) struct FrontLogoAura;

fn logo_aura_gradient(hot: f32) -> BackgroundGradient {
    use bevy::ui::{
        ColorStop, InterpolationColorSpace, RadialGradient, RadialGradientShape, UiPosition,
    };
    let a = hot.clamp(0.0, 1.0);
    BackgroundGradient::from(
        RadialGradient::new(
            UiPosition::CENTER.at_y(Val::Percent(2.0)),
            RadialGradientShape::Ellipse(Val::Percent(46.0), Val::Percent(43.0)),
            vec![
                ColorStop::percent(Color::srgba(1.0, 0.85, 0.48, 0.34 * a), 0.0),
                ColorStop::percent(Color::srgba(0.96, 0.72, 0.32, 0.14 * a), 52.0),
                ColorStop::percent(Color::srgba(0.96, 0.72, 0.32, 0.0), 100.0),
            ],
        )
        .in_color_space(InterpolationColorSpace::Srgba),
    )
}

pub(crate) fn logo_face_color(warmth: f32) -> Color {
    let w = warmth.clamp(-1.0, 1.0);
    if w >= 0.0 {
        Color::LinearRgba(LinearRgba::new(
            1.0 + 0.28 * w,
            1.0 + 0.18 * w,
            1.0 + 0.02 * w,
            1.0,
        ))
    } else {
        let sink = -w;
        Color::LinearRgba(LinearRgba::new(
            1.0 - 0.10 * sink,
            1.0 - 0.12 * sink,
            1.0 - 0.16 * sink,
            1.0,
        ))
    }
}

fn spawn_logo_layers(commands: &mut Commands, logo: Handle<Image>) -> [Entity; 5] {
    use bevy::ui::{
        ColorStop, InterpolationColorSpace, RadialGradient, RadialGradientShape, UiPosition,
    };
    let veil = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(-10.0),
                right: Val::Percent(-10.0),
                top: Val::Percent(-8.0),
                bottom: Val::Percent(-14.0),
                ..default()
            },
            BackgroundGradient::from(
                RadialGradient::new(
                    UiPosition::CENTER.at_y(Val::Percent(4.0)),
                    RadialGradientShape::Ellipse(Val::Percent(47.0), Val::Percent(45.0)),
                    vec![
                        ColorStop::percent(Color::srgba(0.01, 0.02, 0.05, 0.48), 0.0),
                        ColorStop::percent(Color::srgba(0.01, 0.02, 0.05, 0.24), 52.0),
                        ColorStop::percent(Color::srgba(0.01, 0.02, 0.05, 0.0), 100.0),
                    ],
                )
                .in_color_space(InterpolationColorSpace::Srgba),
            ),
            Pickable::IGNORE,
        ))
        .id();
    let silhouette = |commands: &mut Commands, y_px: f32, color: Color, img: Handle<Image>| {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px_fixed(0.0),
                    top: px_fixed(y_px),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ImageNode {
                    image: img,
                    color,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id()
    };
    let deep = silhouette(
        commands,
        11.0,
        Color::srgba(0.01, 0.01, 0.02, 0.36),
        logo.clone(),
    );
    let contact = silhouette(
        commands,
        5.0,
        Color::srgba(0.02, 0.02, 0.04, 0.68),
        logo.clone(),
    );
    let aura = commands
        .spawn((
            FrontLogoAura,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(-8.0),
                right: Val::Percent(-8.0),
                top: Val::Percent(-10.0),
                bottom: Val::Percent(-10.0),
                ..default()
            },
            logo_aura_gradient(0.0),
            Pickable::IGNORE,
        ))
        .id();
    let crisp = commands
        .spawn((
            FrontLogoFace,
            Node {
                position_type: PositionType::Absolute,
                left: px_fixed(0.0),
                top: px_fixed(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ImageNode::new(logo),
            Pickable::IGNORE,
        ))
        .id();
    [veil, deep, contact, aura, crisp]
}

/// Builds the Login screen's Baylee emblem button with true silhouette drop
/// shadows (two offset dark-silhouette `ImageNode` layers + soft elliptical
/// radial dark-veil backdrop) and zero rectangular button box or layout shift.
pub(super) fn front_logo(
    commands: &mut Commands,
    state: &LobbyState,
    logo: Handle<Image>,
    logo_width: f32,
    phone: bool,
    full: bool,
) -> Entity {
    let name = Phrase::MemorialOpen.text(state.lobby.lang());
    let mut a11y = accesskit::Node::new(accesskit::Role::Button);
    a11y.set_label(name);
    let brand = commands
        .spawn((
            super::super::memorial::Cloud { front: true },
            FrontLogoButton(None),
            crate::ui_button::UiButton,
            Role::Hit,
            Name::new(name.to_owned()),
            bevy::a11y::AccessibilityNode(a11y),
            Press::Memorial(super::super::memorial::MemorialPress::Open),
            Stop::new(keys::table_of(state).name, "baylee"),
            UiTransform::default(),
            BackgroundColor(Color::NONE),
            Node {
                width: px_fixed(logo_width),
                max_width: Val::Percent(100.0),
                max_height: Val::Vh(if phone {
                    32.0
                } else if full {
                    28.0
                } else {
                    23.5
                }),
                aspect_ratio: Some(1942.0 / 809.0),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
        ))
        .id();
    let layers = spawn_logo_layers(commands, logo);
    commands.entity(brand).add_children(&layers);
    brand
}

/// Smoothly animates the Login emblem's warm luminous lift and radial aura on
/// hover/focus and subtle settle on press, staying completely quiet at rest.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn animate_front_logo(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    quality: Option<Res<crate::quality::InUse>>,
    state: Res<LobbyState>,
    focus: Res<bevy::input_focus::InputFocus>,
    ring: Res<bevy::input_focus::InputFocusVisible>,
    mut memory: ResMut<FrontLogoMemory>,
    mut buttons: Query<(
        Entity,
        &mut FrontLogoButton,
        &bevy::picking::hover::Hovered,
        Has<bevy::ui::Pressed>,
        Option<&bevy::picking::hover::PickingInteraction>,
        &Children,
    )>,
    mut faces: Query<&mut ImageNode, With<FrontLogoFace>>,
    mut auras: Query<(&mut BackgroundGradient, &mut Visibility), With<FrontLogoAura>>,
) {
    if buttons.is_empty() {
        if memory.warmth.abs() > f32::EPSILON {
            memory.bypass_change_detection().warmth = 0.0;
        }
        return;
    }
    let mut target = 0.0_f32;
    if !state.memorial.open {
        for (e, _, hovered, pressed, pick, _) in &buttons {
            if pressed
                || matches!(
                    pick,
                    Some(bevy::picking::hover::PickingInteraction::Pressed)
                )
            {
                target = -1.0;
                break;
            }
            if hovered.0
                || matches!(
                    pick,
                    Some(bevy::picking::hover::PickingInteraction::Hovered)
                )
                || (ring.0 && focus.get() == Some(e))
            {
                target = 1.0;
            }
        }
    }
    let still = crate::quality::ambient_still(
        prefs.is_some_and(|p| p.all().reduce_motion),
        quality.as_deref(),
    );
    let dt = if time.delta_secs().is_finite() {
        time.delta_secs().clamp(0.0, 0.1)
    } else {
        0.0
    };
    let current = memory.warmth;
    let next = if still || (current - target).abs() < 0.001 {
        target
    } else {
        let step = 1.0 - (-12.0 * dt).exp();
        let cand = current + (target - current) * step;
        if (cand - target).abs() < 0.001 {
            target
        } else {
            cand
        }
    };
    if next.to_bits() != current.to_bits() {
        memory.bypass_change_detection().warmth = next;
    }
    for (_, mut painted, _, _, _, children) in &mut buttons {
        if painted
            .0
            .is_some_and(|last| last.to_bits() == next.to_bits())
        {
            continue;
        }
        if painted.0.is_none() && next == 0.0 {
            painted.0 = Some(0.0);
            continue;
        }
        painted.0 = Some(next);
        let want_face = logo_face_color(next);
        let hot = next.max(0.0);
        let want_vis = if hot > 0.002 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let want_aura = logo_aura_gradient(hot);
        for child in children.iter() {
            if let Ok(mut img) = faces.get_mut(child)
                && img.color != want_face
            {
                img.color = want_face;
            }
            if let Ok((mut grad, mut vis)) = auras.get_mut(child) {
                vis.set_if_neq(want_vis);
                if hot > 0.002 {
                    grad.set_if_neq(want_aura.clone());
                }
            }
        }
    }
}

/// How a [`link`] reads.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Look {
    /// An address or a door to more text: underlined.
    Underlined,
}

/// A word on a mist plate a press reaches: a ghost button in the kit's hit
/// area.
pub(super) fn link(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    look: Look,
    action: impl Bundle,
) -> Entity {
    // A link, not a button: it reads as words (an address may be long), so
    // it holds no button's label budget; its hit area is the kit's.
    let face = commands
        .spawn((
            Node {
                // Room either side of the word, so the chosen language's
                // ground does not end at its letters (owner, 09.10.2026).
                padding: UiRect::axes(kit.m.px(12.0), px_fixed(0.0)),
                min_height: px_fixed(kit.m.control * 0.8),
                flex_grow: 1.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .id();
    let mut words = commands.spawn((
        Text::new(text),
        crate::hud::tf(kit.fonts, kit.m.small),
        TextColor(tokens::INK),
        TextLayout::no_wrap(),
        Pickable::IGNORE,
    ));
    if look == Look::Underlined {
        words.insert((Underline, UnderlineColor(tokens::INK)));
    }
    let words = words.id();
    commands.entity(face).add_child(words);
    let wrapper = controls::hit(commands, kit, face, action);
    commands
        .entity(face)
        .insert(crate::ambience::Feel::rising_to(Color::NONE, tokens::HOVER));
    wrapper
}

/// A separator dot between links.
fn sep(commands: &mut Commands, kit: Kit) -> Entity {
    controls::label(commands, kit, "\u{b7}", kit.m.small, tokens::INK)
}

/// Shared card title lockup for both the Login form (`Baylee Authentifizierung`)
/// and the Gateway Selection form (`Wähle dein Gateway`), with a glowing
/// emblem badge and an accent underline bar.
pub(super) fn card_title_lockup(
    commands: &mut Commands,
    kit: Kit,
    glyph: char,
    title: &str,
) -> Entity {
    let row = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(10.0),
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let badge_bg = Color::srgba(0.09, 0.22, 0.25, 0.92);
    let badge_rim = tokens::ACCENT.with_alpha(0.50);
    let badge = commands
        .spawn((
            Node {
                width: px_fixed(30.0),
                height: px_fixed(30.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(8.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(badge_bg),
            BorderColor::all(badge_rim),
            BoxShadow(vec![ShadowStyle {
                color: tokens::ACCENT.with_alpha(0.24),
                x_offset: px_fixed(0.0),
                y_offset: px_fixed(0.0),
                spread_radius: px_fixed(0.5),
                blur_radius: px_fixed(10.0),
            }]),
            crate::shellkit::sheen::Sheen,
            Pickable::IGNORE,
        ))
        .id();
    let badge_icon = commands
        .spawn((
            Text::new(glyph.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.96),
            TextColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(badge).add_child(badge_icon);

    let title_col = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                row_gap: px_fixed(3.0),
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            Text::new(title),
            crate::hud::tf_bold(kit.fonts, kit.m.head),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    let accent_bar = commands
        .spawn((
            Node {
                width: px_fixed(32.0),
                height: px_fixed(2.0),
                border_radius: BorderRadius::all(px_fixed(1.0)),
                ..default()
            },
            BackgroundColor(tokens::ACCENT.with_alpha(0.72)),
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(title_col)
        .add_children(&[words, accent_bar]);
    commands.entity(row).add_children(&[badge, title_col]);
    row
}

/// Compact square icon button (`30x30` px) for the utility header bar (Music,
/// Settings), matching the height and border of the Language Dropdown.
pub(in crate::lobby) fn header_icon_btn(
    commands: &mut Commands,
    kit: Kit,
    glyph: char,
    active: bool,
    hint: &str,
    action: impl Bundle,
) -> Entity {
    let h = px_fixed(kit.m.control * 0.78);
    let bg = if active {
        Color::srgba(0.08, 0.21, 0.23, 0.72)
    } else {
        Color::srgba(0.04, 0.08, 0.13, 0.56)
    };
    let hot = Color::srgba(0.11, 0.19, 0.28, 0.86);
    let border = if active {
        tokens::ACCENT.with_alpha(0.55)
    } else {
        Color::srgba(0.64, 0.70, 0.78, 0.34)
    };
    let face = commands
        .spawn((
            Role::Button,
            Node {
                width: h,
                min_height: h,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(border),
            surfaces::elevation_control(),
            crate::ambience::Feel::tinting_to(bg, hot),
            crate::ambience::FeelBorder {
                rest: border,
                hot: tokens::ACCENT.with_alpha(0.70),
                press: tokens::ACCENT,
            },
            crate::ambience::FeelShadow,
            super::super::hint::HoverHint(hint.to_string()),
        ))
        .id();
    let icon = commands
        .spawn((
            Text::new(glyph.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.88),
            TextColor(if active { tokens::ACCENT } else { tokens::INK }),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_child(icon);
    controls::hit(commands, kit, face, action)
}

#[cfg(not(target_arch = "wasm32"))]
fn header_sign_out_btn(
    commands: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    table: &'static str,
) -> Entity {
    let lang = state.lobby.lang();
    let h = px_fixed(kit.m.control * 0.78);
    let bg = Color::srgba(0.20, 0.09, 0.12, 0.72);
    let hot = Color::srgba(0.29, 0.12, 0.16, 0.90);
    let border = tokens::DANGER.with_alpha(0.45);
    let face = commands
        .spawn((
            Role::Button,
            Node {
                min_height: h,
                padding: UiRect::axes(kit.m.px(10.0), px_fixed(0.0)),
                column_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(border),
            surfaces::elevation_control(),
            crate::ambience::Feel::tinting_to(bg, hot),
            crate::ambience::FeelBorder {
                rest: border,
                hot: tokens::DANGER.with_alpha(0.80),
                press: tokens::DANGER,
            },
            crate::ambience::FeelShadow,
        ))
        .id();
    let icon = commands
        .spawn((
            Text::new("\u{f2f5}"),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.84),
            TextColor(tokens::DANGER),
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            Text::new(Phrase::CentralSignOut.text(lang)),
            crate::hud::tf_bold(kit.fonts, kit.m.small),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_children(&[icon, words]);
    controls::hit(
        commands,
        kit,
        face,
        (
            Press::Front(FrontPress::CentralSignOut),
            Stop::new(table, "central-sign-out"),
        ),
    )
}

/// Top-right utility cluster anchored to the top-right corner of the window
/// on the Login screen (`!state.central_ready()`).
pub(super) fn top_right_controls(
    commands: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    music_on: bool,
) -> Entity {
    let holder = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px_fixed(14.0),
                right: px_fixed(16.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },
            GlobalZIndex(20),
            Pickable::IGNORE,
        ))
        .id();
    let cluster = header_controls(commands, state, kit, music_on, false);
    // The public ladder, readable without an account, stands with the
    // corner's other tools rather than among the community's links.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let table = keys::table_of(state).name;
        let ladder = header_icon_btn(
            commands,
            kit,
            crate::hud::glyph::TROPHY,
            false,
            Phrase::RankedBoard.text(state.lobby.lang()),
            (
                Press::Ranked(crate::lobby::ranked::Action::Open),
                Stop::new(table, "ranked"),
            ),
        );
        commands.entity(cluster).insert_children(0, &[ladder]);
    }
    commands.entity(holder).add_child(cluster);
    holder
}

/// Top-right utility cluster: Music toggle, Language Dropdown (with flag icon
/// + animated popover), Settings icon button, and optional `Abmelden` button.
pub(super) fn header_controls(
    commands: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    music_on: bool,
    show_logout: bool,
) -> Entity {
    let lang = state.lobby.lang();
    let table = keys::table_of(state).name;
    let cluster = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexEnd,
                column_gap: kit.m.px(6.0),
                row_gap: kit.m.px(4.0),
                flex_wrap: FlexWrap::Wrap,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let music_hint = if music_on {
        Phrase::MusicPlaying.text(lang)
    } else {
        Phrase::MusicSilent.text(lang)
    };
    let music = header_icon_btn(
        commands,
        kit,
        '\u{f001}',
        music_on,
        music_hint,
        (
            Press::Shared(SharedPress::ToggleMusic),
            Stop::new(table, "music"),
        ),
    );
    let lang_btn = lang_dropdown(commands, state, kit, table);
    let settings = header_icon_btn(
        commands,
        kit,
        '\u{f013}',
        false,
        Phrase::Settings.text(lang),
        (
            Press::Settings(SettingsPress::OpenSettings),
            Stop::new(table, "settings"),
        ),
    );
    commands
        .entity(cluster)
        .add_children(&[music, lang_btn, settings]);
    #[cfg(not(target_arch = "wasm32"))]
    if show_logout && state.central_ready() {
        let logout = header_sign_out_btn(commands, state, kit, table);
        commands.entity(cluster).add_child(logout);
    }
    #[cfg(target_arch = "wasm32")]
    let _ = show_logout;
    cluster
}

/// Mini painted flag icon (`18x12` px) for the language selector, matching
/// the flag badges used on `baylee-central`.
pub(in crate::lobby) fn flag_icon(commands: &mut Commands, lang: Lang) -> Entity {
    let flag = commands
        .spawn((
            Node {
                width: px_fixed(18.0),
                height: px_fixed(12.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(2.5)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.22)),
            BackgroundColor(match lang {
                Lang::De => Color::srgb(0.08, 0.08, 0.08),
                Lang::En => Color::srgb(0.01, 0.13, 0.41),
            }),
            Pickable::IGNORE,
        ))
        .id();
    match lang {
        Lang::De => {
            for color in [
                Color::srgb(0.10, 0.10, 0.10),
                Color::srgb(0.86, 0.05, 0.05),
                Color::srgb(1.0, 0.81, 0.0),
            ] {
                let stripe = commands
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_grow: 1.0,
                            ..default()
                        },
                        BackgroundColor(color),
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(flag).add_child(stripe);
            }
        }
        Lang::En => {
            let h_white = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: px_fixed(4.0),
                        ..default()
                    },
                    BackgroundColor(Color::WHITE),
                    Pickable::IGNORE,
                ))
                .id();
            let v_white = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: px_fixed(4.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::WHITE),
                    Pickable::IGNORE,
                ))
                .id();
            let h_red = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: px_fixed(2.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.78, 0.06, 0.18)),
                    Pickable::IGNORE,
                ))
                .id();
            let v_red = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: px_fixed(2.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.78, 0.06, 0.18)),
                    Pickable::IGNORE,
                ))
                .id();
            commands
                .entity(flag)
                .add_children(&[h_white, v_white, h_red, v_red]);
        }
    }
    flag
}

/// Language dropdown button + animated downward popover menu with flag icons.
pub(in crate::lobby) fn lang_dropdown(
    commands: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    table: &'static str,
) -> Entity {
    let lang = state.lobby.lang();
    let open = state.front_menu;
    let holder = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let trigger_bg = if open {
        Color::srgba(0.08, 0.21, 0.23, 0.82)
    } else {
        Color::srgba(0.04, 0.08, 0.13, 0.56)
    };
    let trigger_hot = Color::srgba(0.11, 0.19, 0.28, 0.86);
    let trigger_border = if open {
        tokens::ACCENT.with_alpha(0.70)
    } else {
        Color::srgba(0.64, 0.70, 0.78, 0.34)
    };
    let face = commands
        .spawn((
            Role::Button,
            Node {
                min_height: px_fixed(kit.m.control * 0.78),
                padding: UiRect::axes(kit.m.px(10.0), px_fixed(0.0)),
                column_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(trigger_bg),
            BorderColor::all(trigger_border),
            surfaces::elevation_control(),
            crate::ambience::Feel::tinting_to(trigger_bg, trigger_hot),
            crate::ambience::FeelBorder {
                rest: trigger_border,
                hot: tokens::ACCENT.with_alpha(0.70),
                press: tokens::ACCENT,
            },
            crate::ambience::FeelShadow,
            crate::shellkit::sheen::Sheen,
            super::super::hint::HoverHint(Phrase::Language.text(lang).to_string()),
        ))
        .id();
    let flag = flag_icon(commands, lang);
    let code = commands
        .spawn((
            Text::new(lang.code().to_ascii_uppercase()),
            crate::hud::tf_bold(kit.fonts, kit.m.small),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    let chevron = commands
        .spawn((
            Text::new(if open { "\u{f077}" } else { "\u{f078}" }),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.76),
            TextColor(if open { tokens::ACCENT } else { tokens::MUTED }),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_children(&[flag, code, chevron]);
    let trigger = controls::hit(
        commands,
        kit,
        face,
        (
            Press::Front(FrontPress::FrontMenu),
            Stop::new(table, "lang"),
        ),
    );
    commands.entity(holder).add_child(trigger);
    let menu = lang_menu_popover(commands, kit, table, lang, open);
    commands.entity(holder).add_child(menu);
    holder
}

fn lang_menu_popover(
    commands: &mut Commands,
    kit: Kit,
    table: &'static str,
    lang: Lang,
    open: bool,
) -> Entity {
    let mut menu_cmd = commands.spawn((
        Role::Opaque,
        Node {
            display: if open { Display::Flex } else { Display::None },
            position_type: PositionType::Absolute,
            top: px_fixed(kit.m.hit + 4.0),
            right: px_fixed(0.0),
            min_width: px_fixed(184.0),
            flex_direction: FlexDirection::Column,
            row_gap: px_fixed(3.0),
            padding: UiRect::all(px_fixed(6.0)),
            border: UiRect::all(px_fixed(1.0)),
            border_radius: BorderRadius::all(px_fixed(12.0)),
            ..default()
        },
        if open {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        },
        BackgroundColor(Color::srgba(0.05, 0.09, 0.14, 0.98)),
        BorderColor::all(tokens::PRIMARY_EDGE.with_alpha(0.65)),
        surfaces::popover_shadow(),
        GlobalZIndex(tokens::z::POPOVER),
        Press::Shared(SharedPress::PickerNothing),
    ));
    if open {
        menu_cmd.insert((
            crate::lobby::menus::Opening::default(),
            UiTransform::from_translation(Val2::new(px_fixed(0.0), px_fixed(-5.0))),
        ));
    }
    let menu = menu_cmd.id();

    let header = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(px_fixed(8.0), px_fixed(4.0)),
                column_gap: px_fixed(6.0),
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let header_icon = commands
        .spawn((
            Text::new("\u{f0ac}"),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.78),
            TextColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    let header_text = commands
        .spawn((
            Text::new(Phrase::Language.text(lang)),
            crate::hud::tf_bold(kit.fonts, kit.m.small * 0.84),
            TextColor(tokens::MUTED),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(header)
        .add_children(&[header_icon, header_text]);
    commands.entity(menu).add_child(header);

    for (i, offered) in Lang::ALL.into_iter().enumerate() {
        let item = lang_menu_item(commands, kit, table, i, offered, offered == lang);
        commands.entity(menu).add_child(item);
    }
    menu
}

fn lang_menu_item(
    commands: &mut Commands,
    kit: Kit,
    table: &'static str,
    i: usize,
    offered: Lang,
    on: bool,
) -> Entity {
    let row_bg = if on {
        tokens::SELECTED
    } else {
        Color::srgba(1.0, 1.0, 1.0, 0.02)
    };
    let row_border = if on {
        tokens::ACCENT.with_alpha(0.45)
    } else {
        Color::NONE
    };
    let item_face = commands
        .spawn((
            Role::MenuItem,
            Node {
                width: Val::Percent(100.0),
                min_height: px_fixed(kit.m.control * 0.82),
                padding: UiRect::axes(kit.m.px(10.0), px_fixed(0.0)),
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(row_bg),
            BorderColor::all(row_border),
            crate::ambience::Feel::tinting_to(row_bg, tokens::HOVER),
            crate::ambience::FeelBorder {
                rest: row_border,
                hot: tokens::ACCENT.with_alpha(0.60),
                press: tokens::ACCENT,
            },
        ))
        .id();
    let item_flag = flag_icon(commands, offered);
    let item_words = commands
        .spawn((
            Text::new(offered.name()),
            if on {
                crate::hud::tf_bold(kit.fonts, kit.m.small)
            } else {
                crate::hud::tf(kit.fonts, kit.m.small)
            },
            TextColor(if on { tokens::INK } else { tokens::MUTED }),
            TextLayout::no_wrap(),
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let code_pill = commands
        .spawn((
            Text::new(offered.code().to_ascii_uppercase()),
            crate::hud::tf(kit.fonts, kit.m.small * 0.78),
            TextColor(tokens::MUTED.with_alpha(0.75)),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(item_face)
        .add_children(&[item_flag, item_words, code_pill]);
    if on {
        let check = commands
            .spawn((
                Text::new("\u{f00c}"),
                crate::hud::icon_tf(kit.fonts, kit.m.small * 0.82),
                TextColor(tokens::ACCENT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(item_face).add_child(check);
    }
    let item = controls::hit(
        commands,
        kit,
        item_face,
        (
            Press::Shared(SharedPress::PickLang(offered)),
            Stop::item(table, "lang", u8::try_from(i).unwrap_or(u8::MAX)),
            Current(on),
        ),
    );
    commands
        .entity(item)
        .entry::<Node>()
        .and_modify(|mut node| node.width = Val::Percent(100.0));
    item
}

/// How a door under the sign-in reads: its ground at rest and under the
/// pointer, its rim, and the mark before its word.
struct DoorLook {
    rest: Color,
    hot: Color,
    rim: Color,
    glyph: char,
    glyph_size: f32,
}

/// A door under the sign-in: a whole row's width, at the height and in the
/// type of the others, told apart by its ground and its mark.
fn door(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    look: &DoorLook,
    action: impl Bundle,
) -> Entity {
    let face = commands
        .spawn((
            Role::Button,
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(kit.m.px(13.0), px_fixed(0.0)),
                min_height: px_fixed(kit.m.control * 0.88),
                column_gap: kit.m.px(7.0),
                flex_grow: 1.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(look.rest),
            BorderColor::all(look.rim),
            surfaces::elevation_control(),
            crate::ambience::Feel::tinting_to(look.rest, look.hot),
            crate::ambience::FeelBorder {
                rest: look.rim,
                hot: tokens::ACCENT,
                press: tokens::PRIMARY_EDGE,
            },
            crate::ambience::FeelShadow,
            crate::shellkit::sheen::Sheen,
        ))
        .id();
    let icon = commands
        .spawn((
            Text::new(look.glyph.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * look.glyph_size),
            TextColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            Text::new(text),
            crate::hud::tf_bold(kit.fonts, kit.m.small),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_children(&[icon, words]);
    let wrapper = controls::hit(commands, kit, face, action);
    commands
        .entity(wrapper)
        .entry::<Node>()
        .and_modify(|mut node| node.width = Val::Percent(100.0));
    wrapper
}

/// Styled "Register" CTA button sharing the bottom row of the Login form
/// with [`offline_cta`].
pub(super) fn register_cta(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    action: impl Bundle,
) -> Entity {
    let look = DoorLook {
        rest: Color::srgba(0.10, 0.18, 0.30, 0.68),
        hot: Color::srgba(0.16, 0.26, 0.42, 0.90),
        rim: tokens::PRIMARY_EDGE.with_alpha(0.62),
        glyph: '\u{f234}',
        glyph_size: 0.88,
    };
    door(commands, kit, text, &look, action)
}

/// Prominent "Play offline" action button sharing the bottom row of the
/// Login form.
pub(super) fn offline_cta(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    action: impl Bundle,
) -> Entity {
    let look = DoorLook {
        rest: Color::srgba(0.09, 0.22, 0.21, 0.68),
        hot: Color::srgba(0.15, 0.33, 0.31, 0.90),
        rim: tokens::ACCENT.with_alpha(0.56),
        glyph: '\u{f04b}',
        glyph_size: 0.86,
    };
    door(commands, kit, text, &look, action)
}

/// The row under those two on the form with no `‹`: the quietest of the
/// three, because signing in is what this form is for.
pub(super) fn onward_cta(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    action: impl Bundle,
) -> Entity {
    let look = DoorLook {
        rest: Color::srgba(0.04, 0.08, 0.14, 0.56),
        hot: Color::srgba(0.11, 0.18, 0.26, 0.84),
        rim: Color::srgba(0.64, 0.70, 0.78, 0.34),
        glyph: '\u{f061}',
        glyph_size: 0.84,
    };
    door(commands, kit, text, &look, action)
}

/// The one-line colophon (§3), on every face below Wide: the notice in the
/// policy's own short words (its full text one press away, on About),
/// Scryfall's credit, and the source (the address itself on About, with its
/// code). One line at 640 px. The build stands under the text row
/// ([`version_line`], owner 09.10.2026).
pub(super) fn one_line(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    use crate::lobby::source::ExternalLink;
    let lang = state.lobby.lang();
    let table = keys::table_of(state).name;
    let row = plate(commands, kit);
    let notice = super::links::hyperlink(
        commands,
        kit,
        '\u{f05a}',
        Phrase::ColophonNotice.text(lang),
        true,
        (
            Press::Front(FrontPress::About(true)),
            Stop::new(table, "notice"),
        ),
    );
    let credit = super::links::hyperlink(
        commands,
        kit,
        super::links::EXTERNAL_ICON,
        Phrase::ColophonScryfall.text(lang),
        true,
        Press::Front(FrontPress::OpenExternal(ExternalLink::Scryfall)),
    );
    // The link opens the address the gateway gave only where it passed the
    // check at the door; else it opens About, which says it as text.
    let source = super::links::hyperlink(
        commands,
        kit,
        '\u{f126}',
        Phrase::ColophonSource.text(lang),
        true,
        (
            Press::Front(if state.source_code.is_some() {
                FrontPress::OpenSource
            } else {
                FrontPress::About(true)
            }),
            Stop::new(table, "source"),
        ),
    );
    let parts = [notice, credit, source];
    for (i, part) in parts.into_iter().enumerate() {
        if i > 0 {
            let s = sep(commands, kit);
            commands.entity(row).add_child(s);
        }
        commands.entity(row).add_child(part);
    }
    row
}

/// The full colophon (Wide, Vast): the Fan Content notice word for word and
/// Scryfall's credit, set as a short centred paragraph on a mist plate
/// rather than run into one long line (owner, 09.10.2026), with inline styled
/// hyperlinks on `Fan Content Policy` and `Scryfall`.
pub(super) fn full(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    use crate::lobby::source::ExternalLink;
    let lang = state.lobby.lang();
    let holder = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                padding: UiRect {
                    left: kit.m.px(8.0),
                    right: kit.m.px(8.0),
                    top: px_fixed(2.0),
                    bottom: px_fixed(FOOTER_BOTTOM),
                },
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            Role::Mist,
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                // About two lines of the notice: a paragraph, not a ribbon
                // across the window. A definite width, so the notice is
                // measured wrapped and the plate holds all its lines.
                width: Val::Percent(100.0),
                max_width: kit.m.px(COLOPHON_MEASURE),
                padding: UiRect::axes(kit.m.px(16.0), kit.m.px(8.0)),
                row_gap: kit.m.px(4.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            glass_mist_gradient(),
            glass_mist_border(),
            surfaces::elevation_control(),
            Pickable {
                should_block_lower: false,
                is_hoverable: true,
            },
        ))
        .id();
    let notice = super::links::prose_with_inline_links(
        commands,
        kit,
        FAN_CONTENT_NOTICE,
        &[(
            "Fan Content Policy",
            FrontPress::OpenExternal(ExternalLink::FanContentPolicy),
        )],
        Justify::Center,
        true,
    );
    let credit = super::links::prose_with_inline_links(
        commands,
        kit,
        Phrase::ScryfallCredit.text(lang),
        &[("Scryfall", FrontPress::OpenExternal(ExternalLink::Scryfall))],
        Justify::Center,
        true,
    );
    commands.entity(words).add_children(&[notice, credit]);
    commands.entity(holder).add_child(words);
    holder
}

/// How wide the full colophon's paragraph runs at the default step: the
/// notice in two lines at 1920 px.
const COLOPHON_MEASURE: f32 = 760.0;

/// Community links below the front-door card open a QR-code dialog. The
/// adjacent leaderboard button opens public standings inside the client.
pub(super) fn community_row(
    commands: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    full_colophon: bool,
) -> Entity {
    use super::CommunityKind;
    let lang = state.lobby.lang();
    let table = keys::table_of(state).name;
    let row = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: kit.m.px(8.0),
                row_gap: kit.m.px(6.0),
                flex_wrap: FlexWrap::Wrap,
                max_width: Val::Percent(100.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let whatsapp = community_pill(
        commands,
        kit,
        Phrase::WhatsAppCommunity.text(lang),
        "\u{f086}",
        WHATSAPP_GREEN,
        WHATSAPP_PILL_BG,
        WHATSAPP_PILL_HOT,
        WHATSAPP_GREEN.with_alpha(0.60),
        (
            Press::Front(FrontPress::OpenCommunityDialog(CommunityKind::WhatsApp)),
            Stop::new(table, "whatsapp"),
        ),
    );
    let discord = community_pill(
        commands,
        kit,
        Phrase::DiscordCommunity.text(lang),
        "\u{f500}",
        DISCORD_ACCENT,
        DISCORD_PILL_BG,
        DISCORD_PILL_HOT,
        DISCORD_ACCENT.with_alpha(0.60),
        (
            Press::Front(FrontPress::OpenCommunityDialog(CommunityKind::Discord)),
            Stop::new(table, "discord"),
        ),
    );
    commands
        .entity(discord)
        .insert((Beckon, BoxShadow(vec![glow(0.0)])))
        .entry::<Node>()
        .and_modify(|mut node| {
            node.border_radius = BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL));
        });
    commands.entity(row).add_children(&[whatsapp, discord]);

    if state.source_code.is_some() {
        let github_stop = if full_colophon { "source" } else { "github" };
        let github = community_pill(
            commands,
            kit,
            "GitHub",
            "\u{f126}",
            GITHUB_ACCENT,
            GITHUB_PILL_BG,
            GITHUB_PILL_HOT,
            GITHUB_ACCENT.with_alpha(0.55),
            (
                Press::Front(FrontPress::OpenCommunityDialog(CommunityKind::GitHub)),
                Stop::new(table, github_stop),
            ),
        );
        commands.entity(row).add_child(github);
    }

    let website = community_pill(
        commands,
        kit,
        Phrase::CentralWebsite.text(lang),
        "\u{f0ac}",
        tokens::ACCENT,
        WEBSITE_PILL_BG,
        WEBSITE_PILL_HOT,
        tokens::ACCENT.with_alpha(0.60),
        (
            Press::Front(FrontPress::OpenCommunityDialog(CommunityKind::Website)),
            Stop::new(table, "website"),
        ),
    );
    commands.entity(row).add_child(website);
    row
}

const WHATSAPP_GREEN: Color = Color::srgb(0.32, 0.82, 0.49);
const WHATSAPP_PILL_BG: Color = Color::srgba(0.06, 0.16, 0.11, 0.62);
const WHATSAPP_PILL_HOT: Color = Color::srgba(0.11, 0.26, 0.19, 0.88);
const DISCORD_ACCENT: Color = Color::srgb(0.52, 0.62, 0.98);
const DISCORD_PILL_BG: Color = Color::srgba(0.08, 0.12, 0.22, 0.62);
const DISCORD_PILL_HOT: Color = Color::srgba(0.14, 0.19, 0.33, 0.88);
const GITHUB_ACCENT: Color = Color::srgb(0.82, 0.88, 0.96);
const GITHUB_PILL_BG: Color = Color::srgba(0.07, 0.11, 0.16, 0.62);
const GITHUB_PILL_HOT: Color = Color::srgba(0.13, 0.19, 0.26, 0.88);
const WEBSITE_PILL_BG: Color = Color::srgba(0.06, 0.15, 0.20, 0.62);
const WEBSITE_PILL_HOT: Color = Color::srgba(0.10, 0.24, 0.30, 0.88);

#[allow(clippy::too_many_arguments)] // distinct visual tokens per community pill
fn community_pill(
    commands: &mut Commands,
    kit: Kit,
    label_text: &str,
    icon_glyph: &str,
    accent: Color,
    bg: Color,
    hot: Color,
    border: Color,
    action: impl Bundle,
) -> Entity {
    let face = commands
        .spawn((
            Role::Pill,
            Node {
                min_height: px_fixed(kit.m.control * 0.82),
                padding: UiRect::axes(kit.m.px(12.0), px_fixed(0.0)),
                column_gap: kit.m.px(7.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(border),
            surfaces::elevation_control(),
            crate::ambience::Feel::tinting_to(bg, hot),
            crate::ambience::FeelBorder {
                rest: border,
                hot: accent,
                press: tokens::PRIMARY_EDGE,
            },
            crate::ambience::FeelShadow,
            crate::shellkit::sheen::Sheen,
            UiTransform::default(),
        ))
        .id();
    let dot = commands
        .spawn((
            Node {
                width: px_fixed(6.0),
                height: px_fixed(6.0),
                border_radius: BorderRadius::all(px_fixed(3.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(accent),
            Pickable::IGNORE,
        ))
        .id();
    let icon = commands
        .spawn((
            Text::new(icon_glyph),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.92),
            TextColor(accent),
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            Text::new(label_text),
            crate::hud::tf_bold(kit.fonts, kit.m.small),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    let qr_hint = commands
        .spawn((
            Text::new("\u{f029}"),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.78),
            TextColor(tokens::MUTED.with_alpha(0.75)),
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(face)
        .add_children(&[dot, icon, words, qr_hint]);
    controls::hit(commands, kit, face, action)
}

/// The window corner a tile stands in (kept for frame-rate counter query).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Corner {
    Left,
    #[allow(dead_code)]
    Right,
}

/// A corner tile marker (kept for frame-rate counter query).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct CornerTile {
    pub(crate) corner: Corner,
    #[allow(dead_code)]
    pub(crate) press: FrontPress,
}

/// The address as a dialog or badge shows it: without its `https://`.
pub(crate) fn shown_address(address: &str) -> &str {
    address.strip_prefix("https://").unwrap_or(address)
}

/// The Discord pill's now-and-then glow (owner, 09.10.2026: it draws the
/// eye, and must not nag).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct Beckon;

/// One round of the glow: a pulse, then a long rest.
pub(crate) const BECKON_PERIOD: f32 = 24.0;
/// When in a round the pulse starts: a few seconds after the door opens,
/// then once a round.
pub(crate) const BECKON_AT: f32 = 4.0;
/// How long a pulse lasts.
pub(crate) const BECKON_PULSE: f32 = 1.0;
/// The glow's strongest alpha.
const BECKON_ALPHA: f32 = 0.55;

/// How strongly the tile glows, 0 to 1, `since` seconds after the door
/// first stood: a sine's hump through the pulse, nothing between, and
/// nothing at all under `reduce_motion`.
pub(crate) fn beckon_strength(since: f32, still: bool) -> f32 {
    if still {
        return 0.0;
    }
    let into = since.rem_euclid(BECKON_PERIOD) - BECKON_AT;
    if (0.0..BECKON_PULSE).contains(&into) {
        (std::f32::consts::PI * into / BECKON_PULSE).sin()
    } else {
        0.0
    }
}

/// The glow round a tile at `strength`: the accent, spread past its edge.
fn glow(strength: f32) -> ShadowStyle {
    ShadowStyle {
        color: tokens::ACCENT.with_alpha(BECKON_ALPHA * strength),
        x_offset: px_fixed(0.0),
        y_offset: px_fixed(0.0),
        spread_radius: px_fixed(2.0 * strength),
        blur_radius: px_fixed(16.0),
    }
}

/// Runs the Discord pill's glow.
pub(crate) fn beckon(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut tiles: Query<&mut BoxShadow, With<Beckon>>,
    mut since: Local<f32>,
) {
    if tiles.is_empty() {
        return;
    }
    *since = (*since + time.delta_secs()).rem_euclid(BECKON_PERIOD);
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let want = glow(beckon_strength(*since, still));
    for mut shadow in &mut tiles {
        if shadow.0.first() != Some(&want) {
            shadow.0 = vec![want];
        }
    }
}

/// Client version badge in the bottom-left corner of the window:
/// a leading release-tag glyph, `0.1.0-beta.9c` in bold, followed by
/// `#<number>` smaller and subtler on the exact same horizontal line
/// (`AlignItems::Center`), without `"Build"` or `"this client"`. Clicking it
/// opens the Changelog in the browser.
pub(crate) fn version_line(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let phone = kit.m.frame == Frame::Phone;
    let lang = state.lobby.lang();
    let table = keys::table_of(state).name;
    let bg = Color::srgba(0.035, 0.068, 0.115, 0.50);
    let border = Color::srgba(0.68, 0.74, 0.82, 0.32);
    let hint = Phrase::FrontChangelogHint.text(lang);
    let row = commands
        .spawn((
            Role::Button,
            Node {
                position_type: if phone {
                    PositionType::Relative
                } else {
                    PositionType::Absolute
                },
                left: if phone { Val::Auto } else { px_fixed(16.0) },
                bottom: if phone {
                    Val::Auto
                } else {
                    px_fixed(FOOTER_BOTTOM)
                },
                height: px_fixed(FOOTER_PILL_H),
                padding: UiRect::axes(kit.m.px(11.0), px_fixed(0.0)),
                column_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(border),
            surfaces::elevation_control(),
            crate::ambience::Feel {
                lift: 0.0,
                ..crate::ambience::Feel::tinting_to(bg, Color::srgba(0.09, 0.16, 0.24, 0.84))
            },
            crate::ambience::FeelBorder {
                rest: border,
                hot: tokens::ACCENT.with_alpha(0.70),
                press: tokens::ACCENT,
            },
            crate::ambience::FeelShadow,
            crate::shellkit::sheen::Sheen,
            bevy::picking::hover::PickingInteraction::default(),
            super::super::hint::HoverHint(hint.to_string()),
            Press::Front(FrontPress::OpenChangelog),
        ))
        .id();
    if matches!(state.lobby.screen(), Screen::SignIn { .. }) || state.showing_gateway_hub() {
        commands.entity(row).insert(Stop::new(table, "changelog"));
    }
    let tag_icon = commands
        .spawn((
            Text::new(VERSION_TAG_GLYPH.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.84),
            TextColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    let ver = commands
        .spawn((
            Text::new(baylee_build::VERSION),
            crate::hud::tf_bold(kit.fonts, kit.m.small),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_children(&[tag_icon, ver]);
    if !baylee_build::BUILD_NUMBER.is_empty() {
        let build = commands
            .spawn((
                Text::new(format!("#{}", baylee_build::BUILD_NUMBER)),
                crate::hud::tf(kit.fonts, kit.m.small * 0.86),
                TextColor(tokens::MUTED),
                TextLayout::no_wrap(),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(row).add_child(build);
    }
    row
}

/// Marks the bottom-right `Über` / `About` pill so the contacts drawer can
/// shift it smoothly to the left when open.
#[derive(Component)]
pub(crate) struct InfoPill;

/// Info (`Über Baylee` / `About`) pill button in the bottom-right corner of the
/// window, aligned on the exact same horizontal bottom line (`FOOTER_BOTTOM`)
/// as [`version_line`] and [`full`].
pub(crate) fn info_line(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let phone = kit.m.frame == Frame::Phone;
    let lang = state.lobby.lang();
    let table = keys::table_of(state).name;
    let active = state.about_open;
    let bg = if active {
        Color::srgba(0.08, 0.21, 0.24, 0.72)
    } else {
        Color::srgba(0.035, 0.068, 0.115, 0.50)
    };
    let border = if active {
        tokens::ACCENT.with_alpha(0.65)
    } else {
        Color::srgba(0.68, 0.74, 0.82, 0.32)
    };
    let row = commands
        .spawn((
            InfoPill,
            Role::Button,
            Node {
                position_type: if phone {
                    PositionType::Relative
                } else {
                    PositionType::Absolute
                },
                right: if phone { Val::Auto } else { px_fixed(16.0) },
                bottom: if phone {
                    Val::Auto
                } else {
                    px_fixed(FOOTER_BOTTOM)
                },
                height: px_fixed(FOOTER_PILL_H),
                padding: UiRect::axes(kit.m.px(11.0), px_fixed(0.0)),
                column_gap: kit.m.px(6.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(bg),
            BorderColor::all(border),
            surfaces::elevation_control(),
            crate::ambience::Feel {
                lift: 0.0,
                ..crate::ambience::Feel::tinting_to(
                    bg,
                    if active {
                        Color::srgba(0.11, 0.27, 0.30, 0.90)
                    } else {
                        Color::srgba(0.09, 0.16, 0.24, 0.84)
                    },
                )
            },
            crate::ambience::FeelBorder {
                rest: border,
                hot: tokens::ACCENT.with_alpha(0.70),
                press: tokens::ACCENT,
            },
            crate::ambience::FeelShadow,
            crate::shellkit::sheen::Sheen,
            bevy::picking::hover::PickingInteraction::default(),
            super::super::hint::HoverHint(Phrase::FrontAbout.text(lang).to_string()),
            Press::Front(FrontPress::About(true)),
        ))
        .id();
    if matches!(state.lobby.screen(), Screen::SignIn { .. }) || state.showing_gateway_hub() {
        commands.entity(row).insert(Stop::new(table, "about"));
    }
    let icon = commands
        .spawn((
            Text::new("\u{f05a}"),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.88),
            TextColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    let label = commands
        .spawn((
            Text::new(Phrase::FrontAbout.text(lang)),
            crate::hud::tf_bold(kit.fonts, kit.m.small),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_children(&[icon, label]);
    row
}

/// Compact QR-Code modal dialog opened when clicking any of the 4 community/web
/// pills (`WhatsApp`, `Discord`, `GitHub`, `baylee.app`).
#[allow(clippy::too_many_lines)]
pub(super) fn community_dialog(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
) {
    use super::CommunityKind;
    let Some(kind) = state.community_dialog else {
        return;
    };
    let lang = state.lobby.lang();
    let central_site = super::super::source::central_website_url();
    let (title_str, detail_str, icon_glyph, accent, address, code, open_press, open_label) =
        match kind {
            CommunityKind::WhatsApp => (
                Phrase::TileWhatsApp.text(lang),
                Phrase::CommunityWhatsAppAbout.text(lang),
                '\u{f086}',
                WHATSAPP_GREEN,
                super::super::source::WHATSAPP_URL,
                state.whatsapp_code.as_ref(),
                FrontPress::OpenWhatsApp,
                Phrase::CommunityWhatsAppOpen.text(lang),
            ),
            CommunityKind::Discord => (
                Phrase::TileDiscord.text(lang),
                Phrase::CommunityDiscordAbout.text(lang),
                '\u{f500}',
                DISCORD_ACCENT,
                super::super::source::DISCORD_URL,
                state.discord_code.as_ref(),
                FrontPress::OpenDiscord,
                Phrase::CommunityDiscordOpen.text(lang),
            ),
            CommunityKind::GitHub => (
                Phrase::TileSource.text(lang),
                Phrase::CommunitySourceAbout.text(lang),
                '\u{f126}',
                GITHUB_ACCENT,
                source_address(state),
                state.source_code.as_ref(),
                FrontPress::OpenSource,
                Phrase::CommunitySourceOpen.text(lang),
            ),
            CommunityKind::Website => (
                Phrase::CentralWebsite.text(lang),
                Phrase::CommunityWebsiteAbout.text(lang),
                '\u{f0ac}',
                tokens::ACCENT,
                central_site.as_str(),
                state.website_code.as_ref(),
                FrontPress::OpenWebsite,
                Phrase::CommunityWebsiteOpen.text(lang),
            ),
        };

    let dialog = surfaces::dialog(
        commands,
        kit,
        kit.m.px(396.0),
        accent.with_alpha(0.55),
        Press::Shared(SharedPress::PickerNothing),
    );
    commands
        .entity(dialog)
        .insert(crate::shellkit::glass::ground(
            crate::shellkit::glass::Density::Clear,
        ))
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.align_items = AlignItems::Center;
            node.padding = UiRect::all(kit.m.px(20.0));
            node.row_gap = kit.m.px(14.0);
        });
    let scrim = surfaces::scrim(
        commands,
        kit,
        tokens::z::SHEET,
        Press::Front(FrontPress::CloseCommunityDialog),
        dialog,
    );

    let head = glass_head(
        commands,
        kit,
        icon_glyph,
        accent,
        title_str,
        Press::Front(FrontPress::CloseCommunityDialog),
    );

    let detail = commands
        .spawn((
            Text::new(detail_str),
            crate::hud::tf(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            TextLayout::justify(Justify::Center),
            Node {
                width: Val::Percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    commands.entity(dialog).add_children(&[head, detail]);

    if let Some(code) = code {
        let qr_pic = commands
            .spawn((
                ImageNode::new(code.image.clone()),
                Node {
                    width: px_fixed(148.0),
                    height: px_fixed(148.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let qr_box = commands
            .spawn((
                Node {
                    padding: UiRect::all(px_fixed(10.0)),
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(12.0)),
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.04, 0.07, 0.98)),
                BorderColor::all(accent.with_alpha(0.42)),
                Pickable::IGNORE,
            ))
            .add_child(qr_pic)
            .id();
        commands.entity(dialog).add_child(qr_box);
    }

    let url_pill = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(kit.m.px(12.0), kit.m.px(6.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.04, 0.07, 0.72)),
            BorderColor::all(tokens::BORDER.with_alpha(0.75)),
            Pickable::IGNORE,
        ))
        .id();
    let url_text = commands
        .spawn((
            Text::new(shown_address(address)),
            crate::hud::tf(kit.fonts, kit.m.small * 0.90),
            TextColor(accent),
            TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(url_pill).add_child(url_text);

    let open_btn = controls::button_with_icon(
        commands,
        kit,
        '\u{f0ac}',
        open_label,
        Weight::Primary,
        Live::Yes,
        None,
        Press::Front(open_press),
    );
    commands
        .entity(open_btn)
        .entry::<Node>()
        .and_modify(|mut node| node.width = Val::Percent(100.0));

    commands.entity(dialog).add_children(&[url_pill, open_btn]);
    commands.entity(root).add_child(scrim);
}

/// A QR code in its thin dark frame (used in the About sheet).
pub(super) fn qr(commands: &mut Commands, code: &super::super::source::Code) -> Entity {
    let side = px_fixed(code.side as f32 * super::super::source::MODULE_PX);
    let picture = commands
        .spawn((
            ImageNode::new(code.image.clone()),
            Node {
                width: side,
                height: side,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    qr_frame(commands, picture)
}

/// A code drawn `side` wide whatever its module count, in its frame: the
/// donation sheet's three stand at one size, side by side.
pub(super) fn qr_sized(
    commands: &mut Commands,
    code: &super::super::source::Code,
    side: Val,
) -> Entity {
    let picture = commands
        .spawn((
            ImageNode::new(code.image.clone()),
            Node {
                width: side,
                height: side,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    qr_frame(commands, picture)
}

/// The thin dark frame a code stands in.
fn qr_frame(commands: &mut Commands, picture: Entity) -> Entity {
    commands
        .spawn((
            Node {
                padding: UiRect::all(px_fixed(5.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(8.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.04, 0.07, 0.96)),
            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.14)),
        ))
        .add_child(picture)
        .id()
}

/// The tagline on its airy cut-glass mist plate.
pub(super) fn tagline(commands: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let plate = commands
        .spawn((
            Role::Mist,
            Node {
                padding: UiRect::axes(kit.m.px(12.0), kit.m.px(4.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            glass_mist_gradient(),
            glass_mist_border(),
            surfaces::elevation_control(),
            Pickable::IGNORE,
        ))
        .id();
    let words = controls::label(
        commands,
        kit,
        Phrase::WelcomeNote.text(state.lobby.lang()),
        kit.m.small,
        tokens::INK,
    );
    commands.entity(plate).add_child(words);
    plate
}

/// The About sheet (`Über Baylee`): delegates to [`super::about::about`].
pub(crate) fn about(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    scrolled: &Scrolled,
) {
    super::about::about(commands, root, state, kit, scrolled);
}

/// A glass dialog's head (the community and donation sheets): the glyph on
/// its tinted tile, the title, and the close cross at the far end.
pub(super) fn glass_head(
    commands: &mut Commands,
    kit: Kit,
    glyph: char,
    accent: Color,
    title: &str,
    close: Press,
) -> Entity {
    let head = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: kit.m.px(10.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let head_left = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(10.0),
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let badge = commands
        .spawn((
            Node {
                width: px_fixed(32.0),
                height: px_fixed(32.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(8.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(accent.with_alpha(0.14)),
            BorderColor::all(accent.with_alpha(0.50)),
            Pickable::IGNORE,
        ))
        .id();
    let badge_icon = commands
        .spawn((
            Text::new(glyph.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.text),
            TextColor(accent),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(badge).add_child(badge_icon);
    let title = commands
        .spawn((
            Text::new(title),
            crate::hud::tf_bold(kit.fonts, kit.m.text * 1.08),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(head_left).add_children(&[badge, title]);
    let close_face = commands
        .spawn((
            Role::Button,
            Node {
                width: px_fixed(30.0),
                height: px_fixed(30.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.04)),
            BorderColor::all(tokens::BORDER),
            crate::ambience::Feel::tinting_to(Color::srgba(1.0, 1.0, 1.0, 0.04), tokens::HOVER),
        ))
        .id();
    let close_icon = commands
        .spawn((
            Text::new("\u{f00d}"),
            crate::hud::icon_tf(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(close_face).add_child(close_icon);
    let close_btn = controls::hit(commands, kit, close_face, close);
    commands.entity(head).add_children(&[head_left, close_btn]);
    head
}
