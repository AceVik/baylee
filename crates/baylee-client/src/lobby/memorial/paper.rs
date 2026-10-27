//! Stationary parchment and a finite, interruptible paper leaf; no control scales.
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub(crate) struct PaperMaterial {
    #[uniform(0)]
    fold: Vec4,
}
impl UiMaterial for PaperMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/memorial_paper.wesl".into()
    }
}
#[derive(Resource)]
struct PaperArt {
    sheet: Handle<PaperMaterial>,
    leaf: Handle<PaperMaterial>,
}
#[derive(Component)]
pub(super) struct Leaf;
#[derive(Component)]
pub(super) struct LeafScroll;
#[derive(Resource, Default)]
struct Turn {
    seen: Option<u64>,
    progress: f32,
    previous_scroll: f32,
}

pub(in crate::lobby) struct BookPlugin;
impl Plugin for BookPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((super::preview::PreviewPlugin, super::motion::MotionPlugin));
        app.init_resource::<Turn>()
            .add_systems(
                Update,
                super::chapter_focus
                    .after(crate::shellkit::focus::FocusSystems)
                    .before(crate::lobby::shell::ui),
            )
            .add_systems(
                Update,
                (
                    turn.after(crate::lobby::keyboard::keyboard)
                        .after(crate::lobby::clicks::clicks)
                        .after(crate::lobby::front::keys::activate_by_key)
                        .after(crate::lobby::focusing::keys_press)
                        .after(super::chapter_focus)
                        .before(crate::lobby::shell::ui),
                    super::album::viewer_input,
                ),
            )
            .add_systems(
                PostUpdate,
                super::album::viewer_layout.after(bevy::ui::UiSystems::Layout),
            )
            .init_resource::<super::album::PhotoPan>();
        if !app.world().contains_resource::<AssetServer>() {
            return;
        }
        app.add_plugins(UiMaterialPlugin::<PaperMaterial>::default());
        let mut materials = app.world_mut().resource_mut::<Assets<PaperMaterial>>();
        let sheet = materials.add(PaperMaterial { fold: Vec4::ZERO });
        let leaf = materials.add(PaperMaterial { fold: Vec4::ZERO });
        app.insert_resource(PaperArt { sheet, leaf });
    }
}

pub(super) fn attach(commands: &mut Commands, entity: Entity, leaf: bool) {
    commands.queue(move |world: &mut World| {
        let material = world.get_resource::<PaperArt>().map(|art| {
            if leaf {
                art.leaf.clone()
            } else {
                art.sheet.clone()
            }
        });
        let pose = if leaf {
            let turn = world.resource::<Turn>();
            let album = world.resource::<crate::lobby::LobbyState>().memorial;
            Some(leaf_pose(
                turn.progress,
                album.chapter >= album.previous_chapter,
                album.viewing,
            ))
        } else {
            None
        };
        if let Ok(mut node) = world.get_entity_mut(entity) {
            if let Some(handle) = material {
                node.insert(MaterialNode(handle));
            }
            if let Some(pose) = pose {
                node.insert(pose);
            }
        }
    });
}

/// Seed the old page before layout, including rebuilds halfway through a turn.
pub(super) fn restore_scroll(commands: &mut Commands, entity: Entity) {
    commands.queue(move |world: &mut World| {
        let offset = world.resource::<Turn>().previous_scroll;
        if let Ok(mut node) = world.get_entity_mut(entity) {
            node.insert(ScrollPosition(Vec2::new(0.0, offset)));
        }
    });
}

