//! The moving surfaces the front door is drawn on, and the buttons that
//! answer the pointer.
//!
//! Two things live here because they are the same argument twice: a screen
//! that never moves reads as a screen that has stopped working. The lobby's
//! backdrop drifts, and a button leans towards the pointer and gives way
//! under a press — both small, both continuous, both switched off entirely by
//! [`baylee_client_core::prefs::Preferences::reduce_motion`].
//!
//! The backdrop is a shader rather than a picture for the reason
//! `docs/legal.md` §2 gives about the table's felt: ornament is the easiest
//! thing to borrow by accident, and arithmetic borrows nothing. It is also
//! the cheaper of the two — one full-screen pass with no texture to load, no
//! bytes in the wasm bundle, and nothing to fail to fetch in a browser.

use bevy::picking::hover::PickingInteraction;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::ui::UiTransform;

use bevy::asset::embedded_asset;

/// How fast a button reaches the state the pointer is asking for.
///
/// Exponential like [`crate::table::glide`] and for the same reason: it is
/// frame-rate independent, and a small correction does not take as long as a
/// large one. 18 is about 120 ms to settle, which is under the ~150 ms where
/// a control starts feeling sluggish and over the ~60 ms where it reads as an
/// instant snap.
///
/// Shared rather than restated: the slip under the hover preview
/// ([`crate::hud::slip`]) answers a pointer too, and a second 18 written out
/// there would be the same decision in two places.
pub(crate) const FEEL_RATE: f32 = 18.0;

/// How much an ordinary button grows under the pointer, and gives way under a
/// press — 2.5% either way.
///
/// A *share* and not two scales, because the two were always ±0.025 and a pair
/// of numbers that must mirror each other is a pair that can stop mirroring.
/// It is [`Feel::lift`]'s default rather than the only value: a control small
/// enough to be one mark — a mana pip is 21 px of disc — moves 2.5% by half a
/// pixel, which is a movement nobody sees.
pub(crate) const BUTTON_LIFT: f32 = 0.025;

/// How far a hovered button is lightened towards white, and a pressed one
/// darkened. Small on purpose: the tone still has to read as the same button.
const HOVER_LIFT: f32 = 0.16;
const PRESS_SINK: f32 = 0.10;

/// The parameters of one ambient surface.
#[derive(Clone, Copy, ShaderType, Debug)]
pub struct AmbienceParams {
    /// The ground colour.
    pub low: Vec4,
    /// What the bands are drawn in.
    pub high: Vec4,
    /// Motion and brightness of the bands. Zero stills the surface.
    pub energy: f32,
    /// Offsets the field, so two surfaces on screen at once do not repeat.
    pub seed: f32,
    /// Width over height of the node, so the field is not stretched.
    pub aspect: f32,
    /// Padding to the 16-byte boundary a uniform needs.
    pub pad: f32,
}

/// A drifting field, as a UI material.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct AmbienceMaterial {
    /// Everything the shader reads.
    #[uniform(0)]
    pub params: AmbienceParams,
}

impl UiMaterial for AmbienceMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/ambience.wesl".into()
    }
}

/// A node carrying an ambient surface, so its aspect can be kept honest and
/// its energy answered to the motion preference.
#[derive(Component, Clone, Copy)]
pub struct Ambient {
    /// The energy this surface has when motion is allowed.
    pub energy: f32,
}

impl Default for Ambient {
    fn default() -> Self {
        Self { energy: 1.0 }
    }
}

