//! The shell's header and strips (the shell design, §2.1, §2.7), as kit
//! components: what a screen hands in decides what they say, the frame
//! decides how much of it fits.
//!
//! The brand is the logo (Baylee on the lockup's moon), never a wordmark in
//! text: [`BrandMark`] stands at the header's height and wears the lockup's
//! cut nearest its drawn height in physical pixels ([`sharpen_brand`]), so it
//! is crisp at every scale; its accessible name is the brand's word.
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
    /// "Baylee": the logo's accessible name (the logo is drawn, not
    /// written).
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
    let brand = brand_mark(commands, m.header, look.brand);
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

/// The lockup's width over its height (the trimmed master, 1928 × 809).
pub const LOGO_ASPECT: f32 = 1928.0 / 809.0;

/// How much of the header's height the logo stands in.
pub const LOGO_SHARE: f32 = 0.80;

/// The header's brand: the logo, as an image. Its picture is set by
/// [`sharpen_brand`] after layout (no texture until then: a frame of the
/// default white square would flash on every rebuild otherwise).
#[derive(Component, Clone, Copy, Debug)]
pub struct BrandMark;

/// The lockup's cuts, by height in pixels, smallest first.
pub const LOGO_CUTS: [(f32, &str); 3] = [
    (48.0, "brand/baylee-logo-48.png"),
    (96.0, "brand/baylee-logo-96.png"),
    (192.0, "brand/baylee-logo-192.png"),
];

/// The cuts, loaded once.
#[derive(Resource, Clone)]
pub struct BrandArt(pub [Handle<Image>; 3]);

/// Which cut draws a logo `physical` pixels high: the smallest at least as
/// tall (a downsample by under two, never an upsample), else the largest.
#[must_use]
pub fn cut_for(physical: f32) -> usize {
    LOGO_CUTS
        .iter()
        .position(|(h, _)| *h >= physical - 0.5)
        .unwrap_or(LOGO_CUTS.len() - 1)
}

/// The brand mark for a header `header` pixels tall, named `name` for
/// assistive technology. The builder's own header wears it too.
pub fn brand_mark(commands: &mut Commands, header: f32, name: &str) -> Entity {
    let height = (header * LOGO_SHARE).round();
    let mut node = accesskit::Node::new(accesskit::Role::Image);
    node.set_label(name);
    commands
        .spawn((
            BrandMark,
            Name::new(name.to_string()),
            bevy::a11y::AccessibilityNode(node),
            ImageNode {
                color: Color::NONE,
                ..default()
            },
            Node {
                width: px_fixed((height * LOGO_ASPECT).round()),
                height: px_fixed(height),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// Gives each brand mark the cut its drawn height asks for, after layout and
/// before the frame is drawn; writes only when the cut changes.
pub fn sharpen_brand(
    art: Option<Res<BrandArt>>,
    mut marks: Query<(&ComputedNode, &mut ImageNode), With<BrandMark>>,
) {
    let Some(art) = art else {
        return;
    };
    for (computed, mut image) in &mut marks {
        let handle = &art.0[cut_for(computed.size().y)];
        if image.image != *handle || image.color != Color::WHITE {
            image.image = handle.clone();
            image.color = Color::WHITE;
        }
    }
}

/// Loads the cuts and keeps the marks sharp.
pub(super) fn install(app: &mut App) {
    if let Some(assets) = app.world().get_resource::<AssetServer>() {
        let art = BrandArt(LOGO_CUTS.map(|(_, path)| assets.load(path)));
        app.insert_resource(art);
    }
    app.add_systems(PostUpdate, sharpen_brand.after(bevy::ui::UiSystems::Layout));
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
