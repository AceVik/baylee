//! Three pieces of ink laid along the rim above a seat's own battlefield.
//!
//! "Above" belongs to the seat, not to the viewer. Every seat's band is
//! [`SeatSlot::ledge_corners`](baylee_client_core::layout::SeatSlot::ledge_corners)
//! — the strip straddling the rim nearest the middle of the table, which
//! [`LEDGE_IS_OUTER`](baylee_client_core::layout::LEDGE_IS_OUTER) keeps clear
//! of cards — so a seat across the table reads its name and life *below* its
//! creatures on this screen and beside them from its own chair. The two
//! arrangements before this one chose the edge by which of the two projected
//! higher on screen, which put an opponent's ink beyond their far rim, at the
//! top of the window and as far from their own board as the mat allows.
//!
//! The three share one band and one scale: identity at the end that projects
//! leftmost, counts at the other, the twelve steps between them. The steps
//! used to be a vertical column beside the command-zone rim; they are
//! horizontal here because that is the only place on this table where "the
//! middle, above the board" is free — the middle of the *table* is the
//! firewheel's, and a track laid across the gap between two mats would be
//! drawn over the flames.

#[allow(clippy::wildcard_imports)]
use super::*;

const HEADER_H: f32 = 30.0;
const HEADER_W: f32 = 300.0;
const TRACK_W: f32 = 24.0;
const TRACK_H: f32 = 340.0;

/// What the three panels leave between and beside themselves on the band,
/// before any of them is allowed to grow.
///
/// Four of them: outside the identity, either side of the track, outside the
/// counts. It is only a floor — the band is nearly always longer than the
/// three panels need and the slack stays as felt at the ends, because the
/// panels are placed from the ends and the middle rather than stretched.
const BAND_GAP: f32 = 16.0;

/// Which part of the seat's band carries this piece of its information.
#[derive(Component, Clone, Copy, Debug)]
pub enum Panel {
    /// Name, life, priority and turn at the leftmost end of the band.
    Identity,
    /// Public zone counts at the other end.
    Counts,
    /// The twelve steps, in the middle of the band.
    Phases,
}

impl Panel {
    pub(crate) fn size(self) -> Vec2 {
        match self {
            Self::Identity | Self::Counts => Vec2::new(HEADER_W, HEADER_H),
            Self::Phases => Vec2::new(TRACK_H, TRACK_W),
        }
    }
}

/// The letters remain upright; their centres and scale follow the table.
pub(super) fn place(
    duel: &Duel,
    lens: Option<&crate::table::Lens>,
    player: PlayerId,
    panel: Panel,
    node: &mut Node,
    turn: &mut UiTransform,
) {
    let pose = (|| {
        let lens = lens?;
        let slot = duel
            .layout
            .as_ref()?
            .slots
            .iter()
            .find(|s| s.player == player)?;
        // The seat's own band, projected: the strip along the rim nearest the
        // middle of the table. Taken from the model rather than measured off
        // `half_extent` here, so the ink and `Shelf` — the density probe and
        // the tiny-overview fallback — are describing one rectangle again.
        Some(pose_on(lens.corners(slot.ledge_corners())?, panel))
    })();
    let Some((corner, tilt, scale)) = pose else {
        if node.display != Display::None {
            node.display = Display::None;
        }
        return;
    };
    if node.display != Display::Flex {
        node.display = Display::Flex;
    }
    if node.left != px(corner.x) {
        node.left = px(corner.x);
    }
    if node.top != px(corner.y) {
        node.top = px(corner.y);
    }
    let rotation = Rot2::radians(tilt);
    if turn.rotation != rotation {
        turn.rotation = rotation;
    }
    if turn.scale != Vec2::splat(scale) {
        turn.scale = Vec2::splat(scale);
    }
}

/// Where one panel sits on a seat's projected band, and how big.
///
/// Split out of [`place`] because it is the whole of the arithmetic and none
/// of the Bevy: `corners` is
/// [`SeatSlot::ledge_corners`](baylee_client_core::layout::SeatSlot::ledge_corners)'
/// loop already run through [`crate::table::Lens`], in the order that model
/// promises — the two on the rim first, then the two that meet the lane
/// behind it.
///
/// Returns the **top-left of the un-rotated box**, which is what a `Node`'s
/// `left`/`top` want, together with the turn and the scale
/// [`UiTransform`] applies about the box's own middle.
pub(crate) fn pose_on(corners: [Vec2; 4], panel: Panel) -> (Vec2, f32, f32) {
    let [near_a, near_b, far_b, far_a] = corners;
    // The band's two *ends*, each already at its middle across the strip, so
    // nothing has to be pushed off an edge afterwards. Sorted by projected x,
    // which is also the fold that keeps the ink upright: a seat across the
    // table has its band running right-to-left in its own frame, and reading
    // it left to right on this screen is what turns its bar the right way up.
    let end_a = near_a.midpoint(far_a);
    let end_b = near_b.midpoint(far_b);
    let (left, right) = if end_a.x <= end_b.x {
        (end_a, end_b)
    } else {
        (end_b, end_a)
    };
    let axis = (right - left).normalize_or_zero();
    let width = left.distance(right);
    let depth = near_a.midpoint(near_b).distance(far_a.midpoint(far_b));
    // **One scale for the whole band**, chosen once and asked for three
    // times. Each panel is placed by its own call and a panel that sized
    // itself would grow into its neighbours at exactly the width where it
    // matters: the middle one is the only one that can meet another, and it
    // can meet two.
    let scale = (width / (HEADER_W * 2.0 + TRACK_H + BAND_GAP * 4.0))
        .min(depth / HEADER_H)
        .min(1.0);
    let middle = match panel {
        Panel::Identity => left + axis * (HEADER_W * scale * 0.5 + BAND_GAP * scale),
        Panel::Counts => right - axis * (HEADER_W * scale * 0.5 + BAND_GAP * scale),
        Panel::Phases => left.midpoint(right),
    };
    (
        middle - panel.size() * 0.5,
        axis.y.atan2(axis.x),
        scale.max(0.0),
    )
}

