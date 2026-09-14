//! The hand end of a card being sent.
//!
//! [`baylee_client_core::depart`] is the arithmetic and this is the wiring:
//! one system that notices a card has left the hand, one that flies it away,
//! and a root node of its own to fly it in.
//!
//! # Why the node is kept rather than rebuilt
//!
//! The obvious shape is the one [`crate::touch`] and [`crate::sheen`] use — a
//! resource keyed by `ObjectId` that the hand redraws from. It is wrong here,
//! and for a reason that only applies to a departure: the card is **gone from
//! the board model** by the time this runs, so there is no `HandObject` to
//! build it from, no entry in the view, and nothing to ask for its
//! constructed face. A card drawn from what is left would be the card's art
//! and none of the rest of it, and the one frame where the row's card and the
//! departing one have to be pixel-identical is the first one.
//!
//! So the row's own node is taken out of the hand zone and re-parented here,
//! which is `table::Departing` said in `bevy_ui`: the entity keeps its
//! material, its halo, its corner radius and its constructed face, and loses
//! the two components that made it a card in a hand. It cannot be left where
//! it is, because the hand zone clips its children — the strip sits
//! `LEDGE_H + HAND_HEADROOM` down from the zone's top edge and the zone ends
//! at the window's bottom — and a departure that stayed inside that clip
//! would be a card sliding under an invisible line.
//! stayed inside that clip would be a card sliding under an invisible line.
//!
//! # Z-order, and the one thing it costs
//!
//! The ghost is its own UI root, a sibling of `HudRoot` the way the seat bars
//! are, which puts it over the whole interface — including the prompt bar it
//! rises past. The alternative is under the whole interface, and that is
//! worse in a way a player would see every single time: the hand zone paints a
//! veil over its own bottom edge, so a ghost behind it would darken on the
//! frame it is supposed to be indistinguishable from the card it just was.
//! A quarter of a second of a shrinking card over a panel is the cheaper of
//! the two.

use baylee_client_core::depart::{Flight, toward};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};

/// Root of every departing card. A sibling of `HudRoot`, not a child.
#[derive(Component)]
pub struct DepartRoot;

/// A card that has left the hand and is playing its way out of it.
///
/// It has lost `HandRowCard` and `HandCardVisual` by the time it wears this,
/// so nothing in the hand looks it up any more: [`crate::touch::settle`] does
/// not move it, the pointer does not find it, and the preview cannot open on
/// it. All that is left is the flight and the clock inside it.
#[derive(Component)]
pub struct Leaving {
    /// Where it is going and how long it has to get there.
    pub flight: Flight,
}

/// Notices that a card has left the hand, and takes its node with it.
///
/// Runs **before** the overlay is rebuilt, which is the whole of its
/// ordering: the row standing at this moment was built from the previous
/// board and the card is already out of this one, so the difference between
/// the two is the departure. The re-parenting is queued before the rebuild's
/// despawn for the same reason, and commands are applied in the order the
/// systems ran — so the card is out of the hand zone's tree before the tree is
/// taken down.
///
/// The trigger is the **view** and never the click. Casting is two-stage and
/// a mana run taps lands for several frames before the spell goes anywhere,
/// so a departure minted on the tap would fly away for a spell the player can
/// still take back.
pub fn send_off(
    mut commands: Commands,
    duel: Res<crate::Duel>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    windows: Query<&Window>,
    roots: Query<Entity, With<DepartRoot>>,
    mut row: Query<
        (
            Entity,
            &crate::hud::HandCardVisual,
            &mut Node,
            &ComputedNode,
            &UiGlobalTransform,
        ),
        With<crate::hud::HandRowCard>,
    >,
) {
    // A player who has turned motion off is shown nothing: the card is gone
    // from the board model, so the rebuild that follows simply does not draw
    // it, which is exactly what that setting asks for.
    if prefs.is_some_and(|p| p.all().reduce_motion) {
        return;
    }
    // No board at all is a duel being torn down or not yet started, and every
    // card in the row would read as having left. The teardown despawns them.
    let (Some(board), Ok(window)) = (duel.board.as_ref(), windows.single()) else {
        return;
    };
    let size = Vec2::new(window.width(), window.height());
    let mut root = roots.iter().next();
    for (entity, card, mut node, computed, place) in &mut row {
        if board.hand.iter().any(|held| held.id == card.object) {
            continue;
        }
        // Where the row was drawing it, read off the layout rather than
        // recomputed from the bar's own padding and insets. The node is about
        // to be measured against the window instead of against the strip, and
        // this is the one reading that cannot disagree with what the player
        // is looking at — it already carries the scroll offset, the centring
        // lead and whatever `touch` had the card doing at the time.
        let at = place.translation * computed.inverse_scale_factor;
        let root = *root.get_or_insert_with(|| spawn_root(&mut commands));
        node.left = Val::Px(at.x - crate::hud::HAND_CARD_W / 2.0);
        node.top = Val::Px(at.y - crate::hud::HAND_CARD_H / 2.0);
        commands
            .entity(entity)
            .remove::<(crate::hud::HandRowCard, crate::hud::HandCardVisual)>()
            .insert((
                Leaving {
                    flight: Flight::new(at, toward(at, size)),
                },
                UiTransform::IDENTITY,
            ))
            // Every node of it and not only the root: the art and the face
            // are children, and a child answers the pointer on its own
            // behalf — which for a quarter of a second of a card hanging over
            // the player's own mat is a click eaten and a hover killed.
            .insert_recursive::<Children>(Pickable::IGNORE);
        commands.entity(root).add_child(entity);
    }
}

