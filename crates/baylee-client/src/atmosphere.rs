//! The air over the table: leaves, mist, embers, shafts, fog and flakes.
//!
//! `baylee_client_core::atmosphere` decides *how much* of each there is, from
//! the lands on the battlefield; this draws it. The split is the usual one,
//! and here it earns its keep twice over — the reading of the board is
//! arithmetic a test can argue with, and what is left is one quad and one
//! fragment shader.
//!
//! # One surface, and where it lies
//!
//! Everything is painted on a single quad cut to the slab's own racetrack and
//! laid at [`table::ATMOSPHERE_LIFT`](crate::table::ATMOSPHERE_LIFT) — three
//! and a half thousandths of a unit above the felt, which puts it above every
//! mark that belongs to the table (a seat's mat, the glow under it, the
//! medallion) and **below** the contact shadow under a card, and therefore
//! below the card.
//!
//! That ordering is the whole of the promise the owner was made: *ohne das
//! Spielen zu beeinträchtigen*. The cards are opaque and write depth, so a
//! blended surface underneath them is rejected by the depth test at every
//! pixel a card occupies. Not one card pixel can be touched — by construction,
//! not by restraint, and a future layer that wanted to be a post-process
//! would have to give that up explicitly.
//!
//! # Why it is not three dimensions
//!
//! A leaf in the air would be a billboard somewhere above the felt, and at
//! this camera — about twenty degrees off vertical — it would spend most of
//! its life in front of a card. The quad is the other way round: the fall is
//! *painted*, and the shader is handed [`AtmosphereParams::fall`], the
//! direction on the table plane that a falling thing appears to travel. The
//! parallax is real even so, because a mark drawn as though it were high in
//! the air moves faster across the table than one near the felt, and that is
//! the whole of what sells depth at a camera like this one.
//!
//! # Nothing is shipped
//!
//! No leaf sprite, no snowflake texture, no gradient for a light shaft.
//! `docs/legal.md` §5 decided this for sound this morning and §2 decided it
//! for the felt and the mats long before: ornament is the easiest thing to
//! borrow by accident, and arithmetic borrows nothing.

use baylee_client_core::atmosphere::{self, Weather};
use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

/// Everything the weather shader reads.
#[derive(Clone, Copy, ShaderType, Debug)]
pub struct AtmosphereParams {
    /// The slab's world size, which is how the shader turns a world position
    /// into a point on the table.
    ///
    /// The world position and this, rather than the mesh's own uv, for the
    /// reason the felt gives: a uv origin is a convention of whichever
    /// builder made the mesh, and guessing it wrong mirrors the whole field.
    pub span: Vec2,
    /// The direction, on the table plane, that a falling thing appears to
    /// travel.
    ///
    /// Which is towards the camera's own ground position, and the derivation
    /// is one line: a mark at height `h` seen from a camera at height `H`
    /// over the ground point `c` lands on the plane at `p + (h/H)(p - c)`, so
    /// as `h` falls to nothing the mark slides back towards `c`. It is
    /// recomputed as the player orbits, which is what stops the weather from
    /// being painted on the lens.
    pub fall: Vec2,
    /// Forest.
    pub leaves: f32,
    /// Island.
    pub mist: f32,
    /// Mountain.
    pub embers: f32,
    /// Plains.
    pub shafts: f32,
    /// Swamp.
    pub fog: f32,
    /// Snow.
    pub flakes: f32,
    /// The clock everything moves on: [`MOVING`](crate::cardmat::MOVING) or
    /// [`STILL`](crate::cardmat::STILL), the same two values the cards, the
    /// table and the sky use.
    ///
    /// At `STILL` the air **freezes** rather than emptying. A player who
    /// asked for a table that holds still asked for that and not for a
    /// different room, and a setting that quietly removed the weather would
    /// look like a bug in the weather rather than like the setting working.
    pub motion: f32,
    /// The corner radius the mesh was cut with, so the shader can measure how
    /// far onto the cloth a point is.
    pub corner: f32,
    /// How wide the padded rail runs, in table units.
    ///
    /// The same number the felt is given, and for the opposite purpose: the
    /// felt uses it to draw the rail, and this uses it to know where the
    /// cloth begins — so a veil can stop on the cloth while a leaf goes on
    /// over the leather, which is one of the cheapest things that says the
    /// marks are in the air and the veils are on the table.
    pub rail: f32,
    /// Eased sky exposure shared by the sun, moon and table light.
    pub day: f32,
    /// The player's atmosphere budget, including the ambient dust.
    pub budget: f32,
}

