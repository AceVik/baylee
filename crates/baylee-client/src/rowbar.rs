//! A scrolled battlefield row's scrollbar, and the felt that takes its wheel.
//!
//! A row whose cards do not fit its lane, with every merged card's cell held
//! whole so its count badge lies on no neighbour (the owner, 25.09), shows a
//! run of whole cards and scrolls (`LanePacking::window`). What it does not
//! show is not drawn, so the bar is what says it is there: a track the
//! lane's width with a thumb the share of the row that is shown, standing
//! where in the row the shown run is. In a duel it is a hairline in the seam
//! under the row; round a ring table, where the rows are 0.0185 apart, the
//! three rows' bars stand one behind another in the mat's outer margin.
//!
//! The row's felt takes the wheel: the table's cloth answers no pointer, so
//! a scrolled row lays an invisible pad over its band, under its cards, and
//! the bar has its own invisible grab strip. `hud::scrolls` turns a wheel
//! over either into a card's worth of scrolling (`RowScroll::wheel`), and a
//! sideways wheel over one of the row's cards too; an upright wheel over a
//! card stays its preview's (#259).

use crate::Duel;
use crate::table::{DuelStage, FLOOR_RUNG};
use baylee_client_core::layout::{LaneKind, SeatSlot, TableLayout};
use baylee_client_core::rowscroll::{RowKey, packing_of};
use bevy::prelude::*;
use std::collections::HashMap;

/// How thick the bar is where the rows leave it a seam: a hairline, the
/// seam between two rows of a ring or a crowded duel being 0.0185.
pub const SEAM_BAR: f32 = 0.010;
/// How thick a bar in the mat's outer margin is.
pub const MARGIN_BAR: f32 = 0.03;
/// Where in the outer margin the first row's bar stands, past the playing
/// area's back edge, and how far behind it each later row's.
pub const MARGIN_FIRST: f32 = 0.12;
/// See [`MARGIN_FIRST`].
pub const MARGIN_STEP: f32 = 0.09;
/// How thick the invisible strip round a bar that takes the pointer is.
pub const GRAB: f32 = 0.08;
/// The shortest a thumb gets, so a long row's is still a thing to see.
pub const THUMB_MIN: f32 = 0.25;
// Every row's bar inside the margin.
const _: () = assert!(
    MARGIN_FIRST + MARGIN_STEP * 2.0 + GRAB * 0.5 <= baylee_client_core::tabletop::MAT_MARGIN
);

/// A scrolled row's part: the pad over its felt, the bar's grab strip, its
/// track or its thumb. The pad and the strip take the wheel for `row`.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct RowBar {
    /// Whose row, and which.
    pub row: RowKey,
    /// Which part of it this is.
    pub part: Part,
}

/// The four entities a scrolled row lays on the felt.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Part {
    /// Invisible, over the row's band, under its cards.
    Pad,
    /// Invisible, round the bar.
    Grab,
    /// The bar's whole length.
    Track,
    /// The shown share of the row.
    Thumb,
}

impl Part {
    const ALL: [Self; 4] = [Self::Pad, Self::Grab, Self::Track, Self::Thumb];

    /// How high over the felt it lies: under every card and under the
    /// offer's light, the thumb over its track.
    fn rung(self) -> f32 {
        match self {
            Self::Pad | Self::Grab => FLOOR_RUNG * 0.4,
            Self::Track => FLOOR_RUNG * 0.6,
            Self::Thumb => FLOOR_RUNG * 0.65,
        }
    }
}

/// Where one part lies: its centre in table space, its length along the row
/// and its depth across it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Laid {
    /// Centre, table space.
    pub at: Vec2,
    /// Along the row.
    pub length: f32,
    /// Across it.
    pub depth: f32,
}

/// Where a scrolled row's four parts lie, given the first card shown of
/// `count`, `shown` of them from `first`, `last_first` the furthest the row
/// scrolls.
#[must_use]
pub fn lay(
    layout: &TableLayout,
    slot: &SeatSlot,
    lane: LaneKind,
    shown: std::ops::Range<usize>,
    count: usize,
    last_first: usize,
) -> [Laid; 4] {
    let along = Vec2::new(slot.facing.cos(), -slot.facing.sin());
    let back = -slot.forward();
    let length = slot.lane_width();
    let half = slot.lane_height() * 0.5;
    let pad = Laid {
        at: slot.lane_center(lane),
        length,
        depth: half * 2.0,
    };
    // A duel's bar in the seam under its own row; a ring's in the margin.
    let (centre, thick) = if layout.slots.len() <= 2 {
        (slot.lane_center(lane) + back * half, SEAM_BAR)
    } else {
        let index = LaneKind::ALL.iter().position(|&l| l == lane).unwrap_or(0) as f32;
        let edge = slot.lane_center(LaneKind::Lands) + back * half;
        (
            edge + back * (MARGIN_FIRST + MARGIN_STEP * index),
            MARGIN_BAR,
        )
    };
    #[allow(clippy::cast_precision_loss)] // cards on a row
    let share = shown.len() as f32 / count.max(1) as f32;
    let thumb = (length * share).max(THUMB_MIN).min(length);
    #[allow(clippy::cast_precision_loss)]
    let travel = if last_first == 0 {
        0.0
    } else {
        shown.start as f32 / last_first as f32
    };
    let thumb_at = -length * 0.5 + thumb * 0.5 + (length - thumb) * travel;
    [
        pad,
        Laid {
            at: centre,
            length,
            depth: GRAB,
        },
        Laid {
            at: centre,
            length,
            depth: thick,
        },
        Laid {
            at: centre + along * thumb_at,
            length: thumb,
            depth: thick,
        },
    ]
}