/// The root every departing card is flown in.
fn spawn_root(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            DepartRoot,
            crate::table::DuelStage,
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            Pickable::IGNORE,
            // Over `HudRoot` and over the seat bars, both of which sit at
            // zero. See the header for what that costs and why the other side
            // of the trade is worse.
            ZIndex(1),
        ))
        .id()
}

/// Flies every departing card, and takes it off the screen when it is gone.
///
/// A countdown rather than a settled test, for `table::retire`'s reason: this
/// exit ends at a fiftieth of a card, and "has it arrived" is the wrong
/// question about a card whose destination is nowhere.
pub fn fly(
    time: Res<Time>,
    mut commands: Commands,
    mut flights: Query<(Entity, &mut Leaving, &mut Node, &mut UiTransform)>,
) {
    for (entity, mut leaving, mut node, mut transform) in &mut flights {
        leaving.flight.advance(time.delta_secs());
        if leaving.flight.done() {
            commands.entity(entity).despawn();
            continue;
        }
        let pose = leaving.flight.pose();
        node.left = Val::Px(pose.at.x - crate::hud::HAND_CARD_W / 2.0);
        node.top = Val::Px(pose.at.y - crate::hud::HAND_CARD_H / 2.0);
        // About the node's own centre, which is what `bevy_ui` scales a node
        // about — so the drawn middle stays on the point the flight computed
        // instead of drifting towards the top-left corner as the card
        // shrinks.
        transform.scale = Vec2::splat(pose.scale);
    }
}

/// The systems, *run*.
///
/// A departure declared and never wired is the bug this whole module is
/// fixing, so every assertion here is on an entity that exists, a component
/// it wears or a `Node` that moved — never on `update()` having returned.
#[cfg(test)]
mod running {
    use super::*;
    use baylee_client_core::board::Openings;
    use baylee_client_core::test_support::ViewBuilder;
    use baylee_client_core::{BoardModel, depart as arithmetic};
    use baylee_core::ids::ObjectId;
    use bevy::window::PrimaryWindow;

    /// Where the row is drawing a card, in *physical* pixels — which is what
    /// `UiGlobalTransform` carries, and the reason `inverse_scale_factor` is
    /// read beside it rather than assumed to be one.
    const SCALE: f32 = 2.0;