/// The air.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct AtmosphereMaterial {
    /// Everything the shader reads. No textures: see the module header.
    #[uniform(0)]
    pub params: AtmosphereParams,
}

impl Material for AtmosphereMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/atmosphere.wgsl".into()
    }

    /// Blended, and therefore drawn in the transparent pass — after the
    /// cards, which are opaque, have already written their depth. That is
    /// what turns "it lies under the cards" from a claim about a constant
    /// into a claim the depth buffer enforces on every pixel.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    /// Where the air sits in the transparent pass.
    ///
    /// Being blended, it is *sorted*, and the sort is by distance to the
    /// camera — so without this the air would be painted over a seat's mat at
    /// one end of the table and under it at the other, which is the bug the
    /// whole-table quad made unignorable. [`table::sort_bias`] turns the lift
    /// ladder into the order, and this is simply the air's own rung of it.
    ///
    /// [`table::sort_bias`]: crate::table::sort_bias
    fn depth_bias(&self) -> f32 {
        crate::table::sort_bias(crate::table::ATMOSPHERE_LIFT)
    }
}

/// Marks the one quad, and remembers what is on it.
#[derive(Component)]
pub struct Air {
    /// The weather actually on screen, eased towards the one the board asks
    /// for. Kept here rather than recomputed because easing needs somewhere
    /// to keep where it started.
    shown: Weather,
    /// The world size the quad was cut to, so a table that is re-cut takes
    /// its air with it.
    cut: Vec2,
}

/// Reads the board, eases the air, and keeps one quad in step with the table.
///
/// Also writes the felt's own tint, which is the half of the weather that is
/// *not* on this quad: a grade belongs on the surface it grades, and a
/// blended overlay that tried to darken the cloth would either be a multiply
/// pass of its own or would wash the felt towards a colour instead of
/// multiplying it. `under_weather` in `shaders/felt.wgsl` is the same
/// argument `under_sky` makes one function above it.
#[expect(
    clippy::too_many_arguments,
    reason = "one quad, and everything it needs to be cut, lit and placed"
)]
pub fn breathe(
    mut commands: Commands,
    time: Res<Time>,
    duel: Res<crate::Duel>,
    prefs: Res<crate::prefs::Prefs>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<AtmosphereMaterial>>,
    mut felts: ResMut<Assets<crate::feltmat::FeltMaterial>>,
    slabs: Query<&MeshMaterial3d<crate::feltmat::FeltMaterial>>,
    cameras: Query<&GlobalTransform, With<crate::table::TableCamera>>,
    sky: Query<&crate::sky::Sky>,
    mut air: Query<(
        Entity,
        &mut Air,
        &mut Mesh3d,
        &MeshMaterial3d<AtmosphereMaterial>,
    )>,
) {
    let settings = prefs.all();
    let day = sky.single().map_or(0.5, crate::sky::Sky::daylight);
    let budget = settings.atmosphere.budget();
    let Ok(felt) = slabs.single() else {
        // No table yet, so no air over it. The quad is cut from the slab's
        // own span and there is nothing to guess it from.
        return;
    };
    // The span the table was actually cut to, read off the felt's own
    // material rather than off the layout a second time. One source of truth
    // and one that cannot be a frame behind: if the two disagreed, the air
    // would be a racetrack of a different size lying on the table.
    let Some(span) = felts.get(&felt.0).map(|felt| felt.params.span) else {
        return;
    };

    // What the lands ask for, at the budget the player allows. A table with
    // no view — between games, or before the first frame has arrived — asks
    // for still air rather than for the last weather it saw.
    let want = duel
        .view
        .as_ref()
        .map_or(Weather::ZERO, atmosphere::read)
        .scaled(settings.atmosphere.budget());

    if !settings.atmosphere.visible() {
        // Off draws no quad at all, which is the difference that matters on
        // a phone: a slab-sized blended surface is a near-fullscreen pass
        // every frame, and a set of amplitudes at zero still pays for it.
        for (entity, _, _, _) in &air {
            commands.entity(entity).despawn();
        }
        set_grade(&mut felts, felt, Weather::ZERO);
        return;
    }

    let fall = cameras.single().map_or(Vec2::NEG_Y, |camera| {
        // World is `(x, height, -y)` in table coordinates, so this is its
        // inverse. A camera directly overhead has no ground offset to speak
        // of and `try_normalize` answers `None`; falling towards the near
        // seat is the right thing to do there.
        let eye = camera.translation();
        Vec2::new(eye.x, -eye.z)
            .try_normalize()
            .unwrap_or(Vec2::NEG_Y)
    });
    let motion = if settings.reduce_motion {
        crate::cardmat::STILL
    } else {
        crate::cardmat::MOVING
    };

    let Ok((_, mut shown, mut mesh, handle)) = air.single_mut() else {
        // No quad yet. Cut one at the weather the board is already at, so a
        // player joining a table full of forests is not shown ninety seconds
        // of leaves arriving.
        commands.spawn((
            crate::table::DuelStage,
            Air {
                shown: want,
                cut: span,
            },
            // The air answers no clicks. A pointer over the table means the
            // table, and a leaf is not a thing that can be picked up.
            Pickable::IGNORE,
            Mesh3d(meshes.add(crate::table::flat_table_mesh(span))),
            MeshMaterial3d(materials.add(AtmosphereMaterial {
                params: params_of(want, span, fall, motion, day, budget),
            })),
            Transform::from_xyz(0.0, crate::table::ATMOSPHERE_LIFT, 0.0)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ));
        set_grade(&mut felts, felt, want);
        return;
    };

    let next = atmosphere::ease(shown.shown, want, time.delta_secs());
    let recut = (shown.cut - span).abs().max_element() > 1e-3;
    if recut {
        shown.cut = span;
        *mesh = Mesh3d(meshes.add(crate::table::flat_table_mesh(span)));
    }
    shown.shown = next;
    if let Some(mut material) = materials.get_mut(&handle.0) {
        material.params = params_of(next, span, fall, motion, day, budget);
    }
    set_grade(&mut felts, felt, next);
}

