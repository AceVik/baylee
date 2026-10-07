//! The shell's header and strips (the shell design, §2.1, §2.7), as kit
//! components: what a screen hands in decides what they say, the frame
//! decides how much of it fits.
//!
//! - **Wide / Vast:** brand, the build's short form, the three nav pills,
//!   the gateway pill with its name and counts, the bell, the account pill.
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
use crate::hud::{icon_tf, tf, tf_bold};

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
    /// "Baylee".
    pub brand: &'a str,
    /// The build's short form (`0.1.0-beta.5`), beside the brand on Wide and
    /// Vast only.
    pub build: &'a str,
    /// Play · Decks · Settings, and which is the screen shown.
    pub nav: [&'a str; 3],
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
    /// Controls the screen adds before the bell (the lobby's quick
    /// settings gear), already built.
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
    let m = kit.m;
    let phone = m.frame == Frame::Phone;
    let roomy = matches!(m.frame, Frame::Wide | Frame::Vast);
    let bar = commands
        .spawn((
            Role::Header,
            Node {
                width: Val::Percent(100.0),
                min_height: px_fixed(m.header),
                padding: UiRect::axes(px_fixed(m.body), px_fixed(0.0)),
                column_gap: px_fixed(m.gap),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(tokens::PANEL),
        ))
        .id();
    let brand = commands
        .spawn((
            Text::new(look.brand),
            tf_bold(kit.fonts, m.head),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(bar).add_child(brand);
    if roomy {
        let build = label(commands, kit, look.build, m.small, tokens::MUTED);
        commands.entity(bar).add_child(build);
    }
    for (i, text) in look.nav.iter().enumerate() {
        let pill = controls::nav(
            commands,
            kit,
            text,
            look.active == Some(i),
            (actions.nav)(i),
        );
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
    if let Some(back) = look.return_pill.filter(|_| phone) {
        let pill = controls::button(
            commands,
            kit,
            back,
            Weight::Gold,
            Live::Yes,
            None,
            actions.back,
        );
        commands.entity(bar).add_child(pill);
    } else if roomy {
        let pill = gateway_pill(commands, kit, look, actions.gateway);
        commands.entity(bar).add_child(pill);
    } else {
        let dot = commands
            .spawn((
                Node {
                    width: kit.m.px(10.0),
                    height: kit.m.px(10.0),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                    ..default()
                },
                BackgroundColor(look.reach.colour()),
            ))
            .id();
        let pill = hit(commands, kit, dot, actions.gateway);
        commands.entity(bar).add_child(pill);
    }
    commands.entity(bar).add_children(look.tools);
    let bell = bell(commands, kit, look.unread, actions.bell);
    commands.entity(bar).add_child(bell);
    if let Some(handle) = look.handle {
        let account = if roomy {
            controls::pill(commands, kit, None, handle, true, actions.account)
        } else {
            avatar(commands, kit, handle, actions.account)
        };
        commands.entity(bar).add_child(account);
    }
    bar
}

/// The gateway pill on Wide and Vast: the reach dot, the name (or address)
/// as its own label, the counts after it in muted ink, the caret.
fn gateway_pill(
    commands: &mut Commands,
    kit: Kit,
    look: &HeaderLook,
    action: impl Bundle,
) -> Entity {
    let face = commands
        .spawn((
            Role::Pill,
            Node {
                min_height: kit.m.px(36.0),
                padding: UiRect::axes(kit.m.px(12.0), px_fixed(0.0)),
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            BorderColor::all(tokens::BORDER),
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
    commands
        .entity(face)
        .add_children(&[dot, name, counts, caret]);
    hit(commands, kit, face, action)
}

/// The bell: its glyph, and the unread count on it when there is one.
pub fn bell(commands: &mut Commands, kit: Kit, unread: usize, action: impl Bundle) -> Entity {
    let face = commands
        .spawn((
            Node {
                align_items: AlignItems::FlexStart,
                ..default()
            },
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
                Text::new(unread.min(99).to_string()),
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
        ))
        .id();
    let words = label(commands, kit, &letter, kit.m.small, tokens::INK);
    commands.entity(disc).add_child(words);
    hit(commands, kit, disc, action)
}

/// A strip below the header: a sentence, and its actions on the right.
pub fn strip(commands: &mut Commands, kit: Kit, sentence: &str, actions: &[Entity]) -> Entity {
    let m = kit.m;
    let bar = commands
        .spawn((
            Role::Panel,
            Node {
                width: Val::Percent(100.0),
                min_height: px_fixed(m.hit),
                padding: UiRect::axes(px_fixed(m.body), kit.m.px(4.0)),
                column_gap: px_fixed(m.gap),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                border: UiRect::bottom(px_fixed(1.0)),
                ..default()
            },
            BackgroundColor(tokens::PANEL),
            BorderColor::all(tokens::BORDER),
            SeatStrip,
        ))
        .id();
    let words = commands
        .spawn((
            Text::new(sentence),
            tf(kit.fonts, m.small),
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
    commands.entity(bar).add_child(words);
    commands.entity(bar).add_children(actions);
    bar
}

/// Marks a strip, so a test can count them.
#[derive(Component)]
pub struct SeatStrip;