fn frame(
    commands: &mut Commands,
    root: Entity,
    player: PlayerId,
    panel: Panel,
    surface: Option<Handle<crate::frontal::FrontalMaterial>>,
) -> Entity {
    let size = panel.size();
    let entity = commands
        .spawn((
            SeatBar {
                player,
                placed: None,
            },
            panel,
            UiTransform::default(),
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                width: px(size.x),
                height: px(size.y),
                // One direction for all three: the band runs along the rim
                // and everything written on it runs with it.
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                border: UiRect::bottom(px(1)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(palette::SEAT_BACKING),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.65)),
            Pickable::IGNORE,
        ))
        .id();
    if let Some(handle) = surface {
        commands.entity(entity).insert((
            BackgroundColor(Color::NONE),
            MaterialNode(handle),
            crate::frontal::Hanging,
        ));
    }
    commands.entity(root).add_child(entity);
    entity
}

#[allow(clippy::too_many_arguments)] // reuses the existing typed seat cells
pub(super) fn spawn(
    commands: &mut Commands,
    root: Entity,
    lang: Lang,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    seat: &SeatView,
    orders: &PhaseOrders,
    fonts: &UiFonts,
    surface: Option<Handle<crate::frontal::FrontalMaterial>>,
) {
    let identity = frame(
        commands,
        root,
        seat.player,
        Panel::Identity,
        surface.clone(),
    );
    let cells = [
        caret(commands, view, seat, fonts, 10.0, HEADER_H),
        swatch(commands, view, statics, seat, 3.0, HEADER_H - 8.0),
        name(commands, lang, view, statics, seat, fonts, 142.0, HEADER_H),
        life(commands, seat, fonts, 72.0, HEADER_H, Density::Split),
        hinge(commands, view, seat, fonts, 56.0, HEADER_H),
    ];
    for cell in cells {
        commands.entity(identity).add_child(cell);
    }
    let counts = frame(commands, root, seat.player, Panel::Counts, surface.clone());
    for zone in [Zone::Hand, Zone::Library, Zone::Graveyard, Zone::Exile] {
        let cell = count(commands, view, seat, zone, fonts, 70.0, HEADER_H);
        commands.entity(counts).add_child(cell);
    }
    let track = frame(commands, root, seat.player, Panel::Phases, surface);
    let side = if same_team(statics, seat.player, view.seat) {
        RailSide::Mine
    } else {
        RailSide::Theirs
    };
    let current = RailRow::current(view.phase, view.step);
    for phase in baylee_client_core::automation::RAIL_PHASES {
        let group = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(1),
                    ..default()
                },
                BackgroundColor(palette::SEAT_TRACK),
                Pickable::IGNORE,
            ))
            .id();
        for row in phase.rows().iter().copied() {
            let tile = spawn_tile(
                commands,
                fonts,
                Density::Compact,
                TRACK_W,
                TileState {
                    side,
                    row,
                    skipped: orders
                        .rows_for(side)
                        .any(|(r, skipped)| r == row && skipped),
                    live: row.grants_priority(),
                    now: row == current,
                    gold: view.active == seat.player,
                    selected: orders.selected() == Some((side, row)),
                    lost: seat.has_lost,
                },
            );
            commands.entity(group).add_child(tile);
        }
        commands.entity(track).add_child(group);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A band as [`crate::table::Lens`] hands it over: the two corners on the
    /// rim first, then the two that meet the lane behind it. `flip` is the
    /// seat across the table, whose own left is this screen's right.
    fn band(length: f32, depth: f32, slope: f32, flip: bool) -> [Vec2; 4] {
        let mid = Vec2::new(1500.0, 620.0);
        let along = Vec2::new(1.0, slope).normalize() * (length * 0.5);
        let across = Vec2::new(-along.y, along.x).normalize() * (depth * 0.5);
        let (near, far) = (mid - across, mid + across);
        if flip {
            [near + along, near - along, far - along, far + along]
        } else {
            [near - along, near + along, far + along, far - along]
        }
    }

    /// The four corners a panel is actually drawn at, in the order the box
    /// has them: the scale turns about the box's own middle, so the middle is
    /// where [`pose_on`] put it and only the extent shrinks.
    fn drawn(corners: [Vec2; 4], panel: Panel) -> [Vec2; 4] {
        let (corner, tilt, scale) = pose_on(corners, panel);
        let middle = corner + panel.size() * 0.5;
        let half = panel.size() * scale * 0.5;
        let turn = Rot2::radians(tilt);
        [
            middle + turn * Vec2::new(-half.x, -half.y),
            middle + turn * Vec2::new(half.x, -half.y),
            middle + turn * Vec2::new(half.x, half.y),
            middle + turn * Vec2::new(-half.x, half.y),
        ]
    }

    /// The three never meet, at either end of the range of bands a table
    /// projects — and they keep their order on the *screen*, not in the
    /// seat's own frame, so a seat across the table has its name where every
    /// other seat's name is.
    ///
    /// The short band is the injected finding rather than decoration: at full
    /// length the three panels take less than half the ledge and any
    /// arrangement at all would pass, so the case that can fail is the one
    /// where the scale is doing the work. `BAND_GAP * 4` is what the scale
    /// divides by, so a band at exactly the sum of the three panels leaves
    /// them touching-but-clear, and the assertion below is what says so.
    #[test]
    fn the_three_panels_share_one_band_and_never_meet() {
        let want = HEADER_W * 2.0 + TRACK_H + BAND_GAP * 4.0;
        for length in [want, want * 2.0, 2160.0] {
            for slope in [0.0_f32, 0.06, -0.06] {
                for flip in [false, true] {
                    let corners = band(length, 110.0, slope, flip);
                    let boxes = [Panel::Identity, Panel::Phases, Panel::Counts]
                        .map(|panel| drawn(corners, panel));
                    // The band's own direction as the screen reads it, which is
                    // the axis `pose_on` sorts the two ends onto. Taken from
                    // `corners` it runs the other way for a seat across the
                    // table, and the order below would then read as reversed
                    // rather than as wrong.
                    let along = Vec2::new(1.0, slope).normalize();
                    let at = |b: [Vec2; 4], pick: fn(f32, f32) -> f32| {
                        b.iter().map(|c| c.dot(along)).fold(f32::NAN, pick)
                    };
                    for pair in boxes.windows(2) {
                        let gap = at(pair[1], f32::min) - at(pair[0], f32::max);
                        assert!(
                            gap >= -0.01,
                            "{length} long, slope {slope}, flipped {flip}: two \
                             panels overlap by {}",
                            -gap
                        );
                    }
                    // And in the reader's order, whichever way the seat's own
                    // frame runs: the name is always at the left of the band.
                    assert!(
                        boxes[0][0].x < boxes[2][0].x,
                        "{length} long, slope {slope}, flipped {flip}: the \
                         identity panel is not the leftmost"
                    );
                }
            }
        }
    }

    /// Nothing is written outside the strip the model keeps clear of cards.
    ///
    /// Measured across the band rather than in `y`, because a flank's band
    /// slopes and a box that stayed inside the window could still be standing
    /// on the creature lane behind it.
    #[test]
    fn no_panel_stands_off_its_own_band() {
        for depth in [110.0_f32, 60.0, 26.0] {
            for slope in [0.0_f32, 0.06, -0.06] {
                for flip in [false, true] {
                    let corners = band(2160.0, depth, slope, flip);
                    let along = Vec2::new(1.0, slope).normalize();
                    let across = Vec2::new(-along.y, along.x);
                    let mid = corners
                        .iter()
                        .fold(Vec2::ZERO, |sum, c| sum + *c / corners.len() as f32);
                    for panel in [Panel::Identity, Panel::Phases, Panel::Counts] {
                        for corner in drawn(corners, panel) {
                            let off = (corner - mid).dot(across).abs();
                            assert!(
                                off <= depth * 0.5 + 0.01,
                                "{depth} deep, slope {slope}, flipped {flip}: a \
                                 {panel:?} corner is {off} off the band's middle, \
                                 past its {} of room",
                                depth * 0.5
                            );
                        }
                    }
                }
            }
        }
    }

    /// The ink reads in the viewer's order at every seat: a band running
    /// right-to-left in its owner's frame is still written the right way up.
    #[test]
    fn a_seat_across_the_table_has_its_bar_upright() {
        for slope in [0.0_f32, 0.3, -0.3] {
            for flip in [false, true] {
                let (_, tilt, _) = pose_on(band(2160.0, 110.0, slope, flip), Panel::Identity);
                assert!(
                    tilt.abs() <= std::f32::consts::FRAC_PI_2,
                    "slope {slope}, flipped {flip}: the bar is turned {tilt} and \
                     would be read upside-down"
                );
            }
        }
    }
}
