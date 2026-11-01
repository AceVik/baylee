//! A subtree kept across a rebuild, moved under the tree built in its place
//! and drawn on the very frame it moves.
//!
//! Bevy hands every UI node the camera that draws it and that camera's
//! target (`ComputedUiTargetCamera`, `ComputedUiRenderTargetInfo`) by
//! propagation from its root: each node holds an [`Inherited`] copy, and a
//! node without one is not drawn. Moving a subtree under a parent spawned the
//! same frame loses them for one frame: the parent has inherited nothing yet,
//! so the moved node gives its copy up, and the removal runs down its whole
//! subtree in `PostUpdate` while the root's new copy, equal to the old one,
//! stops at the moved node. The next frame puts everything back. The lobby
//! keeps its header across a rebuild exactly so (`lobby::shell::ui`), and
//! rebuilds on nearly every click, so the bar drew only its own panel for a
//! frame, its logo, pills and buttons gone: the owner's "something flickers
//! on every click, especially the top nav bar" (22.10.2026).
//!
//! [`Kept`] carries what the subtree inherited where it stood and gives it to
//! the new parent before attaching, so the move takes nothing away.

use bevy::app::Inherited;
use bevy::prelude::*;
use bevy::ui::{ComputedUiRenderTargetInfo, ComputedUiTargetCamera};

/// What a UI node inherits from its root: the camera that draws it, and
/// that camera's target.
type UiTarget = (
    Inherited<ComputedUiTargetCamera>,
    Inherited<ComputedUiRenderTargetInfo>,
);

/// A subtree kept across a rebuild, and what it inherited where it stood.
#[derive(Clone)]
pub struct Kept {
    entity: Entity,
    target: Option<UiTarget>,
}

impl Kept {
    /// The subtree at `entity`, as its two inherited parts read now. Either
    /// missing (a subtree never drawn yet) keeps nothing, and the move is a
    /// plain one.
    #[must_use]
    pub fn of(
        entity: Entity,
        camera: Option<&Inherited<ComputedUiTargetCamera>>,
        target: Option<&Inherited<ComputedUiRenderTargetInfo>>,
    ) -> Self {
        Self {
            entity,
            target: camera.cloned().zip(target.cloned()),
        }
    }

    /// The kept subtree's root.
    #[must_use]
    pub const fn entity(&self) -> Entity {
        self.entity
    }

