//! The tour's nodes: the scrim round the hole, the hairline, the bubble and
//! the pill (TOURS.md §1.2). [`draw`] rebuilds them when what they say
//! changes; [`place`] moves them every frame after layout, writing only what
//! moved.

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

/// The anchor's padding inside the hole, and the hairline's offset.
const PAD: f32 = 8.0;
const RING: f32 = 2.0;
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
    layers: Query<Entity, With<TourLayer>>,
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
    for layer in &layers {
        commands.entity(layer).despawn();
    }
    spot.drawn.clone_from(&now);
    spot.hole = None;
    let Some(run) = desk.shown() else {
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
    scrim(&mut commands, run, &desk.setting);
    bubble(&mut commands, kit, run, &desk.setting, lang);
}

fn scrim(commands: &mut Commands, run: &Run, setting: &Setting) {
    let layer = commands
        .spawn((
            TourLayer,
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
        } else {
            node.width = px_fixed(0.0);
            node.height = px_fixed(0.0);
        }
        let child = commands
            .spawn((Scrim(side), node, BackgroundColor(dark)))
            .id();
        commands.entity(layer).add_child(child);
    }
    if text_step {
        return;
    }
    if run.mode == Mode::Narrated {
        // Narrated: the hole is a light, not a door — a press in it does
        // nothing (§3.2). Try-it leaves it open.
        let stop = commands
            .spawn((
                Scrim(4),
                Node {
                    position_type: PositionType::Absolute,
                    width: px_fixed(0.0),
                    height: px_fixed(0.0),
                    ..default()
                },
            ))
            .id();
        commands.entity(layer).add_child(stop);
    }
    let ring = commands
        .spawn((
            Ring,
            Node {
                position_type: PositionType::Absolute,
                width: px_fixed(0.0),
                height: px_fixed(0.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL + RING)),
                ..default()
            },
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

#[allow(clippy::too_many_lines)] // one card, top to bottom as §1.2 lists it
fn bubble(commands: &mut Commands, kit: Kit, run: &Run, setting: &Setting, lang: Lang) {
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
                width: px_fixed(bubble_width(kit)),
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
            GlobalZIndex(setting.z + 6),
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
            GlobalZIndex(setting.z + 6),
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

fn logical(node: &ComputedNode, at: &UiGlobalTransform) -> Rect {
    let scale = node.inverse_scale_factor;
    let centre = at.translation * scale;
    let size = node.size() * scale;
    Rect::from_center_size(centre, size)
}

fn set_rect(node: &mut Node, rect: Rect) {
    let want = (
        Val::Px(rect.min.x),
        Val::Px(rect.min.y),
        Val::Px(rect.width().max(0.0)),
        Val::Px(rect.height().max(0.0)),
    );
    if (node.left, node.top, node.width, node.height) != want {
        node.left = want.0;
        node.top = want.1;
        node.width = want.2;
        node.height = want.3;
    }
}

/// Where the anchor of the step standing is, in logical pixels.
pub(super) fn anchor_rect(
    run: &Run,
    phone: bool,
    anchors: &Query<(
        &TourAnchor,
        &ComputedNode,
        &UiGlobalTransform,
        &InheritedVisibility,
    )>,
) -> Option<Rect> {
    let want = run.current().anchor_on(phone)?;
    // Several nodes may carry one id (the header's nav pills): the hole is
    // round all of them.
    anchors
        .iter()
        .filter(|(a, node, _, shown)| a.0 == want && shown.get() && node.size() != Vec2::ZERO)
        .map(|(_, node, at, _)| logical(node, at))
        .reduce(|a, b| a.union(b))
}

/// Moves the scrim, the hairline and the bubble to the anchor, after
/// layout; writes a node only when its rectangle moved.
#[allow(clippy::type_complexity)] // three queries of one layer
pub(super) fn place(
    desk: Res<TourDesk>,
    mut spot: ResMut<Spotlight>,
    windows: Query<&Window>,
    anchors: Query<(
        &TourAnchor,
        &ComputedNode,
        &UiGlobalTransform,
        &InheritedVisibility,
    )>,
    mut scrims: Query<(&Scrim, &mut Node), (Without<Ring>, Without<Bubble>)>,
    mut rings: Query<&mut Node, (With<Ring>, Without<Scrim>, Without<Bubble>)>,
    mut bubbles: Query<
        (&mut Node, &ComputedNode, &mut Visibility),
        (With<Bubble>, Without<Scrim>, Without<Ring>),
    >,
) {
    let Some(run) = desk.shown() else {
        return;
    };
    let Some(window) = windows.iter().next() else {
        return;
    };
    let (w, h) = (window.width(), window.height());
    let hole = anchor_rect(run, desk.setting.phone, &anchors).map(|r| {
        Rect::new(r.min.x - PAD, r.min.y - PAD, r.max.x + PAD, r.max.y + PAD)
            .intersect(Rect::new(0.0, 0.0, w, h))
    });
    if spot.hole != hole {
        spot.hole = hole;
    }
    if let Some(hole) = hole {
        for (side, mut node) in &mut scrims {
            let rect = match side.0 {
                0 => Rect::new(0.0, 0.0, w, hole.min.y),
                1 => Rect::new(0.0, hole.max.y, w, h),
                2 => Rect::new(0.0, hole.min.y, hole.min.x, hole.max.y),
                3 => Rect::new(hole.max.x, hole.min.y, w, hole.max.y),
                _ => hole,
            };
            set_rect(&mut node, rect);
        }
        for mut node in &mut rings {
            set_rect(
                &mut node,
                Rect::new(
                    hole.min.x - RING,
                    hole.min.y - RING,
                    hole.max.x + RING,
                    hole.max.y + RING,
                ),
            );
        }
    }
    for (mut node, computed, mut shown) in &mut bubbles {
        let size = computed.size() * computed.inverse_scale_factor;
        if size == Vec2::ZERO {
            continue;
        }
        let at = spot_for(hole, size, w, h, desk.setting.top);
        if node.left != Val::Px(at.x) || node.top != Val::Px(at.y) {
            node.left = Val::Px(at.x);
            node.top = Val::Px(at.y);
        }
        shown.set_if_neq(Visibility::Inherited);
    }
}

/// Where the bubble stands (§1.2): below the anchor, centred on it; above
/// it when the anchor is in the lower half; beside it when neither fits;
/// clamped inside the window; centred with no anchor.
#[must_use]
pub fn spot_for(hole: Option<Rect>, size: Vec2, w: f32, h: f32, top: f32) -> Vec2 {
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
    let below = Vec2::new(centre_x, hole.max.y + gap);
    let above = Vec2::new(centre_x, hole.min.y - gap - size.y);
    let right = Vec2::new(hole.max.x + gap, centre_y);
    let left = Vec2::new(hole.min.x - gap - size.x, centre_y);
    let fits_below = hole.max.y + gap + size.y + INSET <= h;
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
    for (fits, at) in order {
        if fits {
            return clamp(at);
        }
    }
    // Nothing fits beside a very large anchor: the bubble stands in the
    // corner of the window farthest from the anchor's centre.
    let x = if hole.center().x > w / 2.0 {
        INSET
    } else {
        w - size.x - INSET
    };
    let y = if lower_half {
        INSET.max(top)
    } else {
        h - size.y - INSET
    };
    clamp(Vec2::new(x, y))
}