/// A button that answers the pointer.
///
/// It carries the tone it was built with rather than reading it back off the
/// node: the animation writes to [`BackgroundColor`] every frame, so after
/// one hover the node's own colour is no longer the colour it started with.
#[derive(Component, Clone, Copy)]
pub struct Feel {
    /// The colour the button rests at.
    pub base: Color,
    /// Where the pointer takes it, when lifting [`Self::base`] towards white
    /// is not the answer.
    ///
    /// [`shade`] mixes the channels towards white and **keeps the alpha**, so
    /// a control that rests at `Color::NONE` is lifted to a brighter nothing
    /// and never answers the pointer at all. That is not a hypothetical: a
    /// step tile the standing orders skip has no ground, and its hover is the
    /// ground a click would give it. So the hot end is stated where it cannot
    /// be derived, and derived everywhere else.
    pub hot: Option<Color>,
    /// How far into the hovered state it currently is, 0 to 1. Negative
    /// values are the pressed side, so one number carries both.
    pub warmth: f32,
    /// How far the pointer also *moves* it: the share it grows by under a
    /// hover, and gives way by under a press.
    ///
    /// [`BUTTON_LIFT`] for a button, which is a small object a pointer can
    /// lift. **Zero** where the control is a *line of writing*: growing a row
    /// grows the sentence on it, and a paragraph that changes size under the
    /// pointer is the one thing a reader cannot read past.
    ///
    /// A number rather than the flag it was, because the two ends of that flag
    /// are not the whole range. A mana pip is a disc 21 px across and the
    /// **whole** of its control — there is no sentence beside it to disturb
    /// and nothing to reflow — so it is the case that wants a *larger*
    /// movement than a button's, and with a flag the only way to say that was
    /// another constant read by one call site.
    pub lift: f32,
}

impl Feel {
    /// A button resting at `base`, hovering a shade lighter.
    #[must_use]
    pub fn new(base: Color) -> Self {
        Self {
            base,
            hot: None,
            warmth: 0.0,
            lift: BUTTON_LIFT,
        }
    }

    /// A button resting at `base` and hovering all the way to `hot`.
    #[must_use]
    pub fn rising_to(base: Color, hot: Color) -> Self {
        Self {
            base,
            hot: Some(hot),
            warmth: 0.0,
            lift: BUTTON_LIFT,
        }
    }

    /// The same, for a control the pointer may light but must not move.
    #[must_use]
    pub fn tinting_to(base: Color, hot: Color) -> Self {
        Self {
            lift: 0.0,
            ..Self::rising_to(base, hot)
        }
    }

    /// The same again, moving by a share the caller states.
    #[must_use]
    pub fn lifting(base: Color, hot: Color, lift: f32) -> Self {
        Self {
            lift,
            ..Self::rising_to(base, hot)
        }
    }
}

/// Optional border color interpolation paired with [`Feel`].
#[derive(Component, Clone, Copy, Debug)]
pub struct FeelBorder {
    /// Resting border color.
    pub rest: Color,
    /// Hovered border color.
    pub hot: Color,
    /// Pressed border color.
    pub press: Color,
}

/// Optional control shadow elevation interpolation paired with [`Feel`]:
/// lifts the drop shadow subtly under hover and settles it closer under press.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct FeelShadow;

/// Registers the material, ships its shader inside the binary, and runs the
/// two animations.
///
/// Embedded rather than loaded from `assets/`, for the same reason
/// [`crate::cardmat::CardMaterialPlugin`] gives: `index.html` copies only the
/// font directory to `dist/`, so a shader on disk would work natively and
/// silently fail to load in a browser.
pub struct AmbiencePlugin;

impl Plugin for AmbiencePlugin {
    fn build(&self, app: &mut App) {
        // The button animation is arithmetic over components and runs
        // anywhere, including in the headless tests that build the lobby's
        // node tree without a renderer.
        app.add_systems(Update, feel)
            .add_systems(PostUpdate, breathe.after(bevy::ui::UiSystems::Layout));
        // The surface is not: embedding a shader needs an asset server, and a
        // UI material needs a render world. A test app has neither, and
        // adding them regardless is a panic in every lobby test rather than
        // an honest "there is nothing to draw on here".
        if !app.world().contains_resource::<AssetServer>() {
            return;
        }
        embedded_asset!(app, "shaders/ambience.wesl");
        // The shell kit's primary-button light (`shellkit::sheen`).
        embedded_asset!(app, "shaders/kit_sheen.wesl");
        embedded_asset!(app, "shaders/chat_glass.wesl");
        app.add_plugins(UiMaterialPlugin::<AmbienceMaterial>::default());
    }
}