    /// Attaches the subtree under `parent`, giving `parent` first what the
    /// subtree inherited, so the subtree keeps its own. The parent's root
    /// hands down the real values in `PostUpdate`; equal, they stop at the
    /// parent, and different (a resize on that frame), they run on down as
    /// on any frame.
    ///
    /// Attach last, once `parent` stands under its root. A parent put under
    /// its own parent after this gives what it was handed up again (Bevy
    /// takes `Inherited` off a node whose new parent has none yet), and that
    /// removal runs down into the subtree in `PostUpdate`: the subtree is
    /// undrawn for the frame, as with no `Kept` at all. The report sheet
    /// attached its footer to the sheet before the sheet went under its
    /// scrim, and its buttons blinked out at every keystroke (feedback of
    /// 28.10.2026).
    pub fn attach(&self, commands: &mut Commands, parent: Entity) {
        if let Some(target) = &self.target {
            commands.entity(parent).insert(target.clone());
        }
        commands.entity(parent).add_child(self.entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ui_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            bevy::text::TextPlugin,
            bevy::ui::UiPlugin,
            bevy::window::WindowPlugin {
                primary_window: None,
                ..default()
            },
            bevy::input::InputPlugin,
            bevy::picking::DefaultPickingPlugins,
        ));
        app.init_asset::<Image>()
            .init_asset::<bevy::image::TextureAtlasLayout>();
        app
    }

    /// A root with a bar under it, and two levels under the bar: the bar and
    /// everything it holds.
    fn tree(app: &mut App) -> (Entity, Entity, Vec<Entity>) {
        let world = app.world_mut();
        let leaf = world.spawn(Node::default()).id();
        let inner = world.spawn(Node::default()).add_child(leaf).id();
        let side = world.spawn(Node::default()).id();
        let bar = world
            .spawn(Node::default())
            .add_children(&[inner, side])
            .id();
        let root = world
            .spawn(Node {
                width: Val::Px(1280.0),
                height: Val::Px(800.0),
                ..default()
            })
            .add_child(bar)
            .id();
        (root, bar, vec![bar, inner, side, leaf])
    }

    /// The nodes of `of` that would not be drawn: either computed part gone.
    fn undrawn(app: &mut App, of: &[Entity]) -> Vec<Entity> {
        of.iter()
            .copied()
            .filter(|&e| {
                let world = app.world();
                world.get::<ComputedUiTargetCamera>(e).is_none()
                    || world.get::<ComputedUiRenderTargetInfo>(e).is_none()
            })
            .collect()
    }

    /// What `lobby::shell::ui` does to a kept header: a new root and a frame
    /// under it, the bar taken off the old root, the old root despawned, the
    /// bar attached to the frame. `carry` is whether the move keeps what the
    /// bar inherited.
    fn rebuild(app: &mut App, old: Entity, bar: Entity, carry: bool) {
        let kept = if carry {
            let world = app.world();
            Kept::of(bar, world.get(bar), world.get(bar))
        } else {
            Kept::of(bar, None, None)
        };
        let world = app.world_mut();
        let mut commands = world.commands();
        let fresh = commands
            .spawn(Node {
                width: Val::Px(1280.0),
                height: Val::Px(800.0),
                ..default()
            })
            .id();
        commands.entity(bar).remove::<ChildOf>();
        commands.entity(old).despawn();
        let frame = commands.spawn(Node::default()).id();
        commands.entity(fresh).add_child(frame);
        kept.attach(&mut commands, frame);
        world.flush();
    }

    /// A rebuild in the report sheet's shape: a new root, a new sheet with a
    /// head and a body of its own, and the kept footer. `attach_last` puts
    /// the sheet under the root before the footer is attached; otherwise the
    /// footer is attached to the sheet first, as the sheet did until the
    /// feedback of 28.10.2026. Returns the new root and the sheet's nodes.
    fn rebuild_sheet(
        app: &mut App,
        old: Entity,
        bar: Entity,
        attach_last: bool,
    ) -> (Entity, [Entity; 3]) {
        let kept = {
            let world = app.world();
            Kept::of(bar, world.get(bar), world.get(bar))
        };
        let world = app.world_mut();
        let mut commands = world.commands();
        let fresh = commands
            .spawn(Node {
                width: Val::Px(1280.0),
                height: Val::Px(800.0),
                ..default()
            })
            .id();
        commands.entity(bar).remove::<ChildOf>();
        commands.entity(old).despawn();
        let sheet = commands.spawn(Node::default()).id();
        let head = commands.spawn(Node::default()).id();
        let body = commands.spawn(Node::default()).id();
        commands.entity(sheet).add_children(&[head, body]);
        if attach_last {
            commands.entity(fresh).add_child(sheet);
            kept.attach(&mut commands, sheet);
        } else {
            kept.attach(&mut commands, sheet);
            commands.entity(fresh).add_child(sheet);
        }
        world.flush();
        (fresh, [sheet, head, body])
    }

    /// The report sheet's footer (feedback of 28.10.2026). Attached to a
    /// sheet that goes under its root only afterwards, the footer and all
    /// it holds are undrawn on each rebuild's frame and back on the next
    /// (Bevy 0.20; when this half fails, the order stopped mattering).
    /// Attached last, nothing is undrawn on any rebuild, the sheet's own new
    /// nodes included.
    #[test]
    fn a_kept_subtree_is_attached_after_its_parent_has_its_place() {
        let mut app = ui_app();
        let (mut root, bar, nodes) = tree(&mut app);
        app.update();
        app.update();
        let (fresh, sheet) = rebuild_sheet(&mut app, root, bar, false);
        root = fresh;
        app.update();
        let mut all = nodes.clone();
        all.extend(sheet);
        let lost = undrawn(&mut app, &all);
        assert_eq!(
            lost, nodes,
            "attached first: the footer's subtree, and only it"
        );
        app.update();
        assert!(undrawn(&mut app, &all).is_empty(), "and back a frame later");

        for round in 0..3 {
            let (fresh, sheet) = rebuild_sheet(&mut app, root, bar, true);
            root = fresh;
            let mut all = nodes.clone();
            all.extend(sheet);
            app.update();
            assert_eq!(
                undrawn(&mut app, &all),
                Vec::<Entity>::new(),
                "attached last, rebuild {round}"
            );
        }
    }

    /// The owner's report of 22.10.2026, at the layer it happens in. A plain
    /// move under a parent built that frame leaves the bar's subtree undrawn
    /// for one frame (Bevy 0.20; when this half fails, Bevy stopped doing it
    /// and [`Kept`] may go); the kept move leaves nothing undrawn, that
    /// frame or the next.
    #[test]
    fn a_kept_subtree_is_drawn_on_the_frame_it_moves() {
        let mut app = ui_app();
        let (root, bar, nodes) = tree(&mut app);
        app.update();
        app.update();
        assert!(
            undrawn(&mut app, &nodes).is_empty(),
            "drawn before the move"
        );

        rebuild(&mut app, root, bar, false);
        app.update();
        let lost = undrawn(&mut app, &nodes);
        assert!(
            !lost.is_empty() && !lost.contains(&bar),
            "a plain move: the bar's panel drawn and what it holds not ({lost:?} of {nodes:?})"
        );
        app.update();
        assert!(
            undrawn(&mut app, &nodes).is_empty(),
            "and back a frame later"
        );

        let mut app = ui_app();
        let (root, bar, nodes) = tree(&mut app);
        app.update();
        app.update();
        rebuild(&mut app, root, bar, true);
        app.update();
        assert_eq!(
            undrawn(&mut app, &nodes),
            Vec::<Entity>::new(),
            "the kept move"
        );
        app.update();
        assert_eq!(
            undrawn(&mut app, &nodes),
            Vec::<Entity>::new(),
            "and after it"
        );

        // Moved again, from the frame it moved to: every rebuild is one.
        let parent = app
            .world()
            .get::<ChildOf>(bar)
            .map(ChildOf::parent)
            .and_then(|frame| app.world().get::<ChildOf>(frame).map(ChildOf::parent))
            .expect("the bar stands in a frame under a root");
        rebuild(&mut app, parent, bar, true);
        app.update();
        assert_eq!(
            undrawn(&mut app, &nodes),
            Vec::<Entity>::new(),
            "the second rebuild"
        );
    }
}
