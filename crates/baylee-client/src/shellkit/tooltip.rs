//! Measured rich details stay within the viewport, including near its lower edge.
use super::controls::{Disabled, FloatingTooltip, FocusTooltip, Tooltip};
use super::px_fixed;
use bevy::picking::hover::PickingInteraction;
use bevy::prelude::*;

pub(super) fn show_detail(
    mut shown: Mut<'_, Visibility>,
    mut node: Mut<'_, Node>,
    computed: Option<&ComputedNode>,
    owner: Rect,
    canvas: Vec2,
) {
    let Some(measured) = computed
        .map(|node| node.size() * node.inverse_scale_factor())
        .filter(|size| size.cmpgt(Vec2::ZERO).all())
    else {
        if node.display != Display::Flex {
            node.display = Display::Flex;
        }
        shown.set_if_neq(Visibility::Hidden);
        return;
    };
    let at = detail_anchor(owner, measured, canvas);
    if node.display != Display::Flex
        || node.left != px_fixed(at.x)
        || node.top != px_fixed(at.y)
        || node.right != Val::Auto
        || node.bottom != Val::Auto
    {
        node.display = Display::Flex;
        node.left = px_fixed(at.x);
        node.top = px_fixed(at.y);
        node.right = Val::Auto;
        node.bottom = Val::Auto;
    }
    shown.set_if_neq(Visibility::Visible);
}

/// Reveals newly measured [`FocusTooltip`] overlays in `PostUpdate` right after
/// `UiSystems::Layout`, shifting their [`UiGlobalTransform`] subtree to the
/// exact measured anchor so Frame 0 neither delays by a frame nor jumps from a
/// guessed size on Frame 1.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn place_detail_tooltips(
    owners: Query<
        (
            Entity,
            &Tooltip,
            Option<&PickingInteraction>,
            Has<Disabled>,
            &ComputedNode,
            &UiGlobalTransform,
        ),
        With<FocusTooltip>,
    >,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    owner_vis: Query<&Visibility, Without<FloatingTooltip>>,
    owner_nodes: Query<&Node, Without<FloatingTooltip>>,
    hovers: Option<Res<bevy::picking::hover::HoverMap>>,
    focus: Option<Res<bevy::input_focus::InputFocus>>,
    focus_visible: Option<Res<bevy::input_focus::InputFocusVisible>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    ui: Option<Res<UiScale>>,
    mut tips: Query<(&mut Visibility, &mut Node, &ComputedNode), With<FloatingTooltip>>,
    mut globals: Query<&mut UiGlobalTransform, Without<FocusTooltip>>,
) {
    let focused = focus
        .as_deref()
        .and_then(bevy::input_focus::InputFocus::get);
    let canvas = windows.single().map_or(Vec2::new(1280.0, 800.0), |w| {
        crate::hud::scale::space(w, ui.as_deref())
    });
    for (owner, tip, interaction, is_disabled, owner_c, owner_t) in &owners {
        let Ok((mut shown, mut node, tip_c)) = tips.get_mut(tip.0) else {
            continue;
        };
        if node.display != Display::Flex || *shown != Visibility::Hidden {
            continue;
        }
        let by_pick = interaction.is_some_and(|i| *i != PickingInteraction::None);
        let by_hover =
            !by_pick && super::controls::hovered_has_ancestor(owner, hovers.as_deref(), &parents);
        let keyboard_tip = focus_visible.as_ref().is_none_or(|visible| visible.0);
        let by_focus = (is_disabled || keyboard_tip) && focused == Some(owner);
        if !(by_pick || by_hover || by_focus)
            || !super::controls::tooltip_owner_visible(owner, &parents, &owner_vis, &owner_nodes)
        {
            continue;
        }
        let owner_inv = owner_c.inverse_scale_factor();
        let owner_size = owner_c.size() * owner_inv;
        let tip_inv = tip_c.inverse_scale_factor();
        let measured = tip_c.size() * tip_inv;
        if owner_size.x <= 0.0
            || owner_size.y <= 0.0
            || !measured.cmpgt(Vec2::ZERO).all()
            || tip_inv <= 0.0
        {
            continue;
        }
        let owner_rect = Rect::from_center_size(owner_t.translation * owner_inv, owner_size);
        let at = detail_anchor(owner_rect, measured, canvas);
        node.left = px_fixed(at.x);
        node.top = px_fixed(at.y);
        node.right = Val::Auto;
        node.bottom = Val::Auto;
        if let Ok(cur_at) = globals.get(tip.0) {
            let cur_min = cur_at.translation * tip_inv - measured * 0.5;
            let delta = (at - cur_min) / tip_inv;
            for target in std::iter::once(tip.0).chain(children.iter_descendants(tip.0)) {
                if let Ok(mut t) = globals.get_mut(target) {
                    *t = UiGlobalTransform::from(bevy::math::Affine2 {
                        matrix2: t.matrix2,
                        translation: t.translation + delta,
                    });
                }
            }
        }
        *shown = Visibility::Visible;
    }
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

    #[test]
    fn first_focus_frame_places_detail_at_exact_measured_anchor_without_fallback_jump() {
        use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};

        let (mut app, _fonts) = crate::shellkit::header::tests::layout_app();
        app.init_resource::<InputFocus>()
            .insert_resource(InputFocusVisible(true));
        super::super::controls::install(&mut app);

        let tip = app
            .world_mut()
            .spawn((
                FloatingTooltip,
                Node {
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    width: px_fixed(380.0),
                    height: px_fixed(180.0),
                    ..default()
                },
                Visibility::Hidden,
            ))
            .id();
        let owner = app
            .world_mut()
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px_fixed(1140.0),
                    top: px_fixed(620.0),
                    width: px_fixed(80.0),
                    height: px_fixed(80.0),
                    ..default()
                },
                Tooltip(tip),
                FocusTooltip,
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(tip),
            Some(&Visibility::Hidden)
        );

        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(owner, FocusCause::Navigated);
        app.update();

        // On the very first frame of focus, the detail overlay must already be
        // visible AND positioned at its exact 380×180 anchor (never at a
        // 300×400 fallback guess).
        assert_eq!(
            app.world().get::<Visibility>(tip),
            Some(&Visibility::Visible)
        );
        let first_center = app
            .world()
            .get::<UiGlobalTransform>(tip)
            .expect("global transform")
            .translation;
        let expected_top_left = detail_anchor(
            Rect::from_center_size(Vec2::new(1180.0, 660.0), Vec2::splat(80.0)),
            Vec2::new(380.0, 180.0),
            Vec2::new(1280.0, 800.0),
        );
        let expected_center = expected_top_left + Vec2::new(190.0, 90.0);
        assert!(
            (first_center - expected_center).length() < 0.5,
            "frame-0 center {first_center:?} must match final center {expected_center:?}"
        );

        app.update();
        let second_center = app
            .world()
            .get::<UiGlobalTransform>(tip)
            .expect("global transform")
            .translation;
        assert!(
            (second_center - first_center).length() < 0.01,
            "zero jump between Frame 0 ({first_center:?}) and Frame 1 ({second_center:?})"
        );
    }
}