/// Adds [`AmbiencePlugin`] unless another plugin already did.
///
/// The lobby and the duel are separate plugins and either may be the only one
/// present; bevy panics on a duplicate, so "whoever gets there first" has to
/// be said out loud — the same shape [`crate::prefs::install`] uses.
pub(crate) fn install(app: &mut App) {
    if !app.is_plugin_added::<AmbiencePlugin>() {
        app.add_plugins(AmbiencePlugin);
    }
}

/// Keeps every ambient surface's aspect and energy in step with its node.
///
/// The aspect has to be written from here because a UI node's size is not
/// known until layout has run, and a field stretched to a 21:9 window looks
/// like a field that was authored for a different screen.
fn breathe(
    nodes: Query<(&Ambient, &ComputedNode, &MaterialNode<AmbienceMaterial>)>,
    materials: Option<ResMut<Assets<AmbienceMaterial>>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    quality: Option<Res<crate::quality::InUse>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let still = crate::quality::ambient_still(
        prefs.is_some_and(|p| p.all().reduce_motion),
        quality.as_deref(),
    );
    for (ambient, computed, handle) in &nodes {
        let size = computed.size();
        let raw_aspect = if size.y > 0.0 { size.x / size.y } else { 1.0 };
        let aspect = if raw_aspect.is_finite() && raw_aspect > 0.0 {
            raw_aspect
        } else {
            1.0
        };
        let energy = if still { 0.0 } else { ambient.energy };
        // Read before written: a write marks the material changed and
        // uploads it again, and this ran every frame for numbers that
        // change only with the window and the setting.
        let unchanged = materials.get(&handle.0).is_none_or(|m| {
            m.params.aspect.to_bits() == aspect.to_bits()
                && m.params.energy.to_bits() == energy.to_bits()
        });
        if unchanged {
            continue;
        }
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.params.aspect = aspect;
            material.params.energy = energy;
        }
    }
}

/// Leans a hovered button towards the pointer and lets a pressed one give
/// way.
///
/// One system for every button in the client, driven by
/// [`PickingInteraction`] — which bevy maintains for anything the pointer can
/// hit, so a button gets this by carrying [`Feel`] and nothing else. Doing it
/// per screen would mean every new screen forgetting it once.
///
/// A face the pointer cannot hit itself (`Pickable::IGNORE`: the kit's
/// controls put the press on a hit area around the face, `shellkit::controls::hit`)
/// answers its parent's interaction: the hit area is what the pointer is
/// over, and the face is what moves. Before this the kit's buttons, chips
/// and segments carried a `Feel` that never felt anything (owner, 09.10.).
#[allow(clippy::type_complexity)] // one row per button, and the parent's state
pub(crate) fn feel(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut buttons: Query<(
        &mut Feel,
        Option<&PickingInteraction>,
        Option<&Pickable>,
        Option<&ChildOf>,
        &mut BackgroundColor,
        &mut UiTransform,
        Option<(&FeelBorder, &mut BorderColor)>,
        Option<(&FeelShadow, &mut BoxShadow)>,
    )>,
    held: Query<&PickingInteraction>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let raw_dt = time.delta_secs();
    let dt = if raw_dt.is_finite() {
        raw_dt.clamp(0.0, 0.1)
    } else {
        0.0
    };
    let step = if still {
        1.0
    } else {
        1.0 - (-FEEL_RATE * dt).exp()
    };
    for (mut feel, own, pickable, parent, mut colour, mut transform, border_opt, shadow_opt) in
        &mut buttons
    {
        let ignored = pickable.is_some_and(|p| !p.is_hoverable && !p.should_block_lower);
        let interaction = if ignored {
            parent.and_then(|p| held.get(p.parent()).ok())
        } else {
            own
        };
        let target = match interaction {
            Some(PickingInteraction::Pressed) => -1.0,
            Some(PickingInteraction::Hovered) => 1.0,
            _ => 0.0,
        };
        // `is_changed` and not `is_added`: a control whose resting tone was
        // changed in place (the report's boxes, `report::form::retick`) has
        // to be painted again although the pointer never moved.
        let settled = (feel.warmth - target).abs() < f32::EPSILON;
        if settled && !feel.is_changed() {
            continue;
        }
        if !settled {
            let mut next_warmth = feel.warmth + (target - feel.warmth) * step;
            if (next_warmth - target).abs() < 0.001 {
                next_warmth = target;
            }
            if next_warmth.to_bits() != feel.warmth.to_bits() {
                feel.warmth = next_warmth;
            } else if !feel.is_changed() {
                continue;
            }
        }

        let hot = feel.warmth.max(0.0);
        let cold = (-feel.warmth).max(0.0);
        // Hover never grows a control's scale (preventing edge-hover flicker);
        // only a press sinks the face slightly inward.
        let scale = Vec2::splat(1.0 - feel.lift * cold);
        if transform.scale != scale {
            transform.scale = scale;
        }
        // The press is a shade either way; only the hover has two readings,
        // and a stated hot end is interpolated in all four channels so an
        // alpha of zero can rise out of nothing.
        let resting = match feel.hot {
            Some(target) => blend(feel.base, target, hot),
            None => shade(feel.base, HOVER_LIFT * hot),
        };
        let next = shade(resting, -PRESS_SINK * cold);
        colour.set_if_neq(BackgroundColor(next));
        if let Some((fb, mut border)) = border_opt {
            let b_hover = blend(fb.rest, fb.hot, hot);
            let b_next = blend(b_hover, fb.press, cold);
            border.set_if_neq(BorderColor::all(b_next));
        }
        if let Some((_, mut shadow)) = shadow_opt {
            shadow.set_if_neq(crate::shellkit::surfaces::elevation_control_at(feel.warmth));
        }
    }
}

