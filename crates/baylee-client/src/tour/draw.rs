//! The tour's nodes: the scrim round the hole, the hairline, the bubble and
//! the pill (TOURS.md §1.2). [`draw`] rebuilds them when what they say
//! changes; [`place`] moves them every frame after layout, writing only what
//! moved.
//!
//! Nothing flickers between steps (owner, 10.10.2026: *"the tours flicker
//! and feel buggy"*). Three causes, each closed here:
//!
//! - A step's rebuild despawned the scrim and spawned it again with its
//!   sides at zero size, so for a frame nothing was dimmed. The scrim is now
//!   kept across steps whose scrim is the same ([`ScrimKey`]), and a new one
//!   is spawned round the hole that stood.
//! - The old bubble went the frame the new one was spawned hidden, so the
//!   bubble blinked out. The old one now stays ([`Leaving`]) until the new
//!   one stands in its place, laid out where [`place`] put it; a bubble is
//!   never shown in a frame whose layout has not yet seen its position.
//! - [`place`] runs after layout, so what it wrote to a scrim's `Node` was
//!   drawn a frame later and the hole trailed a moving anchor. It now also
//!   writes the laid-out size and transform of the scrim's sides and the
//!   hairline (absolute boxes in a full-window layer, so the layout's own
//!   answer next frame is the same), and the hole moves in the anchor's
//!   frame. An anchor missing for a frame (its screen rebuilding) holds
//!   everything where it was instead of sending the bubble to the centre.

use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::tour::{Kind, Mode, Run};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

use super::{Setting, TourAnchor, TourDesk, TourPress};
use crate::hud::{UiFonts, tf};
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::{Frame, ShellMetrics, px_fixed, tokens};

/// What is drawn now, and where the hole stands (for `/state.tour`).
#[derive(Resource, Default, Debug)]
pub struct Spotlight {
    drawn: Option<Drawn>,
    /// The hole round the anchor, in logical pixels, once placed.
    pub hole: Option<Rect>,
    /// What the standing scrim was spawned for.
    scrim: Option<ScrimKey>,
    /// The bubble, in logical pixels, once placed: the card preview stands
    /// clear of it.
    pub bubble: Option<Rect>,
    /// Tour nodes rebuilt: a counter for tests and `/state`.
    pub rebuilds: u64,
}

/// What decides the scrim's shape: kept across steps that agree on it.
#[derive(Clone, Copy, PartialEq, Debug)]
struct ScrimKey {
    mode: Mode,
    text_step: bool,
    z: i32,
    alpha: u32,
}

#[derive(Clone, PartialEq, Debug)]
struct Drawn {
    tour: baylee_client_core::tour::Tour,
    chapter: usize,
    step: usize,
    mode: Mode,
    held: bool,
    lang: Lang,
    phone: bool,
    setting: (i32, bool, bool),
    width: u32,
}

/// Every node of the tour, despawned together.
#[derive(Component)]
pub(super) struct TourLayer;

/// One side of the scrim (0 top, 1 bottom, 2 left, 3 right) or the hole's
/// pointer stop (4, narrated only).
#[derive(Component)]
pub(super) struct Scrim(u8);

/// The hairline round the hole.
#[derive(Component)]
pub(super) struct Ring;

/// The bubble.
#[derive(Component)]
pub(super) struct Bubble;

/// The scrim's layer, kept across steps with one [`ScrimKey`].
#[derive(Component)]
pub(super) struct ScrimLayer;

/// A bubble of the step before, standing until the new one is in place.
#[derive(Component)]
pub(super) struct Leaving;

/// The anchor's padding inside the hole, and the hairline's offset.
const PAD: f32 = 8.0;
const RING: f32 = 2.0;
/// The report form's band (`report::form`), over every HUD rung.
const REPORT_FORM_Z: i32 = 1000;
/// How far inside the window the bubble keeps.
const INSET: f32 = 12.0;

fn metrics(
    width: f32,
    height: f32,
    settings: &crate::settings::ClientSettings,
    input: crate::shellkit::InputClass,
) -> ShellMetrics {
    ShellMetrics::of(
        crate::shellkit::Viewport {
            width,
            height,
            platform: crate::shellkit::Platform::current(),
            input,
        },
        settings.text_size,
    )
}

/// Rebuilds the tour's nodes when its step, mode or words change.
#[allow(clippy::too_many_arguments)] // a Bevy system
pub(super) fn draw(
    mut commands: Commands,
    desk: Res<TourDesk>,
    settings: Res<crate::settings::ClientSettings>,
    fonts: Option<Res<UiFonts>>,
    windows: Query<&Window>,
    input: Option<Res<crate::shellkit::InputClass>>,
    layers: Query<(Entity, Has<ScrimLayer>, Has<Bubble>), With<TourLayer>>,
    mut spot: ResMut<Spotlight>,
) {
    let (width, height) = windows
        .iter()
        .next()
        .map_or((1280.0, 800.0), |w| (w.width(), w.height()));
    let lang = Lang::of(&settings.lang);
    let now = desk.shown().map(|run| Drawn {
        tour: run.tour,
        chapter: run.chapter,
        step: run.step,
        mode: run.mode,
        held: run.held,
        lang,
        phone: desk.setting.phone,
        setting: (desk.setting.z, desk.setting.pending, desk.setting.at_table),
        // The bubble's width follows the window's class; a resize within a
        // class is the placement's.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        width: (width / 100.0) as u32,
    });
    if now == spot.drawn && (now.is_none() || !layers.is_empty()) {
        return;
    }
    let Some(fonts) = fonts else {
        return;
    };
    spot.drawn.clone_from(&now);
    spot.rebuilds += 1;
    let scrim_key = desk
        .shown()
        .filter(|r| r.mode != Mode::Folded)
        .map(|run| ScrimKey {
            mode: run.mode,
            text_step: run.current().anchor_on(desk.setting.phone).is_none(),
            z: desk.setting.z,
            alpha: desk.setting.alpha.to_bits(),
        });
    let keep_scrim = scrim_key.is_some() && scrim_key == spot.scrim;
    let bubble_next = scrim_key.is_some();
    for (layer, is_scrim, is_bubble) in &layers {
        if is_scrim && keep_scrim {
            continue;
        }
        if is_bubble && bubble_next {
            // Stands until the new bubble is laid out in its place.
            commands.entity(layer).remove::<Bubble>().insert(Leaving);
            continue;
        }
        commands.entity(layer).despawn();
    }
    spot.scrim = scrim_key;
    let Some(run) = desk.shown() else {
        spot.hole = None;
        return;
    };
    let input = input.as_deref().copied().unwrap_or_default();
    let kit = Kit {
        fonts: &fonts,
        m: metrics(width, height, &settings, input),
        german: lang == Lang::De,
    };
    if run.mode == Mode::Folded {
        pill(&mut commands, kit, run, &desk.setting, lang);
        return;
    }
    if !keep_scrim {
        let hole = spot
            .hole
            .filter(|_| !scrim_key.is_some_and(|k| k.text_step));
        scrim(&mut commands, run, &desk.setting, hole, (width, height));
    }
    bubble(&mut commands, kit, run, &desk.setting, lang, width);
}

