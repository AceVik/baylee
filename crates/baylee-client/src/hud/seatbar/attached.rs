//! Three independent pieces of ink anchored just outside the battlefield rim.

#[allow(clippy::wildcard_imports)]
use super::*;

const HEADER_H: f32 = 30.0;
const HEADER_W: f32 = 300.0;
const TRACK_W: f32 = 24.0;
const TRACK_H: f32 = 340.0;

/// Which outside edge carries this piece of the seat's information.
#[derive(Component, Clone, Copy)]
pub enum Panel {
    /// Name, life, priority and turn on the upper-left edge.
    Identity,
    /// Public zone counts on the upper-right edge.
    Counts,
    /// The twelve steps beside the command-zone side of the mat.
    Phases,
}

impl Panel {
    fn size(self) -> Vec2 {
        match self {
            Self::Identity | Self::Counts => Vec2::new(HEADER_W, HEADER_H),
            Self::Phases => Vec2::new(TRACK_W, TRACK_H),
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
        let forward = slot.forward();
        let side = Vec2::new(slot.facing.cos(), -slot.facing.sin());
        let margin = baylee_client_core::tabletop::MAT_MARGIN;
        let center = lens.project(slot.center)?;
        // Choose the long edge above the battlefield in the current view,
        // not the historical local-seat shelf behind the lands.
        let a = slot.center + forward * (slot.half_extent.y + margin);
        let b = slot.center - forward * (slot.half_extent.y + margin);
        let top = if lens.project(a)?.y < lens.project(b)?.y {
            a
        } else {
            b
        };
        let edge_a = lens.project(top - side * (slot.half_extent.x + margin))?;
        let edge_b = lens.project(top + side * (slot.half_extent.x + margin))?;
        let (left, right) = if edge_a.x <= edge_b.x {
            (edge_a, edge_b)
        } else {
            (edge_b, edge_a)
        };
        let axis = (right - left).normalize_or_zero();
        let mut tilt = axis.y.atan2(axis.x);
        let normal = Vec2::new(axis.y, -axis.x);
        let width = left.distance(right);
        let size = panel.size();
        let (middle, scale) = match panel {
            Panel::Identity | Panel::Counts => {
                let scale = (width / (HEADER_W * 2.0 + 32.0)).min(1.0);
                let inset = axis * (HEADER_W * scale * 0.5);
                let end = if matches!(panel, Panel::Identity) {
                    left + inset
                } else {
                    right - inset
                };
                (end + normal * (HEADER_H * scale * 0.5 + 2.0), scale)
            }
            Panel::Phases => {
                let edge = slot.center - side * (slot.half_extent.x + margin);
                let middle = lens.project(edge)?;
                let near = lens.project(edge - forward * slot.half_extent.y)?;
                let far = lens.project(edge + forward * slot.half_extent.y)?;
                // Fit between the rim and commander cards, without moving either.
                let gutter = lens.project(edge - side * 0.44)?.distance(middle);
                let scale = (near.distance(far) / TRACK_H)
                    .min(gutter / TRACK_W)
                    .min(1.0);
                let (position, rotation) = phase_pose(near, far, center, scale);
                tilt = rotation;
                (position, scale)
            }
        };
        Some((middle - size * 0.5, tilt, scale))
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

/// Keep the whole column outside the projected side, including its ends.
fn phase_pose(near: Vec2, far: Vec2, center: Vec2, scale: f32) -> (Vec2, f32) {
    let middle = near.midpoint(far);
    let mut axis = (far - near).normalize_or_zero();
    if axis.y < 0.0 {
        axis = -axis;
    }
    let mut outward = Vec2::new(axis.y, -axis.x);
    if outward.dot(middle - center) < 0.0 {
        outward = -outward;
    }
    (
        middle + outward * (TRACK_W * scale * 0.5 + 2.0),
        axis.y.atan2(axis.x) - std::f32::consts::FRAC_PI_2,
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
                flex_direction: if matches!(panel, Panel::Phases) {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
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
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(1),
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

    #[test]
    fn phase_column_clears_both_ends_of_a_sloping_rim() {
        for (near, far) in [
            (Vec2::new(180.0, 600.0), Vec2::new(215.0, 360.0)),
            (Vec2::new(1200.0, 90.0), Vec2::new(1225.0, 290.0)),
        ] {
            let center = Vec2::new(720.0, near.midpoint(far).y);
            let scale = 0.6;
            let (position, tilt) = phase_pose(near, far, center, scale);
            let along = (far - near).normalize();
            let mut outward = Vec2::new(along.y, -along.x);
            if outward.dot(near - center) < 0.0 {
                outward = -outward;
            }
            for x in [-1.0, 1.0] {
                for y in [-1.0, 1.0] {
                    let corner = position
                        + Rot2::radians(tilt) * Vec2::new(x * TRACK_W, y * TRACK_H) * (scale * 0.5);
                    let clearance = (corner - near).dot(outward);
                    assert!(clearance >= 1.99 && clearance <= TRACK_W * scale + 2.01);
                }
            }
        }
    }
}