    fn id(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    /// A board whose local hand holds `slots`, and nothing else.
    fn hand_of(slots: &[u32]) -> BoardModel {
        let view = ViewBuilder::new(2)
            .with_hand(
                slots
                    .iter()
                    .map(|slot| ("Grizzly Bears", 2, *slot))
                    .collect(),
            )
            .build();
        BoardModel::from_view(
            &view,
            Openings::none(),
            |_| 12.0,
            crate::cardart::registry(),
        )
    }

    /// An app with the two systems in it and a hand of `slots` on the board.
    fn harness(slots: &[u32]) -> App {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<crate::Duel>()
            .add_systems(Update, (send_off, fly).chain());
        let mut window = Window::default();
        window.resolution.set(1728.0, 1052.0);
        app.world_mut().spawn((window, PrimaryWindow));
        app.world_mut().resource_mut::<crate::Duel>().board = Some(hand_of(slots));
        app
    }

    /// One node in the hand row, with the layout's answer already on it.
    ///
    /// Both components, because that is what `spawn_hand_zone` puts on a row
    /// card, and a `ComputedNode`/`UiGlobalTransform` pair because a headless
    /// app runs no `bevy_ui` layout and [`send_off`] reads the row's place off
    /// exactly those.
    fn hand_card(app: &mut App, object: ObjectId, centre: Vec2) -> Entity {
        app.world_mut()
            .spawn((
                crate::hud::HandCardVisual { object },
                crate::hud::HandRowCard,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(crate::hud::HAND_CARD_W),
                    height: Val::Px(crate::hud::HAND_CARD_H),
                    ..default()
                },
                ComputedNode {
                    inverse_scale_factor: 1.0 / SCALE,
                    ..default()
                },
                UiGlobalTransform::from(bevy::math::Affine2::from_translation(centre * SCALE)),
            ))
            .id()
    }

    /// A child of the card, the way the art and the shade pane are.
    fn art_of(app: &mut App, card: Entity) -> Entity {
        let art = app.world_mut().spawn(Node::default()).id();
        app.world_mut().entity_mut(card).add_children(&[art]);
        art
    }

