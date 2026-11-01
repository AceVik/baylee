//! The out-of-table shell's tokens, size classes and components.
//!
//! The shell design (`.claude/ux-b6/DESIGN-v5.md` §2) gives every screen
//! outside the table one header, one body, one token set and one component
//! set. This module is that set; the screens move onto it package by package
//! (WP1–WP5), each converting its own module.
//!
//! The design names it `shellui/`. That path was taken: `crate::shellui` is
//! the hover preview's protection shells (`shell_ui.wesl`), so the shell's
//! kit lives here instead.
//!
//! - [`size`]: the five size classes (§2.7), the input class, the five text
//!   steps (§8).
//! - [`metrics`]: [`ShellMetrics`], every size a shell screen takes, scaled
//!   by the text step with floors; `m.px(n)` and [`px_fixed`].
//! - [`tokens`]: colours, spacing, radii and the `GlobalZIndex` bands (§2.2).
//! - [`role`]: what a node *is*, for the dev-control dump and the checks
//!   that read it (overflow, label budgets, contrast, the 44-px hit area).
//! - [`controls`], [`surfaces`], [`states`]: the components of §2.4.
//! - [`focus`]: the `TabOrder` tables, the Tab walker, the focus ring and
//!   the kit's own field editor (`KEYBOARD.md` §1).
//! - [`keys`]: the shell's second keymap wired to the keyboard — the
//!   resolver over the context stack, the text-size chords (§2, §7.1).
//! - [`overlay`]: the `?` overlay (§4.1).
//! - [`header`]: the header and its strips (§2.1).
//! - [`retain`]: a subtree kept across a rebuild, moved without a frame of
//!   it going undrawn.
//! - `gallery` (dev-control builds only): every component in every state,
//!   the screen WP0b-1 is accepted on.

pub mod controls;
pub mod focus;
#[cfg(any(test, all(feature = "dev-control", not(target_arch = "wasm32"))))]
pub mod gallery;
pub mod header;
#[cfg(test)]
mod keyboard_tests;
pub mod keys;
pub mod lint;
pub mod metrics;
pub mod overlay;
pub mod pointer;
pub mod retain;
pub mod role;
pub mod sheen;
pub mod size;
pub mod states;
pub mod surfaces;
pub mod tokens;

pub use metrics::{ShellMetrics, px_fixed};
pub use role::Role;
pub use size::{Frame, InputClass, Platform, TextSize, Viewport};

use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

/// Whether the kit holds the keyboard: the `?` overlay is up, the dev
/// gallery, or a kit field has focus. The lobby's own key handling stands
/// aside while it does, so a key typed into the overlay's search is not
/// also typed behind it; at a table, a sheet with a kit field (the table
/// design's amendment to KEYBOARD §6) is the case the table's keymap asks
/// this about, as it asks the report form's `holds_keyboard`. Like the
/// report form's, a key that closes the sheet is still the sheet's in the
/// frame it closed it: the field editor reads the frame's keys before the
/// sheet goes.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct KitHolds(pub bool);

/// The kit's systems: the input class, focus, the shell keymap, the `?`
/// overlay, and the face step the text size sets.
pub struct ShellKitPlugin;

impl Plugin for ShellKitPlugin {
    fn build(&self, app: &mut App) {
        crate::ui_button::install(app);
        app.add_message::<bevy::input::touch::TouchInput>()
            .add_message::<bevy::input::mouse::MouseMotion>()
            .add_message::<KeyboardInput>()
            .init_resource::<InputClass>()
            .init_resource::<KitHolds>()
            .add_systems(PreUpdate, size::follow_the_input)
            .add_systems(Update, (face_follows_the_text_size, hold_the_keyboard));
        controls::install(app);
        header::install(app);
        focus::install(app);
        keys::install(app);
        overlay::install(app);
        pointer::install(app);
        sheen::install(app);
        #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
        gallery::install(app);
    }
}

/// Adds [`ShellKitPlugin`] unless another plugin already did.
pub(crate) fn install(app: &mut App) {
    if !app.is_plugin_added::<ShellKitPlugin>() {
        app.add_plugins(ShellKitPlugin);
    }
}