/// Packs a weather for the shader.
fn params_of(
    air: Weather,
    span: Vec2,
    fall: Vec2,
    motion: f32,
    day: f32,
    budget: f32,
) -> AtmosphereParams {
    AtmosphereParams {
        span,
        fall,
        leaves: air.leaves,
        mist: air.mist,
        embers: air.embers,
        shafts: air.shafts,
        fog: air.fog,
        flakes: air.flakes,
        motion,
        corner: baylee_client_core::tabletop::table_corner(span),
        rail: baylee_client_core::tabletop::RAIL_WIDTH,
        day,
        budget,
    }
}

/// Puts the weather's tint on the cloth.
///
/// Writes only when it moves, for the reason every other system on this table
/// repeats: a material touched every frame is a uniform uploaded every frame
/// for a table that has not changed.
fn set_grade(
    felts: &mut Assets<crate::feltmat::FeltMaterial>,
    handle: &MeshMaterial3d<crate::feltmat::FeltMaterial>,
    air: Weather,
) {
    let Some(mut felt) = felts.get_mut(&handle.0) else {
        return;
    };
    let grade = air.grade();
    let want = Vec4::new(grade[0], grade[1], grade[2], 0.0);
    if (felt.params.weather - want).abs().max_element() <= 1e-4 {
        return;
    }
    felt.params.weather = want;
}

/// Installs the material, its shader and the one system.
pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/atmosphere.wgsl");
        app.add_plugins(MaterialPlugin::<AtmosphereMaterial>::default());
    }
}

