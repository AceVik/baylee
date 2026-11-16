//! Measured rich details stay within the viewport, including near its lower edge.
use super::px_fixed;
use bevy::prelude::*;

pub(super) fn show_detail(
    mut shown: Mut<'_, Visibility>,
    mut node: Mut<'_, Node>,
    computed: Option<&ComputedNode>,
    owner: Rect,
    canvas: Vec2,
) {
    let measured = computed
        .map(|node| node.size() * node.inverse_scale_factor())
        .filter(|size| size.cmpgt(Vec2::ZERO).all())
        .unwrap_or(Vec2::new(300.0, 400.0));
    let at = detail_anchor(owner, measured, canvas);
    if node.display != Display::Flex || node.left != px_fixed(at.x) || node.top != px_fixed(at.y) {
        node.display = Display::Flex;
        node.left = px_fixed(at.x);
        node.top = px_fixed(at.y);
        node.right = Val::Auto;
        node.bottom = Val::Auto;
    }
    shown.set_if_neq(Visibility::Visible);
}

pub(super) fn detail_anchor(owner: Rect, detail: Vec2, canvas: Vec2) -> Vec2 {
    let gap = 8.0;
    let inset = 10.0;
    let below = canvas.y - owner.max.y - gap - inset;
    let above = owner.min.y - gap - inset;
    let y = if below >= detail.y || below >= above {
        owner.max.y + gap
    } else {
        owner.min.y - gap - detail.y
    };
    let x = if owner.center().x > canvas.x * 0.65 {
        owner.max.x - detail.x
    } else {
        owner.min.x
    };
    Vec2::new(
        x.clamp(inset, (canvas.x - detail.x - inset).max(inset)),
        y.clamp(inset, (canvas.y - detail.y - inset).max(inset)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn long_item_details_fit_at_all_four_edges_and_between_shelf_rows() {
        let canvas = Vec2::new(1280.0, 720.0);
        let detail = Vec2::new(396.0, 480.0);
        for center in [
            Vec2::new(45.0, 45.0),
            Vec2::new(1235.0, 45.0),
            Vec2::new(45.0, 675.0),
            Vec2::new(1235.0, 675.0),
            Vec2::new(750.0, 360.0),
        ] {
            let owner = Rect::from_center_size(center, Vec2::splat(80.0));
            let at = detail_anchor(owner, detail, canvas);
            assert!(at.cmpge(Vec2::splat(10.0)).all());
            assert!((at + detail).cmple(canvas - Vec2::splat(10.0)).all());
        }
    }
}
