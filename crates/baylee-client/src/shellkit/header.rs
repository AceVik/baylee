//! The shell's header and strips (the shell design, §2.1, §2.7), as kit
//! components: what a screen hands in decides what they say, the frame
//! decides how much of it fits.
//!
//! The brand is the logo (Baylee on the lockup's moon), never a wordmark in
//! text: [`BrandMark`] stands at the header's height and wears the lockup's
//! cut nearest its drawn height in physical pixels ([`sharpen_brand`]), so it
//! is crisp at every scale; its accessible name is the brand's word.
//!
//! - **Wide / Vast:** brand, the build's short form, the nav pills,
//!   the gateway pill with its name and counts, the bell, the account pill.
//!   Native desktop uses tighter gaps and stacks the gateway's name over
//!   its counts; that pill absorbs width pressure before any action does.
//! - **Compact / Narrow:** the same, the gateway pill a dot, the account
//!   pill its handle's first letter, no build (it stays in the popover).
//! - **Phone** (44 tall): nav words stay; the gateway pill is a dot, or —
//!   while the player holds a chair — the gold **Return** pill in its place,
//!   and no strip below (S4-3).
//!
//! The strips stand below the header: the seated strip (sentence, Return,
//! Leave at a waiting table) and the reconnecting strip (sentence, Retry
//! now). Neither is drawn on a Phone (the Return pill, and the screen's own
//! error line, stand for them) — the caller decides, this draws.

use bevy::prelude::*;

use super::controls::{self, Kit, Live, Weight, hit, label};
use super::metrics::px_fixed;
use super::role::Role;
use super::{Frame, tokens};
use crate::hud::{icon_tf, tf_bold};

mod brand;
mod status;
mod surface;
pub use brand::*;
pub use status::{StatusActions, status};
pub(super) use surface::install;
pub use surface::{HeaderSurface, HeaderSurfaceDressed};

/// Whether the gateway answers, as the pill's dot says it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reach {
    /// The lobby feed is live.
    Up,
    /// It is being dialled again.
    Down,
    /// No gateway: offline play.
    Offline,
}

impl Reach {
    const fn colour(self) -> Color {
        match self {
            Self::Up => tokens::ACCENT,
            Self::Down => tokens::DANGER,
            Self::Offline => tokens::MUTED,
        }
    }
}

/// What the header shows.
pub struct HeaderLook<'a> {
    /// "Baylee": the logo's accessible name (the logo is drawn, not
    /// written).
    pub brand: &'a str,
    /// The build's short form (`0.1.0-beta.5`), beside the brand on Wide and
    /// Vast only.
    pub build: &'a str,
    /// Play · Decks · (Replays, on a desktop) · Settings, and which is the
    /// screen shown.
    pub nav: &'a [&'a str],
    /// The nav pill that is active, if any.
    pub active: Option<usize>,
    /// The gateway's dot, its name, and its counts (Wide and Vast).
    pub reach: Reach,
    /// The gateway's name (or address).
    pub gateway: &'a str,
    /// "3 tables · 12 online".
    pub counts: &'a str,
    /// Unread bell items.
    pub unread: usize,
    /// The account pill's handle; none offline.
    pub handle: Option<&'a str>,
    /// The Phone header's gold Return pill, in place of the gateway's dot.
    pub return_pill: Option<&'a str>,
    /// Controls the screen adds before the bell (quick settings and social
    /// tools), already built.
    pub tools: &'a [Entity],
}

/// The actions the header's controls carry, built by the caller.
pub struct HeaderActions<N, G, B, A, R> {
    /// Nav pill `i`'s action.
    pub nav: N,
    /// The gateway pill.
    pub gateway: G,
    /// The bell.
    pub bell: B,
    /// The account pill.
    pub account: A,
    /// The Phone header's Return pill.
    pub back: R,
}

/// Identifies a header trigger button so its dropdown popover can center
/// beneath it with a speech-bubble pointer (`Sprechblasen-Nippel`).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeaderTrigger {
    /// The quick-settings gear button.
    Gear,
    /// The notifications bell button.
    Bell,
    /// The user profile / account button.
    Account,
    /// The gateway status pill.
    Gateway,
}