#[cfg(test)]
mod tests {
    /// The one geometric claim the whole design rests on, held where it can
    /// fail: the air is above everything the table itself draws and below
    /// everything a card does.
    ///
    /// Written out as numbers rather than deferred to the `const` assertions
    /// beside the constants, because those say only that the ladder is in
    /// order and this says *which* rungs, which is the part a future lift
    /// would quietly change.
    #[test]
    fn the_air_lies_under_everything_a_card_casts() {
        let air = crate::table::ATMOSPHERE_LIFT;
        assert!(air > 0.002, "the air went under a seat's mat: {air}");
        assert!(
            air < crate::table::CARD_LIFT * 0.5,
            "the air reached a card's contact shadow: {air}"
        );
        assert!(
            air < crate::table::CARD_LIFT,
            "the air reached the cards: {air}"
        );
    }

    /// The sort ladder, both halves.
    ///
    /// A blended surface is painted in an order the *sort* decides, and the
    /// sort key is `distance + depth_bias`. So the ladder is only the drawing
    /// order if every rung's bias beats the spread of distances a table can
    /// produce — otherwise which end of the table a thing sits on decides it,
    /// which is what used to happen.
    #[test]
    fn the_ladder_decides_what_covers_what() {
        use crate::table::{ATMOSPHERE_LIFT, CARD_LIFT, ZONE_LIFT, sort_bias};
        let mat = sort_bias(ZONE_LIFT);
        let air = sort_bias(ATMOSPHERE_LIFT);
        let shadow = sort_bias(CARD_LIFT * 0.5);
        assert!(mat < air && air < shadow, "{mat} {air} {shadow}");
        // The eight-seat ring is the widest table there is, and the camera is
        // never further than `MAX_DISTANCE`; the spread of distances across
        // the slab is bounded by its own diagonal, which is well inside this.
        let widest_table = 120.0_f32;
        assert!(
            air - mat > widest_table && shadow - air > widest_table,
            "a table wider than the gap between two rungs would decide the \
             order instead of the ladder: {mat} {air} {shadow}"
        );
    }

    /// The WGSL is parsed and validated with the same front end wgpu uses.
    /// The alternative is finding a typo when a pipeline is built — in a
    /// browser, where there is no filesystem to look at.
    #[test]
    fn the_atmosphere_shader_compiles() {
        let prelude = "\
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};
struct Globals { time: f32 };
@group(0) @binding(11) var<uniform> globals: Globals;
";
        crate::cardmat::tests::check_wgsl(include_str!("shaders/atmosphere.wgsl"), prelude);
    }
}

/// The system, actually run.
///
/// Everything above this point can be right while nothing is drawn: this
/// client has shipped a combat path that was written and never called, and
/// `Interaction::activate` sat wired to nothing for long enough that a Forest
/// was inert under the pointer. So the assertions here are all on *outcomes*
/// — a material that exists and carries the amplitude the board asked for,
/// a felt whose tint moved — and never on `update()` having returned.
#[cfg(test)]
mod running {
    use super::*;
    use baylee_client_core::atmosphere::Atmosphere;
    use baylee_client_core::test_support::{ViewBuilder, token};
    use baylee_core::generated::subtypes::land;
    use baylee_core::types::{SubtypeSet, TypeSet};
    use baylee_view::{PlayerView, PublicObject};

    fn forest(slot: u32) -> PublicObject {
        let mut object = token(slot, 0, "Forest", 0, 0);
        object.types = TypeSet::LAND;
        object.power = None;
        object.toughness = None;
        object.subtypes = SubtypeSet::from_slice(&[land::FOREST]);
        object
    }

    /// An app with the system, the resources it asks for, a slab already cut
    /// and a camera looking at it from where the rig puts one.
    fn harness(view: Option<PlayerView>, setting: Atmosphere) -> App {
        let mut app = App::new();
        let mut felts = Assets::<crate::feltmat::FeltMaterial>::default();
        let felt = felts.add(crate::feltmat::FeltMaterial {
            params: crate::feltmat::FeltParams {
                wash: Vec4::ZERO,
                source: Vec4::new(0.0, -1.0, 0.0, 1.0),
                ambient: Vec4::new(1.0, 1.0, 1.0, 0.0),
                weather: Vec4::ONE,
                flames: Vec4::ZERO,
                flames_tail: Vec4::new(0.0, 0.0, 1.0, 0.0),
                span: Vec2::new(24.0, 18.0),
                corner: 2.0,
                rail: 0.55,
                motion: crate::cardmat::MOVING,
                gain: crate::feltmat::WASH_GAIN,
                thickness: 0.9,
            },
        });
        let mut prefs = crate::prefs::Prefs::default();
        prefs.edit().atmosphere = setting;
        app.insert_resource(crate::Duel {
            view,
            ..crate::Duel::default()
        })
        .insert_resource(prefs)
        .init_resource::<Time>()
        .insert_resource(Assets::<Mesh>::default())
        .insert_resource(Assets::<AtmosphereMaterial>::default())
        .insert_resource(felts)
        .add_systems(Update, breathe);
        app.world_mut()
            .spawn((MeshMaterial3d(felt), Transform::default()));
        app.world_mut().spawn((
            crate::table::TableCamera,
            Transform::from_xyz(0.0, 17.0, 6.4),
            GlobalTransform::from_xyz(0.0, 17.0, 6.4),
        ));
        app
    }

