//! The shell header's shared animated surface.

#[allow(clippy::wildcard_imports)] // the header module's shared vocabulary
use super::*;

/// Marks the top header bar (`false`) or seated strip (`true`) to receive a
/// continuously animated silk-aurora background shader surface.
#[derive(Component, Clone, Copy, Debug)]
pub struct HeaderSurface(pub bool);

/// Marks a [`HeaderSurface`] whose shader child has already been attached.
#[derive(Component)]
pub struct HeaderSurfaceDressed;

/// Attaches the continuous silk-aurora ambient material to any newly spawned
/// [`HeaderSurface`] (top navigation bar or seated strip).
fn dress_header_surfaces(
    mut commands: Commands,
    bars: Query<(Entity, &HeaderSurface), Without<HeaderSurfaceDressed>>,
    materials: Option<ResMut<Assets<crate::ambience::AmbienceMaterial>>>,
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
    for (bar, kind) in &bars {
        let is_strip = kind.0;
        let (low, high, pad, seed) = if is_strip {
            (
                Color::srgba(0.08, 0.13, 0.19, 0.95),
                Color::srgba(0.19, 0.32, 0.43, 0.97),
                2.2,
                2.1,
            )
        } else {
            (
                Color::srgba(0.08, 0.12, 0.17, 0.96),
                Color::srgba(0.18, 0.31, 0.42, 0.98),
                2.0,
                1.7,
            )
        };
        let energy = if still { 0.0 } else { 1.0 };
        let handle = materials.add(crate::ambience::AmbienceMaterial {
            params: crate::ambience::AmbienceParams {
                low: LinearRgba::from(low).to_f32_array().into(),
                high: LinearRgba::from(high).to_f32_array().into(),
                energy,
                seed,
                aspect: 12.0,
                pad,
            },
        });
        commands
            .entity(bar)
            .queue_silenced(move |mut bar_ent: EntityWorldMut| {
                let surface = bar_ent.world_scope(|world| {
                    world
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: px_fixed(0.0),
                                right: px_fixed(0.0),
                                top: px_fixed(0.0),
                                bottom: px_fixed(0.0),
                                ..default()
                            },
                            MaterialNode(handle),
                            crate::ambience::Ambient { energy: 1.0 },
                            Pickable::IGNORE,
                        ))
                        .id()
                });
                bar_ent
                    .insert(HeaderSurfaceDressed)
                    .insert_children(0, &[surface]);
            });
    }
}

/// Loads the cuts and keeps the marks sharp.
pub(crate) fn install(app: &mut App) {
    if let Some(assets) = app.world().get_resource::<AssetServer>() {
        let art = BrandArt(LOGO_CUTS.map(|(_, path)| assets.load(path)));
        app.insert_resource(art);
    }
    app.add_systems(
        PostUpdate,
        dress_header_surfaces.before(bevy::ui::UiSystems::Layout),
    )
    .add_systems(PostUpdate, sharpen_brand.after(bevy::ui::UiSystems::Layout));
}
