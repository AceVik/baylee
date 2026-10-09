//! The hand as a drawer on a phone (DESIGN-v8 WA11; the rules in
//! `client-core::handdrawer`): the hand zone and the actions bar slide down
//! together until only the bar stands at the bottom edge, and the table
//! takes the height (`Canvas::with_drawer`). A tab on the bar's top edge,
//! at its right end,
//! says how many cards are in the hand and how many of them are castable;
//! a tap on it, a swipe up or down on it, or `I` opens and shuts the drawer.
//!
//! The slide goes through the client's own animation path: a fraction
//! eased over `SLIDE_SECS` (0.28 s) and written to the nodes' `UiTransform`
//! only while it moves; reduced motion is the cut. Nothing in the hand
//! moves under the finger: a press on a card that is sliding is the card's.
//! Off a phone the drawer is always open and nothing is written.

use baylee_client_core::handdrawer::{eased, slide};
use baylee_client_core::tableview::TableFrame;
use bevy::prelude::*;

use crate::hud::{HandScroll, UiFonts, btn_radius, icon_tf, palette, tf_bold};
use crate::{Duel, DuelPhase};

/// The tab on the actions bar's top edge.
#[derive(Component)]
pub struct HandTab;

/// How wide and tall the tab is, logical pixels: a 44-px target under the
/// finger is its hit area (`Touch`), the drawn tab a little smaller.
pub(crate) const TAB_W: f32 = 96.0;
const TAB_H: f32 = 22.0;

/// What the tab was last drawn from.
#[derive(Resource, Default, Clone, PartialEq, Debug)]
pub struct HandTabRevision {
    shown: bool,
    cards: usize,
    castable: usize,
    open: bool,
}

/// Whether the window is a phone's: the one frame with a drawer.
fn on_a_phone(windows: &Query<&Window>) -> bool {
    windows
        .single()
        .is_ok_and(|w| TableFrame::of(w.width(), w.height()) == TableFrame::Phone)
}

/// How far down the zone and the bar stand when the drawer is drawn
/// `shown` open: nothing open, the cards' whole height shut.
#[must_use]
pub fn drop_at(shown: f32) -> f32 {
    (crate::hud::HAND_ZONE_H - crate::hud::LEDGE_H) * (1.0 - eased(shown))
}

/// A node that might ride the drawer, as the slide reads it.
type Rider<'a> = (
    &'a Node,
    &'a mut UiTransform,
    Option<&'a ChildOf>,
    Has<HandScroll>,
    Has<HandTab>,
);

/// Whether a node rides the drawer: the hand zone, the tab, and every panel
/// of the HUD's that stands on the hand zone's top — the actions bar, the
/// players' and the pool's strips, the tray's row, the burger's menu, the
/// drawer of answers, the log's panel: a node of the HUD's own whose bottom
/// is pinned from the bar's bottom up to a strip's height over the zone,
/// and whose top is free. Read off the node, so a panel added on the bar
/// later rides it too; a dialog pinned top and bottom does not.
#[must_use]
pub fn rides(node: &Node, on_the_hud: bool, scroll: bool, tab: bool) -> bool {
    if scroll || tab {
        return true;
    }
    let Val::Px(bottom) = node.bottom else {
        return false;
    };
    let low = crate::hud::HAND_ZONE_H - crate::hud::LEDGE_H - 1.0;
    let high = crate::hud::HAND_ZONE_H + RIDE_REACH;
    on_the_hud
        && node.position_type == PositionType::Absolute
        && node.top == Val::Auto
        && (low..=high).contains(&bottom)
}

/// How far over the hand zone's top a panel may be pinned and still ride
/// it: a strip's height and its lip.
const RIDE_REACH: f32 = 48.0;

/// Slides the drawer toward where it stands and writes the zone's, the
/// bar's and the tab's place while it moves. Writes nothing at rest.
#[allow(clippy::needless_pass_by_value)] // a Bevy system
pub fn slide_the_hand(
    mut duel: ResMut<Duel>,
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    windows: Query<&Window>,
    mut nodes: Query<Rider>,
    huds: Query<(), With<crate::hud::HudRoot>>,
    phase: Option<Res<State<DuelPhase>>>,
) {
    let phone = on_a_phone(&windows);
    // Open while the table is still being prepared under its cover: the
    // entrance shows the whole screen as it will first be played.
    let playing =
        phase.is_some_and(|p| matches!(p.get(), DuelPhase::Playing | DuelPhase::Finished));
    let open = !phone || !playing || duel.hand_drawer_open();
    if duel.hand_drawn_open != open {
        duel.hand_drawn_open = open;
    }
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let shown = slide(duel.hand_shown, open, time.delta_secs(), still);
    if (shown - duel.hand_shown).abs() > f32::EPSILON {
        duel.hand_shown = shown;
    }
    let drop = if phone { drop_at(shown) } else { 0.0 };
    for (node, mut transform, parent, scroll, tab) in &mut nodes {
        let on_the_hud = parent.is_some_and(|p| huds.contains(p.parent()));
        if !rides(node, on_the_hud, scroll, tab) {
            continue;
        }
        transform.set_if_neq(UiTransform {
            translation: Val2::px(0.0, drop),
            ..default()
        });
    }
}