    /// What the material ended up carrying, if a quad was cut at all.
    fn drawn(app: &App) -> Option<AtmosphereParams> {
        let materials = app.world().resource::<Assets<AtmosphereMaterial>>();
        materials.iter().next().map(|(_, m)| m.params)
    }

    #[test]
    fn a_board_of_forests_puts_leaves_in_the_air_and_nothing_else() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, (1..=4).map(forest).collect::<Vec<_>>())
            .build();
        let mut app = harness(Some(view), Atmosphere::Full);
        app.update();
        let params = drawn(&app).expect("no quad was cut over a table of forests");
        assert!(params.leaves > 0.0, "no leaves: {params:?}");
        assert!(
            params.mist == 0.0
                && params.embers == 0.0
                && params.shafts == 0.0
                && params.fog == 0.0
                && params.flakes == 0.0,
            "something other than leaves reached the shader: {params:?}"
        );
        assert!(
            params.span.x > 0.0 && params.corner > 0.0,
            "the air was cut to nothing: {params:?}"
        );
    }

    #[test]
    fn the_felt_is_told_what_is_in_the_air_over_it() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, (1..=4).map(forest).collect::<Vec<_>>())
            .build();
        let mut app = harness(Some(view), Atmosphere::Full);
        app.update();
        let felts = app
            .world()
            .resource::<Assets<crate::feltmat::FeltMaterial>>();
        let tint = felts
            .iter()
            .next()
            .expect("the slab lost its material")
            .1
            .params
            .weather;
        assert!(
            (tint - Vec4::ONE).abs().max_element() > 1e-3,
            "the cloth was never told there were forests on it: {tint:?}"
        );
        // Luma-neutral, which is the promise: hue may move, brightness may not.
        let luma = 0.2126 * tint.x + 0.7152 * tint.y + 0.0722 * tint.z;
        assert!(
            (luma - 1.0).abs() < 0.01,
            "the felt changed brightness: {luma}"
        );
    }

    #[test]
    fn off_cuts_no_quad_at_all() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, (1..=4).map(forest).collect::<Vec<_>>())
            .build();
        let mut app = harness(Some(view), Atmosphere::Off);
        app.update();
        assert!(
            drawn(&app).is_none(),
            "a surface was drawn for a player who asked for none"
        );
    }

    #[test]
    fn the_quiet_setting_puts_half_as_much_in_the_air() {
        let board = || {
            ViewBuilder::new(2)
                .with_battlefield(0, (1..=4).map(forest).collect::<Vec<_>>())
                .build()
        };
        let mut full = harness(Some(board()), Atmosphere::Full);
        full.update();
        let mut soft = harness(Some(board()), Atmosphere::Soft);
        soft.update();
        let (a, b) = (
            drawn(&full).expect("no quad at Full").leaves,
            drawn(&soft).expect("no quad at Soft").leaves,
        );
        assert!((b - a * 0.5).abs() < 1e-5, "Full {a}, Soft {b}");
    }

    #[test]
    fn a_table_with_no_game_on_it_has_still_air() {
        let mut app = harness(None, Atmosphere::Full);
        app.update();
        let params = drawn(&app).expect("no quad was cut");
        assert!(
            params.leaves == 0.0 && params.flakes == 0.0 && params.fog == 0.0,
            "weather over an empty table: {params:?}"
        );
    }
}