/// The transform a part is drawn with: a unit quad on the felt, turned to
/// its seat.
fn transform_of(slot: &SeatSlot, part: Part, laid: Laid) -> Transform {
    Transform {
        translation: Vec3::new(laid.at.x, part.rung(), -laid.at.y),
        rotation: Quat::from_rotation_y(-slot.facing),
        scale: Vec3::new(laid.length, 1.0, laid.depth),
    }
}

/// Brings the card the card cursor walks onto into view, once per card.
pub fn follow_the_rows(mut duel: ResMut<Duel>) {
    let Duel {
        rows,
        board,
        layout,
        hovered,
        ..
    } = &mut *duel;
    if let (Some(board), Some(layout)) = (board.as_ref(), layout.as_ref()) {
        rows.follow(board, layout, *hovered);
    }
}

/// The meshes and materials every bar shares.
pub struct Looks {
    quad: Handle<Mesh>,
    clear: Handle<StandardMaterial>,
    track: Handle<StandardMaterial>,
    thumb: Handle<StandardMaterial>,
}

impl Looks {
    fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        let look = |color: Color| StandardMaterial {
            base_color: color,
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        };
        Self {
            quad: meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(0.5))),
            // Drawn and so picked, and adding nothing to what is under it.
            clear: materials.add(look(Color::NONE)),
            // The blue hour's bands (`ambience.rs`): a faint track and a
            // thumb in its light.
            track: materials.add(look(Color::srgba(0.28, 0.49, 0.9, 0.28))),
            thumb: materials.add(look(Color::srgba(0.62, 0.74, 0.98, 0.9))),
        }
    }

    fn material(&self, part: Part) -> Handle<StandardMaterial> {
        match part {
            Part::Pad | Part::Grab => self.clear.clone(),
            Part::Track => self.track.clone(),
            Part::Thumb => self.thumb.clone(),
        }
    }
}

/// Lays each scrolled row's pad and bar, and takes away those of a row that
/// fits again.
pub fn sync_row_bars(
    mut commands: Commands,
    duel: Res<Duel>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut looks: Local<Option<Looks>>,
    mut parts: Query<(Entity, &RowBar, &mut Transform)>,
) {
    let mut wanted: HashMap<(RowKey, Part), Transform> = HashMap::new();
    if let (Some(board), Some(layout)) = (duel.board.as_ref(), duel.layout.as_ref()) {
        for pod in &board.pods {
            let Some(slot) = layout.slot(pod.player) else {
                continue;
            };
            for lane in &pod.lanes {
                let row = (pod.player, lane.kind);
                let Some(packing) = packing_of(board, layout, row) else {
                    continue;
                };
                if !packing.overflowing {
                    continue;
                }
                let window = packing.window(duel.rows.first(row));
                let laid = lay(
                    layout,
                    slot,
                    lane.kind,
                    window.shown,
                    lane.groups.len(),
                    packing.last_first(),
                );
                for (part, laid) in Part::ALL.into_iter().zip(laid) {
                    wanted.insert((row, part), transform_of(slot, part, laid));
                }
            }
        }
    }
    for (entity, bar, mut transform) in &mut parts {
        match wanted.remove(&(bar.row, bar.part)) {
            Some(target) => {
                transform.set_if_neq(target);
            }
            None => commands.entity(entity).despawn(),
        }
    }
    if wanted.is_empty() {
        return;
    }
    let looks = looks.get_or_insert_with(|| Looks::new(&mut meshes, &mut materials));
    for ((row, part), transform) in wanted {
        let mut spawned = commands.spawn((
            DuelStage,
            RowBar { row, part },
            Mesh3d(looks.quad.clone()),
            MeshMaterial3d(looks.material(part)),
            transform,
        ));
        if matches!(part, Part::Track | Part::Thumb) {
            spawned.insert(Pickable::IGNORE);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