/// Keeps [`KitHolds`] true while the overlay or the gallery is up.
fn hold_the_keyboard(
    overlay: Res<overlay::Overlay>,
    focus: Option<Res<bevy::input_focus::InputFocus>>,
    fields: Query<(), With<focus::ShellField>>,
    #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))] gallery: Option<
        Res<gallery::Gallery>,
    >,
    tour: Option<Res<crate::tour::TourDesk>>,
    chat: Option<Res<crate::social::ChatUi>>,
    mut holds: ResMut<KitHolds>,
) {
    #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
    let gallery = gallery.is_some_and(|g| g.open);
    #[cfg(not(all(feature = "dev-control", not(target_arch = "wasm32"))))]
    let gallery = false;
    let typing = focus
        .as_deref()
        .and_then(bevy::input_focus::InputFocus::get)
        .is_some_and(|f| fields.contains(f));
    // A narrated tour bubble holds the keys as the overlay does (TOURS.md
    // §1.3), so the lobby's handlers that ask this stand aside for it.
    let tour = tour.is_some_and(|t| t.holds_keyboard());
    // The chat's composer (and its other boxes) hold every key while they
    // have the caret (the beta.8 social design §12.1).
    let chat = chat.is_some_and(|c| c.holds_keyboard());
    let now = overlay::holds(&overlay) || gallery || typing || tour || chat;
    if holds.0 != now {
        holds.0 = now;
    }
}

/// The interface's card faces take the shell's text step (WP6's
/// `FaceMode::step`): the setting is the one source, written through when it
/// changes and once at start.
///
/// A dev-control build launched with `BAYLEE_TEXT_STEP` keeps the step that
/// variable names, so WP6's per-step photographs stay what they say.
fn face_follows_the_text_size(
    settings: Option<Res<crate::settings::ClientSettings>>,
    mode: Option<ResMut<crate::face::FaceMode>>,
) {
    let (Some(settings), Some(mut mode)) = (settings, mode) else {
        return;
    };
    if !settings.is_changed() || pinned_by_the_environment() {
        return;
    }
    let step = face_step(settings.text_size);
    if mode.step != step {
        mode.step = step;
    }
}

/// The face's step for a shell step: the same five, numbered alike.
#[must_use]
pub fn face_step(size: TextSize) -> baylee_client_core::textface::Step {
    baylee_client_core::textface::Step::new(size.step())
}

#[cfg(feature = "dev-control")]
fn pinned_by_the_environment() -> bool {
    std::env::var_os("BAYLEE_TEXT_STEP").is_some()
}

#[cfg(not(feature = "dev-control"))]
const fn pinned_by_the_environment() -> bool {
    false
}

/// The window-space rectangle a node's inherited clip leaves visible, in
/// physical pixels, the same space as its `UiGlobalTransform` and
/// `ComputedNode::size`.
///
/// Bevy 0.20 keeps a clip as one rectangle per clipping ancestor, each in
/// that ancestor's own space (so a rotated or scaled panel clips correctly);
/// 0.19 kept their intersection as one window-space `Rect`, and the
/// virtualised lists here ask exactly that: which band of the window is
/// visible. Each rectangle is taken back to window space and the axis-aligned
/// bounds are intersected — exact for this client's unrotated panels, a
/// superset for a rotated one. A fully clipped node gets an empty rectangle,
/// and a clip with no rectangle at all is no clip: `None`, as 0.19 had no
/// `CalculatedClip` there.
pub(crate) fn clip_bounds(clip: &bevy::ui::CalculatedClip) -> Option<Rect> {
    let Some(rects) = clip.rects() else {
        return Some(Rect::default());
    };
    let (first, rest) = rects.split_first()?;
    let mut visible = world_bounds(first);
    for clip_rect in rest {
        visible = visible.intersect(world_bounds(clip_rect));
    }
    Some(visible)
}