#[allow(clippy::too_many_arguments, clippy::float_cmp)] // Exact retained values guard ECS change ticks; no approximate numerical comparison.
fn turn(
    time: Res<Time>,
    state: Res<crate::lobby::LobbyState>,
    prefs: Res<crate::prefs::Prefs>,
    reading: Query<(&ScrollPosition, &crate::lobby::Scrollable), Without<LeafScroll>>,
    mut kept: ResMut<Turn>,
    mut leaves: Query<(&mut UiTransform, &mut Visibility), With<Leaf>>,
    mut scrolls: Query<&mut ScrollPosition, With<LeafScroll>>,
    art: Option<Res<PaperArt>>,
    mut materials: Option<ResMut<Assets<PaperMaterial>>>,
) {
    let album = state.memorial;
    if !album.open {
        if kept.seen.is_some() {
            kept.seen = None;
        }
        return;
    }
    if kept.seen != Some(album.revision) {
        kept.progress = if kept.seen.is_some() { 0.0 } else { 1.0 };
        kept.previous_scroll = reading
            .iter()
            .find(|(_, list)| list.0 == crate::lobby::List::Memorial)
            .map_or(0.0, |(p, _)| p.y);
        kept.seen = Some(album.revision);
    }
    let progress = if prefs.all().reduce_motion {
        1.0
    } else {
        (kept.progress + time.delta_secs() / 0.72).min(1.0)
    };
    if kept.progress != progress {
        kept.progress = progress;
    }
    let t = kept.progress;
    let ease = t * t * (3.0 - 2.0 * t);
    let direction = if album.chapter >= album.previous_chapter {
        -1.0
    } else {
        1.0
    };
    let pose = leaf_pose(t, album.chapter >= album.previous_chapter, album.viewing);
    for (mut transform, mut visible) in &mut leaves {
        transform.set_if_neq(pose.0);
        visible.set_if_neq(pose.1);
    }
    for mut position in &mut scrolls {
        if position.y != kept.previous_scroll {
            position.y = kept.previous_scroll;
        }
    }
    let fold = Vec4::new(ease, direction, 1.0, 0.0);
    if let (Some(art), Some(materials)) = (art, materials.as_mut())
        && materials.get(&art.leaf).is_some_and(|m| m.fold != fold)
        && let Some(mut material) = materials.get_mut(&art.leaf)
    {
        material.fold = fold;
    }
}

fn leaf_pose(t: f32, forward: bool, viewing: bool) -> (UiTransform, Visibility) {
    let ease = t * t * (3.0 - 2.0 * t);
    let direction = if forward { -1.0 } else { 1.0 };
    (
        UiTransform {
            scale: Vec2::new((1.0 - ease).max(0.001), 1.0),
            translation: Val2::new(Val::Percent(direction * ease * 50.0), Val::Px(0.0)),
            rotation: Rot2::radians(direction * (t * std::f32::consts::PI).sin() * 0.012),
        },
        if t >= 1.0 || viewing {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_leaf_pivots_both_ways_and_is_hidden_at_rest() {
        let (left, shown) = leaf_pose(0.5, true, false);
        let (right, _) = leaf_pose(0.5, false, false);
        assert_eq!(shown, Visibility::Inherited);
        assert!((left.scale.x - 0.5).abs() < f32::EPSILON);
        assert_eq!(left.translation.x, Val::Percent(-25.0));
        assert_eq!(right.translation.x, Val::Percent(25.0));
        assert_eq!(leaf_pose(1.0, true, false).1, Visibility::Hidden);
        assert_eq!(leaf_pose(0.2, true, true).1, Visibility::Hidden);
    }
}

#[cfg(test)]
mod idle_tests {
    use super::*;
    #[test]
    fn resting_paper_does_not_mark_materials_or_motion_changed() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<crate::prefs::Prefs>()
            .init_resource::<crate::lobby::LobbyState>()
            .init_resource::<Turn>()
            .init_resource::<Assets<PaperMaterial>>()
            .add_systems(Update, turn);
        let sheet = app
            .world_mut()
            .resource_mut::<Assets<PaperMaterial>>()
            .add(PaperMaterial { fold: Vec4::ZERO });
        let leaf = app
            .world_mut()
            .resource_mut::<Assets<PaperMaterial>>()
            .add(PaperMaterial { fold: Vec4::ZERO });
        app.insert_resource(PaperArt { sheet, leaf });
        app.world_mut()
            .resource_mut::<crate::lobby::LobbyState>()
            .memorial
            .open = true;
        app.update();
        let materials = app
            .world()
            .resource_ref::<Assets<PaperMaterial>>()
            .last_changed();
        let turn_tick = app.world().resource_ref::<Turn>().last_changed();
        app.update();
        assert_eq!(
            app.world()
                .resource_ref::<Assets<PaperMaterial>>()
                .last_changed(),
            materials
        );
        assert_eq!(app.world().resource_ref::<Turn>().last_changed(), turn_tick);
    }
}