/// The shared navigation frame: spacing, wrapping, surface and elevation.
/// Screen-specific controls are attached by the caller.
pub fn frame(commands: &mut Commands, kit: Kit) -> Entity {
    let m = kit.m;
    let roomy = matches!(m.frame, Frame::Wide | Frame::Vast);
    let desktop = super::Platform::current() == super::Platform::Desktop;
    commands
        .spawn((
            Role::Header,
            HeaderSurface(false),
            Node {
                width: Val::Percent(100.0),
                // Large text at a narrow desktop keeps all actions reachable
                // on a second row rather than placing them beyond the window.
                flex_wrap: if !roomy && desktop {
                    FlexWrap::Wrap
                } else {
                    FlexWrap::NoWrap
                },
                min_height: px_fixed(m.header),
                padding: UiRect::axes(px_fixed(m.body), px_fixed(0.0)),
                column_gap: if roomy && desktop {
                    m.px(8.0)
                } else {
                    px_fixed(m.gap)
                },
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                border: UiRect::bottom(px_fixed(1.0)),
                ..default()
            },
            BackgroundColor(tokens::PANEL.with_alpha(surface::OPACITY)),
            BorderColor::all(tokens::BORDER.with_alpha(0.65)),
            GlobalZIndex(tokens::z::POPOVER - 1),
            BoxShadow(vec![ShadowStyle {
                color: Color::srgba(0.0, 0.0, 0.0, 0.36),
                x_offset: px_fixed(0.0),
                y_offset: px_fixed(4.0),
                spread_radius: px_fixed(0.0),
                blur_radius: px_fixed(12.0),
            }]),
        ))
        .id()
}

/// The header bar.
#[allow(clippy::too_many_lines)] // one bar, read left to right
pub fn header<NB, N, G, B, A, R>(
    commands: &mut Commands,
    kit: Kit,
    look: &HeaderLook,
    actions: HeaderActions<N, G, B, A, R>,
) -> Entity
where
    NB: Bundle,
    N: Fn(usize) -> NB,
    G: Bundle,
    B: Bundle,
    A: Bundle,
    R: Bundle,
{
    let _ = look.build;
    let m = kit.m;
    let bar = frame(commands, kit);
    let brand = brand_mark(commands, m.header, look.brand);
    commands.entity(bar).add_child(brand);
    for (i, text) in look.nav.iter().enumerate() {
        let pill = controls::nav(
            commands,
            kit,
            text,
            look.active == Some(i),
            (actions.nav)(i),
        );
        commands.entity(pill).insert(crate::tour::TourAnchor(
            baylee_client_core::tour::Anchor::ShellNav,
        ));
        commands.entity(bar).add_child(pill);
    }
    let gap = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(bar).add_child(gap);
    status(
        commands,
        kit,
        bar,
        look,
        StatusActions {
            gateway: actions.gateway,
            bell: actions.bell,
            account: actions.account,
            back: actions.back,
        },
        false,
    );
    bar
}

/// The gateway pill on Wide and Vast: the reach dot, the name (or address)
/// as its own label, the counts in muted ink, the caret. On native desktop
/// the two labels share a bounded column: a long address clips inside its
/// pill, leaving the account and navigation their full hit areas. The
/// gateway popover still carries the complete address and counts.
#[allow(clippy::too_many_lines)] // single-line gateway pill with dot, name, inline stats, and caret
fn gateway_pill(
    commands: &mut Commands,
    kit: Kit,
    look: &HeaderLook,
    action: impl Bundle,
) -> Entity {
    let desktop = super::Platform::current() == super::Platform::Desktop;
    let face = commands
        .spawn((
            Role::Pill,
            Node {
                min_height: kit.m.px(34.0),
                padding: UiRect::axes(kit.m.px(12.0), px_fixed(0.0)),
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(tokens::BORDER),
            UiTransform::default(),
            crate::ambience::Feel::new(tokens::CONTROL),
        ))
        .id();
    let dot = commands
        .spawn((
            Node {
                width: kit.m.px(8.0),
                height: kit.m.px(8.0),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(look.reach.colour()),
            Pickable::IGNORE,
        ))
        .id();
    let name = label(commands, kit, look.gateway, kit.m.small, tokens::INK);
    let counts = label(commands, kit, look.counts, kit.m.small, tokens::MUTED);
    let caret = commands
        .spawn((
            Text::new("\u{f078}"),
            icon_tf(kit.fonts, kit.m.small * 0.7),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    if desktop {
        commands
            .entity(name)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.flex_shrink = 1.0;
                node.min_width = px_fixed(0.0);
                node.overflow = Overflow::clip_x();
            });
        commands
            .entity(counts)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.flex_shrink = 0.0;
            });
        let words = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: kit.m.px(8.0),
                    min_width: px_fixed(0.0),
                    overflow: Overflow::clip_x(),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(words).add_children(&[name, counts]);
        // `hit` normally protects its child's natural width. This pill
        // alone may shrink; its face must follow that allotted width.
        commands
            .entity(face)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.max_width = Val::Percent(100.0);
                node.min_width = px_fixed(0.0);
            });
        commands.entity(dot).entry::<Node>().and_modify(|mut node| {
            node.flex_shrink = 0.0;
        });
        commands
            .entity(caret)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.flex_shrink = 0.0;
            });
        commands.entity(face).add_children(&[dot, words, caret]);
    } else {
        commands
            .entity(face)
            .add_children(&[dot, name, counts, caret]);
    }
    let wrapper = hit(commands, kit, face, action);
    if desktop {
        commands
            .entity(wrapper)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.flex_shrink = 1.0;
                node.min_width = px_fixed(kit.m.scaled(64.0).max(kit.m.hit));
            });
    }
    wrapper
}