/// One clipping rectangle's axis-aligned bounds in window space.
fn world_bounds(clip_rect: &bevy::ui::CalculatedClipRect) -> Rect {
    let to_world = clip_rect.world_to_clip_local.inverse();
    let local = clip_rect.rect;
    // An overflow-visible axis is (-inf, +inf). Transforming its corners
    // normally computes 0 * inf on the other axis and poisons that axis
    // with NaN; vertical scrollers would then appear to have no height.
    // Project intervals instead, with a zero coefficient contributing zero.
    let project = |x: f32, y: f32, offset: f32| {
        let scaled = |coefficient: f32, min: f32, max: f32| {
            if coefficient == 0.0 {
                (0.0, 0.0)
            } else if coefficient > 0.0 {
                (coefficient * min, coefficient * max)
            } else {
                (coefficient * max, coefficient * min)
            }
        };
        let (x_min, x_max) = scaled(x, local.min.x, local.max.x);
        let (y_min, y_max) = scaled(y, local.min.y, local.max.y);
        (x_min + y_min + offset, x_max + y_max + offset)
    };
    let (left, right) = project(
        to_world.matrix2.x_axis.x,
        to_world.matrix2.y_axis.x,
        to_world.translation.x,
    );
    let (top, bottom) = project(
        to_world.matrix2.x_axis.y,
        to_world.matrix2.y_axis.y,
        to_world.translation.y,
    );
    Rect::new(left, top, right, bottom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Affine2;
    use bevy::ui::CalculatedClip;

    /// Slack for the float arithmetic of taking a rectangle through an
    /// inverse transform.
    const EPS: f32 = 1e-3;

    /// Both ends of two rectangles agree to within [`EPS`] — slack on both
    /// sides, so a rectangle too small fails as surely as one too big.
    fn assert_rect(got: Rect, want: Rect) {
        assert!(
            got.min.abs_diff_eq(want.min, EPS) && got.max.abs_diff_eq(want.max, EPS),
            "got {got:?}, wanted {want:?}"
        );
    }

    /// A clip made of the given rectangles, each with its transform from
    /// window space into the clipping node's own.
    fn clip_of(rects: &[(Rect, Affine2)]) -> CalculatedClip {
        let mut clip = CalculatedClip::default();
        for &(rect, world_to_clip_local) in rects {
            clip.push_rect(rect, world_to_clip_local);
        }
        clip
    }

    /// A node under no clipping ancestor has no visible band to speak of.
    ///
    /// The virtualised lists read `None` as "draw everything"; answering
    /// `Some(empty)` there would hide every row of an unclipped list, and
    /// answering a huge rectangle would make the answer depend on a magic
    /// number.
    #[test]
    fn a_node_with_no_clipping_ancestor_has_no_clip_bounds() {
        assert_eq!(clip_bounds(&CalculatedClip::default()), None);
    }

    /// A fully clipped node sees nothing: an empty rectangle, which a row
    /// test reads as "not visible", rather than `None`, which would read as
    /// "not clipped at all".
    #[test]
    fn a_fully_clipped_node_has_an_empty_clip_bounds() {
        let bounds = clip_bounds(&CalculatedClip::FullyClipped).expect("a rectangle");
        assert!(bounds.is_empty(), "{bounds:?}");
    }

    /// With the clipping node's space the window's, the clip is its
    /// rectangle, unchanged.
    #[test]
    fn a_clip_in_window_space_is_its_own_rectangle() {
        let rect = Rect::new(10.0, 20.0, 110.0, 70.0);
        let clip = clip_of(&[(rect, Affine2::IDENTITY)]);
        assert_rect(clip_bounds(&clip).expect("a rectangle"), rect);
    }

    /// The rectangle is in the clipping node's own space, and the transform
    /// goes from the window into it: an ancestor standing at (100, 50) has
    /// `world_to_clip_local` a translation by (-100, -50), and its local
    /// (0, 0)-(40, 20) is the window's (100, 50)-(140, 70). Taking the
    /// transform the wrong way round would put it at (-100, -50).
    #[test]
    fn a_clip_is_taken_back_from_the_ancestors_space_to_the_window() {
        let clip = clip_of(&[(
            Rect::new(0.0, 0.0, 40.0, 20.0),
            Affine2::from_translation(Vec2::new(-100.0, -50.0)),
        )]);
        assert_rect(
            clip_bounds(&clip).expect("a rectangle"),
            Rect::new(100.0, 50.0, 140.0, 70.0),
        );
    }

    /// `world_to_clip_local` scaling by 2 means local = 2 * window, so the
    /// window is half the local size: (0, 0)-(40, 20) is the window's
    /// (0, 0)-(20, 10). With a shift as well, local = 2 * window - (10, 6),
    /// so window = (local + (10, 6)) / 2. Multiplying by the transform
    /// instead of its inverse would double the rectangle.
    #[test]
    fn a_scaled_ancestors_clip_is_taken_back_through_the_inverse() {
        let clip = clip_of(&[(
            Rect::new(0.0, 0.0, 40.0, 20.0),
            Affine2::from_scale(Vec2::splat(2.0)),
        )]);
        assert_rect(
            clip_bounds(&clip).expect("a rectangle"),
            Rect::new(0.0, 0.0, 20.0, 10.0),
        );
        let clip = clip_of(&[(
            Rect::new(0.0, 0.0, 40.0, 20.0),
            Affine2::from_scale_angle_translation(Vec2::splat(2.0), 0.0, Vec2::new(-10.0, -6.0)),
        )]);
        assert_rect(
            clip_bounds(&clip).expect("a rectangle"),
            Rect::new(5.0, 3.0, 25.0, 13.0),
        );
    }

    /// Two clipping ancestors leave what both leave: the intersection of
    /// their window-space rectangles, each taken through its own transform.
    #[test]
    fn two_clips_leave_their_intersection() {
        let clip = clip_of(&[
            (Rect::new(0.0, 0.0, 100.0, 100.0), Affine2::IDENTITY),
            (
                // Stands at (60, 30): window (60, 30)-(160, 80).
                Rect::new(0.0, 0.0, 100.0, 50.0),
                Affine2::from_translation(Vec2::new(-60.0, -30.0)),
            ),
        ]);
        assert_rect(
            clip_bounds(&clip).expect("a rectangle"),
            Rect::new(60.0, 30.0, 100.0, 80.0),
        );
    }

    /// Two clips that do not meet leave nothing, however far apart.
    #[test]
    fn two_clips_that_do_not_meet_leave_nothing() {
        let clip = clip_of(&[
            (Rect::new(0.0, 0.0, 10.0, 10.0), Affine2::IDENTITY),
            (Rect::new(50.0, 50.0, 60.0, 60.0), Affine2::IDENTITY),
        ]);
        let bounds = clip_bounds(&clip).expect("a rectangle");
        assert!(bounds.is_empty(), "{bounds:?}");
    }

    /// Bevy uses an infinite interval on the visible axis of a scroller.
    /// Its finite axis must survive translation, scaling and reflection;
    /// multiplying ordinary corners instead produces 0 * infinity = NaN.
    #[test]
    fn one_axis_scroll_clips_keep_their_finite_band() {
        let vertical = Rect::new(-f32::INFINITY, -100.0, f32::INFINITY, 100.0);
        let to_world = Affine2::from_scale_angle_translation(
            Vec2::new(2.0, -3.0),
            0.0,
            Vec2::new(50.0, 400.0),
        );
        let bounds = clip_bounds(&clip_of(&[(vertical, to_world.inverse())])).unwrap();
        assert_eq!(bounds.min.x, -f32::INFINITY);
        assert_eq!(bounds.max.x, f32::INFINITY);
        assert!((bounds.min.y - 100.0).abs() < EPS, "{bounds:?}");
        assert!((bounds.max.y - 700.0).abs() < EPS, "{bounds:?}");

        let horizontal = Rect::new(-150.0, -f32::INFINITY, 150.0, f32::INFINITY);
        let viewport = Rect::new(10.0, 20.0, 800.0, 900.0);
        let combined = clip_of(&[
            (viewport, Affine2::IDENTITY),
            (vertical, to_world.inverse()),
            (
                horizontal,
                Affine2::from_translation(Vec2::new(-250.0, 0.0)),
            ),
        ]);
        assert_rect(
            clip_bounds(&combined).unwrap(),
            Rect::new(100.0, 100.0, 400.0, 700.0),
        );
    }

    /// A quarter turn keeps the clip a rectangle: window = R⁻¹ * local, and
    /// a 90 degree turn of (0, 0)-(40, 20) by -90 degrees is
    /// (0, -40)-(20, 0).
    #[test]
    fn a_quarter_turned_clip_is_the_turned_rectangle() {
        let clip = clip_of(&[(
            Rect::new(0.0, 0.0, 40.0, 20.0),
            Affine2::from_angle(std::f32::consts::FRAC_PI_2),
        )]);
        assert_rect(
            clip_bounds(&clip).expect("a rectangle"),
            Rect::new(0.0, -40.0, 20.0, 0.0),
        );
    }

    /// A clip turned by an angle that is no multiple of a quarter is the
    /// axis-aligned bounds of the turned rectangle: a superset, which the
    /// doc comment says it is. All four turned corners are inside it (so
    /// nothing visible is cut), and it is no bigger than the bounds of
    /// those corners (so it is not a looser box than documented).
    #[test]
    fn a_turned_clip_is_the_bounds_of_the_turned_rectangle() {
        let angle = std::f32::consts::FRAC_PI_4;
        let local = Rect::new(0.0, 0.0, 40.0, 20.0);
        let world_to_clip_local = Affine2::from_angle(angle);
        let bounds = clip_bounds(&clip_of(&[(local, world_to_clip_local)])).expect("a rectangle");
        let to_world = world_to_clip_local.inverse();
        let corners = [
            local.min,
            Vec2::new(local.max.x, local.min.y),
            local.max,
            Vec2::new(local.min.x, local.max.y),
        ]
        .map(|corner| to_world.transform_point2(corner));
        for corner in corners {
            assert!(
                corner.x >= bounds.min.x - EPS
                    && corner.x <= bounds.max.x + EPS
                    && corner.y >= bounds.min.y - EPS
                    && corner.y <= bounds.max.y + EPS,
                "{corner:?} lies outside {bounds:?}"
            );
        }
        let (lo, hi) = corners.iter().fold(
            (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)),
            |(lo, hi), c| (lo.min(*c), hi.max(*c)),
        );
        assert_rect(bounds, Rect { min: lo, max: hi });
        assert!(
            bounds.size().x * bounds.size().y > local.size().x * local.size().y,
            "a rotated rectangle's bounds are bigger than the rectangle"
        );
    }

    /// The shell's five steps are the face's five, in order (WP6 reads
    /// `FaceMode::step`; the shell's setting writes it).
    #[test]
    fn the_shell_step_and_the_face_step_are_the_same_five() {
        for (i, size) in TextSize::ALL.into_iter().enumerate() {
            let face = face_step(size);
            assert_eq!(usize::from(face.number()), i + 1);
            assert!((face.factor() - size.factor()).abs() < 1e-6);
        }
        assert_eq!(
            face_step(TextSize::default()),
            baylee_client_core::textface::Step::DEFAULT
        );
    }

    /// Changing the setting moves the faces' step; nothing else does.
    #[test]
    fn the_setting_writes_the_face_step() {
        let mut app = App::new();
        app.insert_resource(crate::settings::ClientSettings::default())
            .insert_resource(crate::face::FaceMode::default())
            .add_systems(Update, face_follows_the_text_size);
        app.update();
        assert_eq!(
            app.world().resource::<crate::face::FaceMode>().step,
            baylee_client_core::textface::Step::DEFAULT
        );
        app.world_mut()
            .resource_mut::<crate::settings::ClientSettings>()
            .text_size = TextSize::Xl;
        app.update();
        assert_eq!(
            app.world().resource::<crate::face::FaceMode>().step,
            baylee_client_core::textface::Step::XL
        );
    }

    #[test]
    fn the_steps_stop_at_both_ends() {
        assert_eq!(TextSize::Xl.larger(), TextSize::Xl);
        assert_eq!(TextSize::Xs.smaller(), TextSize::Xs);
        assert_eq!(TextSize::M.larger(), TextSize::L);
    }
}