    fn tick(app: &mut App, seconds: f32) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(seconds));
        app.update();
    }

    fn node_of(app: &mut App, entity: Entity) -> Node {
        app.world().entity(entity).get::<Node>().cloned().unwrap()
    }

    /// The whole claim: a card that has left the hand is still on the screen,
    /// out of the row, and on its way somewhere.
    #[test]
    fn a_card_that_leaves_the_hand_keeps_its_node() {
        let mut app = harness(&[1, 2]);
        let staying = hand_card(&mut app, id(1), Vec2::new(700.0, 950.0));
        let leaving = hand_card(&mut app, id(2), Vec2::new(820.0, 950.0));
        app.world_mut().resource_mut::<crate::Duel>().board = Some(hand_of(&[1]));
        tick(&mut app, 1.0 / 60.0);

        let world = app.world();
        assert!(
            world.get_entity(leaving).is_ok(),
            "the card that left the hand was despawned with the row"
        );
        assert!(world.entity(leaving).contains::<Leaving>());
        assert!(
            !world.entity(leaving).contains::<crate::hud::HandRowCard>(),
            "a departing card is still being drawn as part of the hand row"
        );
        assert!(
            !world
                .entity(leaving)
                .contains::<crate::hud::HandCardVisual>()
        );
        assert!(
            world.entity(staying).contains::<crate::hud::HandRowCard>(),
            "a card still in the hand was sent away"
        );
        assert!(!world.entity(staying).contains::<Leaving>());
    }

    /// The frame the player must not be able to see: the first one. A
    /// departing card stands exactly where the row had it, so the hand row
    /// closing and the card leaving are two things and not a flash.
    #[test]
    fn a_departing_card_starts_where_the_row_had_it() {
        let centre = Vec2::new(820.0, 950.0);
        let mut app = harness(&[1]);
        let card = hand_card(&mut app, id(1), centre);
        app.world_mut().resource_mut::<crate::Duel>().board = Some(hand_of(&[]));
        // No time at all: the pose written on the frame the card left.
        tick(&mut app, 0.0);

        let node = node_of(&mut app, card);
        assert_eq!(
            node.left,
            Val::Px(centre.x - crate::hud::HAND_CARD_W / 2.0),
            "a departing card jumped sideways on the frame it left"
        );
        assert_eq!(node.top, Val::Px(centre.y - crate::hud::HAND_CARD_H / 2.0));
        let scale = app.world().entity(card).get::<UiTransform>().unwrap().scale;
        assert_eq!(
            scale,
            Vec2::ONE,
            "a departing card started smaller than it was"
        );
    }

    /// It is taken out of the hand zone, which clips its children to a box
    /// that ends at the window's own bottom edge — so a card that stayed a
    /// child of the row would leave under an invisible line.
    #[test]
    fn a_departing_card_is_flown_in_a_root_of_its_own() {
        let mut app = harness(&[1]);
        let card = hand_card(&mut app, id(1), Vec2::new(820.0, 950.0));
        app.world_mut().resource_mut::<crate::Duel>().board = Some(hand_of(&[]));
        tick(&mut app, 1.0 / 60.0);

        let parent = app
            .world()
            .entity(card)
            .get::<ChildOf>()
            .expect("a departing card has a parent")
            .parent();
        assert!(
            app.world().entity(parent).contains::<DepartRoot>(),
            "a departing card is parented to something that is not the departure root"
        );
        assert!(
            app.world()
                .entity(parent)
                .contains::<crate::table::DuelStage>(),
            "the departure root would outlive the duel it belongs to"
        );
    }

    /// It hangs over the player's own mat for a quarter of a second, and a
    /// node in front of the table is a node the pointer reports instead of
    /// it — the mistake a button's own label made once, and this one has
    /// children.
    #[test]
    fn a_departing_card_answers_no_pointer_at_all() {
        let mut app = harness(&[1]);
        let card = hand_card(&mut app, id(1), Vec2::new(820.0, 950.0));
        let art = art_of(&mut app, card);
        app.world_mut().resource_mut::<crate::Duel>().board = Some(hand_of(&[]));
        tick(&mut app, 1.0 / 60.0);

        for (entity, what) in [(card, "the card"), (art, "its art")] {
            assert_eq!(
                app.world().entity(entity).get::<Pickable>().copied(),
                Some(Pickable::IGNORE),
                "{what} still answers the pointer while it is leaving"
            );
        }
    }

    /// It moves, and then it is gone. Both halves, because a flight that
    /// never advanced and a flight that never ended look the same for the
    /// first frame.
    #[test]
    fn a_departing_card_travels_and_is_then_taken_off_the_screen() {
        let centre = Vec2::new(820.0, 950.0);
        let mut app = harness(&[1]);
        let card = hand_card(&mut app, id(1), centre);
        app.world_mut().resource_mut::<crate::Duel>().board = Some(hand_of(&[]));
        tick(&mut app, 1.0 / 60.0);
        let started = node_of(&mut app, card);

        tick(&mut app, arithmetic::LIFE / 3.0);
        let moved = node_of(&mut app, card);
        assert_ne!(moved.top, started.top, "a departing card never moved");
        let Val::Px(top) = moved.top else {
            unreachable!("the pose is written in pixels")
        };
        let Val::Px(was) = started.top else {
            unreachable!("the pose is written in pixels")
        };
        assert!(top < was, "a departing card went downwards");
        let scale = app.world().entity(card).get::<UiTransform>().unwrap().scale;
        assert!(scale.x < 1.0 && scale.x > arithmetic::VANISH);

        tick(&mut app, arithmetic::LIFE);
        assert!(
            app.world().get_entity(card).is_err(),
            "a departing card was left on the screen after its flight"
        );
    }

    /// A player who has turned motion off is shown nothing at all: the card
    /// is out of the board model, so the rebuild that follows simply stops
    /// drawing it.
    #[test]
    fn motion_turned_off_leaves_the_card_where_it_was() {
        let mut app = harness(&[1]);
        let mut prefs = crate::prefs::Prefs::default();
        prefs.edit().reduce_motion = true;
        app.insert_resource(prefs);
        let card = hand_card(&mut app, id(1), Vec2::new(820.0, 950.0));
        app.world_mut().resource_mut::<crate::Duel>().board = Some(hand_of(&[]));
        tick(&mut app, 1.0 / 60.0);

        assert!(!app.world().entity(card).contains::<Leaving>());
        assert!(
            app.world()
                .entity(card)
                .contains::<crate::hud::HandRowCard>(),
            "a card was taken out of the row for a player who asked for no motion"
        );
        let mut roots = app.world_mut().query_filtered::<Entity, With<DepartRoot>>();
        assert!(
            roots.iter(app.world()).next().is_none(),
            "a departure root was built for a player who sees no departures"
        );
    }

    /// A duel with no board at all is one being torn down or not yet
    /// started, and every card in the row would otherwise read as having
    /// left it.
    #[test]
    fn a_table_with_no_board_sends_nothing_away() {
        let mut app = harness(&[1]);
        let card = hand_card(&mut app, id(1), Vec2::new(820.0, 950.0));
        app.world_mut().resource_mut::<crate::Duel>().board = None;
        tick(&mut app, 1.0 / 60.0);

        assert!(!app.world().entity(card).contains::<Leaving>());
        assert!(
            app.world()
                .entity(card)
                .contains::<crate::hud::HandRowCard>()
        );
    }
}