/// Lightens (positive) or darkens (negative) a colour, keeping its alpha.
///
/// Mixing towards white rather than scaling the channels: scaling a colour
/// that is already at 1.0 in one channel shifts its hue, and the accent green
/// turning yellow under the pointer is exactly the kind of thing nobody can
/// name but everybody sees.
fn shade(base: Color, by: f32) -> Color {
    let mut rgba = base.to_srgba();
    let towards = if by >= 0.0 { 1.0 } else { 0.0 };
    let amount = by.abs();
    rgba.red += (towards - rgba.red) * amount;
    rgba.green += (towards - rgba.green) * amount;
    rgba.blue += (towards - rgba.blue) * amount;
    rgba.into()
}

/// `base` lightened by `by` towards white, alpha kept: the hover a stated
/// hot end names when [`Feel::new`]'s own lift is not the one wanted.
#[must_use]
pub(crate) fn lighter(base: Color, by: f32) -> Color {
    shade(base, by)
}

/// `from` at `t` of the way to `to`, **alpha included**.
///
/// The one difference from [`shade`] that matters: a control resting at
/// `Color::NONE` has to be able to rise out of nothing, and an interpolation
/// that kept the alpha could never do it.
pub(crate) fn blend(from: Color, to: Color, t: f32) -> Color {
    let (a, b) = (from.to_srgba(), to.to_srgba());
    Srgba {
        red: a.red + (b.red - a.red) * t,
        green: a.green + (b.green - a.green) * t,
        blue: a.blue + (b.blue - a.blue) * t,
        alpha: a.alpha + (b.alpha - a.alpha) * t,
    }
    .into()
}