/// Stands the tab on a phone and takes it down elsewhere; rebuilt only when
/// what it says changes.
#[allow(clippy::needless_pass_by_value)] // a Bevy system
pub fn sync_hand_tab(
    mut commands: Commands,
    phase: Option<Res<State<DuelPhase>>>,
    fonts: Option<Res<UiFonts>>,
    duel: Res<Duel>,
    windows: Query<&Window>,
    mut revision: ResMut<HandTabRevision>,
    standing: Query<Entity, With<HandTab>>,
) {
    let up = phase.is_some_and(|p| !matches!(p.get(), DuelPhase::Closed));
    let next = if up && on_a_phone(&windows) {
        let cards = duel.view.as_ref().map_or(0, |v| v.hand.len());
        HandTabRevision {
            shown: true,
            cards,
            castable: duel.castable_in_hand(),
            open: duel.hand_drawer_open(),
        }
    } else {
        HandTabRevision::default()
    };
    if *revision == next && (standing.iter().next().is_some() || !next.shown) {
        return;
    }
    revision.clone_from(&next);
    for e in &standing {
        commands.entity(e).despawn();
    }
    let Some(fonts) = fonts.filter(|_| next.shown) else {
        return;
    };
    let ground = palette::DIALOG.with_alpha(0.92);
    let tab = commands
        .spawn((
            HandTab,
            Node {
                position_type: PositionType::Absolute,
                // At the bar's right end: the players' strip stands along
                // its left and middle on a phone.
                bottom: px(crate::hud::HAND_ZONE_H),
                right: px(crate::hud::EDGE),
                width: px(TAB_W),
                height: px(TAB_H),
                column_gap: px(5.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::new(px(1), px(1), px(1), px(0)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(palette::DIALOG_LINE),
            Button,
            crate::ambience::Feel::new(ground),
            GlobalZIndex(650),
        ))
        .observe(tab_down)
        .observe(tab_pressed)
        .id();
    let mut put = |text: String, font: TextFont, colour: Color| {
        let id = commands
            .spawn((Text::new(text), font, TextColor(colour), Pickable::IGNORE))
            .id();
        commands.entity(tab).add_child(id);
    };
    put(
        crate::hud::glyph::HAND.to_string(),
        icon_tf(&fonts, 10.0),
        palette::DIALOG_SOFT,
    );
    put(
        next.cards.to_string(),
        tf_bold(&fonts, 12.0),
        palette::DIALOG_INK,
    );
    if next.castable > 0 {
        // The castable ones in the playable gold the hand's halo is.
        put(
            format!("\u{b7} {}", next.castable),
            tf_bold(&fonts, 12.0),
            palette::ACTIVE,
        );
    }
    put(
        if next.open { "\u{25be}" } else { "\u{25b4}" }.to_string(),
        tf_bold(&fonts, 12.0),
        palette::DIALOG_SOFT,
    );
}

/// Where the finger went down on the tab.
#[derive(Resource, Default)]
pub struct TabPress(Option<Vec2>);

fn tab_down(press: On<Pointer<Press>>, mut at: ResMut<TabPress>) {
    at.0 = Some(press.pointer_location.position);
}

/// The tab let go of: a swipe up opens and down shuts — a travel of a third
/// of the tab's height or more —, a tap toggles. One handler, so a swipe is
/// never also a tap.
fn tab_pressed(mut click: On<Pointer<Click>>, mut at: ResMut<TabPress>, mut duel: ResMut<Duel>) {
    click.propagate(false);
    let dy =
        at.0.take()
            .map_or(0.0, |down| click.pointer_location.position.y - down.y);
    if dy.abs() >= swipe_travel() {
        duel.hand_drawer.swipe(dy < 0.0);
    } else {
        duel.toggle_hand_drawer();
    }
}

/// How far a finger travels on the tab for a swipe, logical pixels.
#[must_use]
pub const fn swipe_travel() -> f32 {
    TAB_H / 3.0
}
