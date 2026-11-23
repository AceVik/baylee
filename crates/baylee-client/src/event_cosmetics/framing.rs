//! Keep the 1v1 artwork upright in each owner's local battlefield plane.
//! The name shelf is outside that field and the camera may tilt the plane.
use bevy::prelude::*;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
use baylee_account::advent::framing::{EDGE, MatFraming};

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Chair {
    #[default]
    Vacant,
    Alone,
    Team(u8),
}

/// Stable match roster, including seats parked by the current camera arrangement.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Default, PartialEq, Eq)]
pub(super) struct Roster([Chair; 8]);

#[cfg(not(target_arch = "wasm32"))]
impl Roster {
    pub(super) fn new(
        seats: impl Iterator<Item = (baylee_core::ids::PlayerId, Option<u8>)>,
    ) -> Self {
        let mut roster = Self::default();
        for (player, team) in seats {
            if let Some(chair) = roster.0.get_mut(usize::from(player.get())) {
                *chair = team.map_or(Chair::Alone, Chair::Team);
            }
        }
        roster
    }

    pub(super) fn framing(&self, owner: u8, guide: Option<&MatFraming>) -> Framing {
        let Some(guide) = guide.filter(|g| g.validate().is_ok()) else {
            return Framing::default();
        };
        let chair = self.0.get(usize::from(owner));
        let mut seats = 0;
        let mut allies = 1;
        let mut sides = 0;
        for (i, team) in self.0.iter().enumerate() {
            if *team != Chair::Vacant {
                seats += 1;
                if *team == Chair::Alone || !self.0[..i].contains(team) {
                    sides += 1;
                }
                if matches!(team, Chair::Team(_)) && Some(team) == chair && i != usize::from(owner)
                {
                    allies += 1;
                }
            }
        }
        let frame = if matches!(chair, None | Some(Chair::Vacant)) {
            guide.duel
        } else {
            guide.for_layout(seats, allies, sides)
        };
        let point = |[x, y]: [u16; 2]| Vec2::new(f32::from(x), f32::from(y)) / f32::from(EDGE);
        let focus = point(frame.focus);
        let protected = frame
            .safe
            .map_or(Rect::from_corners(focus, focus), |[l, t, r, b]| {
                Rect::from_corners(point([l, t]), point([r, b]))
            });
        Framing { focus, protected }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Framing {
    pub(super) focus: Vec2,
    pub(super) protected: Rect,
}

impl Default for Framing {
    fn default() -> Self {
        Self {
            focus: Vec2::splat(0.5),
            protected: Rect::from_corners(Vec2::ZERO, Vec2::ONE),
        }
    }
}

impl Framing {
    /// UV at the field's top left and its UV span. UVs outside the image reveal
    /// the resident felt. Prefer filling the field, up to the magnification at
    /// which a protected edge would fall outside its pane.
    pub(super) fn viewport(self, quad: Vec2, pixels: Vec2, corner: f32) -> Vec4 {
        let field = quad * Vec2::new(1.0, 1.0 - baylee_client_core::tabletop::LEDGE_FRAC);
        let ratio = field.x * pixels.y / (field.y * pixels.x);
        if !ratio.is_finite() || ratio <= 0.0 {
            return Vec4::new(0.0, 0.0, 1.0, 1.0);
        }
        let axis = Vec2::new(ratio, 1.0);
        // The top-left corner of this inset lies at 45° on the cut corner.
        // A protected image corner there survives the shader's rounded mask.
        let radius = corner.min(field.min_element() * 0.5);
        let inset = Vec2::splat(radius * (1.0 - std::f32::consts::FRAC_1_SQRT_2)) / field;
        let scale = (self.protected.size() / (axis * (Vec2::ONE - 2.0 * inset)))
            .max_element()
            .max(1.0 / axis.max_element());
        let span = axis * scale;
        let edge = Vec2::ONE - span;
        let preferred = (self.focus - span * 0.5).clamp(edge.min(Vec2::ZERO), edge.max(Vec2::ZERO));
        let low = self.protected.max - span * (Vec2::ONE - inset);
        // Handle the last ulp when the box exactly fills an axis.
        let high = (self.protected.min - span * inset).max(low);
        let origin = preferred.clamp(low, high);
        Vec4::new(origin.x, origin.y, span.x, span.y)
    }
}