/// Spawns a full-bleed ambient surface, returning it for the caller to place.
///
/// Positioned absolutely and marked [`Pickable::IGNORE`]: it is the ground,
/// and a backdrop that swallowed clicks would make every button on top of it
/// unreachable.
///
/// Where it sits in the stack is the caller's business, and deliberately not
/// decided here. A surface spawned at the root needs a negative
/// [`GlobalZIndex`] to sit under the screen; the same call *inside* an overlay
/// must not have one, or it sinks below the screen it was meant to cover —
/// which is exactly what the loading veil did on its first attempt.
pub fn backdrop(
    commands: &mut Commands,
    materials: &mut Assets<AmbienceMaterial>,
    low: Color,
    high: Color,
    energy: f32,
    seed: f32,
) -> Entity {
    let handle = materials.add(AmbienceMaterial {
        params: AmbienceParams {
            low: LinearRgba::from(low).to_f32_array().into(),
            high: LinearRgba::from(high).to_f32_array().into(),
            energy,
            seed,
            aspect: 1.0,
            pad: 0.0,
        },
    });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            MaterialNode(handle),
            Ambient { energy },
            Pickable::IGNORE,
        ))
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lightening must not move the hue. The accent is a saturated green with
    /// a channel near 1.0, which is exactly where a multiplicative highlight
    /// goes wrong.
    #[test]
    fn a_highlight_keeps_the_colour_it_highlights() {
        let accent = Color::srgb(0.30, 0.78, 0.70);
        let lit = shade(accent, 0.2).to_srgba();
        let base = accent.to_srgba();
        assert!(lit.red > base.red && lit.green > base.green && lit.blue > base.blue);
        // Every channel moves a fifth of the way to white, so the ordering of
        // the channels — which is what hue is — cannot change.
        assert!(lit.green > lit.blue && lit.blue > lit.red);
        let sunk = shade(accent, -0.2).to_srgba();
        assert!(sunk.green < base.green && sunk.green > lit.red * 0.0);
    }

    /// Zero is the resting state and has to leave the colour untouched: the
    /// system writes it every frame to every button on screen.
    #[test]
    fn an_unhovered_button_is_the_colour_it_was_built_with() {
        let tone = Color::srgb(0.11, 0.13, 0.16);
        let rest = shade(tone, 0.0).to_srgba();
        let base = tone.to_srgba();
        assert!((rest.red - base.red).abs() < 1e-6);
        assert!((rest.green - base.green).abs() < 1e-6);
        assert!((rest.blue - base.blue).abs() < 1e-6);
    }

    /// The WGSL is parsed and validated with the same front end wgpu uses.
    /// The alternative is finding a typo when a pipeline is built — in a
    /// browser, where there is no filesystem to look at.
    #[test]
    fn the_ambience_shader_compiles() {
        let prelude = "\
struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
    @location(2) border_radius_x: vec4<f32>,
    @location(3) border_radius_y: vec4<f32>,
    @location(4) @interpolate(flat) size: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
struct Globals { time: f32 };
";
        // `check_wgsl` strips `import` lines, so what is imported has to be
        // put back by hand — the shared grain in this case, which is the
        // whole point of it being shared.
        let prelude = format!("{prelude}{}", include_str!("shaders/noise.wesl"));
        crate::cardmat::tests::check_wgsl(include_str!("shaders/ambience.wesl"), &prelude);
    }

    #[test]
    fn feel_handles_extreme_dt_and_stays_quiet_at_rest() {
        use std::time::Duration;

        let mut app = App::new();
        app.init_resource::<Time>().add_systems(Update, feel);
        let button = app
            .world_mut()
            .spawn((
                Feel::new(Color::srgb(0.11, 0.13, 0.16)),
                PickingInteraction::None,
                BackgroundColor(Color::BLACK),
                UiTransform::default(),
                FeelBorder {
                    rest: Color::srgb(0.20, 0.22, 0.26),
                    hot: Color::srgb(0.55, 0.48, 0.30),
                    press: Color::srgb(0.10, 0.11, 0.14),
                },
                BorderColor::all(Color::BLACK),
                FeelShadow,
                crate::shellkit::surfaces::elevation_control(),
            ))
            .id();

        // First frame paints the initial resting colours from `is_changed()`.
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(16));
        app.update();

        // Second frame at rest: zero change-detection mutations across all components.
        let tick = app.world().read_change_tick();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(16));
        app.update();
        let now = app.world().read_change_tick();
        let w = app.world();
        let btn = w.entity(button);
        let any_changed = btn
            .get_ref::<Feel>()
            .unwrap()
            .last_changed()
            .is_newer_than(tick, now)
            || btn
                .get_ref::<BackgroundColor>()
                .unwrap()
                .last_changed()
                .is_newer_than(tick, now)
            || btn
                .get_ref::<UiTransform>()
                .unwrap()
                .last_changed()
                .is_newer_than(tick, now)
            || btn
                .get_ref::<BorderColor>()
                .unwrap()
                .last_changed()
                .is_newer_than(tick, now)
            || btn
                .get_ref::<BoxShadow>()
                .unwrap()
                .last_changed()
                .is_newer_than(tick, now);
        assert!(!any_changed, "a resting button must touch no component");

        // Hovering with dt == 0.0 must not spuriously mutate `Feel`.
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::ZERO);
        *app.world_mut()
            .get_mut::<PickingInteraction>(button)
            .unwrap() = PickingInteraction::Hovered;
        let tick = app.world().read_change_tick();
        app.update();
        let now = app.world().read_change_tick();
        let changed_on_zero_dt = app
            .world()
            .entity(button)
            .get_ref::<Feel>()
            .unwrap()
            .last_changed()
            .is_newer_than(tick, now);
        assert!(!changed_on_zero_dt, "zero dt cannot advance warmth");

        // A multi-second un-minimise wake spike (dt = 5.0s) must clamp cleanly
        // without overshooting past 1.0 or producing non-finite values.
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(5));
        app.update();
        let warmth = app.world().get::<Feel>(button).unwrap().warmth;
        assert!(
            warmth.is_finite() && warmth > 0.0 && warmth <= 1.0,
            "clamped wake spike must stay in (0, 1], got {warmth}"
        );
        let scale = app.world().get::<UiTransform>(button).unwrap().scale;
        assert!(scale.is_finite());
    }

    #[test]
    fn concurrent_despawn_and_hud_commands_log_zero_ecs_errors() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;

        static ECS_ERRORS: AtomicUsize = AtomicUsize::new(0);
        fn count_ecs_error(
            _err: bevy::ecs::error::BevyError,
            _ctx: bevy::ecs::error::ErrorContext,
        ) {
            ECS_ERRORS.fetch_add(1, Ordering::SeqCst);
        }

        #[derive(Component)]
        struct DoomedRoot;
        #[derive(Component)]
        struct OrphanProbe;

        ECS_ERRORS.store(0, Ordering::SeqCst);
        let mut app = App::new();
        app.set_error_handler(count_ecs_error);
        app.init_resource::<Time>().add_systems(
            Update,
            (
                |mut commands: Commands, roots: Query<Entity, With<DoomedRoot>>| {
                    for root in &roots {
                        commands.entity(root).try_despawn();
                    }
                },
                feel,
                |mut commands: Commands,
                 roots: Query<Entity, With<DoomedRoot>>,
                 buttons: Query<Entity, With<Feel>>| {
                    for root in &roots {
                        let c1 = commands.spawn(OrphanProbe).id();
                        let c2 = commands.spawn(OrphanProbe).id();
                        let c3 = commands.spawn(OrphanProbe).id();
                        crate::hud::add_child_if_alive(&mut commands, root, c1);
                        crate::hud::add_children_if_alive(&mut commands, root, &[c2]);
                        crate::hud::despawn_children_if_alive(&mut commands, root);
                        crate::hud::replace_children_if_alive(&mut commands, root, &[c3]);
                        commands.entity(root).try_despawn();
                    }
                    for button in &buttons {
                        commands.entity(button).try_insert(Visibility::Hidden);
                        commands.entity(button).try_remove::<FeelBorder>();
                        commands.entity(button).try_despawn();
                    }
                },
            )
                .chain(),
        );

        let root = app.world_mut().spawn(DoomedRoot).id();
        let button = app
            .world_mut()
            .spawn((
                Feel::new(Color::srgb(0.2, 0.2, 0.2)),
                PickingInteraction::Hovered,
                BackgroundColor(Color::BLACK),
                UiTransform::default(),
            ))
            .id();
        app.world_mut().entity_mut(root).add_child(button);

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(16));
        app.update();

        assert_eq!(
            ECS_ERRORS.load(Ordering::SeqCst),
            0,
            "concurrent despawn + feel + HUD deferred commands must emit zero ECS errors"
        );
        assert!(
            app.world().get_entity(root).is_err() && app.world().get_entity(button).is_err(),
            "doomed hierarchy was cleanly despawned"
        );
        let leaked = app
            .world_mut()
            .query_filtered::<Entity, With<OrphanProbe>>()
            .iter(app.world())
            .count();
        assert_eq!(
            leaked, 0,
            "children queued onto a concurrently despawned parent must not leak as orphans"
        );
    }
}
