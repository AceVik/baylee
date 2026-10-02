//! Placement of permanent annotations outside prints, previews and other labels.
use glam::Vec2;

/// A rectangle in logical screen pixels.
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    /// Upper-left corner.
    pub min: Vec2,
    /// Lower-right corner.
    pub max: Vec2,
}

impl Bounds {
    fn overlaps(self, other: Self) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }
}

/// Find a nearby free label position, keeping its entire box on screen.
///
/// `occupied` includes card prints, visible previews and labels already placed
/// this frame. Prefer the card's right or left side; otherwise use the closest
/// free position above/below an obstacle, aligned with the source card. Return
/// `None` when no such space exists instead of covering a printing or clipping.
#[must_use]
pub fn anchor(card: Bounds, size: Vec2, window: Vec2, occupied: &[Bounds]) -> Option<Vec2> {
    let margin = Vec2::splat(4.0);
    let limit = window - size - margin;
    if !size.is_finite() || !window.is_finite() || limit.cmplt(margin).any() {
        return None;
    }
    let center = (card.min + card.max) * 0.5;
    let clear = |at: Vec2| {
        at.cmpge(margin).all()
            && at.cmple(limit).all()
            && occupied.iter().all(|r| {
                !r.overlaps(Bounds {
                    min: at,
                    max: at + size,
                })
            })
    };
    for x in [card.max.x + 4.0, card.min.x - size.x - 4.0] {
        let at = Vec2::new(x, center.y - size.y * 0.5);
        if clear(at) {
            return Some(at);
        }
    }
    let xs = [center.x - size.x * 0.5, card.min.x, card.max.x - size.x]
        .map(|x| x.clamp(margin.x, limit.x));
    // A consistent side matters more than subpixel distance differences:
    // equivalent labels should not alternate above/below a row of cards.
    for y in [card.max.y + 4.0, card.min.y - size.y - 4.0] {
        for x in xs {
            let at = Vec2::new(x, y);
            if clear(at) {
                return Some(at);
            }
        }
    }
    let ys = [card.max.y + 4.0, card.min.y - size.y - 4.0]
        .into_iter()
        .chain(
            occupied
                .iter()
                .flat_map(|r| [r.max.y + 4.0, r.min.y - size.y - 4.0]),
        );
    ys.flat_map(|y| xs.map(|x| Vec2::new(x, y)))
        .filter(|at| clear(*at))
        .min_by(|a, b| {
            (*a + size * 0.5)
                .distance_squared(center)
                .total_cmp(&(*b + size * 0.5).distance_squared(center))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds {
            min: Vec2::new(x, y),
            max: Vec2::new(x + w, y + h),
        }
    }

    #[test]
    fn labels_fit_at_every_window_edge_and_avoid_each_other() {
        let window = Vec2::new(640.0, 480.0);
        let size = Vec2::new(120.0, 28.0);
        for (x, y) in [(4.0, 4.0), (590.0, 4.0), (4.0, 416.0), (590.0, 416.0)] {
            let card = rect(x, y, 46.0, 60.0);
            let mut occupied = vec![card];
            for _ in 0..3 {
                let at = anchor(card, size, window, &occupied).unwrap();
                let label = Bounds {
                    min: at,
                    max: at + size,
                };
                assert!(at.cmpge(Vec2::splat(4.0)).all());
                assert!(label.max.cmple(window - Vec2::splat(4.0)).all());
                assert!(occupied.iter().all(|r| !label.overlaps(*r)));
                occupied.push(label);
            }
        }
    }

    #[test]
    fn expanded_counter_text_avoids_the_preview_and_adjacent_prints() {
        let card = rect(570.0, 490.0, 40.0, 60.0);
        let preview = rect(170.0, 60.0, 380.0, 570.0);
        let occupied = [card, preview, rect(620.0, 490.0, 40.0, 60.0)];
        let size = Vec2::new(180.0, 28.0);
        let at = anchor(card, size, Vec2::new(1280.0, 800.0), &occupied).unwrap();
        let label = Bounds {
            min: at,
            max: at + size,
        };
        assert!(occupied.iter().all(|r| !label.overlaps(*r)));
    }

    #[test]
    fn adjacent_labels_prefer_below_despite_subpixel_card_positions() {
        for y in [123.0, 123.123, 123.987] {
            let card = rect(100.0, y, 40.0, 60.0);
            let occupied = [card, rect(50.0, y, 40.0, 60.0), rect(150.0, y, 40.0, 60.0)];
            let at = anchor(card, Vec2::new(70.0, 28.0), Vec2::splat(500.0), &occupied).unwrap();
            assert!(at.y > card.max.y);
        }
    }

    #[test]
    fn no_free_space_does_not_cover_a_card_or_escape_the_window() {
        let card = rect(0.0, 0.0, 100.0, 100.0);
        assert!(anchor(card, Vec2::splat(20.0), Vec2::splat(100.0), &[card]).is_none());
        assert!(anchor(card, Vec2::splat(200.0), Vec2::splat(100.0), &[]).is_none());
    }
}