/// Formats an unread count for a header badge dot (`1`..`99`, then `99+`).
#[must_use]
pub fn format_badge_count(unread: usize) -> String {
    if unread == 0 {
        String::new()
    } else if unread > 99 {
        "99+".to_string()
    } else {
        unread.to_string()
    }
}

/// The bell: its glyph, and the unread count on it when there is one.
pub fn bell(commands: &mut Commands, kit: Kit, unread: usize, action: impl Bundle) -> Entity {
    let face = commands
        .spawn((
            Node {
                min_width: kit.m.px(32.0),
                min_height: kit.m.px(32.0),
                padding: UiRect::axes(kit.m.px(6.0), kit.m.px(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            UiTransform::default(),
            crate::ambience::Feel::rising_to(Color::NONE, tokens::HOVER),
            Pickable::IGNORE,
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new("\u{f0f3}"),
            icon_tf(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_child(glyph);
    if unread > 0 {
        let count = commands
            .spawn((
                Role::Count,
                // Sized by its number, never under 16: a scaled height
                // shorter than the digit's line let it spill (step 1).
                Node {
                    min_width: px_fixed(16.0),
                    min_height: px_fixed(16.0),
                    padding: UiRect::axes(px_fixed(4.0), px_fixed(0.0)),
                    margin: UiRect::left(kit.m.px(-6.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                    ..default()
                },
                BackgroundColor(tokens::GOLD),
                Pickable::IGNORE,
            ))
            .id();
        let n = commands
            .spawn((
                Text::new(format_badge_count(unread)),
                tf_bold(kit.fonts, kit.m.small * 0.85),
                TextColor(tokens::OPAQUE),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(count).add_child(n);
        commands.entity(face).add_child(count);
    }
    hit(commands, kit, face, action)
}

/// The account pill on a narrow header: the handle's first letter in a disc.
pub fn avatar(commands: &mut Commands, kit: Kit, handle: &str, action: impl Bundle) -> Entity {
    let letter: String = handle
        .chars()
        .next()
        .map(char::to_uppercase)
        .into_iter()
        .flatten()
        .collect();
    let disc = commands
        .spawn((
            Role::Pill,
            Node {
                width: kit.m.px(30.0),
                height: kit.m.px(30.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(tokens::BORDER),
            UiTransform::default(),
            crate::ambience::Feel::new(tokens::CONTROL),
        ))
        .id();
    let words = label(commands, kit, &letter, kit.m.small, tokens::INK);
    commands.entity(disc).add_child(words);
    hit(commands, kit, disc, action)
}

/// A contextual strip below the header: a distinct bar with a status badge,
/// sentence, and compact actions on the right, sharing a continuous animated
/// background with the top navigation bar ("wie aus einem Guss").
pub fn strip(commands: &mut Commands, kit: Kit, sentence: &str, actions: &[Entity]) -> Entity {
    let m = kit.m;
    let bar = commands
        .spawn((
            Role::Panel,
            HeaderSurface(true),
            Node {
                width: Val::Percent(100.0),
                min_height: kit.m.px(38.0),
                padding: UiRect::axes(px_fixed(m.body), kit.m.px(4.0)),
                column_gap: kit.m.px(10.0),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                border: UiRect::bottom(px_fixed(1.0)),
                ..default()
            },
            BackgroundColor(tokens::PANEL.with_alpha(surface::OPACITY)),
            BorderColor::all(tokens::PRIMARY_EDGE.with_alpha(0.45)),
            BoxShadow(vec![ShadowStyle {
                color: Color::srgba(0.0, 0.0, 0.0, 0.32),
                x_offset: px_fixed(0.0),
                y_offset: px_fixed(3.0),
                spread_radius: px_fixed(0.0),
                blur_radius: px_fixed(10.0),
            }]),
            SeatStrip,
        ))
        .id();
    let dot = commands
        .spawn((
            Node {
                width: kit.m.px(8.0),
                height: kit.m.px(8.0),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(tokens::GOLD),
            Pickable::IGNORE,
        ))
        .id();
    let words = commands
        .spawn((
            Text::new(sentence),
            tf_bold(kit.fonts, m.small),
            TextColor(tokens::INK),
            Node {
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(bar).add_children(&[dot, words]);
    commands.entity(bar).add_children(actions);
    bar
}

/// Marks a strip, so a test can count them.
#[derive(Component)]
pub struct SeatStrip;

#[cfg(all(
    test,
    not(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))
))]
pub(crate) mod tests;
