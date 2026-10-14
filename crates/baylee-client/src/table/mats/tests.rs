//! Snapshot-derived mana totals joined to the real felt frame system.

use super::*;
use baylee_client_core::test_support::{ViewBuilder, printed, token};
use baylee_core::mana::ManaColor;
use baylee_view::{BoardMana, PlayerView, PublicObject};

mod animation;
mod snapshots;

fn card(slot: u32, controller: u8, name: &str) -> PublicObject {
    let index = baylee_cards::decks::by_name(name).expect("fixture card in registry");
    let def = baylee_cards::by_index(index).expect("fixture definition");
    let mut object = printed(slot, controller, name, 0);
    object.card = Some(baylee_view::CardIdentity {
        index,
        print: baylee_core::ids::PrintRef::new(0),
        face: 0,
    });
    object.rules = object.card.map(baylee_view::RulesFace::from);
    object.subtypes = SubtypeSet::from_slice(def.faces[0].subtypes);
    object.types = def.faces[0].types;
    object
}

fn tower(color: ManaColor) -> PlayerView {
    let mut object = card(1, 0, "Command Tower");
    object.board_mana = Some(BoardMana {
        index: 0,
        colors: vec![color],
    });
    ViewBuilder::new(2).with_battlefield(0, [object]).build()
}

fn received(view: PlayerView) -> Duel {
    let mut duel = Duel::default();
    duel.receive_view(view);
    crate::rebuild_board(&mut duel);
    duel
}

fn reads() -> usize {
    crate::manasources::TABLE_MANA_READS.with(std::cell::Cell::get)
}

fn assert_tally(duel: &Duel, expected: [f32; 5]) {
    assert_eq!(
        duel.table_mana.map(f32::to_bits),
        expected.map(f32::to_bits)
    );
    let reference = duel
        .view
        .as_ref()
        .map_or([0.0; 5], crate::manasources::table_mana);
    assert_eq!(
        duel.table_mana.map(f32::to_bits),
        reference.map(f32::to_bits)
    );
}

struct Table {
    app: App,
    slab: Entity,
    material: Handle<FeltMaterial>,
}

impl Table {
    fn new(view: PlayerView, reduce_motion: bool) -> Self {
        let duel = received(view);
        let layout = duel.settled_layout().expect("received layout");
        let (min, max) = layout.extent().expect("two seats");
        let span = (min.abs().max(max.abs()) + Vec2::splat(SLAB_MARGIN)) * 2.0;
        let (_, tail) = firewheel_of(&duel, duel.board.as_ref().unwrap(), layout);
        let mut prefs = crate::prefs::Prefs::default();
        prefs.edit().reduce_motion = reduce_motion;
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<FeltMaterial>>()
            .insert_resource(prefs)
            .insert_resource(duel)
            .add_systems(Update, sync_table);
        // The production main schedule and the thread-local read probe both
        // run on this thread. No renderer or background baking is installed.
        app.edit_schedule(Update, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(slab_mesh(span));
        let material = app
            .world_mut()
            .resource_mut::<Assets<FeltMaterial>>()
            .add(FeltMaterial {
                params: crate::feltmat::FeltParams::default(),
                veins: Handle::default(),
                warp: Handle::default(),
            });
        // Start with a slab already cut to the layout: these tests exercise
        // lighting, not the independent asynchronous warp bake at creation.
        let slab = app
            .world_mut()
            .spawn((
                Slab {
                    cut: span,
                    shown: Vec4::ZERO,
                    source: Vec4::ZERO,
                    flames: Vec4::ZERO,
                    tail: Vec4::new(0.0, tail.y, tail.z, 0.0),
                    motion: crate::cardmat::STILL,
                },
                Mesh3d(mesh),
                MeshMaterial3d(material.clone()),
            ))
            .id();
        Self {
            app,
            slab,
            material,
        }
    }

    fn step(&mut self, seconds: f32) {
        self.app
            .world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(seconds));
        self.app.update();
    }

    fn shown(&self) -> [f32; 5] {
        let slab = self.app.world().get::<Slab>(self.slab).unwrap();
        let params = &self
            .app
            .world()
            .resource::<Assets<FeltMaterial>>()
            .get(&self.material)
            .unwrap()
            .params;
        assert_eq!(slab.flames, params.flames);
        assert_eq!(slab.tail, params.flames_tail);
        [
            slab.flames.x,
            slab.flames.y,
            slab.flames.z,
            slab.flames.w,
            slab.tail.x,
        ]
    }
}