/// The scrim's side `side` round `hole` in a `w` by `h` window; the pointer
/// stop (4) is the hole itself.
fn side_rect(side: u8, hole: Rect, (w, h): (f32, f32)) -> Rect {
    match side {
        0 => Rect::new(0.0, 0.0, w, hole.min.y),
        1 => Rect::new(0.0, hole.max.y, w, h),
        2 => Rect::new(0.0, hole.min.y, hole.min.x, hole.max.y),
        3 => Rect::new(hole.max.x, hole.min.y, w, hole.max.y),
        _ => hole,
    }
}

fn ring_rect(hole: Rect) -> Rect {
    Rect::new(
        hole.min.x - RING,
        hole.min.y - RING,
        hole.max.x + RING,
        hole.max.y + RING,
    )
}

/// The scrim, spawned round `hole` (the one that stood, so a new scrim's
/// first frame dims what the last one did) or, without one, zero-sized until
/// [`place`] finds the anchor.
fn scrim(
    commands: &mut Commands,
    run: &Run,
    setting: &Setting,
    hole: Option<Rect>,
    window: (f32, f32),
) {
    let layer = commands
        .spawn((
            TourLayer,
            ScrimLayer,
            Node {
                position_type: PositionType::Absolute,
                left: px_fixed(0.0),
                top: px_fixed(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(setting.z),
            Pickable::IGNORE,
        ))
        .id();
    let dark = Color::srgba(0.0, 0.0, 0.0, setting.alpha);
    let text_step = run.current().anchor_on(setting.phone).is_none();
    for side in 0..4u8 {
        let mut node = Node {
            position_type: PositionType::Absolute,
            ..default()
        };
        if text_step && side == 0 {
            node.width = Val::Percent(100.0);
            node.height = Val::Percent(100.0);
        } else if let Some(hole) = hole.filter(|_| !text_step) {
            set_rect(&mut node, side_rect(side, hole, window));
        } else {
            node.width = px_fixed(0.0);
            node.height = px_fixed(0.0);
        }
        let child = commands
            .spawn((Scrim(side), node, BackgroundColor(dark)))
            .id();
        // A try-it step asks the player to act, often on something beside
        // its anchor (the hand for a cast, the table for a visit): the scrim
        // only dims then, and every press goes through it.
        if run.mode == Mode::Try {
            commands.entity(child).insert(Pickable::IGNORE);
        }
        commands.entity(layer).add_child(child);
    }
    if text_step {
        return;
    }
    if run.mode == Mode::Narrated {
        // Narrated: the hole is a light, not a door — a press in it does
        // nothing (§3.2). Try-it leaves it open.
        let mut node = Node {
            position_type: PositionType::Absolute,
            width: px_fixed(0.0),
            height: px_fixed(0.0),
            ..default()
        };
        if let Some(hole) = hole {
            set_rect(&mut node, hole);
        }
        let stop = commands.spawn((Scrim(4), node)).id();
        commands.entity(layer).add_child(stop);
    }
    let mut node = Node {
        position_type: PositionType::Absolute,
        width: px_fixed(0.0),
        height: px_fixed(0.0),
        border: UiRect::all(px_fixed(1.0)),
        border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL + RING)),
        ..default()
    };
    if let Some(hole) = hole {
        set_rect(&mut node, ring_rect(hole));
    }
    let ring = commands
        .spawn((
            Ring,
            node,
            BorderColor::all(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(layer).add_child(ring);
}

fn bubble_width(kit: Kit) -> f32 {
    match kit.m.frame {
        Frame::Phone => 300.0,
        Frame::Compact => 320.0,
        _ => kit.m.scaled(360.0),
    }
}

/// The bubble's band: over the report form for the steps read while it is
/// up (TOURS.md §3.3), and always over the hover preview (Windows 4K pass,
/// 09.10.: a preview opened under a resting pointer hid half of D7's words).
fn bubble_z(anchor: Option<baylee_client_core::tour::Anchor>, z: i32) -> i32 {
    if anchor.is_some_and(baylee_client_core::tour::Anchor::on_report_form) {
        REPORT_FORM_Z + 10
    } else {
        (z + 6).max(crate::lobby::preview::PREVIEW_Z + 1)
    }
}

#[allow(clippy::too_many_lines)] // one card, top to bottom as §1.2 lists it
fn bubble(commands: &mut Commands, kit: Kit, run: &Run, setting: &Setting, lang: Lang, width: f32) {
    let step = run.current();
    let chapter = run.current_chapter();
    let chapters = run.tour.chapters();
    // The bubble's buttons are the kit's compact ones (owner, 09.10.):
    // 32 high under a pointer, 44 under touch, their words at the small
    // size.
    let compact = Kit {
        m: ShellMetrics {
            control: 32.0,
            hit: if kit.m.touch() { 44.0 } else { 32.0 },
            text: kit.m.small,
            ..kit.m
        },
        ..kit
    };
    let card = commands
        .spawn((
            TourLayer,
            Bubble,
            Node {
                position_type: PositionType::Absolute,
                // Never wider than the window less its inset on each side,
                // so the placement can keep all of it inside.
                width: px_fixed(bubble_width(kit).min((width - 2.0 * INSET).max(0.0))),
                max_height: Val::Percent(if setting.phone { 60.0 } else { 80.0 }),
                padding: UiRect::all(px_fixed(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(8.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            BorderColor::all(tokens::BORDER),
            // Over the report form for the steps read while it is up
            // (TOURS.md §3.3): the form spans the window and holds every
            // key, so the bubble's buttons are reached by the pointer only,
            // beside it.
            GlobalZIndex(bubble_z(step.anchor, setting.z)),
            Visibility::Hidden,
        ))
        .id();
    // 1. The chapter line, and the fold control at its right.
    let line = commands
        .spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        })
        .id();
    let total = chapters
        .iter()
        .filter(|c| c.steps.iter().any(|s| s.kind != Kind::Jit))
        .count()
        .max(1);
    let number = chapters[..=run.chapter]
        .iter()
        .filter(|c| c.steps.iter().any(|s| s.kind != Kind::Jit))
        .count()
        .max(1);
    let words = if run.single {
        chapter.name.text(lang).to_string()
    } else {
        Phrase::TourChapterLine.fill(
            lang,
            &[
                &number.to_string(),
                &total.to_string(),
                chapter.name.text(lang),
            ],
        )
    };
    let words = controls::label(commands, kit, &words, kit.m.small, tokens::MUTED);
    let fold = controls::button(
        commands,
        compact,
        "\u{00d7}",
        Weight::Ghost,
        Live::Yes,
        None,
        TourPress::Fold,
    );
    commands.entity(line).add_children(&[words, fold]);
    // 2. The title, one line.
    let title = commands
        .spawn((
            Text::new(step.title.text(lang)),
            tf(kit.fonts, kit.m.head),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    // 3. The body, one paragraph.
    let body = commands
        .spawn((
            Text::new(step.body.text(lang)),
            // One step under the shell's body text (owner, 09.10.): the
            // bubble is a note beside the screen, not a page of it.
            tf(kit.fonts, kit.m.small),
            TextColor(tokens::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(card).add_children(&[line, title, body]);
    if matches!(step.kind, Kind::Try(_)) {
        let free = if setting.at_table {
            Phrase::TourKeysTable
        } else {
            Phrase::TourKeysFree
        };
        let free = controls::label(commands, kit, free.text(lang), kit.m.small, tokens::MUTED);
        commands.entity(card).add_child(free);
    }
    // 4. The dots: one per step of this chapter.
    let dots = commands
        .spawn((
            Node {
                column_gap: px_fixed(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let current = step.id;
    let mut passed = true;
    for dot in run.dots() {
        if dot.id == current {
            passed = false;
        }
        let skipped = run.skipped.contains(&dot.id);
        let (fill, rim) = if dot.id == current {
            (tokens::ACCENT, tokens::ACCENT)
        } else if skipped {
            (Color::NONE, tokens::MUTED)
        } else if passed {
            (tokens::INK.with_alpha(0.4), Color::NONE)
        } else {
            (tokens::MUTED.with_alpha(0.4), Color::NONE)
        };
        let node = commands
            .spawn((
                Node {
                    width: px_fixed(6.0),
                    height: px_fixed(6.0),
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                    ..default()
                },
                BackgroundColor(fill),
                BorderColor::all(rim),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(dots).add_child(node);
    }
    commands.entity(card).add_child(dots);
    // 5. The foot (owner, 09.10.): Back at the left and Next at the right
    // on one line, Skip quiet and centred under them, the box on a line of
    // its own at the very bottom — never squeezed between buttons.
    let (left, right) = if step.kind == Kind::Offer {
        (
            Some(controls::button(
                commands,
                compact,
                Phrase::TourLater.text(lang),
                Weight::Secondary,
                Live::Yes,
                None,
                TourPress::Later,
            )),
            controls::button(
                commands,
                compact,
                Phrase::TourStartPractice.text(lang),
                Weight::Primary,
                Live::Yes,
                Some("Enter"),
                TourPress::Practice,
            ),
        )
    } else {
        let back = run.has_back().then(|| {
            controls::button(
                commands,
                compact,
                Phrase::TourBack.text(lang),
                Weight::Secondary,
                Live::Yes,
                None,
                TourPress::Back,
            )
        });
        // Done only where the tour ends; a lobby chapter's end goes on to
        // the next chapter, on its screen.
        let done = run.last() && (run.single || run.chapter + 1 >= chapters.len());
        let next = if done {
            Phrase::TourDone
        } else {
            Phrase::TourNext
        };
        let live = if run.primary_live() {
            Live::Yes
        } else {
            Live::No(Phrase::TourTryFirst.text(lang))
        };
        let cap = (run.mode == Mode::Narrated).then_some("Enter");
        let next = controls::button(
            commands,
            compact,
            next.text(lang),
            Weight::Primary,
            live,
            cap,
            TourPress::Next,
        );
        (back, next)
    };
    let pair = commands
        .spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            column_gap: px_fixed(8.0),
            margin: UiRect::top(px_fixed(4.0)),
            ..default()
        })
        .id();
    // An empty left keeps Next at the right on a chapter's first step.
    let left = left.unwrap_or_else(|| commands.spawn((Node::default(), Pickable::IGNORE)).id());
    commands.entity(pair).add_children(&[left, right]);
    commands.entity(card).add_child(pair);
    // The room chapter's end: the table the tour opened is a real one, so
    // the way out of it stands in the bubble (09.10.).
    if run.offers_leave() {
        let leave = controls::button(
            commands,
            compact,
            Phrase::TourLeaveTable.text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            TourPress::LeaveTable,
        );
        commands
            .entity(leave)
            .entry::<Node>()
            .and_modify(|mut node| node.align_self = AlignSelf::Center);
        commands.entity(card).add_child(leave);
    }
    if step.kind != Kind::Offer && (!run.last() || run.single) {
        let skip = controls::button(
            commands,
            compact,
            Phrase::TourSkip.text(lang),
            Weight::Ghost,
            Live::Yes,
            None,
            TourPress::Skip,
        );
        commands
            .entity(skip)
            .entry::<Node>()
            .and_modify(|mut node| node.align_self = AlignSelf::Center);
        commands.entity(card).add_child(skip);
    }
    let rule = commands
        .spawn((
            Node {
                height: px_fixed(1.0),
                width: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    let tick = never_again(commands, kit, lang);
    commands.entity(card).add_children(&[rule, tick]);
}

/// The box in the bubble's foot: *Don't show tours again* (one box, as World of Warcraft's).
fn never_again(commands: &mut Commands, kit: Kit, lang: Lang) -> Entity {
    let square = commands
        .spawn((
            Node {
                width: px_fixed(18.0),
                height: px_fixed(18.0),
                border: UiRect::all(px_fixed(1.5)),
                border_radius: BorderRadius::all(px_fixed(4.0)),
                ..default()
            },
            BorderColor::all(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let words = controls::label(
        commands,
        kit,
        Phrase::TourNeverAgain.text(lang),
        kit.m.small,
        tokens::MUTED,
    );
    let row = commands
        .spawn((
            Node {
                column_gap: px_fixed(8.0),
                align_items: AlignItems::Center,
                min_height: px_fixed(28.0),
                align_self: AlignSelf::FlexStart,
                ..default()
            },
            TourPress::NeverAgain,
        ))
        .id();
    commands.entity(row).add_children(&[square, words]);
    row
}

/// The folded bubble: a pill at the top centre with the step's title, a
/// press away (no key: in folded mode the keys are the screen's, §1.3).
fn pill(commands: &mut Commands, kit: Kit, run: &Run, setting: &Setting, lang: Lang) {
    let lane = commands
        .spawn((
            TourLayer,
            Node {
                position_type: PositionType::Absolute,
                top: px_fixed(setting.top + 8.0),
                left: px_fixed(0.0),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px_fixed(4.0),
                ..default()
            },
            GlobalZIndex(bubble_z(None, setting.z)),
            Pickable::IGNORE,
        ))
        .id();
    let words = run.current().title.text(lang);
    let button = controls::button(
        commands,
        kit,
        words,
        Weight::Secondary,
        Live::Yes,
        None,
        TourPress::Unfold,
    );
    commands.entity(lane).add_child(button);
    if run.auto_folded && setting.at_table {
        let waits = controls::label(
            commands,
            kit,
            Phrase::TourTableWaits.text(lang),
            kit.m.small,
            tokens::MUTED,
        );
        commands.entity(lane).add_child(waits);
    }
}

/// A dead primary's reason (a try-it step's Next: "Try it first, or skip")
/// is the kit's tooltip, which stands on the popover rung from the button's
/// left edge. Under the bubble's rung it was drawn behind the bubble, only
/// its tail showing past the bubble's right edge (09.10., de at 150 %): it
/// now stands over the bubble and ends at the button's right edge, inside
/// the bubble.
pub(super) fn lift_tips(
    owners: Query<&controls::Tooltip, (With<TourPress>, Added<controls::Tooltip>)>,
    bubbles: Query<&GlobalZIndex, With<Bubble>>,
    mut tips: Query<(&mut Node, &mut GlobalZIndex), Without<Bubble>>,
) {
    let over = bubbles.iter().map(|z| z.0).max().unwrap_or(0) + 1;
    for tip in &owners {
        if let Ok((mut node, mut z)) = tips.get_mut(tip.0) {
            node.left = Val::Auto;
            node.right = px_fixed(0.0);
            if z.0 < over {
                z.0 = over;
            }
        }
    }
}

fn logical(node: &ComputedNode, at: &UiGlobalTransform) -> Rect {
    let scale = node.inverse_scale_factor;
    let centre = at.translation * scale;
    let size = node.size() * scale;
    Rect::from_center_size(centre, size)
}

/// Whether `node` stands anywhere but `rect`.
fn differs(node: &Node, rect: Rect) -> bool {
    (node.left, node.top, node.width, node.height) != px_rect(rect)
}

fn px_rect(rect: Rect) -> (Val, Val, Val, Val) {
    (
        Val::Px(rect.min.x),
        Val::Px(rect.min.y),
        Val::Px(rect.width().max(0.0)),
        Val::Px(rect.height().max(0.0)),
    )
}

/// Writes `rect` into `node` when it differs; returns whether it did.
fn set_rect(node: &mut Node, rect: Rect) -> bool {
    let want = px_rect(rect);
    if (node.left, node.top, node.width, node.height) == want {
        return false;
    }
    node.left = want.0;
    node.top = want.1;
    node.width = want.2;
    node.height = want.3;
    true
}

/// Where the anchor of the step standing is, in logical pixels.
pub(super) fn anchor_rect(run: &Run, phone: bool, anchors: &Anchors) -> Option<Rect> {
    let want = run.current().anchor_on(phone)?;
    // Several nodes may carry one id (the header's nav pills): the hole is
    // round all of them.
    anchors
        .iter()
        .filter(|(a, node, _, shown)| a.0 == want && shown.get() && node.size() != Vec2::ZERO)
        .map(|(_, node, at, _)| logical(node, at))
        .reduce(|a, b| a.union(b))
}

/// Every node a step may point at, as laid out (never the scrim's own).
pub(super) type Anchors<'w, 's> = Query<
    'w,
    's,
    (
        &'static TourAnchor,
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static InheritedVisibility,
    ),
    (Without<Scrim>, Without<Ring>),
>;

/// Every pressable node on screen but the tour's own: a kit hit area or a
/// table button.
type Controls<'w, 's> = Query<
    'w,
    's,
    (
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static InheritedVisibility,
        Option<&'static crate::shellkit::Role>,
        Has<Button>,
    ),
    (
        Or<(With<crate::shellkit::Role>, With<Button>)>,
        Without<TourPress>,
        Without<Scrim>,
        Without<Ring>,
    ),
>;

/// Moves the scrim, the hairline and the bubble to the anchor, after
/// layout; writes a node only when its rectangle moved.
#[allow(clippy::type_complexity, clippy::too_many_arguments)] // a Bevy system: the layer, the anchor, the controls
pub(super) fn place(
    desk: Res<TourDesk>,
    mut spot: ResMut<Spotlight>,
    windows: Query<&Window>,
    anchors: Anchors,
    mut scrims: Query<
        (&Scrim, &mut Node, &mut ComputedNode, &mut UiGlobalTransform),
        (Without<Ring>, Without<Bubble>),
    >,
    mut rings: Query<
        (&mut Node, &mut ComputedNode, &mut UiGlobalTransform),
        (With<Ring>, Without<Scrim>, Without<Bubble>),
    >,
    leaving: Query<Entity, With<Leaving>>,
    mut commands: Commands,
    mut bubbles: Query<
        (&mut Node, &ComputedNode, &mut Visibility),
        (With<Bubble>, Without<Scrim>, Without<Ring>),
    >,
    controls: Controls,
    strips: Query<
        (&ComputedNode, &UiGlobalTransform, &InheritedVisibility),
        (
            With<crate::shellkit::header::SeatStrip>,
            Without<Scrim>,
            Without<Ring>,
        ),
    >,
) {
    let Some(run) = desk.shown() else {
        if spot.bubble.is_some() {
            spot.bubble = None;
        }
        return;
    };
    let Some(window) = windows.iter().next() else {
        return;
    };
    let (w, h) = (window.width(), window.height());
    let wants_anchor = run.current().anchor_on(desk.setting.phone).is_some();
    let hole = anchor_rect(run, desk.setting.phone, &anchors).map(|r| {
        Rect::new(r.min.x - PAD, r.min.y - PAD, r.max.x + PAD, r.max.y + PAD)
            .intersect(Rect::new(0.0, 0.0, w, h))
    });
    if wants_anchor && hole.is_none() {
        // The anchor's screen is rebuilding (or the step is about to be
        // passed over): everything holds where it stood, rather than the
        // bubble jumping to the centre for a frame.
        return;
    }
    if spot.hole != hole {
        spot.hole = hole;
    }
    if let Some(hole) = hole {
        for (side, mut node, mut computed, mut at) in &mut scrims {
            let rect = side_rect(side.0, hole, (w, h));
            // Asked through `&Node` first: `&mut node` is a write, and
            // marked every side changed every frame, relaying out the UI.
            if differs(&node, rect) && set_rect(&mut node, rect) {
                lay_out_now(&mut computed, &mut at, rect);
            }
        }
        for (mut node, mut computed, mut at) in &mut rings {
            let rect = ring_rect(hole);
            // Asked through `&Node` first: `&mut node` is a write, and
            // marked every side changed every frame, relaying out the UI.
            if differs(&node, rect) && set_rect(&mut node, rect) {
                lay_out_now(&mut computed, &mut at, rect);
            }
        }
    }
    for (mut node, computed, mut shown) in &mut bubbles {
        let size = computed.size() * computed.inverse_scale_factor;
        if size == Vec2::ZERO {
            continue;
        }
        // The anchor's own buttons, which the bubble must not cover.
        let pressable: Vec<Rect> = hole.map_or_else(Vec::new, |hole| {
            controls
                .iter()
                .filter(|(_, _, shown, role, button)| {
                    shown.get()
                        && (*button || role.is_some_and(|r| *r == crate::shellkit::Role::Hit))
                })
                .map(|(node, at, ..)| logical(node, at))
                .filter(|r| !r.intersect(hole).is_empty())
                .collect()
        });
        // The shell's strips ("Seated at …") are never covered either
        // (09.10.: L4's bubble lay on the seated strip).
        let pressable: Vec<Rect> = pressable
            .into_iter()
            .chain(
                strips
                    .iter()
                    .filter(|(node, _, shown)| shown.get() && node.size() != Vec2::ZERO)
                    .map(|(node, at, _)| logical(node, at))
                    .filter(|r| hole.is_none_or(|hole| r.intersect(hole).is_empty())),
            )
            .collect();
        // A part of the report form (T32's attachments): beside the form,
        // level with the part, never over the form's other rows.
        let at = match (hole, sheet_beside(run, &anchors)) {
            (Some(part), Some(sheet)) => spot_beside(part, sheet, size, (w, h))
                .unwrap_or_else(|| spot_for(hole, size, (w, h), desk.setting.top, &pressable)),
            _ => spot_for(hole, size, (w, h), desk.setting.top, &pressable),
        };
        let placed = Some(Rect::from_corners(at, at + size));
        if spot.bubble != placed {
            spot.bubble = placed;
        }
        if node.left != Val::Px(at.x) || node.top != Val::Px(at.y) {
            node.left = Val::Px(at.x);
            node.top = Val::Px(at.y);
            // Not shown in a frame whose layout has not seen where it
            // stands: it would be drawn a frame at the old place.
            continue;
        }
        if shown.set_if_neq(Visibility::Inherited) {
            for old in &leaving {
                commands.entity(old).despawn();
            }
        }
    }
}

/// Writes what layout would make of `rect` for an absolute box in the
/// full-window layer, so it is drawn this frame and not the next.
fn lay_out_now(computed: &mut ComputedNode, at: &mut UiGlobalTransform, rect: Rect) {
    let scale = computed.inverse_scale_factor;
    if scale <= 0.0 {
        return;
    }
    let size = rect.size() / scale;
    computed.size = size;
    computed.unrounded_size = size;
    *at = UiGlobalTransform::from_translation(rect.center() / scale);
}

/// The report form's sheet, when the step stands on a part of it.
fn sheet_beside(run: &Run, anchors: &Anchors) -> Option<Rect> {
    use baylee_client_core::tour::Anchor;
    if run.current().anchor != Some(Anchor::ReportAttachments) {
        return None;
    }
    anchors
        .iter()
        .find(|(a, node, _, shown)| {
            a.0 == Anchor::ReportForm && shown.get() && node.size() != Vec2::ZERO
        })
        .map(|(_, node, at, _)| logical(node, at))
}

/// Beside `sheet`, level with `part` of it (clamped into the window): its
/// right side first, then its left; `None` when neither has the room.
#[must_use]
pub fn spot_beside(part: Rect, sheet: Rect, size: Vec2, (w, h): (f32, f32)) -> Option<Vec2> {
    let gap = 12.0;
    let y = (part.center().y - size.y / 2.0).clamp(INSET, (h - size.y - INSET).max(INSET));
    if sheet.max.x + gap + size.x + INSET <= w {
        Some(Vec2::new(sheet.max.x + gap, y))
    } else if sheet.min.x - gap - size.x >= INSET {
        Some(Vec2::new(sheet.min.x - gap - size.x, y))
    } else {
        None
    }
}

/// Where the bubble stands (§1.2): below the anchor, centred on it; above
/// it when the anchor is in the lower half; beside it when neither fits;
/// clamped inside the window; centred with no anchor. When no side has the
/// room (a sheet nearly the window's size), it stands where it covers none
/// of `controls` — the anchor's buttons — and as little of the anchor as it
/// can.
#[must_use]
pub fn spot_for(
    hole: Option<Rect>,
    size: Vec2,
    (w, h): (f32, f32),
    top: f32,
    controls: &[Rect],
) -> Vec2 {
    let clamp = |p: Vec2| {
        Vec2::new(
            p.x.clamp(INSET, (w - size.x - INSET).max(INSET)),
            p.y.clamp(INSET, (h - size.y - INSET).max(INSET)),
        )
    };
    let Some(hole) = hole else {
        return clamp(Vec2::new((w - size.x) / 2.0, (h - size.y) / 2.0));
    };
    let gap = 12.0;
    let centre_x = hole.center().x - size.x / 2.0;
    let centre_y = hole.center().y - size.y / 2.0;
    // Below steps down past whatever it would cover (a strip under the
    // header, 09.10.), until it covers nothing.
    let mut below = Vec2::new(centre_x, hole.max.y + gap);
    for _ in 0..controls.len() {
        let card = Rect::from_corners(below, below + size);
        match controls.iter().find(|c| !card.intersect(**c).is_empty()) {
            Some(c) if c.max.y + gap > below.y => below.y = c.max.y + gap,
            _ => break,
        }
    }
    let above = Vec2::new(centre_x, hole.min.y - gap - size.y);
    let right = Vec2::new(hole.max.x + gap, centre_y);
    let left = Vec2::new(hole.min.x - gap - size.x, centre_y);
    let fits_below = below.y + size.y + INSET <= h;
    let fits_above = hole.min.y - gap - size.y >= top.min(INSET.max(top));
    let fits_right = hole.max.x + gap + size.x + INSET <= w;
    let fits_left = hole.min.x - gap - size.x >= INSET;
    let lower_half = hole.center().y > h / 2.0;
    let order: [(bool, Vec2); 4] = if lower_half {
        [
            (fits_above, above),
            (fits_below, below),
            (fits_right, right),
            (fits_left, left),
        ]
    } else {
        [
            (fits_below, below),
            (fits_above, above),
            (fits_right, right),
            (fits_left, left),
        ]
    };
    let covers = |at: Vec2| {
        let card = Rect::from_corners(at, at + size);
        controls.iter().any(|c| !card.intersect(*c).is_empty())
    };
    for (fits, at) in order {
        if fits && !covers(clamp(at)) {
            return clamp(at);
        }
    }
    // Nothing fits beside a very large anchor: of the window's corners
    // and the four sides pressed into the window, the place that covers no
    // button of the anchor and the least of the anchor itself.
    let area = |a: Rect, b: Rect| {
        let both = a.intersect(b);
        if both.is_empty() {
            0.0
        } else {
            both.width() * both.height()
        }
    };
    let low = h - size.y - INSET;
    let far = w - size.x - INSET;
    let high = INSET.max(top);
    let candidates = [
        Vec2::new(INSET, high),
        Vec2::new(far, high),
        Vec2::new(INSET, low),
        Vec2::new(far, low),
        clamp(below),
        clamp(above),
        clamp(right),
        clamp(left),
    ];
    candidates
        .into_iter()
        .map(clamp)
        .min_by(|a, b| {
            let cost = |at: Vec2| {
                let card = Rect::from_corners(at, at + size);
                let covered: f32 = controls.iter().map(|c| area(card, *c)).sum();
                (covered, area(card, hole))
            };
            let (ca, ha) = cost(*a);
            let (cb, hb) = cost(*b);
            ca.total_cmp(&cb).then(ha.total_cmp(&hb))
        })
        .unwrap_or_else(|| clamp(below))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(at: Vec2, size: Vec2) -> Rect {
        Rect::from_corners(at, at + size)
    }

    /// The builder's history sheet at 1600 x 938 (D14, live): no side of it
    /// has room for the bubble, and its buttons (Restore in its foot, the
    /// close cross in its head) must stay uncovered.
    #[test]
    fn a_bubble_beside_a_sheet_with_no_room_covers_none_of_its_buttons() {
        let window = (1600.0, 938.0);
        let sheet = Rect::new(312.0, 269.0, 1288.0, 669.0);
        let size = Vec2::new(360.0, 350.0);
        let buttons = [
            Rect::new(1120.0, 610.0, 1270.0, 654.0),
            Rect::new(980.0, 610.0, 1110.0, 654.0),
            Rect::new(1236.0, 280.0, 1280.0, 324.0),
            Rect::new(330.0, 330.0, 700.0, 374.0),
        ];
        let at = spot_for(Some(sheet), size, window, 64.0, &buttons);
        let bubble = card(at, size);
        for b in buttons {
            assert!(bubble.intersect(b).is_empty(), "{bubble:?} covers {b:?}");
        }
        assert!(bubble.min.x >= INSET && bubble.max.x <= window.0 - INSET + 0.01);
        assert!(bubble.min.y >= INSET && bubble.max.y <= window.1 - INSET + 0.01);
    }

    /// D11 on a 4K screen at 150 % (Windows pass, 09.10.): the Save button
    /// stands at the window's right edge; the whole bubble stays inside the
    /// window with its inset, whichever side it takes, at both live sizes.
    #[test]
    fn a_bubble_anchored_at_the_right_edge_stays_inside_the_window() {
        for window in [(1708.0, 1032.0), (2560.0, 1440.0), (1280.0, 720.0)] {
            for size in [Vec2::new(360.0, 260.0), Vec2::new(540.0, 300.0)] {
                for anchor in [
                    Rect::new(window.0 - 120.0, 8.0, window.0 - 4.0, 52.0),
                    Rect::new(
                        window.0 - 60.0,
                        window.1 / 2.0,
                        window.0,
                        window.1 / 2.0 + 44.0,
                    ),
                    Rect::new(window.0 - 200.0, window.1 - 60.0, window.0, window.1),
                ] {
                    let at = spot_for(Some(anchor), size, window, 64.0, &[anchor]);
                    let bubble = card(at, size);
                    assert!(
                        bubble.min.x >= INSET - 0.01 && bubble.max.x <= window.0 - INSET + 0.01,
                        "{window:?} {anchor:?}: {bubble:?}"
                    );
                    assert!(
                        bubble.min.y >= INSET - 0.01 && bubble.max.y <= window.1 - INSET + 0.01,
                        "{window:?} {anchor:?}: {bubble:?}"
                    );
                }
            }
        }
    }

    /// D7 (Windows pass, 09.10.): the card preview a resting pointer opens
    /// over the pool never stands over the bubble, in the lobby's band or
    /// the table's; the report form's steps stay over the form.
    #[test]
    fn the_bubble_stands_over_the_card_preview() {
        use baylee_client_core::tour::Anchor;
        let preview = crate::lobby::preview::PREVIEW_Z;
        for z in [tokens::z::SHEET - 1, 950] {
            assert!(bubble_z(None, z) > preview, "{z}");
            assert!(bubble_z(Some(Anchor::DecksNew), z) > preview, "{z}");
        }
        assert!(bubble_z(Some(Anchor::ReportForm), 19) > REPORT_FORM_Z);
    }

    /// T32 (09.10.): the bubble stands beside the report form, level with
    /// its attachments, covering none of the form, at both live sizes.
    #[test]
    fn the_attachments_bubble_stands_beside_the_form() {
        for window in [(1708.0, 1032.0), (2560.0, 1440.0)] {
            let sheet = Rect::new(
                window.0 / 2.0 - 320.0,
                80.0,
                window.0 / 2.0 + 320.0,
                window.1 - 80.0,
            );
            let part = Rect::new(sheet.min.x + 24.0, 500.0, sheet.max.x - 24.0, 760.0);
            let size = Vec2::new(360.0, 300.0);
            let at = spot_beside(part, sheet, size, window).expect("room beside");
            let bubble = card(at, size);
            assert!(bubble.intersect(sheet).is_empty(), "{window:?}: {bubble:?}");
            assert!(bubble.min.y <= part.max.y && bubble.max.y >= part.min.y);
        }
        let narrow = Rect::new(20.0, 0.0, 1260.0, 720.0);
        assert!(spot_beside(narrow, narrow, Vec2::new(360.0, 300.0), (1280.0, 720.0)).is_none());
    }

    /// L4 (09.10.): the header's nav pills with the seated strip right
    /// under them; the bubble takes another side than the strip's.
    #[test]
    fn a_bubble_never_lies_on_the_seated_strip() {
        let window = (1708.0, 1032.0);
        let size = Vec2::new(360.0, 300.0);
        let nav = Rect::new(192.0, 8.0, 540.0, 52.0);
        let strip = Rect::new(0.0, 64.0, 1708.0, 100.0);
        let at = spot_for(Some(nav), size, window, 64.0, &[strip]);
        let bubble = card(at, size);
        assert!(bubble.intersect(strip).is_empty(), "{bubble:?}");
        assert!(bubble.intersect(nav).is_empty(), "{bubble:?}");
    }

    /// Where a side has room the bubble never touches the anchor at all.
    #[test]
    fn a_bubble_with_room_stands_clear_of_its_anchor() {
        let window = (1708.0, 1032.0);
        let size = Vec2::new(360.0, 300.0);
        for anchor in [
            Rect::new(192.0, 0.0, 540.0, 58.0),
            Rect::new(127.0, 404.0, 561.0, 464.0),
            Rect::new(0.0, 728.0, 1600.0, 784.0),
            Rect::new(1478.0, 65.0, 1674.0, 125.0),
        ] {
            let at = spot_for(Some(anchor), size, window, 64.0, &[anchor]);
            assert!(card(at, size).intersect(anchor).is_empty(), "{anchor:?}");
        }
    }

    /// The tour's two systems over a window and one anchor, laid out by hand
    /// (no layout plugin: what `place` writes is what is read back).
    mod still {
        use super::super::*;
        use crate::tour::TourDesk;
        use baylee_client_core::tour::Tour;

        fn fonts() -> UiFonts {
            UiFonts {
                text: Handle::default(),
                medium: Handle::default(),
                bold: Handle::default(),
                italic: Handle::default(),
                medium_italic: Handle::default(),
                serif: Handle::default(),
                serif_italic: Handle::default(),
                icons: Handle::default(),
                mana: Handle::default(),
            }
        }

        /// A run on its first step with an anchor, and that anchor.
        fn anchored(
            from: usize,
        ) -> (
            baylee_client_core::tour::Run,
            baylee_client_core::tour::Anchor,
        ) {
            let mut run = baylee_client_core::tour::Run::chapter(Tour::Lobby, 0, false, 2)
                .expect("the lobby tour");
            let steps = run.current_chapter().steps.len();
            for step in from..steps {
                run.step = step;
                if let Some(anchor) = run.current().anchor_on(false) {
                    return (run, anchor);
                }
            }
            panic!("no anchored step in the lobby's first chapter");
        }

        fn app() -> (App, Entity) {
            let mut app = App::new();
            app.insert_resource(fonts())
                .init_resource::<crate::settings::ClientSettings>()
                .init_resource::<TourDesk>()
                .init_resource::<Spotlight>()
                .add_systems(Update, (draw, place).chain());
            app.world_mut().spawn(Window::default());
            let (run, anchor) = anchored(0);
            app.world_mut().resource_mut::<TourDesk>().start(run);
            let node = ComputedNode {
                size: Vec2::new(100.0, 40.0),
                ..default()
            };
            let at = app
                .world_mut()
                .spawn((
                    TourAnchor(anchor),
                    node,
                    UiGlobalTransform::from_translation(Vec2::new(300.0, 200.0)),
                    InheritedVisibility::VISIBLE,
                ))
                .id();
            app.update();
            app.update();
            (app, at)
        }

        fn top_side(app: &mut App) -> (Entity, Rect) {
            let mut q = app
                .world_mut()
                .query::<(Entity, &Scrim, &ComputedNode, &UiGlobalTransform)>();
            q.iter(app.world())
                .find(|(_, s, ..)| s.0 == 0)
                .map(|(e, _, n, at)| (e, logical(n, at)))
                .expect("the scrim's top")
        }

        #[test]
        fn an_idle_tour_rebuilds_and_writes_nothing_per_frame() {
            let (mut app, _) = app();
            let rebuilds = app.world().resource::<Spotlight>().rebuilds;
            assert!(rebuilds >= 1);
            let tick = app.world().read_change_tick();
            for _ in 0..10 {
                app.update();
            }
            assert_eq!(app.world().resource::<Spotlight>().rebuilds, rebuilds);
            let mut q = app
                .world_mut()
                .query_filtered::<Ref<Node>, Or<(With<Scrim>, With<Ring>, With<Bubble>)>>();
            let written = q
                .iter(app.world())
                .filter(|n| {
                    n.last_changed()
                        .is_newer_than(tick, app.world().read_change_tick())
                })
                .count();
            assert_eq!(written, 0, "a node of the idle tour was written");
            assert!(
                !app.world()
                    .resource_ref::<Spotlight>()
                    .last_changed()
                    .is_newer_than(tick, app.world().read_change_tick()),
                "the spotlight was written while nothing changed"
            );
        }

        #[test]
        fn an_anchor_move_moves_the_hole_in_the_same_frame() {
            let (mut app, anchor) = app();
            let (_, before) = top_side(&mut app);
            assert!((before.max.y - (180.0 - PAD)).abs() < 0.01, "{before:?}");
            app.world_mut()
                .entity_mut(anchor)
                .insert(UiGlobalTransform::from_translation(Vec2::new(300.0, 400.0)));
            app.update();
            let (_, after) = top_side(&mut app);
            assert!(
                (after.max.y - (380.0 - PAD)).abs() < 0.01,
                "the hole trails its anchor: {after:?}"
            );
        }

        #[test]
        fn a_step_change_keeps_the_scrim_and_the_old_bubble_until_the_new_one_stands() {
            let (mut app, _) = app();
            let (scrim, _) = top_side(&mut app);
            let step = app
                .world()
                .resource::<TourDesk>()
                .shown()
                .map(|r| r.step)
                .unwrap();
            let (next, anchor) = anchored(step + 1);
            app.world_mut().resource_mut::<TourDesk>().start(next);
            let mut a = app.world_mut().query::<&mut TourAnchor>();
            for mut at in a.iter_mut(app.world_mut()) {
                at.0 = anchor;
            }
            app.update();
            assert_eq!(top_side(&mut app).0, scrim, "the scrim was rebuilt");
            let mut q = app
                .world_mut()
                .query_filtered::<(), Or<(With<Bubble>, With<Leaving>)>>();
            assert!(
                q.iter(app.world()).count() >= 1,
                "a frame with no bubble at all"
            );
        }

        #[test]
        fn a_missing_anchor_for_a_frame_holds_the_hole() {
            let (mut app, anchor) = app();
            let hole = app.world().resource::<Spotlight>().hole;
            assert!(hole.is_some());
            app.world_mut()
                .entity_mut(anchor)
                .insert(InheritedVisibility::HIDDEN);
            app.update();
            assert_eq!(app.world().resource::<Spotlight>().hole, hole);
        }
    }
}
